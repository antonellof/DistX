#!/bin/bash
# Comprehensive test script for DistX

set -e

echo "=========================================="
echo "DistX Test Suite"
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo not found. Please install Rust from https://rustup.rs/${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} Rust toolchain found"
echo ""

# Step 1: Check compilation
echo "Step 1: Checking compilation..."
if cargo check --all-targets 2>&1 | grep -q "error"; then
    echo -e "${RED}✗${NC} Compilation errors found"
    cargo check --all-targets
    exit 1
else
    echo -e "${GREEN}✓${NC} Code compiles successfully"
fi
echo ""

# Step 2: Run clippy
echo "Step 2: Running clippy..."
if cargo clippy --all-targets -- -D warnings 2>&1 | grep -q "error"; then
    echo -e "${YELLOW}⚠${NC} Clippy warnings found (non-fatal)"
else
    echo -e "${GREEN}✓${NC} Clippy checks passed"
fi
echo ""

# Step 3: Run unit tests
echo "Step 3: Running unit tests..."
if cargo test --lib 2>&1 | grep -q "test result: FAILED"; then
    echo -e "${RED}✗${NC} Some tests failed"
    cargo test --lib
    exit 1
else
    echo -e "${GREEN}✓${NC} All unit tests passed"
fi
echo ""

# Step 4: Run integration tests
echo "Step 4: Running integration tests..."
if cargo test --test integration_test 2>&1 | grep -q "test result: FAILED"; then
    echo -e "${RED}✗${NC} Some integration tests failed"
    cargo test --test integration_test
    exit 1
else
    echo -e "${GREEN}✓${NC} All integration tests passed"
fi
echo ""

# Step 5: Build release
echo "Step 5: Building release binary..."
cargo build --release
if [ -f "target/release/distx" ]; then
    echo -e "${GREEN}✓${NC} Release binary built successfully"
    ls -lh target/release/distx
else
    echo -e "${RED}✗${NC} Release binary not found"
    exit 1
fi
echo ""

# Step 6: Run benchmarks (if criterion is available)
echo "Step 6: Running benchmarks..."
if cargo bench --bench benchmark 2>&1 | head -20; then
    echo -e "${GREEN}✓${NC} Benchmarks completed"
    echo "Check target/criterion/ for detailed reports"
else
    echo -e "${YELLOW}⚠${NC} Benchmarks may have issues (check output above)"
fi
echo ""

echo "=========================================="
echo -e "${GREEN}All tests completed!${NC}"
echo "=========================================="
echo ""
echo "Next steps:"
echo "  1. Run server: cargo run --release"
echo "  2. View benchmarks: open target/criterion/*/index.html"
echo "  3. Compare performance: see PERFORMANCE.md"

