# RAG Application with vectX

This project builds a fast RAG application to **chat with your docs**.
We use:
- **OpenAI** as the LLM inference engine (GPT-5 mini by default).
- LlamaIndex for orchestrating the RAG app.
- **vectX** VectorDB for storing the embeddings (fast in-memory vector database).
- Streamlit to build the UI.

## Installation and setup

**Setup OpenAI**:

1. **Get an API key** from [OpenAI](https://platform.openai.com/api-keys)

2. **Set your API key** (choose one method):

   **Option A: Use .env file (recommended)**
   ```bash
   # Copy the example file
   cp .env.example .env
   
   # Edit .env and add your key
   # OPENAI_API_KEY=sk-your-key-here
   ```

   **Option B: Export in terminal**
   ```bash
   export OPENAI_API_KEY=sk-your-key-here
   ```

   The app uses OpenAI with `gpt-4o-mini` by default. No other configuration needed!

   **Available models**:
   - Latest: `gpt-5`, `gpt-5.1`, `gpt-5.2`, `o3`, `o3-mini`, `o4-mini`
   - GPT-4 series: `gpt-4o`, `gpt-4o-mini`, `gpt-4-turbo`
   - Legacy: `gpt-3.5-turbo`

   **To use a different model** (in .env or terminal):
   ```bash
   OPENAI_CHAT_MODEL=gpt-5
   ```

**Setup vectX VectorDB**

vectX is a fast, in-memory vector database with Qdrant API compatibility.

**Option 1: Auto-download (Recommended)**

The `start.sh` script automatically downloads the correct binary for your platform:
```bash
./start.sh  # Downloads vectX if not present, starts everything
```

**Option 2: Install from crates.io**
```bash
cargo install vectx

# Run
vectx --http-port 6333 --grpc-port 6334
```

**Option 3: Download pre-built binary**
```bash
# macOS Apple Silicon
curl -LO https://github.com/antonellof/vectX/releases/latest/download/vectx-macos-arm64.tar.gz
tar -xzf vectx-macos-arm64.tar.gz

# macOS Intel
curl -LO https://github.com/antonellof/vectX/releases/latest/download/vectx-macos-x86_64.tar.gz
tar -xzf vectx-macos-x86_64.tar.gz

# Linux x86_64
curl -LO https://github.com/antonellof/vectX/releases/latest/download/vectx-linux-x86_64.tar.gz
tar -xzf vectx-linux-x86_64.tar.gz

# Linux x86_64 (static/musl)
curl -LO https://github.com/antonellof/vectX/releases/latest/download/vectx-linux-x86_64-musl.tar.gz
tar -xzf vectx-linux-x86_64-musl.tar.gz

# Run
./antonellofratepietro/vectx --http-port 6333 --grpc-port 6334
```

**Option 4: Build from source**
```bash
git clone https://github.com/antonellof/vectX.git
cd vectX
cargo build --release
./target/release/vectx --http-port 6333 --grpc-port 6334
```

The server will start on `http://localhost:6333` (same port as Qdrant for compatibility).

**Install Dependencies**:
   Ensure you have Python 3.11 or later installed.
   ```bash
   # Install all dependencies from requirements.txt
   pip install -r requirements.txt
   ```
   
   Or install individually:
   ```bash
   pip install streamlit qdrant-client python-dotenv
   pip install llama-index-core llama-index-embeddings-huggingface llama-index-llms-openai
   pip install torch transformers sentence-transformers
   ```

   Note: We use `qdrant-client` library which works with vectX due to Qdrant API compatibility.

   **Note on torchvision warning:** If you see a torchvision warning about image extensions, you can safely ignore it. The RAG stack only uses text embeddings, not image processing. See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for details.

**Quick Start**:

The easiest way to run everything:
```bash
export OPENAI_API_KEY=your-key-here
./start.sh
```

This will:
1. Download vectX automatically (if not present)
2. Start the vectX vector database
3. Launch the Streamlit app

The app will open in your browser at `http://localhost:8501`

**Manual Start** (if you prefer):

1. **Start vectX server** (in one terminal):
   ```bash
   vectx --http-port 6333 --grpc-port 6334
   ```

2. **Run the RAG app** (in another terminal):
   ```bash
   streamlit run app.py
   ```

**Troubleshooting**:
   If the app doesn't start, run the diagnostic script:
   ```bash
   python3 run_test.py
   ```
   
   This will check:
   - All dependencies are installed
   - vectX server is running
   - OpenAI API key is set
   - App syntax is valid

**Environment Variables** (create a `.env` file):

```bash
# Required: OpenAI API key
OPENAI_API_KEY=your-openai-api-key-here

# Optional: change model (default: gpt-5-mini)
# LLM_MODEL=gpt-4o-mini  # or gpt-4o, gpt-4-turbo, gpt-3.5-turbo, etc.
```


---
## Contribution

Contributions are welcome! Please fork the repository and submit a pull request with your improvements.
