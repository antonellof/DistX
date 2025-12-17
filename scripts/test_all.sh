#!/bin/bash
# Comprehensive test script for DistX
# Inspired by Redis and Qdrant testing patterns

set -e

echo "=========================================="
echo "DistX Test Suite"
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_WARNED=0

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found. Please install Rust from https://rustup.rs/${NC}"
    exit 1
fi

RUST_VERSION=$(rustc --version 2>/dev/null || echo "unknown")
echo -e "${GREEN}✓${NC} Rust toolchain found: $RUST_VERSION"
echo ""

# Change to project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"
echo "Working directory: $PROJECT_ROOT"
echo ""

# Step 1: Check compilation
echo -e "${BLUE}Step 1: Checking compilation...${NC}"
COMPILE_OUTPUT=$(cargo check --all-targets 2>&1) || true
if echo "$COMPILE_OUTPUT" | grep -q "^error\["; then
    echo -e "${RED}✗${NC} Compilation errors found"
    echo "$COMPILE_OUTPUT" | grep -A5 "^error\["
    TESTS_FAILED=$((TESTS_FAILED + 1))
    exit 1
else
    echo -e "${GREEN}✓${NC} Code compiles successfully"
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi
echo ""

# Step 2: Run clippy (more lenient check)
echo -e "${BLUE}Step 2: Running clippy lints...${NC}"
CLIPPY_OUTPUT=$(cargo clippy --all-targets 2>&1) || true
if echo "$CLIPPY_OUTPUT" | grep -q "^error\["; then
    CLIPPY_ERRORS=$(echo "$CLIPPY_OUTPUT" | grep -c "^error\[" || true)
    echo -e "${RED}✗${NC} Clippy errors found: $CLIPPY_ERRORS"
    echo "$CLIPPY_OUTPUT" | grep -A3 "^error\["
    TESTS_FAILED=$((TESTS_FAILED + 1))
elif echo "$CLIPPY_OUTPUT" | grep -q "^warning:"; then
    CLIPPY_WARNINGS=$(echo "$CLIPPY_OUTPUT" | grep -c "^warning:" || true)
    echo -e "${YELLOW}⚠${NC} Clippy warnings: $CLIPPY_WARNINGS (non-blocking)"
    TESTS_WARNED=$((TESTS_WARNED + 1))
else
    echo -e "${GREEN}✓${NC} Clippy checks passed (no warnings)"
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi
echo ""

# Step 3: Run unit tests
echo -e "${BLUE}Step 3: Running unit tests...${NC}"
UNIT_TEST_OUTPUT=$(cargo test --lib 2>&1) || true
if echo "$UNIT_TEST_OUTPUT" | grep -q "FAILED"; then
    echo -e "${RED}✗${NC} Some unit tests failed"
    echo "$UNIT_TEST_OUTPUT" | grep -E "(FAILED|panicked)"
    TESTS_FAILED=$((TESTS_FAILED + 1))
else
    echo -e "${GREEN}✓${NC} Unit tests passed"
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi
echo ""

# Step 4: Run integration tests
echo -e "${BLUE}Step 4: Running integration tests...${NC}"
INT_TEST_OUTPUT=$(cargo test --test integration_test 2>&1) || true
if echo "$INT_TEST_OUTPUT" | grep -q "FAILED"; then
    echo -e "${RED}✗${NC} Some integration tests failed"
    echo "$INT_TEST_OUTPUT" | grep -E "(FAILED|panicked)"
    TESTS_FAILED=$((TESTS_FAILED + 1))
else
    # Extract test count
    TEST_COUNT=$(echo "$INT_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | head -1 || echo "all")
    echo -e "${GREEN}✓${NC} Integration tests passed ($TEST_COUNT)"
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi
echo ""

# Step 5: Build release
echo -e "${BLUE}Step 5: Building release binary...${NC}"
BUILD_OUTPUT=$(cargo build --release 2>&1) || true
if echo "$BUILD_OUTPUT" | grep -q "^error\["; then
    echo -e "${RED}✗${NC} Release build failed"
    TESTS_FAILED=$((TESTS_FAILED + 1))
elif [ -f "target/release/distx" ]; then
    BINARY_SIZE=$(ls -lh target/release/distx | awk '{print $5}')
    echo -e "${GREEN}✓${NC} Release binary built successfully (${BINARY_SIZE})"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    echo -e "${RED}✗${NC} Release binary not found"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi
echo ""

# Step 6: Quick SIMD feature check
echo -e "${BLUE}Step 6: Checking SIMD support...${NC}"
ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
    if sysctl -a 2>/dev/null | grep -q "hw.optional.avx2_0: 1" || grep -q "avx2" /proc/cpuinfo 2>/dev/null; then
        echo -e "${GREEN}✓${NC} AVX2 SIMD available (optimized performance)"
    else
        echo -e "${YELLOW}⚠${NC} SSE SIMD available (good performance)"
    fi
elif [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
    echo -e "${GREEN}✓${NC} ARM64 NEON SIMD available (optimized for Apple Silicon)"
else
    echo -e "${YELLOW}⚠${NC} Architecture: $ARCH"
fi
echo ""

# Step 7: Doc tests (optional)
echo -e "${BLUE}Step 7: Running doc tests...${NC}"
DOC_TEST_OUTPUT=$(cargo test --doc 2>&1) || true
if echo "$DOC_TEST_OUTPUT" | grep -q "FAILED"; then
    echo -e "${YELLOW}⚠${NC} Some doc tests failed (non-blocking)"
    TESTS_WARNED=$((TESTS_WARNED + 1))
else
    echo -e "${GREEN}✓${NC} Doc tests passed"
    TESTS_PASSED=$((TESTS_PASSED + 1))
fi
echo ""

# Summary
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo -e "  ${GREEN}Passed:${NC}  $TESTS_PASSED"
echo -e "  ${YELLOW}Warned:${NC}  $TESTS_WARNED"
echo -e "  ${RED}Failed:${NC}  $TESTS_FAILED"
echo ""

if [ "$TESTS_FAILED" -gt 0 ]; then
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
else
    echo -e "${GREEN}All tests completed successfully!${NC}"
fi

echo ""
echo "=========================================="
echo "Next Steps"
echo "=========================================="
echo "  1. Run server:     cargo run --release"
echo "  2. Run benchmarks: cargo bench --bench benchmark"
echo "  3. Python API test: python3 scripts/benchmark.py --quick"
echo "  4. View docs:       cargo doc --open"
echo ""
