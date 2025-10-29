use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use std::env;

const SERVICE_NAME: &str = "rag";
const DEFAULT_PORT: u16 = 6071;

/// Get the service port from warden or use default
fn get_service_port() -> u16 {
    // Priority order:
    // 1. Command line argument --port
    // 2. Environment variable WARDEN_ASSIGNED_PORT
    // 3. Environment variable SERVICE_PORT  
    // 4. Default port
    
    // Check command line arguments
    let args: Vec<String> = env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            if let Ok(port) = args[i + 1].parse::<u16>() {
                return port;
            }
        }
    }
    
    // Check environment variables
    if let Ok(port_str) = env::var("WARDEN_ASSIGNED_PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            return port;
        }
    }
    
    if let Ok(port_str) = env::var("SERVICE_PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            return port;
        }
    }
    
    DEFAULT_PORT
}

/// GET /api/v1/rag/healthcheck/basic - Health check endpoint
async fn healthcheck() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("true")
}

/// GET /api/v1/rag/status - Service status
async fn status() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "service": SERVICE_NAME,
        "status": "operational",
        "version": "0.1.0"
    }))
}

#[actix_web::main]
async fn main() -> Result<()> {
    let service_port = get_service_port();
    println!("🚀 Starting {} on port {} (assigned by warden)", SERVICE_NAME, service_port);

    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![
                header::ACCEPT,
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
            ]);

        App::new()
            .wrap(cors)
            .route("/api/v1/rag/healthcheck/basic", web::get().to(healthcheck))
            .route("/api/v1/rag/status", web::get().to(status))
    })
    .bind(("0.0.0.0", service_port))?
    .run()
    .await?;

    Ok(())
}
