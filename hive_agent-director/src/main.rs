use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::fs;
use std::env;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use chrono::{Local, Utc};

const SERVICE_NAME: &str = "hive_agent-director";
const DEFAULT_PORT: u16 = 6084;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CameraConfig {
    url: String,
    monitoring_interval_seconds: u64,
    session_interval_seconds: u64,
    session_timeout_minutes: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TriggerConfig {
    enabled: bool,
    prompt: String,
    positive_keywords: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct VisualTriggerDetectionConfig {
    endpoint: String,
    enabled: bool,
    timeout_ms: u64,
    max_tokens: u32,
    active_trigger: String,
    triggers: HashMap<String, TriggerConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct SceneAnalysisConfig {
    enabled: bool,
    prompt: String,
    timeout_ms: u64,
    max_tokens: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ResponseGenerationConfig {
    endpoint: String,
    enabled: bool,
    prompt: String,
    timeout_ms: u64,
    max_tokens: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Config {
    camera: CameraConfig,
    visual_trigger_detection: VisualTriggerDetectionConfig,
    scene_analysis: SceneAnalysisConfig,
    response_generation: ResponseGenerationConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            camera: CameraConfig {
                url: "http://localhost:6082".to_string(),
                monitoring_interval_seconds: 5,
                session_interval_seconds: 30,
                session_timeout_minutes: 60,
            },
            visual_trigger_detection: VisualTriggerDetectionConfig {
                endpoint: "http://localhost:11434/api/generate".to_string(),
                enabled: false,
                timeout_ms: 30000,
                max_tokens: 10,
                active_trigger: "person_detection".to_string(),
                triggers: {
                    let mut map = HashMap::new();
                    map.insert("person_detection".to_string(), TriggerConfig {
                        enabled: true,
                        prompt: "Is there a person in this image? Answer only 'true' or 'false'.".to_string(),
                        positive_keywords: vec!["true".to_string(), "yes".to_string(), "person".to_string()],
                    });
                    map
                },
            },
            scene_analysis: SceneAnalysisConfig {
                enabled: true,
                prompt: "Describe what you see in this image in detail.".to_string(),
                timeout_ms: 30000,
                max_tokens: 500,
            },
            response_generation: ResponseGenerationConfig {
                endpoint: "http://localhost:5080/gim/llm_mid/ask_question".to_string(),
                enabled: true,
                prompt: "Generate a friendly greeting based on the scene.".to_string(),
                timeout_ms: 30000,
                max_tokens: 200,
            },
        }
    }
}

struct AppState {
    session_active: Arc<Mutex<bool>>,
    session_dir: Arc<Mutex<Option<String>>>,
    session_start_time: Arc<Mutex<Option<chrono::DateTime<Utc>>>>,
}

fn get_service_port() -> u16 {
    env::args()
        .collect::<Vec<String>>()
        .windows(2)
        .find(|w| w[0] == "--port")
        .and_then(|w| w[1].parse().ok())
        .or_else(|| env::var("WARDEN_ASSIGNED_PORT").ok()?.parse().ok())
        .or_else(|| env::var("SERVICE_PORT").ok()?.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

async fn check_for_trigger(image_path: &str, config: &VisualTriggerDetectionConfig) -> (bool, String) {
    // Get the active trigger configuration
    let trigger = match config.triggers.get(&config.active_trigger) {
        Some(t) if t.enabled => t,
        _ => return (false, String::new()),
    };
    
    // Read image bytes
    let image_bytes = match fs::read(image_path) {
        Ok(data) => data,
        Err(_) => return (false, String::new()),
    };
    
    let client = reqwest::Client::new();
    
    // HiveMind Vision LLM format
    let request_body = serde_json::json!({
        "timeout": config.timeout_ms,
        "question": [&trigger.prompt],
        "image_buffer": [image_bytes],
        "output_max_token_count": config.max_tokens
    });
    
    match client.post(&config.endpoint)
        .json(&request_body)
        .timeout(Duration::from_millis(config.timeout_ms))
        .send()
        .await
    {
        Ok(response) => {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                // HiveMind returns {"ok": true, "result": ["response text"]}
                if json["ok"].as_bool().unwrap_or(false) {
                    if let Some(result) = json["result"].as_array() {
                        if let Some(text) = result.first().and_then(|v| v.as_str()) {
                            let text_lower = text.to_lowercase();
                            // Check if any positive keyword is present
                            for keyword in &trigger.positive_keywords {
                                if text_lower.contains(&keyword.to_lowercase()) {
                                    return (true, config.active_trigger.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Vision trigger detection error: {}", e);
        }
    }
    
    (false, String::new())
}

async fn analyze_image(image_path: &str, vision_url: &str, config: &SceneAnalysisConfig) -> Option<String> {
    let image_bytes = match fs::read(image_path) {
        Ok(data) => data,
        Err(_) => return None,
    };
    
    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "timeout": config.timeout_ms,
        "question": [&config.prompt],
        "image_buffer": [image_bytes],
        "output_max_token_count": config.max_tokens
    });
    
    match client.post(vision_url)
        .json(&request_body)
        .timeout(Duration::from_millis(config.timeout_ms))
        .send()
        .await
    {
        Ok(response) => {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                if json["ok"].as_bool().unwrap_or(false) {
                    if let Some(result) = json["result"].as_array() {
                        if let Some(text) = result.first().and_then(|v| v.as_str()) {
                            return Some(text.to_string());
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Vision analysis error: {}", e);
        }
    }
    None
}

async fn generate_text(context: &str, config: &ResponseGenerationConfig) -> Option<String> {
    let client = reqwest::Client::new();
    let full_prompt = format!("{}\n\nContext: {}", config.prompt, context);
    
    let request_body = serde_json::json!({
        "timeout": config.timeout_ms,
        "question": [full_prompt],
        "output_max_token_count": config.max_tokens
    });
    
    match client.post(&config.endpoint)
        .json(&request_body)
        .timeout(Duration::from_millis(config.timeout_ms))
        .send()
        .await
    {
        Ok(response) => {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                if json["ok"].as_bool().unwrap_or(false) {
                    if let Some(result) = json["result"].as_array() {
                        if let Some(text) = result.first().and_then(|v| v.as_str()) {
                            return Some(text.to_string());
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Text generation error: {}", e);
        }
    }
    None
}

async fn monitoring_loop(state: Arc<Mutex<AppState>>, config: Config) {
    println!("🎬 Starting monitoring loop...");
    
    let client = reqwest::Client::new();
    fs::create_dir_all("generated_image_captures/sessions").ok();
    
    loop {
        // Check if we're in a session
        let (session_active, session_dir, session_start) = {
            let app_state = state.lock().unwrap();
            let active = *app_state.session_active.lock().unwrap();
            let dir = app_state.session_dir.lock().unwrap().clone();
            let start = *app_state.session_start_time.lock().unwrap();
            (active, dir, start)
        };
        
        // Check session timeout
        if session_active {
            if let Some(start_time) = session_start {
                let elapsed = Utc::now() - start_time;
                if elapsed.num_minutes() > config.camera.session_timeout_minutes as i64 {
                    let app_state = state.lock().unwrap();
                    *app_state.session_active.lock().unwrap() = false;
                    *app_state.session_dir.lock().unwrap() = None;
                    *app_state.session_start_time.lock().unwrap() = None;
                    println!("⏱️ Session timeout reached, returning to monitoring");
                    continue;
                }
            }
        }
        
        // Capture image
        let capture_url = format!("{}/capture-image", config.camera.url);
        if let Ok(response) = client.get(&capture_url).send().await {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                if json["ok"].as_bool().unwrap_or(false) {
                    let filename = json["filename"].as_str().unwrap_or("");
                    let source_image = format!("generated_image_captures/{}", 
                        filename.split('/').last().unwrap_or(""));
                    
                    if session_active {
                        // In session - save to session folder
                        if let Some(ref dir) = session_dir {
                            let timestamp = Local::now().format("%H%M%S").to_string();
                            let session_image = format!("{}/capture_{}.png", dir, timestamp);
                            fs::copy(&source_image, &session_image).ok();
                            println!("📸 Session capture saved: {}", session_image);
                        }
                    } else {
                        // Not in session - check for triggers
                        if config.visual_trigger_detection.enabled {
                            let (triggered, trigger_type) = check_for_trigger(&source_image, &config.visual_trigger_detection).await;
                            if triggered {
                                // Person detected! Create session
                                let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
                                let session_dir = format!("generated_image_captures/sessions/session_{}", timestamp);
                                fs::create_dir_all(&session_dir).ok();
                                
                                // Save trigger image (both as trigger.png and as first capture)
                                let trigger_image = format!("{}/trigger.png", session_dir);
                                fs::copy(&source_image, &trigger_image).ok();
                                
                                // Also save as first timestamped capture in sequence
                                let first_capture_name = format!("{}/capture_{}.png", session_dir, 
                                    Local::now().format("%H%M%S"));
                                fs::copy(&source_image, &first_capture_name).ok();
                                println!("📸 Trigger image saved as both trigger.png and {}", 
                                    first_capture_name.split('/').last().unwrap_or("capture"));
                                
                                // Analyze image and generate response if configured
                                if config.scene_analysis.enabled {
                                    println!("🔍 Analyzing the scene...");
                                    if let Some(analysis) = analyze_image(&trigger_image, &config.visual_trigger_detection.endpoint, &config.scene_analysis).await {
                                        println!("📝 Scene analysis complete");
                                        
                                        // Save analysis to file
                                        let analysis_file = format!("{}/analysis.txt", session_dir);
                                        fs::write(&analysis_file, &analysis).ok();
                                        
                                        // Generate speech/text response if configured
                                        if config.response_generation.enabled {
                                            println!("💬 Generating response...");
                                            if let Some(generated_text) = generate_text(&analysis, &config.response_generation).await {
                                                println!("🗣️ Response: {}", generated_text);
                                                
                                                // Save generated text to file
                                                let speech_file = format!("{}/generated_speech.txt", session_dir);
                                                fs::write(&speech_file, &generated_text).ok();
                                                
                                                // Save combined session info
                                                let session_info = serde_json::json!({
                                                    "timestamp": Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                                                    "trigger_type": trigger_type.clone(),
                                                    "trigger_image": "trigger.png",
                                                    "analysis": analysis,
                                                    "generated_speech": generated_text,
                                                    "scene_analysis_config": config.scene_analysis,
                                                    "response_generation_config": config.response_generation
                                                });
                                                let info_file = format!("{}/session_info.json", session_dir);
                                                fs::write(&info_file, serde_json::to_string_pretty(&session_info).unwrap_or_default()).ok();
                                            }
                                        }
                                    }
                                }
                                
                                // Start session
                                let app_state = state.lock().unwrap();
                                *app_state.session_active.lock().unwrap() = true;
                                *app_state.session_dir.lock().unwrap() = Some(session_dir.clone());
                                *app_state.session_start_time.lock().unwrap() = Some(Utc::now());
                                
                                println!("🚨 TRIGGER DETECTED: {}! Session started: {}", 
                                    trigger_type.replace("_", " ").to_uppercase(), session_dir);
                            }
                        }
                    }
                }
            }
        }
        
        // Wait for next interval
        let interval = if session_active { config.camera.session_interval_seconds } else { config.camera.monitoring_interval_seconds };
        sleep(Duration::from_secs(interval)).await;
    }
}

async fn healthcheck() -> impl Responder {
    HttpResponse::Ok().content_type("text/plain").body("true")
}

async fn status(data: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let app_state = data.lock().unwrap();
    let session_active = *app_state.session_active.lock().unwrap();
    let session_dir = app_state.session_dir.lock().unwrap().clone();
    
    HttpResponse::Ok().json(serde_json::json!({
        "service": SERVICE_NAME,
        "status": "operational",
        "session_active": session_active,
        "session_directory": session_dir,
    }))
}

async fn end_session(data: web::Data<Arc<Mutex<AppState>>>) -> impl Responder {
    let app_state = data.lock().unwrap();
    *app_state.session_active.lock().unwrap() = false;
    *app_state.session_dir.lock().unwrap() = None;
    *app_state.session_start_time.lock().unwrap() = None;
    
    println!("📍 Session ended, returning to monitoring");
    
    HttpResponse::Ok().json(serde_json::json!({
        "message": "Session ended"
    }))
}

#[actix_web::main]
async fn main() -> Result<()> {
    let port = get_service_port();
    println!("🤖 Starting {} on port {}", SERVICE_NAME, port);
    
    // Load config
    let config_path = "director_config.json";
    let config: Config = fs::read_to_string(config_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    
    // Save default config if it doesn't exist
    if !std::path::Path::new(config_path).exists() {
        fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
        println!("📝 Created default config file");
    }
    
    println!("📷 Camera: {}", config.camera.url);
    println!("🧠 Visual Triggers: {} (Active: {})", 
        if config.visual_trigger_detection.enabled { "Enabled" } else { "Disabled" },
        config.visual_trigger_detection.active_trigger);
    
    // List available triggers
    if config.visual_trigger_detection.enabled {
        let enabled_triggers: Vec<String> = config.visual_trigger_detection.triggers
            .iter()
            .filter(|(_, t)| t.enabled)
            .map(|(name, _)| name.clone())
            .collect();
        println!("   Available triggers: {:?}", enabled_triggers);
    }
    println!("🔍 Scene Analysis: {}", if config.scene_analysis.enabled { "Enabled" } else { "Disabled" });
    println!("💬 Response Generation: {}", if config.response_generation.enabled { "Enabled" } else { "Disabled" });
    println!("⏱️ Monitoring interval: {}s", config.camera.monitoring_interval_seconds);
    println!("⏱️ Session interval: {}s", config.camera.session_interval_seconds);
    println!("⏱️ Session timeout: {} minutes", config.camera.session_timeout_minutes);
    
    // Initialize state
    let app_state = Arc::new(Mutex::new(AppState {
        session_active: Arc::new(Mutex::new(false)),
        session_dir: Arc::new(Mutex::new(None)),
        session_start_time: Arc::new(Mutex::new(None)),
    }));
    
    // Start monitoring loop
    let loop_state = app_state.clone();
    let loop_config = config.clone();
    tokio::spawn(async move {
        monitoring_loop(loop_state, loop_config).await;
    });
    
    // Start web server
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST"]);

        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(cors)
            .route("/api/v1/hive_agent-director/healthcheck/basic", web::get().to(healthcheck))
            .route("/api/v1/hive_agent-director/status", web::get().to(status))
            .route("/api/v1/hive_agent-director/session/end", web::post().to(end_session))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await?;

    Ok(())
}