use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, RwLock},
    thread,
    time::Duration,
};

const WARDEN_PORT: u16 = 6080;
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(10);
const SERVICE_START_DELAY: Duration = Duration::from_secs(2);

lazy_static! {
    static ref WARDEN_STATE: Arc<Mutex<WardenState>> = Arc::new(Mutex::new(WardenState::default()));
    static ref CONFIG_PATH: RwLock<PathBuf> = RwLock::new(PathBuf::new());
    static ref RUNNING_PROCESSES: Arc<Mutex<HashMap<String, Child>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref HEALTH_CHECK_FAILURES: Arc<Mutex<HashMap<String, u32>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceConfig {
    name: String,
    #[serde(default)]
    uuid: Option<String>,
    enabled: bool,
    running: bool,
    healthy: bool,
    failed: bool,
    boot_attempts: u32,
    boot_timeout_millisecs: u64,
    healthcheck_attempts: u32,
    healthcheck_timeout_millisecs: u64,
    port: u16,
    version: String,
    #[serde(default = "default_health_path")]
    health_path: String,
}

fn default_health_path() -> String {
    "healthcheck/basic".to_string()
}

#[derive(Debug, Clone, Default)]
struct WardenState {
    services: HashMap<String, ServiceConfig>,
    ports_in_use: Vec<u16>,
}

/// Check if a port is currently in use
fn is_port_in_use(port: u16) -> bool {
    match TcpListener::bind(format!("127.0.0.1:{}", port)) {
        Ok(_) => false,
        Err(_) => true,
    }
}

/// Find an available port in a range
fn find_available_port(start: u16, end: u16) -> Option<u16> {
    for port in start..=end {
        if !is_port_in_use(port) {
            return Some(port);
        }
    }
    None
}

/// Load services configuration from JSON file
fn load_services_config(path: &Path) -> Result<Vec<ServiceConfig>> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let services: Vec<ServiceConfig> = serde_json::from_str(&contents)?;
    Ok(services)
}

/// Save services configuration to JSON file
fn save_services_config(path: &Path, services: &[ServiceConfig]) -> Result<()> {
    let json = serde_json::to_string_pretty(services)?;
    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

/// Initialize warden state from configuration file
fn initialize_from_config(config_path: &Path) -> Result<()> {
    let services = load_services_config(config_path)?;
    let mut state = WARDEN_STATE.lock().unwrap();
    
    // Clear existing state
    state.services.clear();
    state.ports_in_use.clear();
    
    // Load services into state
    for service in services {
        if service.enabled || service.running {
            state.ports_in_use.push(service.port);
        }
        state.services.insert(service.name.clone(), service);
    }
    
    println!("📋 Loaded {} services from configuration", state.services.len());
    Ok(())
}

/// Persist current state to configuration file
fn persist_to_config() -> Result<()> {
    let config_path = CONFIG_PATH.read().unwrap().clone();
    let state = WARDEN_STATE.lock().unwrap();
    
    let mut services: Vec<ServiceConfig> = state.services.values().cloned().collect();
    services.sort_by(|a, b| a.port.cmp(&b.port));
    
    save_services_config(&config_path, &services)?;
    Ok(())
}

/// Get executable path for a service
fn get_service_executable(service_name: &str) -> PathBuf {
    let exe_path = env::current_exe().unwrap();
    let exe_dir = exe_path.parent().unwrap();
    
    // Service executables are in the same directory as the warden in release mode
    // Use the service name as-is (Cargo keeps hyphens in binary names)
    let mut exe_name = service_name.to_string();
    
    // Add .exe extension on Windows
    if cfg!(target_os = "windows") {
        exe_name.push_str(".exe");
    }
    
    exe_dir.join(exe_name)
}

/// Start a service process
fn start_service(service: &ServiceConfig) -> Result<Child> {
    let exe_path = get_service_executable(&service.name);
    
    if !exe_path.exists() {
        return Err(anyhow::anyhow!("Service executable not found: {}", exe_path.display()));
    }
    
    println!("🚀 Starting service: {} on port {}", service.name, service.port);
    
    // Pass the port to the service as a command line argument
    // Services should accept --port or use environment variable
    let child = Command::new(&exe_path)
        .arg("--port")
        .arg(service.port.to_string())
        .env("SERVICE_PORT", service.port.to_string())
        .env("WARDEN_ASSIGNED_PORT", service.port.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    Ok(child)
}

/// Stop a service process
fn stop_service(service_name: &str) -> Result<()> {
    let mut processes = RUNNING_PROCESSES.lock().unwrap();
    
    if let Some(mut child) = processes.remove(service_name) {
        println!("🛑 Stopping service: {}", service_name);
        
        // Try graceful termination first
        #[cfg(unix)]
        {
            // On Unix, kill() sends SIGKILL by default
            // For now, we'll just use kill() directly
            let _ = child.kill();
            
            // Give it a moment to complete
            thread::sleep(Duration::from_millis(100));
        }
        
        #[cfg(windows)]
        {
            // On Windows, kill() is the only option
            child.kill()?;
        }
        
        #[cfg(not(any(unix, windows)))]
        {
            // Fallback for other platforms
            child.kill()?;
        }
        
        child.wait()?;
    }
    
    Ok(())
}

/// Check if a service is healthy via HTTP health check
async fn check_service_health(service: &ServiceConfig) -> bool {
    let health_url = format!(
        "http://127.0.0.1:{}/{}",
        service.port, service.health_path
    );
    
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build() {
        Ok(c) => c,
        Err(_) => return false,
    };
    
    match client.get(&health_url).send().await {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                text.trim() == "true"
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// Start all enabled services
fn start_enabled_services() {
    let state = WARDEN_STATE.lock().unwrap();
    let services: Vec<ServiceConfig> = state.services.values()
        .filter(|s| s.enabled && s.name != "hive_agent-warden") // Don't try to start ourselves
        .cloned()
        .collect();
    drop(state);
    
    for service in services {
        let mut state = WARDEN_STATE.lock().unwrap();
        let mut processes = RUNNING_PROCESSES.lock().unwrap();
        
        // Skip if already running
        if processes.contains_key(&service.name) {
            continue;
        }
        
        match start_service(&service) {
            Ok(child) => {
                processes.insert(service.name.clone(), child);
                if let Some(svc) = state.services.get_mut(&service.name) {
                    svc.running = true;
                    svc.healthy = false; // Will be set by health check
                }
                println!("✅ Started: {}", service.name);
            }
            Err(e) => {
                eprintln!("❌ Failed to start {}: {}", service.name, e);
                if let Some(svc) = state.services.get_mut(&service.name) {
                    svc.running = false;
                    svc.failed = true;
                }
            }
        }
        
        // Small delay between service starts
        thread::sleep(SERVICE_START_DELAY);
    }
}

/// Monitor services and restart if needed
fn monitor_services_loop() {
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        loop {
            thread::sleep(HEALTH_CHECK_INTERVAL);
            
            let state = WARDEN_STATE.lock().unwrap();
            let services: Vec<ServiceConfig> = state.services.values()
                .filter(|s| s.enabled && s.name != "hive_agent-warden")
                .cloned()
                .collect();
            drop(state);
            
            for service in services {
                // Check if process is still alive
                let mut processes = RUNNING_PROCESSES.lock().unwrap();
                let is_alive = if let Some(child) = processes.get_mut(&service.name) {
                    matches!(child.try_wait(), Ok(None))
                } else {
                    false
                };
                drop(processes);
                
                // Update running status
                let mut state = WARDEN_STATE.lock().unwrap();
                if let Some(svc) = state.services.get_mut(&service.name) {
                    svc.running = is_alive;
                    
                    if is_alive {
                        // Check health
                        let healthy = rt.block_on(check_service_health(&service));
                        svc.healthy = healthy;
                        
                        if !healthy {
                            let mut failures = HEALTH_CHECK_FAILURES.lock().unwrap();
                            let count = failures.entry(service.name.clone()).or_insert(0);
                            *count += 1;
                            
                            // Restart after 3 consecutive failures
                            if *count >= 3 && svc.boot_attempts > 0 {
                                drop(state);
                                drop(failures);
                                
                                println!("🔄 Restarting unhealthy service: {}", service.name);
                                let _ = stop_service(&service.name);
                                thread::sleep(Duration::from_secs(1));
                                
                                // Try to restart
                                if let Ok(child) = start_service(&service) {
                                    let mut processes = RUNNING_PROCESSES.lock().unwrap();
                                    processes.insert(service.name.clone(), child);
                                    
                                    let mut state = WARDEN_STATE.lock().unwrap();
                                    if let Some(svc) = state.services.get_mut(&service.name) {
                                        svc.running = true;
                                        svc.boot_attempts -= 1;
                                    }
                                    
                                    let mut failures = HEALTH_CHECK_FAILURES.lock().unwrap();
                                    failures.remove(&service.name);
                                }
                            }
                        } else {
                            // Reset failure count on success
                            let mut failures = HEALTH_CHECK_FAILURES.lock().unwrap();
                            failures.remove(&service.name);
                        }
                    } else if svc.enabled && !svc.failed {
                        // Service should be running but isn't - try to start it
                        drop(state);
                        
                        println!("🔄 Starting stopped service: {}", service.name);
                        if let Ok(child) = start_service(&service) {
                            let mut processes = RUNNING_PROCESSES.lock().unwrap();
                            processes.insert(service.name.clone(), child);
                            
                            let mut state = WARDEN_STATE.lock().unwrap();
                            if let Some(svc) = state.services.get_mut(&service.name) {
                                svc.running = true;
                            }
                        }
                    }
                }
            }
            
            // Persist state changes
            let _ = persist_to_config();
        }
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// API Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/v1/warden/healthcheck/basic - Health check endpoint
async fn healthcheck_handler() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("true")
}

/// GET /api/v1/warden/status - Get current warden status
async fn status_handler() -> impl Responder {
    let state = WARDEN_STATE.lock().unwrap();
    let response = serde_json::json!({
        "status": "operational",
        "services_count": state.services.len(),
        "ports_in_use": state.ports_in_use,
        "timestamp": chrono::Local::now().to_rfc3339(),
    });
    HttpResponse::Ok().json(response)
}

/// GET /api/v1/warden/services - Get all services
async fn services_handler() -> impl Responder {
    let state = WARDEN_STATE.lock().unwrap();
    let services: Vec<ServiceConfig> = state.services.values().cloned().collect();
    HttpResponse::Ok().json(services)
}

/// POST /api/v1/warden/service/{name}/enable - Enable a service
async fn enable_service_handler(path: web::Path<String>) -> impl Responder {
    let name = path.into_inner();
    let mut state = WARDEN_STATE.lock().unwrap();

    if let Some(service) = state.services.get_mut(&name) {
        service.enabled = true;
        service.failed = false; // Reset failed status when enabling
        let service_copy = service.clone();
        
        // Release lock before persisting
        drop(state);
        
        // Persist changes to config file
        if let Err(e) = persist_to_config() {
            eprintln!("Failed to persist config: {}", e);
        }
        
        // Start the service if it's not already running
        let processes = RUNNING_PROCESSES.lock().unwrap();
        if !processes.contains_key(&name) {
            drop(processes);
            if let Ok(child) = start_service(&service_copy) {
                let mut processes = RUNNING_PROCESSES.lock().unwrap();
                processes.insert(name.clone(), child);
                
                let mut state = WARDEN_STATE.lock().unwrap();
                if let Some(svc) = state.services.get_mut(&name) {
                    svc.running = true;
                }
            }
        }
        
        HttpResponse::Ok().json(serde_json::json!({
            "status": "success",
            "message": format!("{} enabled", name),
            "service": service_copy
        }))
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "status": "error",
            "message": format!("Service {} not found", name)
        }))
    }
}

/// POST /api/v1/warden/service/{name}/disable - Disable a service
async fn disable_service_handler(path: web::Path<String>) -> impl Responder {
    let name = path.into_inner();
    let mut state = WARDEN_STATE.lock().unwrap();

    if let Some(service) = state.services.get_mut(&name) {
        service.enabled = false;
        service.running = false;
        service.healthy = false;
        let service_copy = service.clone();
        
        // Release lock before persisting
        drop(state);
        
        // Stop the service process
        let _ = stop_service(&name);
        
        // Persist changes to config file
        if let Err(e) = persist_to_config() {
            eprintln!("Failed to persist config: {}", e);
        }
        
        HttpResponse::Ok().json(serde_json::json!({
            "status": "success",
            "message": format!("{} disabled", name),
            "service": service_copy
        }))
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "status": "error",
            "message": format!("Service {} not found", name)
        }))
    }
}

/// POST /api/v1/warden/port/allocate - Allocate a new port for a service
async fn allocate_port_handler(
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let service_name = body.get("service_name").and_then(|v| v.as_str());
    let preferred_port = body.get("preferred_port").and_then(|v| v.as_u64()).map(|p| p as u16);

    match (service_name, preferred_port) {
        (Some(name), Some(port)) => {
            if !is_port_in_use(port) {
                let mut state = WARDEN_STATE.lock().unwrap();
                state.ports_in_use.push(port);
                HttpResponse::Ok().json(serde_json::json!({
                    "status": "success",
                    "service": name,
                    "port": port
                }))
            } else {
                // Find alternative port
                if let Some(new_port) = find_available_port(6000, 7000) {
                    let mut state = WARDEN_STATE.lock().unwrap();
                    state.ports_in_use.push(new_port);
                    HttpResponse::Ok().json(serde_json::json!({
                        "status": "reassigned",
                        "service": name,
                        "requested_port": port,
                        "assigned_port": new_port
                    }))
                } else {
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "status": "error",
                        "message": "No available ports found"
                    }))
                }
            }
        }
        _ => HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "Missing required fields: service_name, preferred_port"
        })),
    }
}

/// GET /api/v1/warden/port/check/{port} - Check if a port is in use
async fn port_check_handler(path: web::Path<u16>) -> impl Responder {
    let port = path.into_inner();
    let in_use = is_port_in_use(port);
    HttpResponse::Ok().json(serde_json::json!({
        "port": port,
        "in_use": in_use
    }))
}

#[actix_web::main]
async fn main() -> Result<()> {
    println!("🚀 Starting Hive Agent Warden on port {}", WARDEN_PORT);

    // Determine config path
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or_else(|| anyhow::anyhow!("Cannot determine exe directory"))?;
    let config_path = exe_dir.join("deps").join("core_microservices.json");
    
    // If config doesn't exist in deps, try parent directory deps
    let config_path = if config_path.exists() {
        config_path
    } else {
        // Try the workspace deps directory
        exe_dir.parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("hive_agent-warden").join("deps").join("core_microservices.json"))
            .filter(|p| p.exists())
            .unwrap_or(config_path)
    };
    
    println!("📁 Using config file: {}", config_path.display());
    
    // Store config path for later use
    {
        let mut path = CONFIG_PATH.write().unwrap();
        *path = config_path.clone();
    }
    
    // Initialize state from config file
    match initialize_from_config(&config_path) {
        Ok(_) => println!("✅ Configuration loaded successfully"),
        Err(e) => {
            eprintln!("⚠️  Failed to load config: {}", e);
            eprintln!("   Creating empty state...");
        }
    }

    println!("📋 Warden initialized");
    
    // Start monitoring loop
    println!("🔍 Starting service monitoring...");
    monitor_services_loop();
    
    // Give monitoring thread time to start
    thread::sleep(Duration::from_secs(1));
    
    // Start all enabled services
    println!("🚀 Starting enabled services...");
    start_enabled_services();

    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                header::ACCEPT,
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
            ]);

        App::new()
            .wrap(cors)
            .route("/api/v1/warden/healthcheck/basic", web::get().to(healthcheck_handler))
            .route("/api/v1/warden/status", web::get().to(status_handler))
            .route("/api/v1/warden/services", web::get().to(services_handler))
            .route("/api/v1/warden/service/{name}/enable", web::post().to(enable_service_handler))
            .route("/api/v1/warden/service/{name}/disable", web::post().to(disable_service_handler))
            .route("/api/v1/warden/port/allocate", web::post().to(allocate_port_handler))
            .route("/api/v1/warden/port/check/{port}", web::get().to(port_check_handler))
    })
    .bind(("0.0.0.0", WARDEN_PORT))?
    .run()
    .await?;

    Ok(())
}
