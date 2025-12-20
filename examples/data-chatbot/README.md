# DistX Data Chatbot

> **Query your tabular data with natural language. Find similar records. Get explainable results.**

A demo application showing DistX's structured similarity features integrated with an AI chatbot. Import CSV data, ask natural language questions, and get explainable similarity search results.

## What This Demo Shows

| Feature | Description |
|---------|-------------|
| **CSV Import** | Upload or paste CSV data and start querying immediately |
| **Auto Schema** | Field types and weights inferred automatically |
| **Semantic Search** | Client-side embeddings (OpenAI) for text similarity |
| **Structured Reranking** | Price, category, and other fields scored separately |
| **Explainable Results** | See which fields contributed to each match |
| **Query by Example** | "Find products similar to iPhone" |

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
        │  OpenAI  │ │  DistX   │ │ Postgres │
        │   LLM    │ │  Client  │ │  (Chat)  │
        └──────────┘ └──────────┘ └──────────┘
                           │
                           ▼
              ┌─────────────────────────┐
              │   DistX Server          │
              │   (Vector Database)     │
              │                         │
              │ • Stores embeddings     │
              │ • ANN search            │
              │ • Structured reranking  │
              │ • Explainable scores    │
              └─────────────────────────┘
```

## Quick Start

### 1. Start DistX

```bash
# Option A: Docker
docker run -p 6333:6333 distx/distx

# Option B: From source (in the distx root)
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

# DistX connection (default: localhost:6333)
DISTX_URL=http://localhost:6333

# Database for chat history
POSTGRES_URL=postgresql://user:pass@localhost:5432/chatbot
```

### 3. Install & Run

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

| Tool | Description |
|------|-------------|
| `importData` | Import CSV data with auto schema inference |
| `listDataCollections` | List all available collections |
| `exploreData` | View schema and sample data |
| `findSimilar` | Query by example with explanations |
| `findSimilarById` | Find records similar to an existing one |
| `compareSimilarity` | Compare two records in detail |

## How It Works

1. **Import**: When you upload CSV data:
   - Schema is inferred (field types, weights)
   - Text fields are embedded using OpenAI
   - Vectors + payloads stored in DistX

2. **Query**: When you ask "find similar to X":
   - Query text is embedded
   - DistX performs ANN search
   - Results reranked by structured similarity
   - Per-field contributions explained

3. **Reranking**: Combines:
   - **Vector similarity** (semantic meaning)
   - **Numeric distance** (price proximity)
   - **Categorical match** (same category)
   - **Weights** (field importance)

## Key Differences from Traditional RAG

| Traditional RAG | DistX Data Chatbot |
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
- [DistX](https://github.com/antonellof/distx) - Vector database
- [Qdrant JS Client](https://github.com/qdrant/qdrant-js) - API client
- [PostgreSQL](https://postgresql.org) - Chat history

## Related Documentation

- [DistX Similarity Engine](../../documentation/SIMILARITY_ENGINE.md)
- [Comparison with Alternatives](../../documentation/COMPARISON.md)
- [Demo Output](../../documentation/SIMILARITY_DEMO.md)

## License

MIT
