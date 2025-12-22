# vectX Data Chatbot

> **Query tabular data with natural language. Vector search, filtering, recommendations — all with explainable results.**

A demo application that showcases all vectX features through an AI chatbot interface. Upload CSV data, ask natural language questions, and get results with per-field score breakdowns.

## What This Demo Shows

| Feature | Description |
|---------|-------------|
| **CSV Import** | Upload via attachment or paste in chat |
| **Auto Schema** | Field types and weights inferred automatically |
| **Semantic Search** | Text → OpenAI embeddings → vector similarity |
| **Filtering** | Exact matches, ranges, text search |
| **Faceted Search** | Aggregated counts by category |
| **Recommendations** | Based on positive/negative examples |
| **Explainable Results** | Per-field contribution breakdown |
| **Full CRUD** | Get, update, delete records |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Data Chatbot UI                          │
│                   (Next.js + React)                         │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    AI SDK + OpenAI                          │
│        • Chat responses    • Tool execution                 │
│        • Embeddings        • Streaming                      │
└─────────────────────────────────────────────────────────────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │  OpenAI  │ │  vectX   │ │ Postgres │
        │   LLM    │ │  Client  │ │  (Chat)  │
        └──────────┘ └──────────┘ └──────────┘
                           │
                           ▼
              ┌─────────────────────────┐
              │   vectX Server          │
              │   (Vector Database)     │
              │                         │
              │ • Stores embeddings     │
              │ • ANN search            │
              │ • Structured reranking  │
              │ • Explainable scores    │
              └─────────────────────────┘
```

## Quick Start

### 1. Start vectX

```bash
# Option A: Docker
docker run -p 6333:6333 antonellofratepietro/vectx

# Option B: From source (in the vectx root)
cargo run --release
```

### 2. Setup Environment

```bash
cd examples/data-chatbot
cp .env.example .env.local
```

Edit `.env.local`:
```env
# Required: OpenAI for chat and embeddings
OPENAI_API_KEY=sk-...

# PDF Parser Service (default: http://localhost:8000)
# Only needed if running PDF parser service separately
PDF_PARSER_SERVICE_URL=http://localhost:8000

# vectX connection (default: localhost:6333)
VECTX_URL=http://localhost:6333

# Database for chat history
POSTGRES_URL=postgresql://user:pass@localhost:5432/chatbot
```

### 3. Start PDF Parser Service (Optional for local dev)

The PDF parser service is automatically started with Docker Compose. For local development:

```bash
cd pdf-parser-service
pip install -r requirements.txt
python main.py
```

The service will run on http://localhost:8000

### 4. Install & Run

```bash
pnpm install
pnpm db:migrate
pnpm dev
```

Open http://localhost:3000

## Example Conversation

```
You: Here's my product data:
name,price,category,brand
iPhone 15 Pro,1199,electronics,Apple
Galaxy S24,999,electronics,Samsung
MacBook Pro,2499,computers,Apple
Dell XPS 15,1799,computers,Dell
AirPods Pro,249,accessories,Apple

AI: ✅ Imported 5 products with embeddings! Here's the inferred schema:

| Field    | Type        | Weight | Distance     |
|----------|-------------|--------|--------------|
| name     | text        | 0.35   | semantic     |
| price    | number      | 0.25   | relative     |
| category | categorical | 0.20   | exact        |
| brand    | categorical | 0.20   | exact        |

You can now ask questions like:
- "Find products similar to iPhone"
- "Show me cheaper alternatives to MacBook"
- "Find all Apple products"

You: Find products similar to iPhone 15 Pro

AI: Found 4 similar products:

| Rank | Product       | Score | Top Contributors           |
|------|---------------|-------|----------------------------|
| 1    | Galaxy S24    | 0.72  | category: 0.20, price: 0.18|
| 2    | AirPods Pro   | 0.45  | brand: 0.20, name: 0.10    |
| 3    | MacBook Pro   | 0.38  | brand: 0.20                |
| 4    | Dell XPS 15   | 0.25  | category: 0.05             |

The Galaxy S24 matched best because:
- Same category (electronics): +0.20
- Similar price range ($999 vs $1199): +0.18
- Semantic similarity in product names
```

## Available AI Tools

### Semantic & Similarity Search
| Tool | Description | Example Query |
|------|-------------|---------------|
| `vectorSearch` | Semantic search by text | "Find products like iPhone" |
| `findSimilar` | Query by example with reranking | "Find phones similar to Galaxy but cheaper" |
| `recommend` | Recommendations from examples | "More like #1 and #5, not like #3" |
| `findSimilarById` | Similar to existing record | "Find items like record 42" |

### Filtering & Browsing
| Tool | Description | Example Query |
|------|-------------|---------------|
| `filterRecords` | Exact filter conditions | "Show all electronics under $500" |
| `textSearch` | Substring/text match | "Find products with 'Pro' in name" |
| `getFacets` | Aggregated counts | "How many in each category?" |
| `countRecords` | Count matching records | "How many products in stock?" |

### Data Operations
| Tool | Description | Example Query |
|------|-------------|---------------|
| `listDataCollections` | List collections with schemas | "What data do I have?" |
| `exploreData` | View schema and samples | "Show me the products schema" |
| `getRecord` | Get specific record | "Show me record #123" |
| `compareSimilarity` | Compare two records | "How different are #1 and #5?" |
| `importData` | Import CSV from chat | (paste CSV data) |
| `deleteRecords` | Delete by filter | "Remove discontinued items" |

## Example Queries

After uploading your CSV, try these:

**Similarity Search:**
- "Find products similar to iPhone"
- "Show me items like MacBook but cheaper"
- "Find customers similar to our best account"

**Filtering & Browsing:**
- "Show all electronics in stock"
- "List products under $100 in the Accessories category"
- "How many products are in each category?"

**Analysis:**
- "How different are products #1 and #5?"
- "Find potential duplicates in the data"
- "What are the top 5 most expensive items?"

**Recommendations:**
- "Recommend products like #3 and #7"
- "Find alternatives to discontinued items"

---

## How It Works

### 1. Import
```
CSV Upload → Schema Inference → OpenAI Embeddings → vectX Storage
```
- Field types detected automatically (text, number, categorical)
- Text fields embedded using OpenAI `text-embedding-3-small`
- Vectors + structured payloads stored in vectX

### 2. Query
```
Natural Language → AI Tool Selection → vectX API → Formatted Results
```
- AI chooses the right tool (vector search, filter, facet, etc.)
- Queries sent to vectX with appropriate parameters
- Results formatted with explanations

### 3. Hybrid Search
When similarity is needed, vectX combines:
- **Vector similarity** — semantic meaning from embeddings
- **Structured reranking** — per-field scoring with weights
- **Filtering** — exact matches and ranges

## Key Differences from Traditional RAG

| Traditional RAG | vectX Data Chatbot |
|-----------------|-------------------|
| Unstructured text (PDFs) | Structured data (CSV, tables) |
| Black-box similarity | Explainable per-field scores |
| Text chunks | Complete records |
| One embedding per chunk | Semantic + structured hybrid |

## Use Cases

- **E-commerce**: "Find products similar to X but cheaper"
- **CRM**: "Show customers similar to our best accounts"
- **Data Quality**: "Find potential duplicates in this dataset"
- **Analysis**: "Which records are most similar/different?"
- **ERP**: "Find alternative suppliers with similar specs"

## Tech Stack

- [Next.js](https://nextjs.org) - React framework
- [AI SDK](https://ai-sdk.dev) - LLM integration
- [OpenAI](https://openai.com) - Chat and embeddings
- [vectX](https://github.com/antonellof/vectX) - Vector database
- [Qdrant JS Client](https://github.com/qdrant/qdrant-js) - API client
- [FastAPI](https://fastapi.tiangolo.com) - Python PDF parser service (PyMuPDF/pypdf)
- [PostgreSQL](https://postgresql.org) - Chat history

## Related Documentation

- [vectX Similarity Engine](../../documentation/SIMILARITY_ENGINE.md)
- [Comparison with Alternatives](../../documentation/COMPARISON.md)
- [Demo Output](../../documentation/SIMILARITY_DEMO.md)

## License

MIT
