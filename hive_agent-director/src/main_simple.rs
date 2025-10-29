use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::env;
use tokio::time::{sleep, Duration};

const SERVICE_NAME: &str = "hive_agent-director";
const DEFAULT_PORT: u16 = 6084;

/// Simple config for the capture loop
#[derive(Debug, Deserialize, Serialize)]
struct Config {
    enabled: bool,
    interval_seconds: u64,
    camera_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 5,
            camera_url: "http://localhost:6082".to_string(),
        }
    }
}

/// Simple capture loop
async fn capture_loop(config: Config) {
    if !config.enabled {
        println!("📷 Capture loop disabled in config");
        return;
    }

    println!("🎬 Starting capture loop (every {} seconds)", config.interval_seconds);
    
    let client = reqwest::Client::new();
    let mut capture_count = 0;
    
    loop {
        capture_count += 1;
        println!("📸 Capture #{}", capture_count);
        
        match client.get(&format!("{}/capture-image", config.camera_url)).send().await {
            Ok(resp) => {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if json["ok"].as_bool().unwrap_or(false) {
                        println!("   ✅ Saved: {}", json["filename"].as_str().unwrap_or("unknown"));
                    } else {
                        println!("   ❌ Failed: {}", json["error"].as_str().unwrap_or("unknown error"));
                    }
                }
            }
            Err(e) => println!("   ❌ Error: {}", e),
        }
        
        sleep(Duration::from_secs(config.interval_seconds)).await;
    }
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

async fn healthcheck() -> impl Responder {
    HttpResponse::Ok().content_type("text/plain").body("true")
}

async fn status() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "service": SERVICE_NAME,
        "status": "operational",
        "version": "0.1.0"
    }))
}

#[actix_web::main]
async fn main() -> Result<()> {
    let port = get_service_port();
    println!("🚀 Starting {} on port {}", SERVICE_NAME, port);
    
    // Load config or use default
    let config: Config = fs::read_to_string("director_config.json")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    
    // Save config if it doesn't exist
    if !std::path::Path::new("director_config.json").exists() {
        fs::write("director_config.json", serde_json::to_string_pretty(&config)?)?;
        println!("📝 Created default config file");
    }
    
    // Start capture loop in background
    tokio::spawn(capture_loop(config));
    
    // Start web server
    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![header::ACCEPT, header::CONTENT_TYPE, header::AUTHORIZATION]);

        App::new()
            .wrap(cors)
            .route("/api/v1/hive_agent-director/healthcheck/basic", web::get().to(healthcheck))
            .route("/api/v1/hive_agent-director/status", web::get().to(status))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await?;

    Ok(())
}
