use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;

const SERVICE_NAME: &str = "hive_agent-text-to-speech-generation-loop";
const SERVICE_PORT: u16 = 6074;

/// GET /api/v1/hive_agent-text-to-speech-generation-loop/healthcheck/basic - Health check endpoint
async fn healthcheck() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("true")
}

/// GET /api/v1/hive_agent-text-to-speech-generation-loop/status - Service status
async fn status() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "service": SERVICE_NAME,
        "status": "operational",
        "version": "0.1.0"
    }))
}

#[actix_web::main]
async fn main() -> Result<()> {
    println!("🚀 Starting {} on port {}", SERVICE_NAME, SERVICE_PORT);

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
            .route("/api/v1/hive_agent-text-to-speech-generation-loop/healthcheck/basic", web::get().to(healthcheck))
            .route("/api/v1/hive_agent-text-to-speech-generation-loop/status", web::get().to(status))
    })
    .bind(("0.0.0.0", SERVICE_PORT))?
    .run()
    .await?;

    Ok(())
}
