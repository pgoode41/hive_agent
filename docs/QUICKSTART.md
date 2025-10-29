# Hive Agent - Quick Start Guide

Get up and running with Hive Agent in minutes.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Building](#building)
- [Running Services](#running-services)
- [Testing](#testing)
- [Common Tasks](#common-tasks)

---

## Prerequisites

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
- **Cargo**: Comes with Rust
- **curl**: For testing APIs

Verify installation:
```bash
rustc --version
cargo --version
```

---

## Building

### Build Everything

```bash
cd /home/nibbles/Documents/hive_agent
cargo build --release
```

**Output**: Binaries will be in `target/release/`

### Build Specific Service

```bash
# Build just the warden
cargo build --release -p hive_agent-warden

# Build just RAG service
cargo build --release -p rag

# Build multiple services
cargo build --release -p hive_agent-warden -p rag -p hive_agent-director
```

### Check for Compilation Issues (No Build)

```bash
cargo check
```

---

## Running Services

### Using the Warden (Recommended Method)

The Warden automatically manages all services - starts them, monitors health, and restarts if needed.

```bash
# Build all services first
cd /home/nibbles/Documents/hive_agent
cargo build --release

# Start the warden - it will start all enabled services
./target/release/hive_agent-warden
```

**Output**:
```
🚀 Starting Hive Agent Warden on port 6080
📁 Using config file: .../deps/core_microservices.json
📋 Loaded 13 services from configuration
✅ Configuration loaded successfully
📋 Warden initialized
🔍 Starting service monitoring...
🚀 Starting enabled services...
✅ Started: rag
✅ Started: hive_agent-director
✅ Started: hive_agent-tools
... (all services start automatically)
```

**What Happens**:
1. Warden loads configuration from `core_microservices.json`
2. Starts all services with `"enabled": true`
3. Monitors health every 10 seconds
4. Restarts any service that crashes or becomes unhealthy
5. Persists all state changes to configuration

### Managing Services with Warden

```bash
# Check all services status
curl http://localhost:6080/api/v1/warden/services

# Enable a service (starts it immediately)
curl -X POST http://localhost:5080/api/v1/warden/service/rag/enable

# Disable a service (stops it immediately)  
curl -X POST http://localhost:5080/api/v1/warden/service/rag/disable

# Check warden status
curl http://localhost:6080/api/v1/warden/status
```

### Run Services Manually (Not Recommended)

Only use this for debugging specific services:

```bash
# Run individual service (without warden management)
./target/release/rag

# Run multiple manually
./target/release/hive_agent-director &
./target/release/hive_agent-tools &
```

**Note**: Manual runs bypass warden's health monitoring and auto-restart features.

---

## Testing

### Check Service Health

```bash
# Check warden health
curl http://0.0.0.0:5080/api/v1/warden/healthcheck/basic

# Check RAG service health
curl http://0.0.0.0:6071/api/v1/rag/healthcheck/basic
```

### Get Service Status

```bash
# Get warden status
curl http://0.0.0.0:5080/api/v1/warden/status | jq

# Get all managed services
curl http://0.0.0.0:5080/api/v1/warden/services | jq

# Get specific service status
curl http://0.0.0.0:5071/api/v1/rag/status | jq
```

### Service Lifecycle Management

```bash
# Enable a service
curl -X POST http://0.0.0.0:5080/api/v1/warden/service/rag/enable

# Disable a service
curl -X POST http://0.0.0.0:5080/api/v1/warden/service/rag/disable
```

### Port Management

```bash
# Check if a port is in use
curl http://0.0.0.0:5080/api/v1/warden/port/check/5071 | jq

# Allocate a new port
curl -X POST http://0.0.0.0:5080/api/v1/warden/port/allocate \
  -H "Content-Type: application/json" \
  -d '{"service_name": "test_service", "preferred_port": 5100}'
```

---

## Common Tasks

### Add a New Service

1. Create the service directory:
```bash
cd /home/nibbles/Documents/hive_agent
cargo new --lib new_service
```

2. Add to workspace `Cargo.toml`:
```toml
[workspace]
members = [
    "rag",
    # ... other services ...
    "new_service"
]
```

3. Create `new_service/src/main.rs` with REST API
4. Add to `hive_agent-warden/deps/core_microservices.json`

### Check Project Structure

```bash
tree -L 2 -I 'target|.git'
```

### Run Tests

```bash
# Run all tests
cargo test

# Run tests for specific package
cargo test -p hive_agent-warden
```

### Clean Build Artifacts

```bash
cargo clean
```

### Format Code

```bash
cargo fmt
```

### Lint Code

```bash
cargo clippy
```

---

## Test the Director Service

The Director provides AI-powered monitoring with automatic person detection:

### Quick Test
```bash
# Check if director is running
curl http://localhost:6084/api/v1/hive_agent-director/status

# Should return:
{
  "service": "hive_agent-director",
  "status": "operational",
  "session_active": false,  # or true if person detected
  "session_directory": null  # or path to session folder
}
```

### How It Works
1. **Monitoring Mode** (default):
   - Captures images every 5 seconds
   - Sends to Vision LLM for person detection
   - Waits for triggers

2. **Session Mode** (when person detected):
   - Creates timestamped folder: `sessions/session_YYYYMMDD_HHMMSS/`
   - Saves trigger image
   - Captures every 30 seconds
   - Auto-ends after 60 minutes

### Configuration
Edit `director_config.json`:
```json
{
  "camera_url": "http://localhost:6082",
  "vision_llm_url": "http://192.168.0.46:5080/gim/llm_mid_visual/ask_question",
  "vision_llm_enabled": true,  # Set to false to disable detection
  "monitoring_interval": 5,
  "session_interval": 30,
  "session_timeout_minutes": 60
}
```

### Manual Session Control
```bash
# End an active session
curl -X POST http://localhost:6084/api/v1/hive_agent-director/session/end
```

## Troubleshooting

### Port Already in Use

If a service fails to start due to port conflicts:

```bash
# Find what's using a port
lsof -i :5080

# Kill the process
kill -9 <PID>
```

### Service Won't Start

Check the service logs:

```bash
# If running in warden managed mode
# Logs are in hive_agent-warden/logs/
ls -la ./target/release/logs/
```

### Compilation Errors

1. Verify Rust version: `rustc --version` (should be 1.70+)
2. Update dependencies: `cargo update`
3. Clean and rebuild: `cargo clean && cargo build`

### Connection Refused

Make sure services are actually running:

```bash
# List listening ports
netstat -tln | grep LISTEN

# Or use ss
ss -tln | grep LISTEN
```

---

## Architecture Overview

```
┌─────────────────────────────────────┐
│       Warden (5080)                 │
│   Central Orchestrator              │
├─────────────────────────────────────┤
│                                     │
│  ┌─────────────────────────────┐   │
│  │ Core Services               │   │
│  ├─────────────────────────────┤   │
│  │ RAG (5071)                  │   │
│  │ Generation Loop Services    │   │
│  │ Player Services             │   │
│  │ Director (5084)             │   │
│  │ Tools (5083)                │   │
│  │ Camera Server (5082)        │   │
│  └─────────────────────────────┘   │
│                                     │
└─────────────────────────────────────┘
```

---

## API Quick Reference

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/v1/warden/healthcheck/basic` | GET | Check warden health |
| `/api/v1/warden/status` | GET | Get warden status |
| `/api/v1/warden/services` | GET | List all services |
| `/api/v1/warden/service/{name}/enable` | POST | Enable a service |
| `/api/v1/warden/service/{name}/disable` | POST | Disable a service |
| `/api/v1/warden/port/allocate` | POST | Allocate port |
| `/api/v1/warden/port/check/{port}` | GET | Check port status |
| `/api/v1/{service}/healthcheck/basic` | GET | Service health check |
| `/api/v1/{service}/status` | GET | Service status |

For complete API documentation, see [API.md](./API.md)

---

## Development Workflow

1. **Make changes** to service code
2. **Run cargo check** to verify syntax
3. **Build** the service or entire workspace
4. **Start services** in development mode
5. **Test APIs** using curl
6. **Check logs** for debugging
7. **Format & lint** before committing

---

## File Structure

```
hive_agent/
├── Cargo.toml                          # Workspace configuration
├── core_microservices.json             # Service configuration (symlink)
├── docs/
│   ├── API.md                         # Complete API documentation
│   └── QUICKSTART.md                  # This file
├── hive_agent-warden/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                    # Warden orchestrator
│   │   └── lib.rs
│   └── deps/
│       └── core_microservices.json    # Service configuration
├── rag/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   └── lib.rs
├── hive_agent-*/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── lib.rs
└── target/
    ├── debug/                         # Debug binaries
    └── release/                       # Release binaries
```

---

## Next Steps

1. **Read the [full API documentation](./API.md)**
2. **Build the project**: `cargo build --release`
3. **Start the warden**: `./target/release/hive_agent-warden`
4. **Test endpoints**: Use the curl examples above
5. **Explore service code** and add custom endpoints

---

**Happy coding!** 🚀

For issues or questions, check the API documentation or service source code.
