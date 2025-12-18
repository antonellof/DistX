# Makefile for DistX

.PHONY: build test bench clean run check clippy docker-build docker-run docker-stop docker-logs

# Build in release mode
build:
	cargo build --release

# Run tests
test:
	cargo test --release

# Run benchmarks (requires criterion - install with cargo add --dev criterion)
# bench:
# 	cargo bench

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
test-all: check clippy test

# Install dependencies (Linux)
install-deps-linux:
	sudo apt-get update
	sudo apt-get install -y liblmdb-dev build-essential protobuf-compiler

# Install dependencies (macOS)
install-deps-macos:
	brew install lmdb protobuf

# ============================================
# Docker Commands
# ============================================

# Build Docker image
docker-build:
	docker build -t distx/distx:latest .

# Run with Docker
docker-run:
	docker run -d --name distx \
		-p 6333:6333 \
		-p 6334:6334 \
		-v distx_storage:/qdrant/storage \
		distx/distx:latest

# Run with Docker (interactive, for debugging)
docker-run-it:
	docker run -it --rm --name distx \
		-p 6333:6333 \
		-p 6334:6334 \
		-v distx_storage:/qdrant/storage \
		distx/distx:latest

# Stop Docker container
docker-stop:
	docker stop distx && docker rm distx

# View Docker logs
docker-logs:
	docker logs -f distx

# Run with docker-compose
docker-compose-up:
	docker-compose up -d

# Stop docker-compose
docker-compose-down:
	docker-compose down

# Pull from Docker Hub (when published)
docker-pull:
	docker pull distx/distx:latest

# Push to Docker Hub (requires login)
docker-push:
	docker push distx/distx:latest

# Clean Docker resources
docker-clean:
	docker stop distx 2>/dev/null || true
	docker rm distx 2>/dev/null || true
	docker rmi distx/distx:latest 2>/dev/null || true
	docker volume rm distx_storage 2>/dev/null || true

# ============================================
# Performance Comparison
# ============================================

perf-compare:
	@echo "=== DistX Performance ==="
	@echo "Run benchmarks after adding criterion dev-dependency"
	@echo ""
	@echo "=== Redis Performance ==="
	@echo "Run: redis-benchmark -t set,get -n 100000"

# ============================================
# Development
# ============================================

# Format code
fmt:
	cargo fmt

# Format check
fmt-check:
	cargo fmt --check

# Full CI check
ci: fmt-check clippy test
	@echo "All CI checks passed!"

