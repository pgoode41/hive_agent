#!/bin/bash

# Performance Test
# Measures response times and throughput

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== Hive Agent Performance Test ===${NC}"
echo

# Configuration
WARDEN_URL="http://localhost:6080"
TEST_SERVICE_URL="http://localhost:6071/api/v1/rag/healthcheck/basic"
ITERATIONS=100

# Function to measure response time
measure_response_time() {
    local url=$1
    local time_ms=$(curl -o /dev/null -s -w "%{time_total}\n" "$url" 2>/dev/null)
    echo "$time_ms"
}

# Function to calculate statistics
calculate_stats() {
    local -n arr=$1
    local count=${#arr[@]}
    
    if [ $count -eq 0 ]; then
        echo "No data"
        return
    fi
    
    # Sort array
    IFS=$'\n' sorted=($(sort -g <<<"${arr[*]}"))
    
    # Calculate min, max, avg
    local min=${sorted[0]}
    local max=${sorted[$((count-1))]}
    
    local sum=0
    for val in "${arr[@]}"; do
        sum=$(echo "$sum + $val" | bc)
    done
    local avg=$(echo "scale=3; $sum / $count" | bc)
    
    # Calculate median
    local median
    if [ $((count % 2)) -eq 0 ]; then
        local mid1=${sorted[$((count/2-1))]}
        local mid2=${sorted[$((count/2))]}
        median=$(echo "scale=3; ($mid1 + $mid2) / 2" | bc)
    else
        median=${sorted[$((count/2))]}
    fi
    
    # Calculate 95th percentile
    local p95_idx=$(echo "scale=0; $count * 0.95 / 1" | bc)
    local p95=${sorted[$p95_idx]}
    
    echo "  Min: ${min}s"
    echo "  Max: ${max}s"
    echo "  Avg: ${avg}s"
    echo "  Median: ${median}s"
    echo "  95th percentile: ${p95}s"
}

# Test 1: Warden Response Time
echo "1. Testing Warden Response Times ($ITERATIONS requests)..."
echo -n "   Progress: "

declare -a warden_times
errors=0

for ((i=1; i<=ITERATIONS; i++)); do
    if [ $((i % 10)) -eq 0 ]; then
        echo -n "$i "
    fi
    
    time_ms=$(measure_response_time "${WARDEN_URL}/api/v1/warden/status")
    
    if [ -n "$time_ms" ] && [ "$time_ms" != "0.000" ]; then
        warden_times+=("$time_ms")
    else
        ((errors++))
    fi
done
echo

echo "   Results:"
calculate_stats warden_times
if [ $errors -gt 0 ]; then
    echo -e "   ${YELLOW}⚠ Failed requests: $errors${NC}"
fi

echo

# Test 2: Service Health Check Response Time
echo "2. Testing Service Health Check Times ($ITERATIONS requests)..."
echo -n "   Progress: "

declare -a service_times
errors=0

for ((i=1; i<=ITERATIONS; i++)); do
    if [ $((i % 10)) -eq 0 ]; then
        echo -n "$i "
    fi
    
    time_ms=$(measure_response_time "$TEST_SERVICE_URL")
    
    if [ -n "$time_ms" ] && [ "$time_ms" != "0.000" ]; then
        service_times+=("$time_ms")
    else
        ((errors++))
    fi
done
echo

echo "   Results:"
calculate_stats service_times
if [ $errors -gt 0 ]; then
    echo -e "   ${YELLOW}⚠ Failed requests: $errors${NC}"
fi

echo

# Test 3: Concurrent Request Handling
echo "3. Testing Concurrent Requests..."
echo "   Sending 50 concurrent requests to warden..."

START_TIME=$(date +%s%N)

# Launch concurrent requests
for ((i=1; i<=50; i++)); do
    curl -s "${WARDEN_URL}/api/v1/warden/status" > /dev/null 2>&1 &
done

# Wait for all background jobs
wait

END_TIME=$(date +%s%N)
DURATION=$(echo "scale=3; ($END_TIME - $START_TIME) / 1000000000" | bc)

echo "   Completed 50 concurrent requests in ${DURATION}s"
THROUGHPUT=$(echo "scale=2; 50 / $DURATION" | bc)
echo "   Throughput: ${THROUGHPUT} req/s"

echo

# Test 4: Service Discovery Performance
echo "4. Testing Service Discovery Performance..."

START_TIME=$(date +%s%N)
for ((i=1; i<=10; i++)); do
    curl -s "${WARDEN_URL}/api/v1/warden/services" > /dev/null 2>&1
done
END_TIME=$(date +%s%N)

AVG_TIME=$(echo "scale=3; ($END_TIME - $START_TIME) / 10000000000" | bc)
echo "   Average time to list all services: ${AVG_TIME}s"

echo

# Test 5: Memory Usage Check (if possible)
echo "5. Checking Memory Usage..."

# Try to get warden PID
WARDEN_PID=$(ps aux | grep -E "hive_agent-warden" | grep -v grep | awk '{print $2}' | head -n1)

if [ -n "$WARDEN_PID" ]; then
    MEM_INFO=$(ps -p "$WARDEN_PID" -o pid,vsz,rss,comm --no-headers)
    VSZ=$(echo "$MEM_INFO" | awk '{print $2}')
    RSS=$(echo "$MEM_INFO" | awk '{print $3}')
    
    VSZ_MB=$(echo "scale=2; $VSZ / 1024" | bc)
    RSS_MB=$(echo "scale=2; $RSS / 1024" | bc)
    
    echo "   Warden (PID: $WARDEN_PID)"
    echo "   Virtual Memory: ${VSZ_MB} MB"
    echo "   Resident Memory: ${RSS_MB} MB"
else
    echo "   Could not find warden process"
fi

echo
echo "================================================================"
echo -e "${BLUE}Performance Summary${NC}"
echo "================================================================"

# Analyze results
WARNINGS=0
if [ ${#warden_times[@]} -gt 0 ]; then
    avg_warden=$(echo "scale=3; $(IFS=+; echo "${warden_times[*]}") / ${#warden_times[@]}" | bc)
    # Note: times are in seconds, so 0.100 = 100ms
    if (( $(echo "$avg_warden > 0.100" | bc -l) )); then
        echo -e "${YELLOW}⚠ Warden response time is high (>100ms average)${NC}"
        WARNINGS=$((WARNINGS + 1))
    else
        echo -e "${GREEN}✓ Warden response time is excellent (avg: ${avg_warden}s)${NC}"
    fi
fi

if [ ${#service_times[@]} -gt 0 ]; then
    avg_service=$(echo "scale=3; $(IFS=+; echo "${service_times[*]}") / ${#service_times[@]}" | bc)
    # Note: times are in seconds, so 0.050 = 50ms
    if (( $(echo "$avg_service > 0.050" | bc -l) )); then
        echo -e "${YELLOW}⚠ Service health check is slow (>50ms average)${NC}"
        WARNINGS=$((WARNINGS + 1))
    else
        echo -e "${GREEN}✓ Service health check is excellent (avg: ${avg_service}s)${NC}"
    fi
fi

if (( $(echo "$THROUGHPUT < 10" | bc -l) )); then
    echo -e "${YELLOW}⚠ Low throughput (<10 req/s)${NC}"
    WARNINGS=$((WARNINGS + 1))
else
    echo -e "${GREEN}✓ Good throughput (${THROUGHPUT} req/s)${NC}"
fi

echo
if [ $WARNINGS -eq 0 ]; then
    echo -e "${GREEN}✅ PERFORMANCE TEST PASSED${NC}"
    echo "System is performing well within expected parameters."
else
    echo -e "${YELLOW}⚠ PERFORMANCE TEST COMPLETED WITH WARNINGS${NC}"
    echo "Found $WARNINGS performance concerns that may need attention."
fi
