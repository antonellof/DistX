#!/bin/bash
#
# DistX Similarity Engine Demo Runner
#
# This script runs the similarity demo with optional Docker container management.
#
# Usage:
#   ./run_similarity_demo.sh           # Run demo (assumes DistX is running)
#   ./run_similarity_demo.sh --start   # Start DistX container, then run demo
#   ./run_similarity_demo.sh --stop    # Stop DistX container
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONTAINER_NAME="distx-demo"
IMAGE_NAME="distx:similarity"
DISTX_URL="http://localhost:6333"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

start_distx() {
    log_info "Starting DistX container..."
    
    # Stop existing container if running
    docker stop $CONTAINER_NAME 2>/dev/null || true
    docker rm $CONTAINER_NAME 2>/dev/null || true
    
    # Check if image exists
    if ! docker image inspect $IMAGE_NAME >/dev/null 2>&1; then
        log_warn "Image '$IMAGE_NAME' not found. Building..."
        cd "$SCRIPT_DIR/.."
        docker build -t $IMAGE_NAME .
    fi
    
    # Start container
    docker run -d --name $CONTAINER_NAME \
        -p 6333:6333 -p 6334:6334 \
        $IMAGE_NAME
    
    # Wait for container to be ready
    log_info "Waiting for DistX to be ready..."
    for i in {1..30}; do
        if curl -s "$DISTX_URL/healthz" >/dev/null 2>&1; then
            log_info "DistX is ready!"
            return 0
        fi
        sleep 1
    done
    
    log_error "DistX failed to start. Check logs: docker logs $CONTAINER_NAME"
    exit 1
}

stop_distx() {
    log_info "Stopping DistX container..."
    docker stop $CONTAINER_NAME 2>/dev/null || true
    docker rm $CONTAINER_NAME 2>/dev/null || true
    log_info "Container stopped."
}

run_demo() {
    log_info "Running Similarity Engine demo..."
    
    # Check Python
    if ! command -v python3 &>/dev/null; then
        log_error "Python 3 is required. Please install Python 3."
        exit 1
    fi
    
    # Install requirements
    pip3 install -q requests tabulate 2>/dev/null || pip install -q requests tabulate 2>/dev/null || true
    
    # Run demo
    python3 "$SCRIPT_DIR/similarity_demo.py" --url "$DISTX_URL" "$@"
}

show_usage() {
    echo "DistX Similarity Engine Demo"
    echo ""
    echo "Usage:"
    echo "  $0              Run demo (assumes DistX is already running)"
    echo "  $0 --start      Start DistX container and run demo"
    echo "  $0 --stop       Stop DistX container"
    echo "  $0 --help       Show this help"
    echo ""
    echo "Demo options (passed to Python script):"
    echo "  --demo products     Run only products demo"
    echo "  --demo suppliers    Run only suppliers demo"
    echo "  --demo explain      Run only explainability demo"
    echo "  --csv FILE          Use custom CSV file"
    echo ""
    echo "Examples:"
    echo "  $0 --start                    # Start DistX and run full demo"
    echo "  $0 --demo products            # Run only products demo"
    echo "  $0 --csv my_products.csv      # Use custom data"
}

# Main
case "${1:-}" in
    --start)
        start_distx
        shift
        run_demo "$@"
        ;;
    --stop)
        stop_distx
        ;;
    --help|-h)
        show_usage
        ;;
    *)
        run_demo "$@"
        ;;
esac
