# Hive Agent API Documentation

Complete API reference for the Hive Agent microservices ecosystem.

## Table of Contents

1. [Overview](#overview)
2. [System Architecture](#system-architecture)
3. [Warden Service API](#warden-service-api)
4. [Core Services API](#core-services-api)
5. [Data Types & Schemas](#data-types--schemas)
6. [Error Handling](#error-handling)
7. [Service Ports Reference](#service-ports-reference)

---

## Overview

Hive Agent is a distributed microservices system for AI agent processing. The system consists of 13 interconnected services managed by the **Warden**, a central orchestration and port management service.

### Key Features

- **Service Orchestration**: Warden manages service lifecycle and health
- **Port Management**: Dynamic port allocation and availability checking
- **Health Checking**: Continuous service health monitoring
- **REST API**: All services expose standard REST endpoints
- **Configuration Management**: Centralized service configuration via `core_microservices.json`

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Hive Agent Ecosystem                      │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Warden (Port 5080) - Central Orchestrator           │   │
│  │  • Service lifecycle management                      │   │
│  │  • Port allocation & health checking                 │   │
│  │  • Service enable/disable control                    │   │
│  └──────────────────────────────────────────────────────┘   │
│           ▲                                                   │
│           │                                                   │
│  ┌────────┴──────────────────────────────────────────────┐  │
│  │                                                        │  │
│  ▼                    ▼                    ▼              ▼  │
│ ┌─────────┐      ┌─────────┐         ┌──────────┐    ┌────┐│
│ │   RAG   │      │ Director│         │ Camera   │    │...││
│ │ 5071    │      │ 5084    │         │ Server   │    │    ││
│ │         │      │         │         │ 5082     │    │    ││
│ └─────────┘      └─────────┘         └──────────┘    └────┘│
│  ▼                    ▼                    ▼              ▼  │
│ ┌──────────────────────────────────────────────────────┐   │
│ │  Generation & Processing Services                     │   │
│ │  • Image-to-Text (5072)                              │   │
│ │  • Speech-to-Text (5073)                             │   │
│ │  • Text-to-Speech (5074)                             │   │
│ │  • Text Generation (5075)                            │   │
│ └──────────────────────────────────────────────────────┘   │
│  ▼                    ▼                    ▼              ▼  │
│ ┌──────────────────────────────────────────────────────┐   │
│ │  Player & Output Services                             │   │
│ │  • Image-to-Text Player (5076)                       │   │
│ │  • Audio Player (5077)                               │   │
│ │  • Text-to-Speech Player (5078)                      │   │
│ │  • Text Player (5079)                                │   │
│ │  • Tools (5083)                                       │   │
│ └──────────────────────────────────────────────────────┘   │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

---

## Warden Service API

**Base URL**: `http://0.0.0.0:6080`

The Warden is the central orchestration service that manages all other services. It provides automatic service management, health monitoring, and recovery capabilities.

### Key Features
- **Automatic Service Management**: Starts all enabled services on startup
- **Health Monitoring**: Continuous health checks every 10 seconds
- **Auto-Recovery**: Restarts failed or unhealthy services automatically
- **State Persistence**: All changes are saved to `core_microservices.json`
- **Process Management**: Full lifecycle control of service processes

### 1. Health Check

**Endpoint**: `GET /api/v1/warden/healthcheck/basic`

**Description**: Basic health check endpoint. Returns `true` if the warden is operational.

**Request**:
```bash
curl -X GET http://0.0.0.0:6080/api/v1/warden/healthcheck/basic
```

**Response**: 
```
200 OK
Content-Type: text/plain

true
```

**Type**: `boolean` (literal string "true")

---

### 2. Get Warden Status

**Endpoint**: `GET /api/v1/warden/status`

**Description**: Retrieve comprehensive status information about the warden and managed services.

**Request**:
```bash
curl -X GET http://0.0.0.0:6080/api/v1/warden/status
```

**Response**: 
```json
{
  "status": "operational",
  "services_count": 13,
  "ports_in_use": [5080, 5071, 5072, 5073, 5074, 5075, 5076, 5077, 5078, 5079, 5082, 5083, 5084],
  "timestamp": "2025-10-28T18:30:45.123456+00:00"
}
```

**Response Type**:
```typescript
{
  status: "operational",          // enum: "operational" | "degraded" | "failed"
  services_count: number,         // Total number of managed services
  ports_in_use: number[],        // Array of currently allocated ports
  timestamp: string              // ISO 8601 formatted timestamp
}
```

---

### 3. Get All Services

**Endpoint**: `GET /api/v1/warden/services`

**Description**: Retrieve information about all managed services.

**Request**:
```bash
curl -X GET http://0.0.0.0:6080/api/v1/warden/services
```

**Response**:
```json
[
  {
    "name": "hive_agent-warden",
    "port": 5080,
    "enabled": true,
    "healthy": true,
    "running": true,
    "version": "0.1.0"
  },
  {
    "name": "rag",
    "port": 5071,
    "enabled": true,
    "healthy": false,
    "running": false,
    "version": "0.1.0"
  }
]
```

**Response Type**:
```typescript
ServiceConfig[] = [
  {
    name: string,               // Service identifier (e.g., "rag", "hive_agent-director")
    port: number,              // Allocated port (5000-6000 range)
    enabled: boolean,          // Whether service is enabled
    healthy: boolean,          // Current health status
    running: boolean,          // Whether service process is running
    version: string            // Semantic version of service
  }
]
```

---

### 4. Enable Service

**Endpoint**: `POST /api/v1/warden/service/{name}/enable`

**Description**: Enable a specific service by name. Sets enabled flag to true.

**Parameters**:
- `{name}` (path): Service name (e.g., "rag", "hive_agent-director")

**Request**:
```bash
curl -X POST http://0.0.0.0:6080/api/v1/warden/service/rag/enable
```

**Response**: 
```json
{
  "status": "success",
  "message": "rag enabled",
  "service": {
    "name": "rag",
    "port": 5071,
    "enabled": true,
    "healthy": false,
    "running": false,
    "version": "0.1.0"
  }
}
```

**Error Response**:
```json
{
  "status": "error",
  "message": "Service unknown_service not found"
}
```

---

### 5. Disable Service

**Endpoint**: `POST /api/v1/warden/service/{name}/disable`

**Description**: Disable a specific service by name. Stops the service and sets enabled flag to false.

**Parameters**:
- `{name}` (path): Service name

**Request**:
```bash
curl -X POST http://0.0.0.0:6080/api/v1/warden/service/rag/disable
```

**Response**:
```json
{
  "status": "success",
  "message": "rag disabled",
  "service": {
    "name": "rag",
    "port": 5071,
    "enabled": false,
    "healthy": false,
    "running": false,
    "version": "0.1.0"
  }
}
```

---

### 6. Allocate Port

**Endpoint**: `POST /api/v1/warden/port/allocate`

**Description**: Allocate a port for a service. If the preferred port is unavailable, automatically finds an alternative.

**Request Body**:
```json
{
  "service_name": "my_service",
  "preferred_port": 5100
}
```

**Request**:
```bash
curl -X POST http://0.0.0.0:6080/api/v1/warden/port/allocate \
  -H "Content-Type: application/json" \
  -d '{
    "service_name": "my_service",
    "preferred_port": 5100
  }'
```

**Response** (Port Available):
```json
{
  "status": "success",
  "service": "my_service",
  "port": 5100
}
```

**Response** (Port Unavailable - Auto-reassigned):
```json
{
  "status": "reassigned",
  "service": "my_service",
  "requested_port": 5100,
  "assigned_port": 5101
}
```

**Error Response**:
```json
{
  "status": "error",
  "message": "No available ports found"
}
```

**Request Type**:
```typescript
{
  service_name: string,    // Name of the service requiring a port
  preferred_port: number   // Requested port number (5000-6000)
}
```

---

### 7. Check Port Status

**Endpoint**: `GET /api/v1/warden/port/check/{port}`

**Description**: Check if a specific port is currently in use.

**Parameters**:
- `{port}` (path): Port number to check

**Request**:
```bash
curl -X GET http://0.0.0.0:6080/api/v1/warden/port/check/5071
```

**Response**:
```json
{
  "port": 5071,
  "in_use": true
}
```

**Response Type**:
```typescript
{
  port: number,           // The checked port number
  in_use: boolean        // Whether the port is currently in use
}
```

---

## Core Services API

All core services (non-warden) expose the following standard endpoints:

### Service Base URLs

- **RAG**: `http://0.0.0.0:6071`
- **Image-to-Text Generation**: `http://0.0.0.0:6072`
- **Speech-to-Text Generation**: `http://0.0.0.0:6073`
- **Text-to-Speech Generation**: `http://0.0.0.0:6074`
- **Text Generation**: `http://0.0.0.0:6075`
- **Image-to-Text Player**: `http://0.0.0.0:6076`
- **Audio Player**: `http://0.0.0.0:6077`
- **Text-to-Speech Player**: `http://0.0.0.0:6078`
- **Text Player**: `http://0.0.0.0:6079`
- **Camera Server**: `http://0.0.0.0:6082` (Cross-platform camera capture)
- **Tools**: `http://0.0.0.0:6083`
- **Director**: `http://0.0.0.0:6084`

### 1. Health Check (All Services)

**Endpoint**: `GET /api/v1/{service_name}/healthcheck/basic`

**Description**: Health check endpoint for any service. Returns `true` if the service is healthy.

**Request**:
```bash
curl -X GET http://0.0.0.0:6071/api/v1/rag/healthcheck/basic
```

**Response**:
```
200 OK
Content-Type: text/plain

true
```

**Response Type**: `boolean` (literal string "true")

---

### 2. Service Status (All Services)

**Endpoint**: `GET /api/v1/{service_name}/status`

**Description**: Retrieve status information for a specific service.

**Request**:
```bash
curl -X GET http://0.0.0.0:6071/api/v1/rag/status
```

**Response**:
```json
{
  "service": "rag",
  "status": "operational",
  "version": "0.1.0"
}
```

**Response Type**:
```typescript
{
  service: string,              // Service name
  status: "operational" | "degraded" | "failed",  // Current status
  version: string              // Service version (semantic versioning)
}
```

---

## Director Service API

The Director provides intelligent monitoring with Vision LLM integration for trigger-based actions.

### Director Endpoints

#### 1. Status

**Endpoint**: `GET /api/v1/hive_agent-director/status`

Returns current director status including session state.

**Response**:
```json
{
  "service": "hive_agent-director",
  "status": "operational",
  "session_active": false,
  "session_directory": null
}
```

When a session is active:
```json
{
  "service": "hive_agent-director",
  "status": "operational",
  "session_active": true,
  "session_directory": "generated_image_captures/sessions/session_20251029_021346"
}
```

#### 2. End Session

**Endpoint**: `POST /api/v1/hive_agent-director/session/end`

Manually ends the current session and returns to monitoring mode.

**Response**:
```json
{
  "message": "Session ended"
}
```

### Director Configuration

The director uses a JSON configuration file (`director_config.json`) with a flexible trigger system:

```json
{
  "camera": {
    "url": "http://localhost:6082",
    "monitoring_interval_seconds": 5,
    "session_interval_seconds": 30,
    "session_timeout_minutes": 60
  },
  
  "visual_trigger_detection": {
    "endpoint": "http://192.168.0.46:5080/gim/llm_mid_visual/ask_question",
    "enabled": true,
    "timeout_ms": 30000,
    "max_tokens": 50,
    "active_trigger": "person_detection",
    "triggers": {
      "person_detection": {
        "enabled": true,
        "prompt": "Is there a person in this image? Answer only 'true' or 'false'.",
        "positive_keywords": ["true", "yes", "person", "people", "human"]
      },
      "vehicle_detection": {
        "enabled": false,
        "prompt": "Is there a vehicle in this image?",
        "positive_keywords": ["true", "yes", "vehicle", "car", "truck"]
      },
      "animal_detection": {
        "enabled": false,
        "prompt": "Is there an animal in this image?",
        "positive_keywords": ["true", "yes", "animal", "dog", "cat", "pet"]
      },
      // Additional triggers: motion_detection, package_detection, anomaly_detection
    }
  },
  
  "scene_analysis": {
    "enabled": true,
    "prompt": "Describe what you see in detail...",
    "timeout_ms": 30000,
    "max_tokens": 500
  },
  
  "response_generation": {
    "endpoint": "http://192.168.0.46:5080/gim/llm_mid/ask_question",
    "enabled": true,
    "prompt": "Generate a friendly greeting...",
    "timeout_ms": 30000,
    "max_tokens": 200
  }
}
```

### Operating Modes

1. **Monitoring Mode**:
   - Captures images at configured interval (default: 5 seconds)
   - Sends images to Vision LLM for trigger detection
   - Checks active trigger type (person, vehicle, animal, motion, package, or anomaly)
   - Uses positive keywords to determine if trigger condition is met

2. **Active Session Mode**:
   - Activated when the configured trigger is detected
   - Creates timestamped session folder with trigger type recorded
   - Captures images at session interval (default: 30 seconds)
   - Performs scene analysis using Vision LLM (if enabled)
   - Generates contextual response using Text LLM (if enabled)
   - Saves trigger image, analysis, and generated response
   - Automatically ends after timeout or manual intervention

### Session Structure

When triggered, the director creates:
```
generated_image_captures/sessions/session_YYYYMMDD_HHMMSS/
├── trigger.png              # The image that triggered the session
├── capture_HHMMSS.png       # Same image, first in sequence
├── capture_HHMMSS.png       # Subsequent captures at session interval
├── analysis.txt             # Scene analysis from Vision LLM (if enabled)
├── generated_speech.txt     # Response from Text LLM (if enabled)
└── session_info.json        # Complete session metadata including:
    ├── timestamp            # When session started
    ├── trigger_type         # Which trigger activated (e.g., "person_detection")
    ├── analysis             # Full scene analysis text
    ├── generated_speech     # Generated response text
    └── config               # Snapshot of analysis and generation configs
```

---

## Camera Server Specific API

The Camera Server provides additional endpoints for camera capture beyond the standard service endpoints.

### Additional Camera Endpoints

#### 1. Capture Image

**Endpoint**: `GET /capture-image`

Captures a single frame from the camera and saves it as PNG.

**Response**:
```json
{
  "ok": true,
  "filename": "generated_image_captures/captured_image_1.png",
  "counter": 1,
  "message": "Image saved as generated_image_captures/captured_image_1.png",
  "error": null
}
```

#### 2. List Available Cameras

**Endpoint**: `GET /cameras`

Enumerates all detected camera devices on the system.

**Response**:
```json
{
  "cameras": [
    {
      "index": 0,
      "name": "C922 Pro Stream Webcam",
      "description": "Video4Linux Device @ /dev/video0",
      "backend": "Index(0)"
    }
  ],
  "count": 1,
  "platform": "Linux"
}
```

#### 3. Advanced Health Check

**Endpoint**: `GET /health`

Returns detailed health status including camera availability.

**Response**:
```json
{
  "status": "ok",
  "camera_active": true,
  "camera_index": 0,
  "platform": "Linux"
}
```

**Platform Support**:
- **Linux**: V4L2 backend
- **Windows**: Windows Media Foundation
- **macOS**: AVFoundation

**Error Handling**:
- Returns 503 Service Unavailable if no camera is detected
- Images are saved to `generated_image_captures/` directory
- Each capture increments a counter for unique filenames

---

## Data Types & Schemas

### ServiceConfig

Used in warden responses to describe a managed service.

```typescript
interface ServiceConfig {
  name: string;                           // Unique service identifier
  port: number;                           // Allocated port (range: 5000-6000)
  enabled: boolean;                       // Administrative enable/disable flag
  healthy: boolean;                       // Health check status (read-only during normal operation)
  running: boolean;                       // Process running state (read-only)
  version: string;                        // Semantic version (e.g., "0.1.0")
  
  // Optional fields (present in configuration file only)
  uuid?: string;                          // Unique identifier for service instance
  failed?: boolean;                       // Failure flag set when max attempts exceeded
  boot_attempts?: number;                 // Max bootstrap attempts before marking failed
  boot_timeout_millisecs?: number;        // Timeout for service to become healthy (ms)
  healthcheck_attempts?: number;          // Max consecutive failed health checks
  healthcheck_timeout_millisecs?: number; // Timeout for health check request (ms)
  health_path?: string;                   // Relative path to health check endpoint
  assigned_cores?: number[];              // (Future) CPU cores assigned to service
}
```

### WardenStatus

Response from warden status endpoint.

```typescript
interface WardenStatus {
  status: "operational" | "degraded" | "failed";
  services_count: number;                 // Total managed services
  ports_in_use: number[];                 // List of currently allocated ports
  timestamp: string;                      // ISO 8601 timestamp
}
```

### ServiceStatus

Response from individual service status endpoint.

```typescript
interface ServiceStatus {
  service: string;
  status: "operational" | "degraded" | "failed";
  version: string;
}
```

### ErrorResponse

Standard error response format.

```typescript
interface ErrorResponse {
  status: "error";
  message: string;                        // Human-readable error description
}
```

### PortAllocationResponse

Response from port allocation endpoint.

```typescript
interface PortAllocationResponse {
  status: "success" | "reassigned" | "error";
  service: string;
  port?: number;                          // Assigned port (success only)
  requested_port?: number;                // Original requested port (reassigned only)
  assigned_port?: number;                 // New assigned port (reassigned only)
  message?: string;                       // Error message (error only)
}
```

---

## Error Handling

### HTTP Status Codes

| Code | Meaning | Example |
|------|---------|---------|
| 200 | Success | Service found and operation completed |
| 400 | Bad Request | Missing required fields in request body |
| 404 | Not Found | Service doesn't exist |
| 500 | Internal Server Error | Warden or service internal error |

### Error Response Format

All error responses follow this format:

```json
{
  "status": "error",
  "message": "Descriptive error message"
}
```

### Common Error Scenarios

**Service Not Found**:
```json
{
  "status": "error",
  "message": "Service unknown_service not found"
}
```

**No Available Ports**:
```json
{
  "status": "error",
  "message": "No available ports found"
}
```

**Missing Required Fields**:
```json
{
  "status": "error",
  "message": "Missing required fields: service_name, preferred_port"
}
```

---

## Service Ports Reference

**IMPORTANT**: Ports are **dynamically assigned** by the warden based on system availability. The ports shown below are **examples/preferences only**.

### Dynamic Port Assignment
The warden automatically:
1. Attempts to use the preferred port from configuration
2. Finds an alternative port if preferred is unavailable
3. Updates the configuration with actual assigned port
4. Passes the assigned port to the service at startup

### Finding Current Ports
```bash
# Get actual ports currently in use
curl http://localhost:6080/api/v1/warden/services | python3 -c "
import sys, json
services = json.load(sys.stdin)
for s in services:
    print(f\"{s['name']}: {s['port']}\")"
```

### Example Port Assignments
| Service | Preferred Port* | Status Endpoint | Health Check |
|---------|----------------|---|---|
| Warden | 6080 | `/api/v1/warden/status` | `/api/v1/warden/healthcheck/basic` |
| RAG | 6071 | `/api/v1/rag/status` | `/api/v1/rag/healthcheck/basic` |
| Image-to-Text Generation | 6072 | `/api/v1/hive_agent-image-to-text-generation-loop/status` | `/api/v1/hive_agent-image-to-text-generation-loop/healthcheck/basic` |
| Speech-to-Text Generation | 6073 | `/api/v1/hive_agent-speech-to-text-generation-loop/status` | `/api/v1/hive_agent-speech-to-text-generation-loop/healthcheck/basic` |
| Text-to-Speech Generation | 6074 | `/api/v1/hive_agent-text-to-speech-generation-loop/status` | `/api/v1/hive_agent-text-to-speech-generation-loop/healthcheck/basic` |
| Text Generation | 6075 | `/api/v1/hive_agent-text-generation-loop/status` | `/api/v1/hive_agent-text-generation-loop/healthcheck/basic` |
| Image-to-Text Player | 6076 | `/api/v1/hive_agent-image-to-text-player-loop/status` | `/api/v1/hive_agent-image-to-text-player-loop/healthcheck/basic` |
| Audio Player | 6077 | `/api/v1/hive_agent-audio-player/status` | `/api/v1/hive_agent-audio-player/healthcheck/basic` |
| Text-to-Speech Player | 6078 | `/api/v1/hive_agent-text-to-speech-player-loop/status` | `/api/v1/hive_agent-text-to-speech-player-loop/healthcheck/basic` |
| Text Player | 6079 | `/api/v1/hive_agent-text-player-loop/status` | `/api/v1/hive_agent-text-player-loop/healthcheck/basic` |
| Camera Server | 6082 | `/api/v1/hive_agent-camera-server/status` | `/api/v1/hive_agent-camera-server/healthcheck/basic` |
| Tools | 6083 | `/api/v1/hive_agent-tools/status` | `/api/v1/hive_agent-tools/healthcheck/basic` |
| Director | 6084 | `/api/v1/hive_agent-director/status` | `/api/v1/hive_agent-director/healthcheck/basic` |

**\* Actual ports may differ based on availability**

---

## Usage Examples

### Complete Workflow Example

```bash
# 1. Check warden health
curl http://0.0.0.0:6080/api/v1/warden/healthcheck/basic

# 2. Get all services
curl http://0.0.0.0:6080/api/v1/warden/services

# 3. Check RAG service health
curl http://0.0.0.0:6071/api/v1/rag/healthcheck/basic

# 4. Get RAG service status
curl http://0.0.0.0:6071/api/v1/rag/status

# 5. Check if port 6100 is available
curl http://0.0.0.0:6080/api/v1/warden/port/check/6100

# 6. Allocate a port for a new service
curl -X POST http://0.0.0.0:6080/api/v1/warden/port/allocate \
  -H "Content-Type: application/json" \
  -d '{"service_name": "custom_service", "preferred_port": 6100}'

# 7. Enable a service
curl -X POST http://0.0.0.0:6080/api/v1/warden/service/rag/enable

# 8. Disable a service
curl -X POST http://0.0.0.0:6080/api/v1/warden/service/rag/disable
```

### cURL Examples with Pretty Output

```bash
# Get warden status with formatted output
curl -s http://0.0.0.0:5080/api/v1/warden/status | jq .

# Get all services with formatted output
curl -s http://0.0.0.0:5080/api/v1/warden/services | jq .

# Enable service and display response
curl -s -X POST http://0.0.0.0:5080/api/v1/warden/service/rag/enable | jq .
```

---

## Configuration Reference

The `core_microservices.json` file located at `hive_agent-warden/deps/core_microservices.json` contains the service configuration:

```json
[
  {
    "name": "service_name",
    "uuid": "550e8400-e29b-41d4-a716-446655440001",
    "enabled": true,
    "running": false,
    "healthy": false,
    "failed": false,
    "boot_attempts": 3,
    "boot_timeout_millisecs": 5000,
    "healthcheck_attempts": 3,
    "healthcheck_timeout_millisecs": 5000,
    "port": 5071,
    "version": "0.1.0",
    "health_path": "api/v1/service_name/healthcheck/basic"
  }
]
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-10-28 | Initial API documentation with 13 services |

---

**Last Updated**: October 28, 2025
**Documentation Version**: 1.0
**API Version**: 1.0
