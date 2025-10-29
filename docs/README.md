# Hive Agent Documentation

Complete documentation for the Hive Agent microservices ecosystem.

## Documentation Files

### 🛡️ [WARDEN.md](./WARDEN.md) - Comprehensive Warden Documentation (NEW!)
**The most important component** - Complete guide to the service orchestrator:
- **Architecture** - Internal components and design
- **Service Management** - Automatic starting, stopping, and monitoring
- **Health Monitoring** - Continuous checks and auto-recovery
- **Configuration** - Understanding `core_microservices.json`
- **Development Guide** - Adding services and modifying behavior
- **Troubleshooting** - Common issues and solutions
- **Best Practices** - Production deployment guidelines

**Use this when you need:**
- Understanding how services are managed
- Configuring automatic recovery
- Adding new services to the ecosystem
- Debugging service issues
- Production deployment

---

### 📷 [CAMERA_SERVER.md](./CAMERA_SERVER.md) - Camera Server Documentation (NEW!)
**Cross-platform camera capture service** - Complete guide to the camera server:
- **Platform Support** - Windows, macOS, and Linux compatibility
- **API Endpoints** - Image capture, camera listing, and status
- **Integration Examples** - Working with other Hive Agent services
- **Configuration** - Port settings and warden integration
- **Troubleshooting** - Platform-specific issues and solutions
- **Development Guide** - Adding features and customization

**Use this when you need:**
- Setting up camera capture capabilities
- Integrating image capture into workflows
- Debugging camera-related issues
- Understanding cross-platform camera support
- Building vision-based features

---

### 🤖 Director Service - AI-Powered Monitoring Orchestrator (NEW!)
**Intelligent trigger-based monitoring system** with Vision LLM integration:
- **Dual-Mode Operation** - Monitoring mode (5s) and Active Session mode (30s)
- **Vision LLM Integration** - Works with HiveMind or Ollama for person detection
- **Session Management** - Automatic session creation with timestamped folders
- **Configurable Triggers** - Define custom detection conditions
- **Auto-Timeout** - Sessions automatically end after configured duration

**Key Features:**
- Monitors camera feed every 5 seconds
- Uses Vision LLM to detect persons or other triggers
- Creates timestamped session folders when triggered
- Switches to 30-second capture interval during sessions
- Saves trigger image and all session captures
- Returns to monitoring after timeout or manual end

**Configuration** (`director_config.json`):
```json
{
  "camera_url": "http://localhost:6082",
  "vision_llm_url": "http://192.168.0.46:5080/gim/llm_mid_visual/ask_question",
  "vision_llm_enabled": true,
  "monitoring_interval": 5,
  "session_interval": 30,
  "session_timeout_minutes": 60
}
```

---

### 📚 [API.md](./API.md) - Complete API Reference
Comprehensive documentation of all REST APIs including:
- **Warden Service API** - 7 endpoints for orchestration and port management
- **Core Services API** - Standard endpoints for all 12 services
- **Data Types & Schemas** - TypeScript interfaces for all request/response types
- **Error Handling** - HTTP status codes and error responses
- **Service Ports Reference** - Quick lookup table for all services
- **Usage Examples** - cURL commands and workflow examples

**Key Sections:**
- System Architecture diagram
- All 13 services with their endpoints
- Request/response examples with types
- Configuration reference

**Use this when you need:**
- Details about a specific endpoint
- Request/response format examples
- Understanding data types
- Error handling patterns

---

### 🚀 [QUICKSTART.md](./QUICKSTART.md) - Getting Started Guide
Quick reference for developers to get started:
- **Using the Warden** - Recommended way to run services
- Prerequisites and setup
- Building the project
- Managing services via API
- Testing endpoints
- Common tasks
- Troubleshooting

**Key Sections:**
- Warden-based service management (recommended)
- Build commands (full workspace or individual services)
- Service control via API
- Testing with curl
- Common development tasks
- Architecture overview

**Use this when you:**
- First start working with Hive Agent
- Need build/run commands
- Want to control services
- Need quick API reference table

---

## Service Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    WARDEN (5080)                         │
│           Central Orchestration Service                  │
│                                                           │
│  Features:                                               │
│  • Automatic Service Starting                            │
│  • Health Monitoring (every 10s)                         │
│  • Auto-Recovery (restarts failed services)              │
│  • State Persistence                                     │
│  • Process Management                                    │
└──────────────────┬───────────────────────────────────────┘
                   │ Manages & Monitors
        ┌──────────┼──────────┬────────────┐
        │          │          │            │
   ┌────▼───┐ ┌────▼────┐ ┌──▼──┐    ┌───▼───┐
   │  RAG   │ │Director │ │Tools│ ...│Camera │
   │ 5071   │ │ 5084    │ │5083 │    │ 5082  │
   └────────┘ └─────────┘ └─────┘    └───────┘
        │          │          │            │
   ┌────────────────────────────────────────┐
   │  Generation & Player Services           │
   │  • Image/Speech/Text-to-Text (5072-75) │
   │  • Image/Audio/Text/TTS Players (5076)│
   └────────────────────────────────────────┘
```

---

## 13 Services Overview

**Note**: Ports shown are **preferred/example** values. The warden dynamically assigns available ports at runtime.

| Service | Example Port* | Purpose |
|---------|--------------|---------|
| **Warden** | 6080 | Central orchestrator, port management, service lifecycle |
| **RAG** | 6071 | Retrieval-Augmented Generation service |
| **Image-to-Text Generation** | 6072 | Convert images to text descriptions |
| **Speech-to-Text Generation** | 6073 | Convert audio to text transcription |
| **Text-to-Speech Generation** | 6074 | Convert text to audio |
| **Text Generation** | 6075 | Generate text from prompts |
| **Image-to-Text Player** | 6076 | Play/display image-to-text results |
| **Audio Player** | 6077 | Play audio output |
| **Text-to-Speech Player** | 6078 | Play text-to-speech output |
| **Text Player** | 6079 | Display text output |
| **Camera Server** | 6082 | Cross-platform camera capture (Windows/macOS/Linux) |
| **Tools** | 6083 | Utility tools and helpers |
| **Director** | 6084 | AI monitoring with Vision LLM person detection |

**\* Actual ports are dynamically assigned based on availability**

### Finding Actual Ports
```bash
# Get current port assignments from warden
curl http://localhost:6080/api/v1/warden/services | \
  python3 -m json.tool | grep -E '"name"|"port"'
```

---

## Getting Started

1. **First Time?** → Read [QUICKSTART.md](./QUICKSTART.md)
2. **Understanding the Warden?** → Read [WARDEN.md](./WARDEN.md)
3. **Need API Details?** → Check [API.md](./API.md)
4. **Quick Start:**
   ```bash
   # Build everything
   cargo build --release
   
   # Start warden (automatically starts all services)
   ./target/release/hive_agent-warden
   
   # Check service status
   curl http://localhost:6080/api/v1/warden/services
   ```

---

## Common Tasks

### Check Documentation
```bash
# View API documentation (Warden endpoints)
less docs/API.md

# View quickstart guide
less docs/QUICKSTART.md
```

### Build and Run
```bash
# Build entire workspace
cargo build --release

# Run the warden
./target/release/hive_agent-warden

# In another terminal, run a service
./target/release/rag
```

### Test APIs
```bash
# Check warden health
curl http://0.0.0.0:5080/api/v1/warden/healthcheck/basic

# Get all services
curl http://0.0.0.0:5080/api/v1/warden/services | jq

# Get RAG status
curl http://0.0.0.0:5071/api/v1/rag/status | jq
```

---

## API Endpoint Quick Reference

### Warden Endpoints
- `GET /api/v1/warden/healthcheck/basic` - Health check
- `GET /api/v1/warden/status` - Warden status
- `GET /api/v1/warden/services` - List all services
- `POST /api/v1/warden/service/{name}/enable` - Enable service
- `POST /api/v1/warden/service/{name}/disable` - Disable service
- `POST /api/v1/warden/port/allocate` - Allocate port
- `GET /api/v1/warden/port/check/{port}` - Check port status

### Service Endpoints (All Services)
- `GET /api/v1/{service}/healthcheck/basic` - Health check
- `GET /api/v1/{service}/status` - Service status

**Full details in [API.md](./API.md)**

---

## Configuration

Service configuration is located at:
```
hive_agent-warden/deps/core_microservices.json
```

Each service entry includes:
- Service name and UUID
- Port assignment
- Enable/disable state
- Health check timeouts
- Boot attempt limits
- Version information

---

## Project Structure

```
hive_agent/
├── Cargo.toml                    # Workspace root
├── docs/
│   ├── README.md               # This file
│   ├── API.md                  # Complete API docs
│   └── QUICKSTART.md           # Getting started guide
├── hive_agent-warden/
│   ├── src/main.rs             # Warden implementation
│   └── deps/
│       └── core_microservices.json  # Service config
├── rag/
├── hive_agent-image-to-text-*/
├── hive_agent-speech-to-text-*/
├── hive_agent-text-to-speech-*/
├── hive_agent-text-generation-loop/
├── hive_agent-*-player-loop/
├── hive_agent-audio-player/
├── hive_agent-text-player-loop/
├── hive_agent-camera-server/
├── hive_agent-tools/
├── hive_agent-director/
└── target/
    ├── debug/
    └── release/
```

---

## Development Guidelines

1. **Code Style**: Run `cargo fmt` before committing
2. **Linting**: Check with `cargo clippy`
3. **Testing**: Use `cargo test`
4. **Building**: `cargo build --release` for production
5. **API Design**: Follow REST conventions used in existing services

---

## Support Resources

### Internal Documentation
- [API.md](./API.md) - Detailed API reference with examples
- [QUICKSTART.md](./QUICKSTART.md) - Development quick reference

### External Resources
- [Actix Web Documentation](https://actix.rs/)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)

---

## FAQ

**Q: How do I add a new service?**
A: See [QUICKSTART.md - Add a New Service](./QUICKSTART.md#add-a-new-service)

**Q: What's the port range?**
A: Services use ports 6000-7000, currently allocated 6071-6084

**Q: How do I test if a service is running?**
A: Use `curl http://0.0.0.0:{port}/api/v1/{service}/healthcheck/basic`

**Q: Where are service logs?**
A: Check the service's stdout or `hive_agent-warden/logs/` directory

**Q: How do I change a service's port?**
A: Edit `core_microservices.json` and update the `port` field

---

## Documentation Version

- **Last Updated**: October 28, 2025
- **Documentation Version**: 1.0
- **Hive Agent Version**: 0.1.0
- **API Version**: 1.0

---

**Questions?** Check the relevant documentation file or review service source code.

**Ready to start?** → [QUICKSTART.md](./QUICKSTART.md) 🚀
