# Hive Agent Test Suite

Comprehensive testing framework for the Hive Agent microservices ecosystem.

## 📋 Overview

This directory contains automated tests to verify the functionality, performance, and reliability of the Hive Agent system. All tests are written in bash for portability and ease of use.

## 🚀 Quick Start

### Prerequisites
1. Build the project first:
```bash
cd /home/nibbles/Documents/hive_agent
cargo build --release
```

2. Start the warden:
```bash
./target/release/hive_agent-warden
```

3. Run tests:
```bash
cd tests
chmod +x *.sh
./run_all_tests.sh
```

## 📁 Test Files

| Test File | Description | Duration |
|-----------|-------------|----------|
| `quick_test.sh` | Basic health check of system | ~5 seconds |
| `full_system_test.sh` | Comprehensive system validation | ~2 minutes |
| `test_port_management.sh` | Verify port range compliance (6000-7000) | ~30 seconds |
| `test_auto_recovery.sh` | Test automatic service restart | ~45 seconds |
| `test_performance.sh` | Measure response times and throughput | ~1 minute |
| `run_all_tests.sh` | Execute all tests in sequence | ~5 minutes |

## 🧪 Test Descriptions

### Quick Test (`quick_test.sh`)
Fast verification that the system is operational.
- Checks warden health
- Counts running services
- Basic system validation

**Use when:** You need a quick sanity check.

### Full System Test (`full_system_test.sh`)
Comprehensive validation of all components.
- Tests all service endpoints
- Verifies port configuration
- Checks service management
- Validates configuration persistence
- Network connectivity tests
- Basic load testing

**Use when:** After major changes or before deployment.

### Port Management Test (`test_port_management.sh`)
Ensures all services use the correct port range.
- Verifies 6000-7000 range usage
- Checks for conflicts with 5000-6000
- Validates port assignments
- Detects port conflicts

**Use when:** After port configuration changes.

### Auto-Recovery Test (`test_auto_recovery.sh`)
Tests the warden's ability to restart crashed services.
- Kills a service process
- Waits for automatic restart
- Verifies new process creation
- Validates health restoration

**Use when:** Testing reliability features.

### Performance Test (`test_performance.sh`)
Measures system performance metrics.
- Response time analysis
- Throughput testing
- Concurrent request handling
- Memory usage monitoring

**Use when:** Optimizing performance or load testing.

## 📊 Test Output

All tests provide:
- ✅ **Color-coded results** (green=pass, red=fail, yellow=warning)
- 📈 **Progress indicators**
- 📝 **Detailed reports** saved to timestamped files
- 📊 **Summary statistics**

## 🎯 Running Specific Tests

### Run individual test:
```bash
./quick_test.sh
```

### Run all tests:
```bash
./run_all_tests.sh
```

### Run with specific options:
```bash
./run_all_tests.sh
# Then select:
# 1) All tests
# 2) Essential tests only
# 3) Performance tests only
```

## 📈 Test Reports

Test results are saved in the `tests` directory:
- `test_report_YYYYMMDD_HHMMSS.txt` - Detailed test results
- `test_summary_YYYYMMDD_HHMMSS.txt` - Summary of all tests

## 🔧 Troubleshooting

### Common Issues

**Tests fail with "Warden not running":**
```bash
# Start the warden first
cd ..
./target/release/hive_agent-warden
```

**Permission denied errors:**
```bash
# Make tests executable
chmod +x *.sh
```

**Services not responding:**
```bash
# Check if services are built
cd ..
cargo build --release

# Check running services
curl http://localhost:6080/api/v1/warden/services
```

**Port conflicts:**
```bash
# Check what's using ports
netstat -tuln | grep LISTEN | grep -E '60[0-9]{2}'

# Kill conflicting processes
pkill -f 'hive_agent'
```

## 🎨 Customization

### Modify test parameters:

Edit test files to adjust:
- `TEST_DELAY` - Delay between operations
- `MAX_WAIT` - Timeout values
- `ITERATIONS` - Number of test iterations
- Port ranges and service lists

### Add new tests:

1. Create new test file: `test_my_feature.sh`
2. Follow the existing test structure
3. Add to `run_all_tests.sh`:
```bash
TESTS+=(
    "test_my_feature.sh:My Feature Test"
)
```

## 🔍 Understanding Results

### Pass Rates
- **100%**: All systems functioning perfectly
- **80-99%**: System operational with minor issues
- **< 80%**: Significant issues need attention

### Response Times (Performance Test)
- **Excellent**: < 50ms
- **Good**: 50-100ms
- **Acceptable**: 100-200ms
- **Poor**: > 200ms

### Health Check Status
- **true**: Service is healthy
- **false/timeout**: Service needs attention

## 📝 Best Practices

1. **Run tests regularly**:
   - After code changes
   - Before deployments
   - As part of CI/CD pipeline

2. **Start with quick_test.sh**:
   - Fast validation
   - Identifies major issues early

3. **Review test reports**:
   - Check for patterns in failures
   - Monitor performance trends
   - Track system stability

4. **Keep tests updated**:
   - Add tests for new features
   - Update port numbers if changed
   - Adjust thresholds as needed

## 🤝 Contributing

To add new tests:
1. Create test file in this directory
2. Follow existing naming convention
3. Use color codes for output
4. Generate report files
5. Add to `run_all_tests.sh`

## 📚 Related Documentation

- [Warden Documentation](../docs/WARDEN.md)
- [API Documentation](../docs/API.md)
- [Quick Start Guide](../docs/QUICKSTART.md)

---

**Note**: All tests assume the warden is running and services are built with `cargo build --release`.
