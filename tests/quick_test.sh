#!/bin/bash

# Quick System Test - Basic health verification
# Run this for a fast check that everything is working

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "=== Hive Agent Quick Test ==="
echo

# Test warden
if curl -s -f http://localhost:6080/api/v1/warden/healthcheck/basic > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Warden is running"
else
    echo -e "${RED}✗${NC} Warden is not responding"
    exit 1
fi

# Count running services
RUNNING=$(curl -s http://localhost:6080/api/v1/warden/services 2>/dev/null | grep -o '"running":true' | wc -l)
HEALTHY=$(curl -s http://localhost:6080/api/v1/warden/services 2>/dev/null | grep -o '"healthy":true' | wc -l)

echo "Services Running: $RUNNING"
echo "Services Healthy: $HEALTHY"

if [ "$RUNNING" -ge 10 ]; then
    echo -e "${GREEN}✓${NC} System operational"
else
    echo -e "${RED}✗${NC} Some services are not running"
    exit 1
fi

echo
echo -e "${GREEN}Quick test passed!${NC}"
