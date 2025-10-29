#!/bin/bash

# Test Port Management
# Verifies that all services are using the correct port range

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== Port Management Test ===${NC}"
echo

ERRORS=0

# Test 1: Check configured ports
echo "1. Checking configured ports in JSON..."
CONFIG_FILE="/home/nibbles/Documents/hive_agent/hive_agent-warden/deps/core_microservices.json"

if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${RED}✗ Config file not found${NC}"
    exit 1
fi

# Extract all ports from config
PORTS=$(python3 -c "
import json
with open('$CONFIG_FILE') as f:
    services = json.load(f)
    for service in services:
        print(f\"{service['name']}:{service['port']}\")
" 2>/dev/null)

if [ -z "$PORTS" ]; then
    echo -e "${RED}✗ Failed to parse config${NC}"
    exit 1
fi

echo "   Configured services and ports:"
while IFS=: read -r service port; do
    if [ $port -ge 6000 ] && [ $port -le 7000 ]; then
        echo -e "   ${GREEN}✓${NC} $service: $port"
    else
        echo -e "   ${RED}✗${NC} $service: $port (OUT OF RANGE!)"
        ERRORS=$((ERRORS + 1))
    fi
done <<< "$PORTS"

echo

# Test 2: Check actual listening ports
echo "2. Checking actual listening ports..."

# Get all listening ports in our range
LISTENING_PORTS=$(netstat -tuln 2>/dev/null | grep LISTEN | grep -oE ':[0-9]+' | cut -d: -f2 | sort -u)

echo "   Ports in 6000-7000 range:"
for port in $LISTENING_PORTS; do
    if [ $port -ge 6000 ] && [ $port -le 7000 ]; then
        echo -e "   ${GREEN}✓${NC} Port $port is listening"
    fi
done

echo "   Ports in 5000-6000 range (should be empty):"
FOUND_5000_RANGE=false
for port in $LISTENING_PORTS; do
    if [ $port -ge 5000 ] && [ $port -lt 6000 ]; then
        echo -e "   ${RED}✗${NC} Port $port is listening (CONFLICT!)"
        ERRORS=$((ERRORS + 1))
        FOUND_5000_RANGE=true
    fi
done

if [ "$FOUND_5000_RANGE" = false ]; then
    echo -e "   ${GREEN}✓${NC} No services in 5000-6000 range"
fi

echo

# Test 3: Verify services respond on configured ports
echo "3. Testing service responses on configured ports..."

declare -A SERVICE_PORTS=(
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

for service in "${!SERVICE_PORTS[@]}"; do
    port="${SERVICE_PORTS[$service]}"
    
    # Try to connect to the service
    if timeout 2 bash -c "echo > /dev/tcp/localhost/$port" 2>/dev/null; then
        # Port is open, now check if it responds correctly
        if curl -s -f "http://localhost:${port}/api/v1/${service}/healthcheck/basic" > /dev/null 2>&1; then
            echo -e "   ${GREEN}✓${NC} $service responds on port $port"
        else
            echo -e "   ${YELLOW}⚠${NC} $service port $port open but not responding to health check"
        fi
    else
        # Check if service is supposed to be running
        if curl -s http://localhost:6080/api/v1/warden/services 2>/dev/null | grep -q "\"name\":\"$service\".*\"running\":true"; then
            echo -e "   ${RED}✗${NC} $service should be on port $port but not responding"
            ERRORS=$((ERRORS + 1))
        else
            echo -e "   ${YELLOW}○${NC} $service not running (port $port)"
        fi
    fi
done

echo

# Test 4: Check for port conflicts
echo "4. Checking for port conflicts..."

# Get list of all ports from warden
WARDEN_PORTS=$(curl -s http://localhost:6080/api/v1/warden/services 2>/dev/null | grep -oE '"port":[0-9]+' | cut -d: -f2 | sort | uniq -c | sort -rn)

CONFLICTS_FOUND=false
while read -r count port; do
    if [ "$count" -gt 1 ]; then
        echo -e "   ${RED}✗${NC} Port $port assigned to $count services (CONFLICT!)"
        ERRORS=$((ERRORS + 1))
        CONFLICTS_FOUND=true
    fi
done <<< "$WARDEN_PORTS"

if [ "$CONFLICTS_FOUND" = false ]; then
    echo -e "   ${GREEN}✓${NC} No port conflicts detected"
fi

echo

# Test 5: Verify warden's port tracking
echo "5. Checking warden's port tracking..."

PORTS_IN_USE=$(curl -s http://localhost:6080/api/v1/warden/status 2>/dev/null | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    ports = data.get('ports_in_use', [])
    for port in sorted(ports):
        print(port)
except:
    pass
" 2>/dev/null)

if [ -n "$PORTS_IN_USE" ]; then
    echo "   Warden tracking ports:"
    for port in $PORTS_IN_USE; do
        if [ $port -ge 6000 ] && [ $port -le 7000 ]; then
            echo -e "   ${GREEN}✓${NC} $port"
        else
            echo -e "   ${RED}✗${NC} $port (out of range!)"
            ERRORS=$((ERRORS + 1))
        fi
    done
else
    echo -e "   ${YELLOW}⚠${NC} Could not retrieve warden's port list"
fi

echo
echo "================================================================"

# Summary
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✅ PORT MANAGEMENT TEST PASSED${NC}"
    echo "All services are using ports in the correct 6000-7000 range."
    echo "No conflicts detected with the 5000-6000 range."
    exit 0
else
    echo -e "${RED}❌ PORT MANAGEMENT TEST FAILED${NC}"
    echo "Found $ERRORS port-related issues."
    echo
    echo "Common issues:"
    echo "- Services not built with latest port configuration"
    echo "- Old service instances still running"
    echo "- Manual port assignments conflicting"
    echo
    echo "To fix:"
    echo "1. Stop all services: pkill -f 'hive_agent'"
    echo "2. Rebuild: cargo build --release"
    echo "3. Restart warden: ./target/release/hive_agent-warden"
    exit 1
fi
