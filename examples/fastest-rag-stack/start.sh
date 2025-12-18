#!/bin/bash
# Startup script for RAG Stack

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DISTX_VERSION="v0.1.1"
DISTX_BIN_DIR="$SCRIPT_DIR/.distx"
DISTX_BIN="$DISTX_BIN_DIR/distx"

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

download_distx() {
    local platform=$(detect_platform)
    
    if [ "$platform" = "unsupported" ]; then
        echo "❌ Unsupported platform: $(uname -s) $(uname -m)"
        echo "Please build from source:"
        echo "  git clone https://github.com/antonellof/DistX.git"
        echo "  cd DistX && cargo build --release"
        exit 1
    fi
    
    local download_url="https://github.com/antonellof/DistX/releases/download/${DISTX_VERSION}/distx-${platform}.tar.gz"
    local temp_file="/tmp/distx-${platform}.tar.gz"
    
    echo "📦 Downloading DistX ${DISTX_VERSION} for ${platform}..."
    echo "   URL: $download_url"
    
    # Create bin directory
    mkdir -p "$DISTX_BIN_DIR"
    
    # Download
    if command -v curl &> /dev/null; then
        curl -L -o "$temp_file" "$download_url"
    elif command -v wget &> /dev/null; then
        wget -O "$temp_file" "$download_url"
    else
        echo "❌ Neither curl nor wget found. Please install one of them."
        exit 1
    fi
    
    # Extract to temp dir first
    echo "📂 Extracting..."
    local temp_extract="/tmp/distx_extract_$$"
    mkdir -p "$temp_extract"
    tar -xzf "$temp_file" -C "$temp_extract"
    
    # Find and move the binary (archive contains distx/distx)
    if [ -f "$temp_extract/distx/distx" ]; then
        cp "$temp_extract/distx/distx" "$DISTX_BIN"
        rm -rf "$temp_extract"
    else
        echo "❌ Binary not found in archive"
        ls -la "$temp_extract"
        rm -rf "$temp_extract"
        exit 1
    fi
    
    # Make executable
    chmod +x "$DISTX_BIN"
    
    # Cleanup
    rm -f "$temp_file"
    
    echo "✅ DistX installed to $DISTX_BIN"
}

echo "Starting RAG Stack with DistX..."
echo ""

# Kill any existing DistX processes first
if pgrep -f "distx" > /dev/null 2>&1; then
    echo "Killing existing DistX processes..."
    pkill -9 -f "distx" 2>/dev/null
    sleep 2
fi

# Also kill any process using port 6333
if lsof -ti:6333 > /dev/null 2>&1; then
    echo "Killing process on port 6333..."
    lsof -ti:6333 | xargs kill -9 2>/dev/null
    sleep 1
fi

# Check if DistX binary exists, download if not
if [ ! -f "$DISTX_BIN" ]; then
    echo "DistX binary not found. Downloading from GitHub releases..."
    download_distx
fi

# Check if DistX is running
if ! curl -s http://localhost:6333/collections > /dev/null 2>&1; then
    echo "Starting DistX server..."
    
    if [ -f "$DISTX_BIN" ]; then
        # Use persistent data directory
        DISTX_DATA_DIR="$SCRIPT_DIR/.distx_data"
        mkdir -p "$DISTX_DATA_DIR"
        
        # Start DistX in background with persistent data dir
        "$DISTX_BIN" --http-port 6333 --grpc-port 6334 --data-dir "$DISTX_DATA_DIR" > /tmp/distx.log 2>&1 &
        DISTX_PID=$!
        echo "Started DistX (PID: $DISTX_PID)"
        
        # Wait for DistX to be ready
        echo "Waiting for DistX to start..."
        for i in {1..10}; do
            if curl -s http://localhost:6333/collections > /dev/null 2>&1; then
                echo "✅ DistX is ready!"
                break
            fi
            sleep 1
        done
        
        if ! curl -s http://localhost:6333/collections > /dev/null 2>&1; then
            echo "⚠️  DistX didn't start properly. Check /tmp/distx.log"
            echo ""
            cat /tmp/distx.log | tail -20
            echo ""
            read -p "Press Enter to continue anyway, or Ctrl+C to exit..."
        fi
    else
        echo "❌ DistX binary not found and download failed."
        exit 1
    fi
else
    echo "✅ DistX is already running"
fi

# Check OpenAI API key
if [ -z "$OPENAI_API_KEY" ]; then
    echo "⚠️  OPENAI_API_KEY is not set!"
    echo ""
    echo "Please set it:"
    echo "  export OPENAI_API_KEY=your-key-here"
    echo ""
    read -p "Press Enter to continue anyway, or Ctrl+C to exit..."
fi

# Cleanup function
cleanup() {
    echo ""
    echo "Shutting down..."
    if [ -n "$DISTX_PID" ]; then
        echo "Stopping DistX (PID: $DISTX_PID)..."
        kill $DISTX_PID 2>/dev/null
    fi
}
trap cleanup EXIT

# Start Streamlit
echo ""
echo "Starting Streamlit app..."
echo "The app will open in your browser at http://localhost:8501"
echo ""
streamlit run app.py
