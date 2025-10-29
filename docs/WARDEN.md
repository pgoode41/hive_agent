# Hive Agent Warden - Comprehensive Documentation

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Core Features](#core-features)
4. [Configuration](#configuration)
5. [Service Management](#service-management)
6. [Health Monitoring](#health-monitoring)
7. [API Reference](#api-reference)
8. [Development Guide](#development-guide)
9. [Troubleshooting](#troubleshooting)
10. [Best Practices](#best-practices)

---

## Overview

The **Hive Agent Warden** is a production-ready service orchestrator that manages the lifecycle of all Hive Agent microservices. It ensures high availability through automatic health monitoring, service recovery, and state persistence.

### Key Responsibilities

- **Service Lifecycle Management**: Start, stop, enable, disable services
- **Health Monitoring**: Continuous HTTP health checks every 10 seconds
- **Automatic Recovery**: Restart failed or unhealthy services
- **Port Management**: Dynamic port allocation and conflict resolution
- **State Persistence**: Maintain service states across restarts
- **Cross-Platform Support**: Works on Windows, macOS, and Linux

### Design Principles

1. **Zero Downtime**: Services are automatically restarted on failure
2. **Self-Healing**: Unhealthy services are detected and recovered
3. **Stateful Management**: All changes persist to configuration
4. **Platform Agnostic**: Consistent behavior across all operating systems
5. **API-First**: All operations available via REST API

---

## Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────────┐
│                        WARDEN (Port 5080)                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Config     │  │   Process    │  │   Health     │          │
│  │   Manager    │  │   Manager    │  │   Monitor    │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                  │                  │                   │
│  ┌──────▼──────────────────▼──────────────────▼──────────────┐  │
│  │                    State Management                        │  │
│  │  • Services HashMap                                        │  │
│  │  • Running Processes                                       │  │
│  │  • Health Status                                          │  │
│  │  • Port Allocations                                       │  │
│  └────────────────────────┬───────────────────────────────────┘  │
│                           │                                       │
│  ┌────────────────────────▼───────────────────────────────────┐  │
│  │              core_microservices.json                       │  │
│  │         (Persistent Configuration Storage)                 │  │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
└───────────────────────┬───────────────────────────────────────────┘
                        │
    ┌───────────────────┼───────────────────────────────────┐
    │                   │                                   │
    ▼                   ▼                                   ▼
┌─────────┐      ┌─────────────┐                    ┌────────────┐
│   RAG   │      │  Director   │        ...         │   Tools    │
│  5071   │      │    5084     │                    │   5083     │
└─────────┘      └─────────────┘                    └────────────┘
```

### Internal Components

#### 1. Configuration Manager
- Loads `core_microservices.json` on startup
- Persists state changes immediately
- Validates service configurations
- Manages port allocations

#### 2. Process Manager
- Spawns service processes
- Tracks process PIDs
- Handles graceful/forced termination
- Platform-specific process handling

#### 3. Health Monitor
- Runs in dedicated thread
- Performs HTTP health checks every 10 seconds
- Tracks failure counts
- Triggers automatic restarts

#### 4. State Management
- Thread-safe state via `Arc<Mutex<T>>`
- Atomic operations for concurrent access
- Real-time state synchronization
- Memory and disk state consistency

---

## Core Features

### 1. Automatic Service Starting

When the warden starts, it:

1. Loads configuration from `core_microservices.json`
2. Identifies all enabled services
3. Starts services sequentially with 2-second delays
4. Begins health monitoring immediately

### 2. Health Monitoring Loop

```rust
Every 10 seconds:
  For each enabled service:
    1. Check if process is alive
    2. If alive: perform HTTP health check
    3. If unhealthy: increment failure counter
    4. If failures >= 3: restart service
    5. Update state and persist
```

### 3. Automatic Recovery

**Restart Triggers:**
- Process crash (immediate restart)
- 3 consecutive health check failures
- Service stopped but still enabled

**Restart Process:**
1. Stop existing process (if any)
2. Wait 1 second
3. Start new process
4. Decrement `boot_attempts`
5. Reset health failure counter

### 4. Dynamic Port Management

**Intelligent Port Allocation:**
The warden ensures each service gets a working port, regardless of system conflicts.

**Port Assignment Process:**
1. Read preferred port from `core_microservices.json`
2. Check if preferred port is available on the system
3. If available: assign and use it
4. If not available: find next open port (6000-7000 range)
5. Update configuration with actual assigned port
6. Pass port to service on startup
7. Persist new port assignment to config

**Example Flow:**
```rust
// Service wants port 6071 (from config)
if is_port_in_use(6071) {
    // Port taken, find alternative
    new_port = find_available_port(6000, 7000)  // e.g., 6095
    
    // Update config
    service.port = new_port
    save_config()
    
    // Start service with new port
    start_service_with_port(service, new_port)
}
```

**Key Points:**
- Ports in documentation are **examples only**
- Actual ports depend on system availability
- Warden handles all port conflicts automatically
- Services receive their port assignment at startup
- Configuration always reflects current port assignments

---

## Configuration

### File Location

```
hive_agent-warden/deps/core_microservices.json
```

### Configuration Schema

```json
{
  "name": "service-name",                    // Service identifier
  "uuid": "550e8400-...",                    // Unique service ID
  "enabled": true,                           // Should start automatically
  "running": false,                          // Current running state (runtime)
  "healthy": false,                          // Current health status (runtime)
  "failed": false,                           // Permanent failure flag
  "boot_attempts": 3,                        // Max restart attempts
  "boot_timeout_millisecs": 5000,            // Time before first health check
  "healthcheck_attempts": 3,                 // Failures before restart
  "healthcheck_timeout_millisecs": 5000,     // Health check HTTP timeout
  "port": 5071,                              // Service port
  "version": "0.1.0",                        // Service version
  "health_path": "api/v1/service/healthcheck/basic"  // Health endpoint
}
```

### Service States

| State | Description | Persistence |
|-------|-------------|-------------|
| `enabled` | Service should be running | ✅ Persisted |
| `running` | Process is currently active | ✅ Persisted |
| `healthy` | Health checks passing | ✅ Persisted |
| `failed` | Exceeded boot attempts | ✅ Persisted |

---

## Service Management

### Starting Services

**Automatic Start (on warden startup):**
```rust
// All enabled services start automatically
cargo run --release -p hive_agent-warden
```

**Manual Start (via API):**
```bash
curl -X POST http://localhost:5080/api/v1/warden/service/{name}/enable
```

### Stopping Services

**Manual Stop:**
```bash
curl -X POST http://localhost:5080/api/v1/warden/service/{name}/disable
```

**Effects:**
- Process terminated immediately
- `enabled` set to `false`
- `running` set to `false`
- Changes persisted to configuration

### Service Lifecycle

```
┌─────────┐    Enable    ┌─────────┐   Start Process   ┌─────────┐
│Disabled │─────────────▶│ Enabled │──────────────────▶│ Running │
└─────────┘              └─────────┘                   └────┬────┘
     ▲                        ▲                              │
     │                        │                              │
     │      Disable          │         Health Check         ▼
     └────────────────────────┴──────────────────────┌─────────┐
                                                      │ Healthy │
                                                      └─────────┘
```

---

## Health Monitoring

### Health Check Process

1. **HTTP Request**: `GET http://127.0.0.1:{port}/{health_path}`
2. **Expected Response**: Plain text `"true"`
3. **Timeout**: 5 seconds
4. **Interval**: 10 seconds

### Failure Handling

```rust
if !healthy {
    failures[service] += 1;
    if failures[service] >= 3 {
        restart_service(service);
        failures[service] = 0;
    }
}
```

### Health States

| Failures | Action |
|----------|--------|
| 0 | Service healthy |
| 1-2 | Warning state, monitoring continues |
| 3+ | Automatic restart triggered |

---

## API Reference

### Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/warden/healthcheck/basic` | Warden health check |
| GET | `/api/v1/warden/status` | System status overview |
| GET | `/api/v1/warden/services` | List all services with states |
| POST | `/api/v1/warden/service/{name}/enable` | Enable and start service |
| POST | `/api/v1/warden/service/{name}/disable` | Disable and stop service |
| POST | `/api/v1/warden/port/allocate` | Allocate port for service |
| GET | `/api/v1/warden/port/check/{port}` | Check port availability |

### Example Responses

**GET /api/v1/warden/status**
```json
{
  "status": "operational",
  "services_count": 13,
  "ports_in_use": [5071, 5072, ...],
  "timestamp": "2025-10-28T18:30:45.123456+00:00"
}
```

**GET /api/v1/warden/services**
```json
[
  {
    "name": "rag",
    "port": 5071,
    "enabled": true,
    "running": true,
    "healthy": true,
    "failed": false,
    "version": "0.1.0"
  }
]
```

---

## Development Guide

### Adding a New Service

1. **Update Configuration:**
```json
// Add to core_microservices.json
{
  "name": "new-service",
  "uuid": "generate-uuid",
  "enabled": true,
  "port": 6090,  // Preferred port (may be reassigned if unavailable)
  ...
}
```

2. **Create Service Binary with Dynamic Port Support:**
```rust
use std::env;

const DEFAULT_PORT: u16 = 5090;

fn get_service_port() -> u16 {
    // Check command line: --port <number>
    let args: Vec<String> = env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--port" && i + 1 < args.len() {
            if let Ok(port) = args[i + 1].parse::<u16>() {
                return port;
            }
        }
    }
    
    // Check environment variable from warden
    if let Ok(port) = env::var("WARDEN_ASSIGNED_PORT")
        .and_then(|p| Ok(p.parse::<u16>().unwrap_or(DEFAULT_PORT))) {
        return port;
    }
    
    DEFAULT_PORT
}

#[actix_web::main]
async fn main() {
    let port = get_service_port();
    println!("Starting on port {} (assigned by warden)", port);
    
    // Implement health check endpoint
    HttpServer::new(|| {
        App::new()
            .route("/api/v1/new-service/healthcheck/basic", 
                   web::get().to(|| async { HttpResponse::Ok().body("true") }))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
```

3. **Build and Deploy:**
```bash
cargo build --release -p new-service
# Binary must be in same directory as warden
```

### How Services Receive Port Assignments

The warden passes the assigned port to services in THREE ways:

1. **Command Line Argument**: `--port 5095`
2. **Environment Variable**: `WARDEN_ASSIGNED_PORT=5095`
3. **Environment Variable**: `SERVICE_PORT=5095`

Services should check in this order and fall back to their default port if none are provided.

### Modifying Warden Behavior

**Key Files:**
- `src/main.rs` - Main logic and API handlers
- `deps/core_microservices.json` - Service configurations

**Important Constants:**
```rust
const WARDEN_PORT: u16 = 6080;                    // API port
const HEALTH_CHECK_INTERVAL: Duration = 10s;      // Check frequency
const SERVICE_START_DELAY: Duration = 2s;         // Between starts
```

### Testing Changes

```bash
# Build warden
cargo build --release -p hive_agent-warden

# Run with visible output
RUST_LOG=debug ./target/release/hive_agent-warden

# Monitor services
watch 'curl -s http://localhost:5080/api/v1/warden/services | python3 -m json.tool'
```

---

## Troubleshooting

### Common Issues

#### Services Not Starting

**Check executable exists:**
```bash
ls -la target/release/ | grep service-name
```

**Check port availability:**
```bash
lsof -i :5071
```

**Check logs:**
```bash
# Warden output shows start failures
# Service output via stdout/stderr pipes
```

#### Health Checks Failing

**Test manually:**
```bash
curl http://localhost:5071/api/v1/service/healthcheck/basic
# Should return: true
```

**Common causes:**
- Service not fully started (wait for boot_timeout)
- Wrong health_path in configuration
- Service crashed after starting

#### Port Conflicts

**Error:** "Port already in use"
```bash
# Find process using port
lsof -i :5071
kill -9 <PID>

# Or use warden's port allocation
curl -X POST http://localhost:5080/api/v1/warden/port/allocate \
  -H "Content-Type: application/json" \
  -d '{"service_name": "service", "preferred_port": 5071}'
```

### Debug Mode

**Enable verbose logging:**
```rust
// In main.rs
println!("🔍 Debug: {}", variable);
```

**Monitor state changes:**
```bash
# Watch configuration file
watch cat hive_agent-warden/deps/core_microservices.json
```

---

## Best Practices

### 1. Service Design

✅ **DO:**
- Implement `/healthcheck/basic` endpoint
- Return plain text `"true"` when healthy
- Start listening within `boot_timeout_millisecs`
- Handle graceful shutdown

❌ **DON'T:**
- Block during startup
- Return JSON from health endpoint
- Ignore port configuration
- Hold exclusive resources

### 2. Configuration Management

✅ **DO:**
- Set reasonable `boot_timeout_millisecs` (5000-30000)
- Use unique UUIDs per service
- Document port allocations
- Version your services

❌ **DON'T:**
- Use ports outside 5000-6000 range
- Duplicate service names
- Set boot_attempts to 0
- Modify runtime fields manually

### 3. Development Workflow

✅ **DO:**
- Test services individually first
- Build all services before starting warden
- Monitor warden output during development
- Use API for service control

❌ **DON'T:**
- Kill warden abruptly (loses state)
- Manually edit config while running
- Start services outside warden
- Ignore health check failures

### 4. Production Deployment

✅ **DO:**
- Run warden as a system service
- Monitor warden health externally
- Set up log rotation
- Configure firewall rules

❌ **DON'T:**
- Expose ports publicly
- Run without health monitoring
- Ignore failed services
- Skip configuration backups

---

## Architecture Decisions

### Why These Design Choices?

1. **JSON Configuration**: Human-readable, easy to modify, version control friendly
2. **HTTP Health Checks**: Simple, universal, language agnostic
3. **10-Second Interval**: Balance between responsiveness and resource usage
4. **3-Failure Threshold**: Prevents flapping while ensuring quick recovery
5. **Port Range 5000-6000**: Avoids system ports, sufficient for microservices

### Future Enhancements

Potential improvements for consideration:

1. **Distributed Mode**: Multiple warden instances with coordination
2. **Metrics Collection**: Prometheus/Grafana integration
3. **Resource Limits**: CPU/Memory constraints per service
4. **Rolling Updates**: Zero-downtime deployments
5. **Service Dependencies**: Start order and dependency management
6. **Log Aggregation**: Centralized logging system
7. **Circuit Breakers**: Advanced failure handling
8. **Load Balancing**: Multiple instances per service

---

## Maintenance Guide

### Regular Tasks

**Daily:**
- Check service health status
- Review any failed services
- Monitor resource usage

**Weekly:**
- Review logs for patterns
- Update service versions
- Test disaster recovery

**Monthly:**
- Update dependencies
- Review port allocations
- Performance profiling

### Monitoring Commands

```bash
# Service health overview
curl -s http://localhost:5080/api/v1/warden/services | \
  python3 -c "import sys, json; d=json.load(sys.stdin); \
  print(f'Total: {len(d)}, Running: {sum(1 for s in d if s["running"])}, \
  Healthy: {sum(1 for s in d if s["healthy"])}')"

# Failed services
curl -s http://localhost:5080/api/v1/warden/services | \
  python3 -m json.tool | grep -B5 '"failed": true'

# Port usage
curl -s http://localhost:5080/api/v1/warden/status | \
  python3 -c "import sys, json; print('Ports:', json.load(sys.stdin)['ports_in_use'])"
```

---

## Conclusion

The Hive Agent Warden is a robust, production-ready service orchestrator designed for reliability and ease of use. By following this documentation, developers can:

- Understand the complete architecture
- Properly configure and deploy services  
- Troubleshoot common issues
- Extend functionality safely
- Maintain system health

For questions or contributions, refer to the main project documentation and follow the established patterns for consistency.

---

**Document Version**: 1.0  
**Last Updated**: October 28, 2025  
**Warden Version**: 0.1.0
