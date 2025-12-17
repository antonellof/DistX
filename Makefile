# Makefile for DistX

.PHONY: build test bench clean run check clippy

# Build in release mode
build:
	cargo build --release

# Run tests
test:
	cargo test --release

# Run benchmarks
bench:
	cargo bench --bench benchmark

# Check for errors
check:
	cargo check

# Run clippy
clippy:
	cargo clippy -- -D warnings

# Clean build artifacts
clean:
	cargo clean

# Run the server
run:
	cargo run --release

# Run with custom data directory
run-dev:
	cargo run --release -- --data-dir ./data --http-port 6333

# Full test suite
test-all: check clippy test bench

# Install dependencies (Linux)
install-deps-linux:
	sudo apt-get update
	sudo apt-get install -y liblmdb-dev build-essential

# Install dependencies (macOS)
install-deps-macos:
	brew install lmdb

# Performance comparison
perf-compare:
	@echo "=== DistX Performance ==="
	cargo bench --bench benchmark
	@echo ""
	@echo "=== Redis Performance ==="
	@echo "Run: redis-benchmark -t set,get -n 100000"
	@echo ""
	@echo "=== HelixDB Performance ==="
	@echo "Check HelixDB benchmark suite"

