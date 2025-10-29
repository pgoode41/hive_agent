#!/bin/bash

# Run All Tests
# Executes all test suites in sequence

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Get test directory
TEST_DIR="$(cd "$(dirname "$0")" && pwd)"

echo
echo "================================================================"
echo -e "${CYAN}     HIVE AGENT - COMPLETE TEST SUITE RUNNER${NC}"
echo "================================================================"
echo

# Check if warden is running first
echo -n "Checking if warden is running... "
if curl -s -f http://localhost:6080/api/v1/warden/healthcheck/basic > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Warden is running${NC}"
else
    echo -e "${RED}✗ Warden is not running${NC}"
    echo
    echo "Please start the warden first:"
    echo "  cd /home/nibbles/Documents/hive_agent"
    echo "  ./target/release/hive_agent-warden"
    echo
    exit 1
fi

echo

# Define tests to run in order
declare -a TESTS=(
    "quick_test.sh:Quick System Check"
    "test_port_management.sh:Port Management Test"
    "full_system_test.sh:Full System Test"
    "test_auto_recovery.sh:Auto-Recovery Test"
    "test_performance.sh:Performance Test"
)

# Track results
declare -A TEST_RESULTS
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0

# Function to run a test
run_test() {
    local test_file=$1
    local test_name=$2
    local test_path="${TEST_DIR}/${test_file}"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    echo "================================================================"
    echo -e "${BLUE}Running: ${test_name}${NC}"
    echo "================================================================"
    
    if [ ! -f "$test_path" ]; then
        echo -e "${YELLOW}⚠ Test file not found: $test_file${NC}"
        TEST_RESULTS["$test_name"]="SKIPPED"
        SKIPPED_TESTS=$((SKIPPED_TESTS + 1))
        echo
        return
    fi
    
    # Make sure test is executable
    chmod +x "$test_path"
    
    # Run the test
    START_TIME=$(date +%s)
    
    if bash "$test_path"; then
        END_TIME=$(date +%s)
        DURATION=$((END_TIME - START_TIME))
        echo
        echo -e "${GREEN}✓ $test_name PASSED (${DURATION}s)${NC}"
        TEST_RESULTS["$test_name"]="PASSED"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        END_TIME=$(date +%s)
        DURATION=$((END_TIME - START_TIME))
        echo
        echo -e "${RED}✗ $test_name FAILED (${DURATION}s)${NC}"
        TEST_RESULTS["$test_name"]="FAILED"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    
    echo
    
    # Add delay between tests
    sleep 2
}

# Ask user if they want to run specific tests or all
echo "Select test mode:"
echo "1) Run all tests"
echo "2) Run essential tests only (quick + port management + full system)"
echo "3) Run performance tests only"
echo "4) Exit"
echo
read -p "Enter choice [1-4]: " choice

case $choice in
    1)
        echo -e "${CYAN}Running all tests...${NC}"
        for test_entry in "${TESTS[@]}"; do
            IFS=':' read -r test_file test_name <<< "$test_entry"
            run_test "$test_file" "$test_name"
        done
        ;;
    2)
        echo -e "${CYAN}Running essential tests only...${NC}"
        run_test "quick_test.sh" "Quick System Check"
        run_test "test_port_management.sh" "Port Management Test"
        run_test "full_system_test.sh" "Full System Test"
        ;;
    3)
        echo -e "${CYAN}Running performance tests only...${NC}"
        run_test "test_performance.sh" "Performance Test"
        ;;
    4)
        echo "Exiting..."
        exit 0
        ;;
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac

# Display final summary
echo
echo "================================================================"
echo -e "${CYAN}TEST SUITE SUMMARY${NC}"
echo "================================================================"
echo

# Calculate pass rate
if [ $TOTAL_TESTS -gt 0 ]; then
    PASS_RATE=$(( (PASSED_TESTS * 100) / TOTAL_TESTS ))
else
    PASS_RATE=0
fi

# Display results
echo "Test Results:"
echo "-------------"
for test_name in "${!TEST_RESULTS[@]}"; do
    result="${TEST_RESULTS[$test_name]}"
    case $result in
        PASSED)
            echo -e "${GREEN}✓${NC} $test_name: $result"
            ;;
        FAILED)
            echo -e "${RED}✗${NC} $test_name: $result"
            ;;
        SKIPPED)
            echo -e "${YELLOW}○${NC} $test_name: $result"
            ;;
    esac
done

echo
echo "Summary:"
echo "--------"
echo "Total Tests:    $TOTAL_TESTS"
echo -e "Passed:         ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed:         ${RED}$FAILED_TESTS${NC}"
echo -e "Skipped:        ${YELLOW}$SKIPPED_TESTS${NC}"
echo "Pass Rate:      ${PASS_RATE}%"

echo

# Generate timestamp for report
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

# Save summary to file
SUMMARY_FILE="${TEST_DIR}/test_summary_$(date +%Y%m%d_%H%M%S).txt"
{
    echo "HIVE AGENT TEST SUITE SUMMARY"
    echo "=============================="
    echo "Date: $TIMESTAMP"
    echo ""
    echo "Results:"
    for test_name in "${!TEST_RESULTS[@]}"; do
        echo "- $test_name: ${TEST_RESULTS[$test_name]}"
    done
    echo ""
    echo "Statistics:"
    echo "- Total: $TOTAL_TESTS"
    echo "- Passed: $PASSED_TESTS"
    echo "- Failed: $FAILED_TESTS"
    echo "- Skipped: $SKIPPED_TESTS"
    echo "- Pass Rate: ${PASS_RATE}%"
} > "$SUMMARY_FILE"

echo "Test summary saved to: $SUMMARY_FILE"
echo

# Exit code based on results
if [ $FAILED_TESTS -eq 0 ] && [ $SKIPPED_TESTS -eq 0 ]; then
    echo -e "${GREEN}✅ ALL TESTS PASSED!${NC}"
    exit 0
elif [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${YELLOW}⚠ TESTS COMPLETED WITH SKIPS${NC}"
    exit 0
else
    echo -e "${RED}❌ SOME TESTS FAILED${NC}"
    exit 1
fi
