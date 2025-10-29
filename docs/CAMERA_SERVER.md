# Hive Agent Camera Server Documentation

## 📷 Overview

The Hive Agent Camera Server is a cross-platform microservice that provides camera capture capabilities to the Hive Agent ecosystem. Built with Rust and using the `nokhwa` library, it offers a RESTful API for image capture, camera enumeration, and status monitoring.

---

## ✨ Key Features

- **Cross-Platform Support**: Works on Linux, Windows, and macOS
- **Auto-Detection**: Automatically discovers and configures available cameras
- **RESTful API**: Simple HTTP endpoints for all operations
- **Warden Integration**: Fully managed by the Hive Agent Warden
- **Graceful Degradation**: Continues running even without camera hardware
- **Multiple Format Support**: Handles MJPEG, RGB, and other common formats
- **Dynamic Port Assignment**: Accepts port configuration from warden

---

## 🖥️ Platform Support

| Platform | Camera Backend | Status | Notes |
|----------|---------------|--------|-------|
| **Linux** | Video4Linux (V4L2) | ✅ Fully Supported | Best performance, native support |
| **Windows** | Windows Media Foundation | ✅ Fully Supported | Requires Windows 10+ |
| **macOS** | AVFoundation | ✅ Fully Supported | Requires macOS 10.14+ |

---

## 🚀 Getting Started

### Prerequisites

- Rust 1.70+ installed
- Camera device (optional - server runs without it)
- Hive Agent Warden running

### Building

```bash
# Build the camera server
cd /home/nibbles/Documents/hive_agent
cargo build --release -p hive_agent-camera-server
```

### Running

#### Via Warden (Recommended)

The warden automatically starts and manages the camera server:

```bash
# Start the warden - it will start all services including camera server
./target/release/hive_agent-warden
```

#### Standalone Mode

```bash
# Run with default port (6082)
./target/release/hive_agent-camera-server

# Run with custom port
./target/release/hive_agent-camera-server --port 8080
```

---

## 📡 API Endpoints

### Base URL
```
http://localhost:6082
```

### 1. Capture Image

Capture a single frame from the camera and save it as PNG.

**Endpoint**: `GET /capture-image`  
**Alternative**: `GET /api/v1/hive_agent-camera-server/capture`

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

**Error Response**:
```json
{
  "ok": false,
  "filename": null,
  "counter": null,
  "message": "No camera available",
  "error": "Camera not initialized"
}
```

### 2. List Available Cameras

Enumerate all detected camera devices.

**Endpoint**: `GET /cameras`

**Response**:
```json
{
  "cameras": [
    {
      "index": 0,
      "name": "C922 Pro Stream Webcam",
      "description": "Video4Linux Device @ /dev/video0",
      "backend": "Index(0)"
    },
    {
      "index": 1,
      "name": "Built-in Camera",
      "description": "Video4Linux Device @ /dev/video1",
      "backend": "Index(1)"
    }
  ],
  "count": 2,
  "platform": "Linux"
}
```

### 3. Service Status

Get detailed status of the camera server.

**Endpoint**: `GET /api/v1/hive_agent-camera-server/status`

**Response**:
```json
{
  "service": "hive_agent-camera-server",
  "status": "operational",
  "version": "0.1.0",
  "platform": "Linux",
  "camera_active": true,
  "camera_index": 0,
  "camera_info": {
    "name": "C922 Pro Stream Webcam",
    "description": "uvcvideo",
    "backend": "Index(0)"
  },
  "capture_count": 5,
  "camera_library": "nokhwa (cross-platform)"
}
```

### 4. Health Check

Basic health check for warden monitoring.

**Endpoint**: `GET /api/v1/hive_agent-camera-server/healthcheck/basic`

**Response**: `true` (plain text)

### 5. Advanced Health Check

Detailed health status including camera availability.

**Endpoint**: `GET /health`

**Response**:
```json
{
  "status": "ok",
  "camera_active": true,
  "camera_index": 0,
  "platform": "Linux"
}
```

---

## ⚙️ Configuration

### Port Configuration

The camera server determines its port in the following priority order:

1. **Command Line Argument**: `--port 6082`
2. **Environment Variable**: `WARDEN_ASSIGNED_PORT=6082`
3. **Environment Variable**: `SERVICE_PORT=6082`
4. **Default**: `6082`

### Warden Configuration

In `core_microservices.json`:

```json
{
  "name": "hive_agent-camera-server",
  "uuid": "550e8400-e29b-41d4-a716-446655440011",
  "enabled": true,
  "running": false,
  "healthy": false,
  "failed": false,
  "boot_attempts": 3,
  "boot_timeout_millisecs": 5000,
  "healthcheck_attempts": 3,
  "healthcheck_timeout_millisecs": 5000,
  "port": 6082,
  "version": "0.1.0",
  "health_path": "api/v1/hive_agent-camera-server/healthcheck/basic"
}
```

### Output Directory

Captured images are saved to: `generated_image_captures/`

The directory is created automatically if it doesn't exist.

---

## 🔗 Integration Examples

### With Other Hive Agent Services

#### 1. Image-to-Text Pipeline

```bash
# Capture an image
IMAGE=$(curl -s http://localhost:6082/capture-image | jq -r .filename)

# Send to image-to-text service (example)
curl -X POST http://localhost:6072/api/v1/hive_agent-image-to-text-generation-loop/process \
  -H "Content-Type: application/json" \
  -d "{\"image_path\": \"$IMAGE\"}"
```

#### 2. Director Integration

The Director service can trigger camera captures for decision-making:

```python
# Python example for director service
import requests

def capture_for_analysis():
    # Capture image
    response = requests.get("http://localhost:6082/capture-image")
    if response.json()["ok"]:
        image_path = response.json()["filename"]
        # Process with other services
        return analyze_image(image_path)
```

#### 3. Monitoring Script

```bash
#!/bin/bash
# Monitor camera status
while true; do
    STATUS=$(curl -s http://localhost:6082/api/v1/hive_agent-camera-server/status)
    ACTIVE=$(echo $STATUS | jq -r .camera_active)
    COUNT=$(echo $STATUS | jq -r .capture_count)
    
    echo "Camera Active: $ACTIVE | Captures: $COUNT"
    sleep 10
done
```

---

## 🛠️ Development Guide

### Architecture

```
┌─────────────────────────────────────────┐
│          Camera Server                  │
├─────────────────────────────────────────┤
│  REST API Layer (Actix Web)            │
├─────────────────────────────────────────┤
│  Camera Abstraction Layer              │
├─────────────────────────────────────────┤
│  nokhwa Library                        │
├─────────────────────────────────────────┤
│  Platform-Specific Backend             │
│  ├── Linux: V4L2                       │
│  ├── Windows: Media Foundation         │
│  └── macOS: AVFoundation              │
└─────────────────────────────────────────┘
```

### Adding New Features

#### 1. Add Video Streaming

```rust
// In main.rs
async fn video_stream(data: web::Data<Arc<Mutex<AppState>>>) -> HttpResponse {
    // Implement MJPEG streaming
    // Use Server-Sent Events or WebSocket
}

// Add route
.route("/stream", web::get().to(video_stream))
```

#### 2. Add Camera Settings Control

```rust
#[derive(Deserialize)]
struct CameraSettings {
    resolution: (u32, u32),
    fps: u32,
    format: String,
}

async fn update_settings(
    data: web::Data<Arc<Mutex<AppState>>>,
    settings: web::Json<CameraSettings>
) -> HttpResponse {
    // Apply new camera settings
}
```

#### 3. Add Image Processing

```rust
// Add image processing capabilities
use image::{imageops, DynamicImage};

fn process_image(img: DynamicImage) -> DynamicImage {
    // Apply filters, resize, etc.
    imageops::resize(&img, 640, 480, imageops::FilterType::Lanczos3)
}
```

### Dependencies

Key dependencies in `Cargo.toml`:

```toml
[dependencies]
actix-web = "4"          # Web framework
actix-cors = "0.7"       # CORS support
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"            # Error handling
image = "0.24"          # Image processing
nokhwa = { version = "0.10", features = ["input-native", "output-threaded"] }
```

---

## 🔍 Troubleshooting

### Common Issues

#### 1. No Camera Detected

**Symptoms**: Server runs but reports no camera available

**Solutions**:
- **Linux**: Check permissions: `ls -l /dev/video*`
  ```bash
  # Add user to video group
  sudo usermod -a -G video $USER
  # Logout and login again
  ```
- **Windows**: Ensure camera drivers are installed
- **macOS**: Grant camera permissions in System Preferences

#### 2. Camera In Use

**Symptoms**: "Failed to open camera" error

**Solutions**:
```bash
# Linux: Find process using camera
fuser /dev/video0

# Kill the process
kill -9 <PID>
```

#### 3. Poor Image Quality

**Symptoms**: Blurry or dark images

**Solutions**:
- Ensure adequate lighting
- Camera may need time to auto-adjust (add delay)
- Try different resolutions in code

#### 4. High CPU Usage

**Symptoms**: Excessive CPU consumption

**Solutions**:
- Reduce capture frequency
- Lower resolution
- Use MJPEG format instead of RAW

### Debug Mode

Enable detailed logging:

```bash
RUST_LOG=debug ./target/release/hive_agent-camera-server
```

### Platform-Specific Issues

#### Linux
- Missing V4L2 libraries: `sudo apt-get install libv4l-dev`
- USB bandwidth issues with multiple cameras

#### Windows
- Windows Defender may block camera access
- USB 3.0 cameras may need specific drivers

#### macOS
- Privacy settings must allow terminal/app camera access
- Some USB cameras may not be recognized

---

## 📊 Performance Considerations

### Resource Usage

- **Memory**: ~15-30 MB idle, ~50-100 MB during capture
- **CPU**: < 1% idle, 5-15% during capture
- **Disk**: Each capture ~100KB-5MB depending on resolution

### Optimization Tips

1. **Use MJPEG format** for lower CPU usage
2. **Implement frame caching** to avoid repeated captures
3. **Limit concurrent captures** to prevent resource exhaustion
4. **Use lower resolutions** for analysis, higher for storage

---

## 🔒 Security Considerations

1. **Access Control**: No built-in authentication - use reverse proxy
2. **Rate Limiting**: Implement to prevent DoS
3. **File System**: Captured images are world-readable by default
4. **Network**: Binds to 0.0.0.0 - consider binding to localhost only

### Securing the Service

```nginx
# Nginx reverse proxy with auth
location /camera/ {
    auth_basic "Camera Access";
    auth_basic_user_file /etc/nginx/.htpasswd;
    proxy_pass http://localhost:6082/;
}
```

---

## 📝 API Usage Examples

### cURL Examples

```bash
# Capture multiple images
for i in {1..5}; do
    curl http://localhost:6082/capture-image
    sleep 1
done

# Get camera info with jq
curl -s http://localhost:6082/cameras | jq '.cameras[0]'

# Monitor capture count
watch -n 1 'curl -s http://localhost:6082/api/v1/hive_agent-camera-server/status | jq .capture_count'
```

### Python Client

```python
import requests
import json
from pathlib import Path

class CameraClient:
    def __init__(self, base_url="http://localhost:6082"):
        self.base_url = base_url
    
    def capture(self):
        """Capture an image"""
        response = requests.get(f"{self.base_url}/capture-image")
        return response.json()
    
    def list_cameras(self):
        """List available cameras"""
        response = requests.get(f"{self.base_url}/cameras")
        return response.json()
    
    def get_status(self):
        """Get service status"""
        response = requests.get(f"{self.base_url}/api/v1/hive_agent-camera-server/status")
        return response.json()

# Usage
client = CameraClient()
result = client.capture()
if result["ok"]:
    print(f"Image saved: {result['filename']}")
```

### JavaScript/Node.js Client

```javascript
const axios = require('axios');

class CameraClient {
    constructor(baseUrl = 'http://localhost:6082') {
        this.baseUrl = baseUrl;
    }
    
    async capture() {
        const response = await axios.get(`${this.baseUrl}/capture-image`);
        return response.data;
    }
    
    async listCameras() {
        const response = await axios.get(`${this.baseUrl}/cameras`);
        return response.data;
    }
    
    async getStatus() {
        const response = await axios.get(`${this.baseUrl}/api/v1/hive_agent-camera-server/status`);
        return response.data;
    }
}

// Usage
const client = new CameraClient();
client.capture().then(result => {
    if (result.ok) {
        console.log(`Image saved: ${result.filename}`);
    }
});
```

---

## 🚀 Future Enhancements

### Planned Features

- [ ] Live video streaming endpoint
- [ ] Multiple camera simultaneous capture
- [ ] Camera settings adjustment API
- [ ] Image format selection (JPEG, PNG, BMP)
- [ ] Thumbnail generation
- [ ] Motion detection
- [ ] Face detection integration
- [ ] QR/Barcode scanning
- [ ] Scheduled capture tasks
- [ ] WebSocket support for real-time updates

### Contribution Guidelines

1. Fork the repository
2. Create a feature branch
3. Implement with tests
4. Update documentation
5. Submit pull request

---

## 📚 Related Documentation

- [Warden Documentation](./WARDEN.md) - Service orchestration
- [API Documentation](./API.md) - Complete API reference
- [Quick Start Guide](./QUICKSTART.md) - Getting started
- [System Architecture](./README.md) - Overall system design

---

## 📄 License

Part of the Hive Agent ecosystem - see main project license.

---

**Last Updated**: October 2024  
**Version**: 0.1.0  
**Maintainer**: Hive Agent Team
