use actix_web::{web, App, HttpResponse, HttpServer, middleware};
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::fs;
use std::env;
use anyhow::Result;
use image::DynamicImage;
use nokhwa::{Camera, query};
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType, ApiBackend};
use nokhwa::pixel_format::RgbFormat;

const SERVICE_NAME: &str = "hive_agent-camera-server";
const DEFAULT_PORT: u16 = 6082;

// Response types matching the Python API
#[derive(Debug, Serialize, Deserialize)]
struct CaptureResponse {
    ok: bool,
    filename: Option<String>,
    counter: Option<u32>,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    camera_active: bool,
    camera_index: Option<u32>,
    platform: String,
}

// Shared state for the camera
struct AppState {
    camera: Arc<Mutex<Option<Camera>>>,
    counter: Arc<Mutex<u32>>,
    camera_index: Option<u32>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            camera: Arc::new(Mutex::new(None)),
            counter: Arc::new(Mutex::new(0)),
            camera_index: None,
        }
    }
}

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

/// Get the current platform as a string
fn get_platform() -> String {
    if cfg!(target_os = "linux") {
        "Linux".to_string()
    } else if cfg!(target_os = "windows") {
        "Windows".to_string()
    } else if cfg!(target_os = "macos") {
        "macOS".to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Find a working camera device using nokhwa (cross-platform)
fn find_working_camera() -> Result<(Camera, u32)> {
    println!("🔍 Searching for available cameras (nokhwa - cross-platform)...");
    
    // Try to get available cameras
    let backend = ApiBackend::Auto;
    let cameras = match query(backend) {
        Ok(cams) => {
            println!("📷 Found {} camera(s)", cams.len());
            // Get indices from detected cameras
            (0..cams.len()).map(|i| i as u32).collect::<Vec<u32>>()
        }
        Err(e) => {
            eprintln!("⚠️ Failed to query cameras: {}", e);
            vec![0, 1, 2] // Try first 3 indices anyway
        }
    };
    
    // Try each camera index
    for index in cameras {
        println!("🔍 Trying camera index: {}", index);
        
        // Try to create camera with default format
        let camera_index = CameraIndex::Index(index);
        
        // Try to open camera with auto format
        let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        
        match Camera::new(camera_index.clone(), format) {
            Ok(camera) => {
                // Camera opened successfully, check if we can use it
                let info = camera.info();
                println!("✅ Found working camera at index {}: {}", 
                      index, info.human_name());
                return Ok((camera, index));
            }
            Err(e) => {
                eprintln!("   Failed to open camera at index {}: {}", index, e);
            }
        }
    }
    
    eprintln!("⚠️ No working cameras found - server will run without camera");
    Err(anyhow::anyhow!("No working cameras found"))
}

/// Capture an image from the camera using nokhwa
async fn capture_image(data: web::Data<Arc<Mutex<AppState>>>) -> HttpResponse {
    let state = data.lock().unwrap();
    
    // Check if camera is available
    let camera_available = state.camera.lock().unwrap().is_some();
    
    if !camera_available {
        return HttpResponse::ServiceUnavailable().json(CaptureResponse {
            ok: false,
            filename: None,
            counter: None,
            message: Some("No camera available".to_string()),
            error: Some("Camera not initialized".to_string()),
        });
    }
    
    // Increment counter
    let counter = {
        let mut c = state.counter.lock().unwrap();
        *c += 1;
        *c
    };
    
    // Capture frame
    let capture_result = {
        let mut camera_guard = state.camera.lock().unwrap();
        if let Some(camera) = camera_guard.as_mut() {
            // Open stream if not already open
            if !camera.is_stream_open() {
                if let Err(e) = camera.open_stream() {
                    eprintln!("Failed to open camera stream: {}", e);
                    return HttpResponse::InternalServerError().json(CaptureResponse {
                        ok: false,
                        filename: None,
                        counter: Some(counter),
                        message: None,
                        error: Some(format!("Failed to open stream: {}", e)),
                    });
                }
            }
            
            // Capture a frame
            match camera.frame() {
                Ok(buffer) => {
                    // Decode the buffer to an RGB image
                    let decoded = buffer.decode_image::<RgbFormat>();
                    match decoded {
                        Ok(img) => {
                            // Convert to DynamicImage
                            let rgb_image = image::RgbImage::from_raw(
                                img.width(),
                                img.height(),
                                img.into_vec()
                            );
                            match rgb_image {
                                Some(rgb) => Ok(DynamicImage::ImageRgb8(rgb)),
                                None => {
                                    // Fallback: try to load from raw buffer
                                    let raw = buffer.buffer();
                                    if let Ok(img) = image::load_from_memory(raw) {
                                        Ok(img)
                                    } else {
                                        Err("Failed to convert image".to_string())
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to decode frame: {}", e);
                            // Try to get raw buffer as fallback
                            let raw = buffer.buffer();
                            if let Ok(img) = image::load_from_memory(raw) {
                                Ok(img)
                            } else {
                                Err(format!("Failed to decode frame: {}", e))
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to capture frame: {}", e);
                    Err(format!("Failed to capture frame: {}", e))
                }
            }
        } else {
            Err("No camera available".to_string())
        }
    };
    
    // Process and save the captured frame
    match capture_result {
        Ok(img) => {
            // Create output directory if it doesn't exist
            let output_dir = "generated_image_captures";
            fs::create_dir_all(output_dir).unwrap_or_else(|e| {
                eprintln!("Failed to create output directory: {}", e);
            });
            
            // Generate filename
            let filename = format!("{}/captured_image_{}.png", output_dir, counter);
            
            // Save the image
            match img.save(&filename) {
                Ok(_) => {
                    println!("📸 Saved image: {}", filename);
                    HttpResponse::Ok().json(CaptureResponse {
                        ok: true,
                        filename: Some(filename.clone()),
                        counter: Some(counter),
                        message: Some(format!("Image saved as {}", filename)),
                        error: None,
                    })
                }
                Err(e) => {
                    eprintln!("Failed to save image: {:?}", e);
                    HttpResponse::InternalServerError().json(CaptureResponse {
                        ok: false,
                        filename: None,
                        counter: Some(counter),
                        message: None,
                        error: Some(format!("Failed to save image: {:?}", e)),
                    })
                }
            }
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(CaptureResponse {
                ok: false,
                filename: None,
                counter: Some(counter),
                message: None,
                error: Some(e),
            })
        }
    }
}

/// Health check endpoint for warden
async fn healthcheck_basic() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("true")
}

/// Advanced health check endpoint with camera status
async fn health_check_advanced(data: web::Data<Arc<Mutex<AppState>>>) -> HttpResponse {
    let state = data.lock().unwrap();
    let camera_active = state.camera.lock().unwrap().is_some();
    
    HttpResponse::Ok().json(HealthResponse {
        status: if camera_active { "ok" } else { "degraded" }.to_string(),
        camera_active,
        camera_index: state.camera_index,
        platform: get_platform(),
    })
}

/// Service status endpoint
async fn status(data: web::Data<Arc<Mutex<AppState>>>) -> HttpResponse {
    let state = data.lock().unwrap();
    let camera_active = state.camera.lock().unwrap().is_some();
    
    // Get camera info if available
    let camera_info = if camera_active {
        let camera_guard = state.camera.lock().unwrap();
        if let Some(camera) = camera_guard.as_ref() {
            let info = camera.info();
            Some(serde_json::json!({
                "name": info.human_name(),
                "description": info.description(),
                "backend": format!("{:?}", info.index()),
            }))
        } else {
            None
        }
    } else {
        None
    };
    
    HttpResponse::Ok().json(serde_json::json!({
        "service": SERVICE_NAME,
        "status": if camera_active { "operational" } else { "degraded" },
        "version": "0.1.0",
        "platform": get_platform(),
        "camera_active": camera_active,
        "camera_index": state.camera_index,
        "camera_info": camera_info,
        "capture_count": *state.counter.lock().unwrap(),
        "camera_library": "nokhwa (cross-platform)"
    }))
}

/// List available cameras
async fn list_cameras() -> HttpResponse {
    let mut cameras = Vec::new();
    
    // Try to query cameras using the proper API
    let backend = ApiBackend::Auto;
    match query(backend) {
        Ok(camera_list) => {
            for (i, cam_info) in camera_list.iter().enumerate() {
                cameras.push(serde_json::json!({
                    "index": i,
                    "name": cam_info.human_name(),
                    "description": cam_info.description(),
                    "backend": format!("{:?}", cam_info.index()),
                }));
            }
        }
        Err(e) => {
            eprintln!("Failed to query cameras: {}", e);
            // Try manual detection of first few indices
            for i in 0..3 {
                let index = CameraIndex::Index(i);
                let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
                
                if let Ok(camera) = Camera::new(index, format) {
                    let info = camera.info();
                    cameras.push(serde_json::json!({
                        "index": i,
                        "name": info.human_name(),
                        "description": info.description(),
                    }));
                }
            }
        }
    }
    
    HttpResponse::Ok().json(serde_json::json!({
        "cameras": cameras,
        "count": cameras.len(),
        "platform": get_platform(),
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = get_service_port();
    
    println!("🚀 Starting {} on port {}", SERVICE_NAME, port);
    println!("📁 Output directory: generated_image_captures");
    println!("🖥️ Platform: {}", get_platform());
    println!("📷 Camera library: nokhwa (cross-platform support)");
    
    // Create output directory
    fs::create_dir_all("generated_image_captures").unwrap_or_else(|e| {
        eprintln!("Failed to create output directory: {}", e);
    });
    
    // Initialize app state
    let mut app_state = AppState::new();
    
    // Try to find and initialize a camera
    match find_working_camera() {
        Ok((mut camera, index)) => {
            println!("🎥 Camera initialized successfully (index {})", index);
            
            // Get camera info
            let info = camera.info();
            println!("📷 Camera: {}", info.human_name());
            println!("📝 Description: {}", info.description());
            
            // Try to open the stream
            match camera.open_stream() {
                Ok(_) => println!("📹 Camera stream opened successfully"),
                Err(e) => eprintln!("⚠️ Failed to open camera stream: {}", e),
            }
            
            app_state.camera = Arc::new(Mutex::new(Some(camera)));
            app_state.camera_index = Some(index);
        }
        Err(e) => {
            eprintln!("⚠️ No camera found: {:?}", e);
            eprintln!("   Server will run in degraded mode (no capture available)");
            eprintln!("   Health checks will still work");
        }
    }
    
    let state = Arc::new(Mutex::new(app_state));
    
    println!("🌐 Starting HTTP server on 0.0.0.0:{}", port);
    println!("📍 Endpoints:");
    println!("   - Health: /api/v1/hive_agent-camera-server/healthcheck/basic");
    println!("   - Status: /api/v1/hive_agent-camera-server/status");
    println!("   - Capture: /capture-image");
    println!("   - List cameras: /cameras");
    
    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(middleware::Logger::default())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
            )
            // Warden expected endpoints
            .route("/api/v1/hive_agent-camera-server/healthcheck/basic", web::get().to(healthcheck_basic))
            .route("/api/v1/hive_agent-camera-server/status", web::get().to(status))
            // Camera functionality endpoints
            .route("/capture-image", web::get().to(capture_image))
            .route("/health", web::get().to(health_check_advanced))
            .route("/cameras", web::get().to(list_cameras))
            // Alternative capture endpoint
            .route("/api/v1/hive_agent-camera-server/capture", web::get().to(capture_image))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}