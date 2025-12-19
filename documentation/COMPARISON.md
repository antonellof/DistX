# DistX Similarity Engine: Competitive Comparison

> How does DistX's Similarity Engine compare to other solutions for finding similar records in tabular data?

## Executive Summary

| Solution | Embeddings Required | Schema-Driven | Explainability | Setup Complexity | Best For |
|----------|:------------------:|:-------------:|:--------------:|:----------------:|----------|
| **DistX Similarity** | ❌ No | ✅ Yes | ✅ Per-field | ⭐ Simple | Structured data, explainable similarity |
| Qdrant + OpenAI | ✅ Yes | ❌ No | ❌ No | ⭐⭐⭐ Complex | Semantic text/image search |
| Pinecone | ✅ Yes | ❌ No | ❌ No | ⭐⭐ Medium | Managed vector search |
| Weaviate | ✅ Yes | Partial | ❌ No | ⭐⭐⭐ Complex | Multi-modal AI apps |
| PostgreSQL + pgvector | ✅ Yes | ❌ No | ❌ No | ⭐⭐ Medium | Existing Postgres users |
| Elasticsearch | Optional | Partial | Limited | ⭐⭐⭐ Complex | Full-text + structured |
| Solr MoreLikeThis | ❌ No | ❌ No | Limited | ⭐⭐⭐ Complex | Document similarity |
| Algolia Recommend | ✅ Yes | ❌ No | ❌ No | ⭐⭐ Medium | E-commerce (managed) |
| Typesense | ✅ Yes | ❌ No | ❌ No | ⭐⭐ Medium | Fast typo-tolerant search |
| Custom SQL | ❌ No | ❌ Manual | ❌ No | ⭐⭐⭐⭐ Hard | Simple exact matching |

---

## Detailed Comparison

### 1. Vector Databases (Qdrant, Pinecone, Weaviate, Milvus)

**How they work:**
```
Your Data → External ML Model → Embeddings → Vector DB → ANN Search → Results
            (OpenAI, Cohere)     (384-1536d)
```

**DistX Similarity Engine:**
```
Your Data → Similarity Schema → Auto-Embedding → ANN + Rerank → Explainable Results
            (JSON config)       (built-in)
```

| Aspect | Vector Databases | DistX Similarity |
|--------|-----------------|------------------|
| **Embedding generation** | External (OpenAI, Cohere, HuggingFace) | Built-in, deterministic |
| **Cost per insert** | $0.0001-0.001 (API calls) | $0 (no external calls) |
| **Latency per insert** | 50-500ms (API roundtrip) | <1ms (local computation) |
| **Explainability** | Black box - "score: 0.87" | Per-field breakdown |
| **Offline capability** | ❌ Requires ML API | ✅ Fully offline |
| **Structured data** | Must embed all fields together | Native field-level control |
| **Weight control** | Cannot adjust post-embedding | Dynamic at query time |

**When to use Vector DBs instead:**
- Semantic text search ("find documents about climate change")
- Image/audio similarity
- When you already have an embedding pipeline
- Unstructured data (long documents, transcripts)

---

### 2. PostgreSQL + pgvector

**How it works:**
```sql
-- Still need external embeddings!
INSERT INTO products (name, price, embedding) 
VALUES ('iPhone', 999, '[0.1, 0.2, ...]'::vector);

-- Hybrid search requires manual query building
SELECT * FROM products 
WHERE price BETWEEN 500 AND 1500
ORDER BY embedding <-> query_embedding
LIMIT 10;
```

**DistX Similarity Engine:**
```bash
# No embeddings needed, just insert data
curl -X PUT /collections/products/points -d '{
  "points": [{"id": 1, "payload": {"name": "iPhone", "price": 999}}]
}'

# Query by example
curl -X POST /collections/products/similar -d '{
  "example": {"name": "phone", "price": 800}
}'
```

| Aspect | PostgreSQL + pgvector | DistX Similarity |
|--------|----------------------|------------------|
| **Setup** | Extension + embedding pipeline | Single binary |
| **Schema changes** | ALTER TABLE migrations | JSON schema update |
| **Embedding cost** | External ML API required | None |
| **Query syntax** | SQL with vector operators | Simple JSON |
| **Explainability** | None | Per-field scores |
| **Weight adjustment** | New embedding required | Query-time override |

**When to use pgvector instead:**
- You're already all-in on PostgreSQL
- Need ACID transactions with vectors
- Have existing embedding infrastructure
- Want SQL-level joins with vector search

---

### 3. Elasticsearch / OpenSearch

**How it works:**
```json
// Hybrid search requires complex DSL
{
  "query": {
    "script_score": {
      "query": {"match": {"name": "prosciutto"}},
      "script": {
        "source": "cosineSimilarity(params.query_vector, 'embedding') + 1.0",
        "params": {"query_vector": [0.1, 0.2, ...]}
      }
    }
  }
}
```

**DistX Similarity Engine:**
```json
{
  "example": {"name": "prosciutto", "price": 5.0},
  "limit": 10
}
```

| Aspect | Elasticsearch | DistX Similarity |
|--------|--------------|------------------|
| **Query complexity** | Complex DSL, script_score | Simple JSON |
| **Embedding** | External required for vectors | Built-in |
| **Field weighting** | Manual boosting per query | Schema-defined + overrides |
| **Explainability** | Debug explain API | Native per-field scores |
| **Memory usage** | High (JVM heap) | Low (Rust native) |
| **Setup** | Cluster management | Single binary |

**When to use Elasticsearch instead:**
- Full-text search is primary use case
- Need aggregations, faceting, analytics
- Already running ELK stack
- Need distributed cluster for massive scale

---

### 4. Apache Solr MoreLikeThis

**How it works:**
```xml
<!-- Find documents like document ID 123 -->
<query>
  <requestHandler name="/mlt" class="solr.MoreLikeThisHandler">
    <lst name="defaults">
      <str name="mlt.fl">title,description</str>
      <int name="mlt.mintf">1</int>
    </lst>
  </requestHandler>
</query>
```

| Aspect | Solr MLT | DistX Similarity |
|--------|----------|------------------|
| **Algorithm** | TF-IDF on text fields | Multi-type field similarity |
| **Numeric fields** | Not supported well | Native relative/absolute distance |
| **Categorical** | Treated as text | Exact match with weighting |
| **Boolean fields** | Not supported | Native support |
| **Configuration** | XML config files | JSON schema |
| **Explainability** | Term-level debug | Per-field contribution |

**When to use Solr MLT instead:**
- Document-centric similarity (articles, papers)
- Already running Solr infrastructure
- Text-only similarity needs

---

### 5. Search-as-a-Service (Algolia, Typesense)

**How they work:**

| Feature | Algolia Recommend | Typesense | DistX Similarity |
|---------|------------------|-----------|------------------|
| **Pricing** | Per-search pricing | Self-hosted free | Self-hosted free |
| **Similarity source** | User behavior + ML | Vector embeddings | Field schema |
| **Setup time** | Hours (data upload) | Hours (embedding gen) | Minutes |
| **Explainability** | "AI-powered" | None | Per-field scores |
| **Customization** | Limited | Vector control | Full weight control |
| **Offline** | ❌ Cloud only | ✅ Self-hosted | ✅ Self-hosted |

**When to use Algolia/Typesense instead:**
- Need managed infrastructure
- User behavior-based recommendations
- Typo-tolerant search is priority

---

### 6. Custom SQL / Rule-Based Systems

**Traditional approach:**
```sql
-- Manual similarity scoring
SELECT *, 
  (CASE WHEN category = 'salumi' THEN 0.3 ELSE 0 END) +
  (1.0 - ABS(price - 5.0) / 10.0) * 0.3 +
  (CASE WHEN name ILIKE '%prosciutto%' THEN 0.4 ELSE 0 END) AS score
FROM products
ORDER BY score DESC
LIMIT 10;
```

**Problems with this approach:**
- ❌ No fuzzy text matching
- ❌ Hardcoded weights in queries
- ❌ Manual score normalization
- ❌ Slow on large datasets (no index)
- ❌ Complex maintenance

**DistX Similarity Engine:**
```json
{
  "example": {"name": "prosciutto", "price": 5.0, "category": "salumi"},
  "weights": {"price": 0.5}  // Override at query time
}
```

---

## Feature Matrix

| Feature | DistX | Qdrant | Pinecone | Weaviate | pgvector | Elastic |
|---------|:-----:|:------:|:--------:|:--------:|:--------:|:-------:|
| No external embeddings | ✅ | ❌ | ❌ | ❌ | ❌ | Partial |
| Query by example | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Per-field explainability | ✅ | ❌ | ❌ | ❌ | ❌ | Debug |
| Dynamic weight override | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Schema-driven | ✅ | ❌ | ❌ | Partial | ❌ | Partial |
| Numeric field similarity | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Boolean field similarity | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Categorical exact match | ✅ | Filter | Filter | Filter | Filter | Filter |
| Text fuzzy matching | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Offline / Air-gapped | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ |
| Single binary deploy | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Qdrant API compatible | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |

---

## Cost Comparison (10M records, 1M queries/month)

| Solution | Insert Cost | Query Cost | Infrastructure | Total/Month |
|----------|-------------|------------|----------------|-------------|
| **DistX** | $0 | $0 | ~$50 (self-host) | **~$50** |
| Pinecone | $0 | ~$200 | Managed | **~$500** |
| Qdrant Cloud | $0 | ~$100 | Managed | **~$300** |
| OpenAI + Vector DB | ~$1000 | ~$100 | ~$100 | **~$1200** |
| Algolia | N/A | Usage-based | Managed | **~$1000+** |
| Self-host + OpenAI | ~$1000 | $0 | ~$50 | **~$1050** |

*Assumes OpenAI ada-002 at $0.0001/1K tokens for embedding generation*

---

## When to Use DistX Similarity Engine

### ✅ Perfect For:

1. **Structured/Tabular Data**
   - Product catalogs
   - Customer records (CRM)
   - Supplier databases (ERP)
   - Inventory management

2. **When Explainability Matters**
   - Regulated industries (finance, healthcare)
   - Customer-facing recommendations
   - Audit requirements

3. **Cost-Sensitive Applications**
   - No ML API costs
   - No embedding infrastructure
   - Simple self-hosting

4. **Offline/Air-Gapped Environments**
   - On-premise deployments
   - Edge computing
   - Privacy-sensitive data

5. **Rapid Prototyping**
   - Minutes to first query
   - No ML pipeline setup
   - Iterate on schema quickly

### ❌ Not Ideal For:

1. **Semantic Text Search**
   - "Find articles about climate policy" → Use vector DB + embeddings

2. **Image/Audio Similarity**
   - Visual product search → Use CLIP embeddings + vector DB

3. **Unstructured Documents**
   - Long-form content → Use embeddings for chunking

4. **User Behavior-Based Recommendations**
   - "Customers who bought X also bought Y" → Use Algolia Recommend or custom ML

---

## Migration Path

### From Qdrant/Pinecone/Weaviate:

If you're using vector databases for structured data similarity:

```python
# Before: External embedding + vector search
embedding = openai.embed(product_description)
results = qdrant.search(embedding, limit=10)

# After: Direct similarity query
results = distx.similar({
    "name": product_name,
    "price": price,
    "category": category
}, limit=10)
```

**Benefits:**
- Remove OpenAI/Cohere API dependency
- Reduce latency (no API roundtrip)
- Get per-field explainability
- Adjust weights without re-embedding

### From Custom SQL:

```sql
-- Before: Complex manual scoring
SELECT *, complex_score_calculation() FROM products ORDER BY score;

-- After: Schema-driven similarity
PUT /collections/products {"similarity_schema": {...}}
POST /collections/products/similar {"example": {...}}
```

**Benefits:**
- Fuzzy text matching built-in
- No manual score normalization
- ANN index for performance
- Maintainable schema definition

---

## Conclusion

DistX Similarity Engine fills a unique gap:

| Need | Traditional Solution | DistX Advantage |
|------|---------------------|-----------------|
| Structured data similarity | Vector DB + embeddings | No embeddings needed |
| Explainable results | Custom implementation | Built-in per-field scores |
| Dynamic weights | Re-embed everything | Query-time overrides |
| Simple setup | Complex ML pipeline | JSON schema + go |

**Try it:**
```bash
docker run -p 6333:6333 distx:similarity
python scripts/similarity_demo.py
```
