use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::fs;
use std::env;
use tokio::time::{sleep, Duration};
use chrono::{Local, Utc};

const SERVICE_NAME: &str = "hive_agent-director";
const DEFAULT_PORT: u16 = 6084;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Config {
    camera_url: String,
    vision_llm_url: String,
    vision_llm_enabled: bool,
    monitoring_interval: u64,
    session_interval: u64,
    session_timeout_minutes: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            camera_url: "http://localhost:6082".to_string(),
            vision_llm_url: "http://localhost:11434/api/generate".to_string(),
            vision_llm_enabled: false,
            monitoring_interval: 5,
            session_interval: 30,
            session_timeout_minutes: 60,
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

async fn check_for_person(image_path: &str, vision_url: &str) -> bool {
    // Read image bytes
    let image_bytes = match fs::read(image_path) {
        Ok(data) => data,
        Err(_) => return false,
    };
    
    let client = reqwest::Client::new();
    
    // HiveMind Vision LLM format
    let request_body = serde_json::json!({
        "timeout": 30000,
        "question": ["Is there a person in this image? Answer only 'true' or 'false'."],
        "image_buffer": [image_bytes],
        "output_max_token_count": 10
    });
    
    match client.post(vision_url)
        .json(&request_body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
    {
        Ok(response) => {
            if let Ok(json) = response.json::<serde_json::Value>().await {
                // HiveMind returns {"ok": true, "result": ["response text"]}
                if json["ok"].as_bool().unwrap_or(false) {
                    if let Some(result) = json["result"].as_array() {
                        if let Some(text) = result.first().and_then(|v| v.as_str()) {
                            return text.to_lowercase().contains("true");
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Vision LLM error: {}", e);
        }
    }
    
    false
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
                if elapsed.num_minutes() > config.session_timeout_minutes as i64 {
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
        let capture_url = format!("{}/capture-image", config.camera_url);
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
                        // Not in session - check for person
                        if config.vision_llm_enabled {
                            if check_for_person(&source_image, &config.vision_llm_url).await {
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
                                
                                // Start session
                                let app_state = state.lock().unwrap();
                                *app_state.session_active.lock().unwrap() = true;
                                *app_state.session_dir.lock().unwrap() = Some(session_dir.clone());
                                *app_state.session_start_time.lock().unwrap() = Some(Utc::now());
                                
                                println!("🚨 PERSON DETECTED! Session started: {}", session_dir);
                            }
                        }
                    }
                }
            }
        }
        
        // Wait for next interval
        let interval = if session_active { config.session_interval } else { config.monitoring_interval };
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
    
    println!("📷 Camera: {}", config.camera_url);
    println!("🧠 Vision LLM: {}", if config.vision_llm_enabled { "Enabled" } else { "Disabled" });
    println!("⏱️ Monitoring interval: {}s", config.monitoring_interval);
    println!("⏱️ Session interval: {}s", config.session_interval);
    println!("⏱️ Session timeout: {} minutes", config.session_timeout_minutes);
    
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