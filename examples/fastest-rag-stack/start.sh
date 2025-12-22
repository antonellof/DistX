#!/bin/bash
# Startup script for RAG Stack
# Supports both Docker and local binary execution of vectX

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VECTX_VERSION="v0.2.1"
VECTX_BIN_DIR="$SCRIPT_DIR/.vectx"
VECTX_BIN="$VECTX_BIN_DIR/vectx"
VECTX_DATA_DIR="$SCRIPT_DIR/.vectx_data"
VECTX_DOCKER_CONTAINER="vectx"
VECTX_STARTED_BY_US=""
VECTX_MODE=""  # "docker" or "local"
ENV_FILE="$SCRIPT_DIR/.env"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Load .env file if it exists
load_env() {
    if [ -f "$ENV_FILE" ]; then
        echo -e "${BLUE}📄 Loading environment from .env file...${NC}"
        # Export all variables from .env (ignore comments and empty lines)
        set -a
        source "$ENV_FILE"
        set +a
        return 0
    fi
    return 1
}

# Check if .env.example should be copied
setup_env() {
    if [ ! -f "$ENV_FILE" ]; then
        if [ -f "$SCRIPT_DIR/.env.example" ]; then
            echo -e "${YELLOW}⚠️  No .env file found.${NC}"
            echo ""
            echo "Would you like to create one from .env.example?"
            read -p "Create .env file? [Y/n]: " create_env
            create_env=${create_env:-Y}
            
            if [[ "$create_env" =~ ^[Yy] ]]; then
                cp "$SCRIPT_DIR/.env.example" "$ENV_FILE"
                echo -e "${GREEN}✅ Created .env file from .env.example${NC}"
                echo ""
                echo -e "${YELLOW}Please edit .env and add your OpenAI API key:${NC}"
                echo "   $ENV_FILE"
                echo ""
                read -p "Press Enter after editing .env, or Ctrl+C to exit..."
                
                # Reload after editing
                load_env
            fi
        fi
    fi
}

# Detect platform and set download URL
detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)
    
    case "$os" in
        darwin)
            case "$arch" in
                arm64|aarch64) echo "macos-arm64" ;;
                x86_64) echo "macos-x86_64" ;;
                *) echo "unsupported" ;;
            esac
            ;;
        linux)
            case "$arch" in
                x86_64) echo "linux-x86_64" ;;
                *) echo "unsupported" ;;
            esac
            ;;
        *)
            echo "unsupported"
            ;;
    esac
}

download_vectx() {
    local platform=$(detect_platform)
    
    if [ "$platform" = "unsupported" ]; then
        echo -e "${RED}❌ Unsupported platform: $(uname -s) $(uname -m)${NC}"
        echo "Please build from source:"
        echo "  git clone https://github.com/antonellof/vectX.git"
        echo "  cd vectX && cargo build --release"
        return 1
    fi
    
    local download_url="https://github.com/antonellof/vectX/releases/download/${VECTX_VERSION}/vectx-${platform}.tar.gz"
    local temp_file="/tmp/vectx-${platform}.tar.gz"
    
    echo -e "${BLUE}📦 Downloading vectX ${VECTX_VERSION} for ${platform}...${NC}"
    echo "   URL: $download_url"
    
    # Create bin directory
    mkdir -p "$VECTX_BIN_DIR"
    
    # Download
    if command -v curl &> /dev/null; then
        curl -L -o "$temp_file" "$download_url"
    elif command -v wget &> /dev/null; then
        wget -O "$temp_file" "$download_url"
    else
        echo -e "${RED}❌ Neither curl nor wget found. Please install one of them.${NC}"
        return 1
    fi
    
    # Extract to temp dir first
    echo "📂 Extracting..."
    local temp_extract="/tmp/vectx_extract_$$"
    mkdir -p "$temp_extract"
    tar -xzf "$temp_file" -C "$temp_extract"
    
    # Find and move the binary (archive contains antonellofratepietro/vectx)
    if [ -f "$temp_extract/antonellofratepietro/vectx" ]; then
        cp "$temp_extract/antonellofratepietro/vectx" "$VECTX_BIN"
        rm -rf "$temp_extract"
    else
        echo -e "${RED}❌ Binary not found in archive${NC}"
        ls -la "$temp_extract"
        rm -rf "$temp_extract"
        return 1
    fi
    
    # Make executable
    chmod +x "$VECTX_BIN"
    
    # Cleanup
    rm -f "$temp_file"
    
    echo -e "${GREEN}✅ vectX installed to $VECTX_BIN${NC}"
    return 0
}

# Check if Docker is available
is_docker_available() {
    command -v docker &> /dev/null && docker info &> /dev/null
}

# Check if vectX is running via Docker
is_vectx_docker_running() {
    docker ps --format '{{.Names}}' 2>/dev/null | grep -q "^${VECTX_DOCKER_CONTAINER}$"
}

# Check if vectX is responding on port 6333
is_vectx_responding() {
    curl -s http://localhost:6333/healthz > /dev/null 2>&1
}

# Start vectX via Docker
start_vectx_docker() {
    echo -e "${BLUE}🐳 Starting vectX via Docker...${NC}"
    
    # Remove existing stopped container if any
    docker rm -f "$VECTX_DOCKER_CONTAINER" 2>/dev/null
    
    # Create data directory
    mkdir -p "$VECTX_DATA_DIR"
    
    # Start container
    docker run -d \
        --name "$VECTX_DOCKER_CONTAINER" \
        -p 6333:6333 \
        -p 6334:6334 \
        -v "$VECTX_DATA_DIR:/qdrant/storage" \
        vectx:latest > /dev/null 2>&1
    
    if [ $? -eq 0 ]; then
        VECTX_STARTED_BY_US="docker"
        VECTX_MODE="docker"
        return 0
    else
        # Try pulling from registry if local image not found
        echo "   Local image not found, trying to pull..."
        docker pull antonellofratepietro/vectx:latest 2>/dev/null || docker pull ghcr.io/antonellof/vectx:latest 2>/dev/null
        docker run -d \
            --name "$VECTX_DOCKER_CONTAINER" \
            -p 6333:6333 \
            -p 6334:6334 \
            -v "$VECTX_DATA_DIR:/qdrant/storage" \
            antonellofratepietro/vectx:latest > /dev/null 2>&1
        
        if [ $? -eq 0 ]; then
            VECTX_STARTED_BY_US="docker"
            VECTX_MODE="docker"
            return 0
        fi
    fi
    
    return 1
}

# Start vectX via local binary
start_vectx_local() {
    echo -e "${BLUE}📁 Starting vectX via local binary...${NC}"
    
    # Download if not exists
    if [ ! -f "$VECTX_BIN" ]; then
        echo "vectX binary not found. Downloading..."
        if ! download_vectx; then
            return 1
        fi
    fi
    
    # Create data directory
    mkdir -p "$VECTX_DATA_DIR"
    
    # Start in background
    "$VECTX_BIN" --http-port 6333 --grpc-port 6334 --data-dir "$VECTX_DATA_DIR" > /tmp/vectx.log 2>&1 &
    VECTX_PID=$!
    
    VECTX_STARTED_BY_US="local:$VECTX_PID"
    VECTX_MODE="local"
    echo "   Started vectX (PID: $VECTX_PID)"
    return 0
}

# Wait for vectX to be ready
wait_for_vectx() {
    echo "   Waiting for vectX to start..."
    for i in {1..15}; do
        if is_vectx_responding; then
            return 0
        fi
        sleep 1
    done
    return 1
}

# Cleanup function
cleanup() {
    echo ""
    echo "Shutting down..."
    
    if [ -n "$VECTX_STARTED_BY_US" ]; then
        if [[ "$VECTX_STARTED_BY_US" == "docker" ]]; then
            echo "Stopping vectX Docker container..."
            docker stop "$VECTX_DOCKER_CONTAINER" > /dev/null 2>&1
            docker rm "$VECTX_DOCKER_CONTAINER" > /dev/null 2>&1
        elif [[ "$VECTX_STARTED_BY_US" == local:* ]]; then
            local pid="${VECTX_STARTED_BY_US#local:}"
            echo "Stopping vectX (PID: $pid)..."
            kill "$pid" 2>/dev/null
        fi
    fi
}
trap cleanup EXIT

# ============================================================================
# Main Script
# ============================================================================

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║           🚀 Starting RAG Stack with vectX 🚀              ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Load environment variables from .env
load_env || setup_env

# Check if vectX is already running
if is_vectx_responding; then
    # Check if it's running via Docker
    if is_vectx_docker_running; then
        echo -e "${GREEN}✅ vectX is already running via Docker (container: $VECTX_DOCKER_CONTAINER)${NC}"
        VECTX_MODE="docker"
    else
        echo -e "${GREEN}✅ vectX is already running (local or external)${NC}"
        VECTX_MODE="external"
    fi
else
    # vectX is not running - ask user how to start it
    echo -e "${YELLOW}vectX is not running. How would you like to start it?${NC}"
    echo ""
    
    # Check what options are available
    DOCKER_AVAILABLE=false
    LOCAL_AVAILABLE=false
    
    if is_docker_available; then
        DOCKER_AVAILABLE=true
        echo "  1) 🐳 Docker (recommended - easy setup)"
    fi
    
    if [ -f "$VECTX_BIN" ]; then
        LOCAL_AVAILABLE=true
        echo "  2) 📁 Local binary (already downloaded)"
    else
        LOCAL_AVAILABLE=true
        echo "  2) 📁 Local binary (will download from GitHub)"
    fi
    
    echo "  3) ❌ Exit"
    echo ""
    
    # Default choice
    if $DOCKER_AVAILABLE; then
        DEFAULT_CHOICE=1
    else
        DEFAULT_CHOICE=2
    fi
    
    read -p "Enter choice [default: $DEFAULT_CHOICE]: " choice
    choice=${choice:-$DEFAULT_CHOICE}
    
    case $choice in
        1)
            if ! $DOCKER_AVAILABLE; then
                echo -e "${RED}❌ Docker is not available. Please install Docker or choose local binary.${NC}"
                exit 1
            fi
            
            if start_vectx_docker; then
                if wait_for_vectx; then
                    echo -e "${GREEN}✅ vectX started successfully via Docker!${NC}"
                else
                    echo -e "${RED}❌ vectX failed to start. Check Docker logs:${NC}"
                    docker logs "$VECTX_DOCKER_CONTAINER" 2>&1 | tail -20
                    exit 1
                fi
            else
                echo -e "${RED}❌ Failed to start vectX via Docker${NC}"
                exit 1
            fi
            ;;
        2)
            if start_vectx_local; then
                if wait_for_vectx; then
                    echo -e "${GREEN}✅ vectX started successfully!${NC}"
                else
                    echo -e "${RED}❌ vectX failed to start. Check /tmp/vectx.log:${NC}"
                    cat /tmp/vectx.log | tail -20
                    exit 1
                fi
            else
                echo -e "${RED}❌ Failed to start vectX${NC}"
                exit 1
            fi
            ;;
        3)
            echo "Exiting..."
            exit 0
            ;;
        *)
            echo -e "${RED}Invalid choice. Exiting.${NC}"
            exit 1
            ;;
    esac
fi

echo ""

# Check OpenAI API key
if [ -z "$OPENAI_API_KEY" ]; then
    echo -e "${YELLOW}⚠️  OPENAI_API_KEY is not set!${NC}"
    echo ""
    echo "You can set it in one of these ways:"
    echo ""
    echo "  1. Create a .env file (recommended):"
    echo "     cp .env.example .env"
    echo "     # Then edit .env and add your key"
    echo ""
    echo "  2. Export in terminal:"
    echo "     export OPENAI_API_KEY=sk-your-key-here"
    echo ""
    echo "  Get your API key at: https://platform.openai.com/api-keys"
    echo ""
    read -p "Press Enter to continue anyway, or Ctrl+C to exit..."
else
    echo -e "${GREEN}✅ OpenAI API key loaded${NC}"
fi

# Start Streamlit
echo ""
echo -e "${BLUE}Starting Streamlit app...${NC}"
echo "The app will open in your browser at http://localhost:8501"
echo ""
echo -e "${GREEN}vectX API: http://localhost:6333${NC}"
echo -e "${GREEN}vectX Dashboard: http://localhost:6333/dashboard${NC}"
echo ""

streamlit run "$SCRIPT_DIR/app.py"
