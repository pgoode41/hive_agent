#!/bin/bash

# Hive Agent Full System Test Suite
# Tests all aspects of the warden and microservices

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
WARDEN_PORT=6080
WARDEN_URL="http://localhost:${WARDEN_PORT}"
TEST_DELAY=2
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Test results storage
declare -A TEST_RESULTS

# Print header
print_header() {
    echo
    echo "================================================================"
    echo "  HIVE AGENT SYSTEM - FULL TEST SUITE"
    echo "  Date: $(date)"
    echo "================================================================"
    echo
}

# Test function with automatic result tracking
run_test() {
    local test_name=$1
    local test_command=$2
    local expected_result=$3
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    echo -n "Testing: $test_name... "
    
    if eval "$test_command" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ PASSED${NC}"
        TEST_RESULTS["$test_name"]="PASSED"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        echo -e "${RED}✗ FAILED${NC}"
        TEST_RESULTS["$test_name"]="FAILED"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi
}

# Test endpoint availability
test_endpoint() {
    local url=$1
    local name=$2
    
    run_test "$name" "curl -s -f \"$url\" -o /dev/null"
}

# Test JSON response
test_json_response() {
    local url=$1
    local name=$2
    local json_path=$3
    local expected=$4
    
    local cmd="curl -s \"$url\" | python3 -c \"import sys, json; data = json.load(sys.stdin); print(data${json_path})\""
    
    if [ -n "$expected" ]; then
        cmd="[ \"\$(curl -s \"$url\" | python3 -c \\\"import sys, json; data = json.load(sys.stdin); print(data${json_path})\\\")\" == \"$expected\" ]"
    fi
    
    run_test "$name" "$cmd"
}

# Check if service is running
is_service_running() {
    local service_name=$1
    curl -s "${WARDEN_URL}/api/v1/warden/services" | grep -q "\"name\":\"$service_name\".*\"running\":true"
}

# Check if service is healthy
is_service_healthy() {
    local service_name=$1
    curl -s "${WARDEN_URL}/api/v1/warden/services" | grep -q "\"name\":\"$service_name\".*\"healthy\":true"
}

# Start tests
print_header

echo -e "${BLUE}=== Phase 1: Warden Core Tests ===${NC}"
echo

# Test warden endpoints
test_endpoint "${WARDEN_URL}/api/v1/warden/healthcheck/basic" "Warden Health Check"
test_endpoint "${WARDEN_URL}/api/v1/warden/status" "Warden Status Endpoint"
test_endpoint "${WARDEN_URL}/api/v1/warden/services" "Warden Services List"

# Test warden response content
run_test "Warden Operational Status" "curl -s ${WARDEN_URL}/api/v1/warden/status | grep -q '\"status\":\"operational\"'"

echo
echo -e "${BLUE}=== Phase 2: Port Range Verification ===${NC}"
echo

# Check that services are using 6000-7000 range
run_test "Services using 6000-7000 range" "curl -s ${WARDEN_URL}/api/v1/warden/services | grep -E '\"port\":60[0-9]{2}' | wc -l | grep -q '^[1-9]'"

# Check no services on 5000-6000 range
run_test "No services on 5000-6000 range" "! netstat -tuln 2>/dev/null | grep -E ':(50[0-9]{2}|5[1-9][0-9]{2})' | grep -q LISTEN"

echo
echo -e "${BLUE}=== Phase 3: Service Health Checks ===${NC}"
echo

# Define all services with their ports
declare -A SERVICES=(
    ["rag"]="6071"
    ["hive_agent-image-to-text-generation-loop"]="6072"
    ["hive_agent-speech-to-text-generation-loop"]="6073"
    ["hive_agent-text-to-speech-generation-loop"]="6074"
    ["hive_agent-text-generation-loop"]="6075"
    ["hive_agent-image-to-text-player-loop"]="6076"
    ["hive_agent-audio-player"]="6077"
    ["hive_agent-text-to-speech-player-loop"]="6078"
    ["hive_agent-text-player-loop"]="6079"
    ["hive_agent-camera-server"]="6082"
    ["hive_agent-tools"]="6083"
    ["hive_agent-director"]="6084"
)

# Test each service health endpoint
for service in "${!SERVICES[@]}"; do
    port="${SERVICES[$service]}"
    service_url="http://localhost:${port}/api/v1/${service}/healthcheck/basic"
    test_endpoint "$service_url" "${service} health (port ${port})"
done

echo
echo -e "${BLUE}=== Phase 4: Service Status Endpoints ===${NC}"
echo

# Test status endpoints for a subset of services
test_endpoint "http://localhost:6071/api/v1/rag/status" "RAG Status Endpoint"
test_endpoint "http://localhost:6075/api/v1/hive_agent-text-generation-loop/status" "Text Generation Status"
test_endpoint "http://localhost:6084/api/v1/hive_agent-director/status" "Director Status"

echo
echo -e "${BLUE}=== Phase 5: Service Management Tests ===${NC}"
echo

# Test disable/enable functionality
echo "Testing service management (disable/enable)..."

# Disable RAG service
run_test "Disable RAG service" "curl -s -X POST ${WARDEN_URL}/api/v1/warden/service/rag/disable -o /dev/null"

sleep 3  # Give time for service to stop

# Check if RAG is disabled
run_test "RAG is disabled" "! is_service_running rag"

# Re-enable RAG service
run_test "Enable RAG service" "curl -s -X POST ${WARDEN_URL}/api/v1/warden/service/rag/enable -o /dev/null"

sleep $TEST_DELAY

# Check if RAG is running again
run_test "RAG is running again" "is_service_running rag"

echo
echo -e "${BLUE}=== Phase 6: Service Count Verification ===${NC}"
echo

# Count running services
RUNNING_COUNT=$(curl -s "${WARDEN_URL}/api/v1/warden/services" | grep -o '"running":true' | wc -l)
ENABLED_COUNT=$(curl -s "${WARDEN_URL}/api/v1/warden/services" | grep -o '"enabled":true' | wc -l)

echo "Services Running: $RUNNING_COUNT"
echo "Services Enabled: $ENABLED_COUNT"

run_test "At least 10 services running" "[ $RUNNING_COUNT -ge 10 ]"

echo
echo -e "${BLUE}=== Phase 7: Response Content Tests ===${NC}"
echo

# Test that health checks return "true"
for service in "${!SERVICES[@]}"; do
    port="${SERVICES[$service]}"
    response=$(curl -s "http://localhost:${port}/api/v1/${service}/healthcheck/basic" 2>/dev/null)
    if [ "$response" == "true" ]; then
        echo -e "${GREEN}✓${NC} ${service} returns 'true'"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}✗${NC} ${service} invalid response: '$response'"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
done

echo
echo -e "${BLUE}=== Phase 8: Configuration Persistence Test ===${NC}"
echo

# Check if config file exists
CONFIG_FILE="/home/nibbles/Documents/hive_agent/hive_agent-warden/deps/core_microservices.json"
run_test "Config file exists" "[ -f \"$CONFIG_FILE\" ]"

# Check if config is valid JSON
run_test "Config is valid JSON" "python3 -c \"import json; json.load(open('$CONFIG_FILE'))\""

echo
echo -e "${BLUE}=== Phase 9: Network Connectivity Tests ===${NC}"
echo

# Test that services are listening on correct ports
for service in "${!SERVICES[@]}"; do
    port="${SERVICES[$service]}"
    run_test "Port ${port} is listening" "netstat -tuln 2>/dev/null | grep -q ':${port}.*LISTEN'"
done

echo
echo -e "${BLUE}=== Phase 10: Load Test (Basic) ===${NC}"
echo

# Send multiple rapid requests to test stability
echo "Sending 10 rapid requests to warden..."
SUCCESS_COUNT=0
for i in {1..10}; do
    if curl -s -f "${WARDEN_URL}/api/v1/warden/status" > /dev/null 2>&1; then
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    fi
done
run_test "Warden handled 10 rapid requests" "[ $SUCCESS_COUNT -eq 10 ]"

echo
echo "================================================================"
echo -e "${BLUE}TEST RESULTS SUMMARY${NC}"
echo "================================================================"
echo

# Calculate pass rate
if [ $TOTAL_TESTS -gt 0 ]; then
    PASS_RATE=$(( (PASSED_TESTS * 100) / TOTAL_TESTS ))
else
    PASS_RATE=0
fi

# Display summary with colors based on results
if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}✓ ALL TESTS PASSED!${NC}"
    SUMMARY_COLOR=$GREEN
elif [ $PASS_RATE -ge 80 ]; then
    SUMMARY_COLOR=$YELLOW
else
    SUMMARY_COLOR=$RED
fi

echo
echo -e "Total Tests:  ${TOTAL_TESTS}"
echo -e "Passed:       ${GREEN}${PASSED_TESTS}${NC}"
echo -e "Failed:       ${RED}${FAILED_TESTS}${NC}"
echo -e "Pass Rate:    ${SUMMARY_COLOR}${PASS_RATE}%${NC}"

echo
echo "================================================================"

# Generate detailed report file
REPORT_FILE="/home/nibbles/Documents/hive_agent/tests/test_report_$(date +%Y%m%d_%H%M%S).txt"
{
    echo "HIVE AGENT TEST REPORT"
    echo "======================"
    echo "Date: $(date)"
    echo "Pass Rate: ${PASS_RATE}%"
    echo ""
    echo "DETAILED RESULTS:"
    echo "-----------------"
    for test_name in "${!TEST_RESULTS[@]}"; do
        echo "- ${test_name}: ${TEST_RESULTS[$test_name]}"
    done
} > "$REPORT_FILE"

echo
echo "Detailed report saved to: $REPORT_FILE"

# Exit with appropriate code
if [ $FAILED_TESTS -eq 0 ]; then
    exit 0
else
    exit 1
fi
