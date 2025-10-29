#!/bin/bash

# Test Auto-Recovery Feature
# This script tests if the warden automatically restarts crashed services

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== Hive Agent Auto-Recovery Test ===${NC}"
echo
echo "This test will kill a service and verify the warden restarts it."
echo

# Select test service (RAG)
SERVICE_NAME="rag"
SERVICE_PORT=6071

echo "Target Service: $SERVICE_NAME (port $SERVICE_PORT)"
echo

# Step 1: Verify service is running
echo -n "1. Checking if $SERVICE_NAME is currently running... "
if curl -s -f "http://localhost:${SERVICE_PORT}/api/v1/${SERVICE_NAME}/healthcheck/basic" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Running${NC}"
else
    echo -e "${RED}✗ Not running${NC}"
    echo "Please ensure the service is running before testing auto-recovery."
    exit 1
fi

# Step 2: Find the service PID
echo -n "2. Finding $SERVICE_NAME process... "
SERVICE_PID=$(ps aux | grep -E "target/release/${SERVICE_NAME}( |$)" | grep -v grep | awk '{print $2}' | head -n1)

if [ -z "$SERVICE_PID" ]; then
    echo -e "${RED}✗ Could not find process${NC}"
    echo "Unable to locate the service process."
    exit 1
else
    echo -e "${GREEN}✓ Found PID: $SERVICE_PID${NC}"
fi

# Step 3: Kill the service
echo -n "3. Killing service (PID: $SERVICE_PID)... "
# Try regular kill first, then force if needed
kill $SERVICE_PID 2>/dev/null || kill -15 $SERVICE_PID 2>/dev/null || kill -9 $SERVICE_PID 2>/dev/null
sleep 1

if ps -p $SERVICE_PID > /dev/null 2>&1; then
    # If we can't kill it directly, try sudo (may fail) or skip
    echo -e "${YELLOW}⚠ Regular kill failed, trying alternative method${NC}"
    # Alternative: crash the service by sending bad requests or use warden API
    pkill -f "target/release/${SERVICE_NAME}" 2>/dev/null || true
    sleep 1
    if ps -p $SERVICE_PID > /dev/null 2>&1; then
        echo -e "${RED}✗ Could not terminate process (permission denied)${NC}"
        echo "Note: This test requires permission to kill processes."
        echo "Alternative: Test warden's response to crashed services."
        exit 1
    fi
else
    echo -e "${GREEN}✓ Process terminated${NC}"
fi

# Step 4: Verify service is down
echo -n "4. Verifying service is down... "
if curl -s -f "http://localhost:${SERVICE_PORT}/api/v1/${SERVICE_NAME}/healthcheck/basic" > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠ Service still responding (may be cached)${NC}"
else
    echo -e "${GREEN}✓ Service is down${NC}"
fi

# Step 5: Wait for auto-recovery
echo "5. Waiting for warden to detect and restart service..."
echo -n "   Waiting"

MAX_WAIT=30  # Maximum 30 seconds
WAITED=0
RECOVERED=false

while [ $WAITED -lt $MAX_WAIT ]; do
    echo -n "."
    sleep 2
    WAITED=$((WAITED + 2))
    
    if curl -s -f "http://localhost:${SERVICE_PORT}/api/v1/${SERVICE_NAME}/healthcheck/basic" > /dev/null 2>&1; then
        RECOVERED=true
        break
    fi
done

echo

# Step 6: Check recovery status
echo -n "6. Checking recovery status... "
if [ "$RECOVERED" = true ]; then
    echo -e "${GREEN}✓ Service recovered in ${WAITED} seconds${NC}"
    
    # Verify it has a new PID
    NEW_PID=$(ps aux | grep -E "target/release/${SERVICE_NAME}( |$)" | grep -v grep | awk '{print $2}' | head -n1)
    
    if [ -n "$NEW_PID" ] && [ "$NEW_PID" != "$SERVICE_PID" ]; then
        echo -e "   ${GREEN}✓ New process started (PID: $NEW_PID)${NC}"
    fi
    
    # Check service status in warden
    if curl -s http://localhost:6080/api/v1/warden/services | grep -q "\"name\":\"${SERVICE_NAME}\".*\"running\":true.*\"healthy\":true"; then
        echo -e "   ${GREEN}✓ Service marked as healthy in warden${NC}"
    fi
    
    echo
    echo -e "${GREEN}✅ AUTO-RECOVERY TEST PASSED${NC}"
    echo "The warden successfully detected the crashed service and restarted it."
    exit 0
else
    echo -e "${RED}✗ Service did not recover within ${MAX_WAIT} seconds${NC}"
    echo
    echo -e "${RED}❌ AUTO-RECOVERY TEST FAILED${NC}"
    echo "The warden did not restart the service in time."
    echo
    echo "Troubleshooting:"
    echo "- Check if the warden is running: curl http://localhost:6080/api/v1/warden/healthcheck/basic"
    echo "- Check warden logs for errors"
    echo "- Verify health monitoring is enabled"
    exit 1
fi
