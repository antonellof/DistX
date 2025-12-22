import { tool } from "ai";
import { z } from "zod";
import { getVectXClient, generateEmbedding } from "@/lib/vectx";

// Debug logging helper
const debug = (action: string, data?: unknown) => {
  const msg = data 
    ? `[vectX] 🔎 ${action}: ${JSON.stringify(data)}`
    : `[vectX] 🔎 ${action}`;
  console.warn(msg);
};

/**
 * Standard vector search with filtering
 */
export const vectorSearch = tool({
  description: `Semantic search with optional filtering.

Use when user wants to find items similar to something.

Parameters:
- query: The thing to find similar to (required)
- filter: Additional constraints like category, availability, price (optional)

Combine the search term AND all filters in this ONE call.
Don't make a separate filterRecords call for the same query.

Filter syntax:
- Exact: { "Category": "Electronics" }
- Range: { "Price": { "lte": 200 } }
- Multiple: { "Category": "Electronics", "Availability": "in_stock" }`,

  inputSchema: z.object({
    collectionName: z.string().describe("Name of the collection to search"),
    query: z.string().describe("Text query to search for (will be converted to embedding)"),
    filter: z.record(z.unknown()).optional().describe("Filter conditions: { field: value } for exact match, { field: { gte, lte } } for range"),
    limit: z.number().optional().default(10).describe("Maximum results to return"),
  }),

  execute: async (input) => {
    debug("vectorSearch called", { collection: input.collectionName, query: input.query, limit: input.limit });
    
    try {
      // Generate embedding for the query
      const queryVector = await generateEmbedding(input.query);
      debug("Generated query embedding", { dim: queryVector.length });

      // Build the search request
      const searchBody: Record<string, unknown> = {
        vector: queryVector,
        limit: input.limit,
        with_payload: true,
      };

      // Build filter if provided
      if (input.filter && Object.keys(input.filter).length > 0) {
        const mustConditions: unknown[] = [];
        
        for (const [field, value] of Object.entries(input.filter)) {
          if (typeof value === 'object' && value !== null && ('gte' in value || 'lte' in value || 'gt' in value || 'lt' in value)) {
            // Range filter
            mustConditions.push({ key: field, range: value });
          } else {
            // Exact match
            mustConditions.push({ key: field, match: { value } });
          }
        }
        
        if (mustConditions.length > 0) {
          searchBody.filter = { must: mustConditions };
          debug("Applied filter", { filter: searchBody.filter });
        }
      }

      const response = await fetch(
        `${process.env.VECTX_URL || 'http://localhost:6333'}/collections/${input.collectionName}/points/search`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(searchBody),
        }
      );

      if (!response.ok) {
        throw new Error(`Search failed: ${await response.text()}`);
      }

      const data = await response.json();
      const results = data.result || [];
      debug("Search results", { count: results.length });

      return {
        success: true,
        query: input.query,
        filter: input.filter,
        resultCount: results.length,
        results: results.map((r: { id: string | number; score: number; payload: Record<string, unknown> }, i: number) => ({
          rank: i + 1,
          id: r.id,
          score: Math.round(r.score * 1000) / 1000,
          data: r.payload,
        })),
        message: `Found ${results.length} results for "${input.query}"`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Vector search failed",
      };
    }
  },
});

/**
 * Full-text search within text fields
 */
export const textSearch = tool({
  description: `Search for exact text matches or substrings within text fields.

Unlike vector search (semantic similarity), this finds exact text matches.

Use this when:
- User wants to find records containing specific words
- User is searching for exact product names, codes, or IDs
- You need substring matching`,

  inputSchema: z.object({
    collectionName: z.string().describe("Name of the collection"),
    field: z.string().describe("Text field to search in (e.g., 'Name', 'Description')"),
    text: z.string().describe("Text to search for"),
    limit: z.number().optional().default(10).describe("Maximum results"),
  }),

  execute: async (input) => {
    debug("textSearch called", { collection: input.collectionName, field: input.field, text: input.text });
    
    try {
      const response = await fetch(
        `${process.env.VECTX_URL || 'http://localhost:6333'}/collections/${input.collectionName}/points/scroll`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            filter: {
              must: [{
                key: input.field,
                match: { text: input.text }
              }]
            },
            limit: input.limit,
            with_payload: true,
          }),
        }
      );

      if (!response.ok) {
        throw new Error(`Text search failed: ${await response.text()}`);
      }

      const data = await response.json();
      const points = data.result?.points || [];
      debug("Text search results", { count: points.length });

      return {
        success: true,
        field: input.field,
        searchText: input.text,
        resultCount: points.length,
        results: points.map((p: { id: string | number; payload: Record<string, unknown> }, i: number) => ({
          rank: i + 1,
          id: p.id,
          data: p.payload,
        })),
        message: `Found ${points.length} records containing "${input.text}" in ${input.field}`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Text search failed",
      };
    }
  },
});

/**
 * Get aggregated counts by field values (faceted search)
 */
export const getFacets = tool({
  description: `Get aggregated counts for a field's values (like category breakdown).

Use this to answer questions like:
- "How many products are in each category?"
- "What are the available statuses and their counts?"
- "Show me a breakdown by brand"`,

  inputSchema: z.object({
    collectionName: z.string().describe("Name of the collection"),
    field: z.string().describe("Field to aggregate by (e.g., 'Category', 'Brand', 'Availability')"),
    limit: z.number().optional().default(20).describe("Maximum number of facet values to return"),
  }),

  execute: async (input) => {
    debug("getFacets called", { collection: input.collectionName, field: input.field });
    
    try {
      const response = await fetch(
        `${process.env.VECTX_URL || 'http://localhost:6333'}/collections/${input.collectionName}/facet`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            key: input.field,
            limit: input.limit,
          }),
        }
      );

      if (!response.ok) {
        throw new Error(`Facet query failed: ${await response.text()}`);
      }

      const data = await response.json();
      const hits = data.result?.hits || [];
      debug("Facet results", { count: hits.length });

      return {
        success: true,
        field: input.field,
        facets: hits.map((h: { value: string; count: number }) => ({
          value: h.value,
          count: h.count,
        })),
        totalValues: hits.length,
        message: `Found ${hits.length} unique values for "${input.field}"`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Facet query failed",
      };
    }
  },
});

/**
 * Recommend items based on positive/negative examples
 */
export const recommend = tool({
  description: `Get recommendations based on positive and negative examples.

Provide IDs of items the user likes (positive) and dislikes (negative) to get personalized recommendations.

Use this when:
- User says "more like this" or "similar to these"
- User provides multiple examples to learn from
- User wants to exclude certain types of items`,

  inputSchema: z.object({
    collectionName: z.string().describe("Name of the collection"),
    positive: z.array(z.union([z.string(), z.number()])).describe("IDs of items the user likes"),
    negative: z.array(z.union([z.string(), z.number()])).optional().describe("IDs of items to avoid"),
    filter: z.record(z.unknown()).optional().describe("Additional filter conditions"),
    limit: z.number().optional().default(10).describe("Maximum recommendations"),
  }),

  execute: async (input) => {
    debug("recommend called", { 
      collection: input.collectionName, 
      positive: input.positive, 
      negative: input.negative 
    });
    
    try {
      const body: Record<string, unknown> = {
        positive: input.positive,
        limit: input.limit,
        with_payload: true,
      };

      if (input.negative && input.negative.length > 0) {
        body.negative = input.negative;
      }

      if (input.filter && Object.keys(input.filter).length > 0) {
        const mustConditions: unknown[] = [];
        for (const [field, value] of Object.entries(input.filter)) {
          if (typeof value === 'object' && value !== null && ('gte' in value || 'lte' in value)) {
            mustConditions.push({ key: field, range: value });
          } else {
            mustConditions.push({ key: field, match: { value } });
          }
        }
        if (mustConditions.length > 0) {
          body.filter = { must: mustConditions };
        }
      }

      const response = await fetch(
        `${process.env.VECTX_URL || 'http://localhost:6333'}/collections/${input.collectionName}/points/recommend`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        }
      );

      if (!response.ok) {
        throw new Error(`Recommend failed: ${await response.text()}`);
      }

      const data = await response.json();
      const results = data.result || [];
      debug("Recommend results", { count: results.length });

      return {
        success: true,
        basedOn: {
          positive: input.positive,
          negative: input.negative || [],
        },
        resultCount: results.length,
        results: results.map((r: { id: string | number; score: number; payload: Record<string, unknown> }, i: number) => ({
          rank: i + 1,
          id: r.id,
          score: Math.round(r.score * 1000) / 1000,
          data: r.payload,
        })),
        message: `Found ${results.length} recommendations based on ${input.positive.length} positive examples`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Recommendation failed",
      };
    }
  },
});

/**
 * Filter and browse records without vector search
 */
export const filterRecords = tool({
  description: `List/browse records by exact filter conditions only.

Use when user wants to list or browse without similarity search.
Good for: "show all in stock", "list electronics", "products under $100"

If user mentions "like X" or "similar to X", use vectorSearch instead (with filters).`,

  inputSchema: z.object({
    collectionName: z.string().describe("Name of the collection"),
    filter: z.record(z.unknown()).describe("Filter conditions: { field: value } or { field: { gte, lte } }"),
    limit: z.number().optional().default(20).describe("Maximum results"),
    offset: z.number().optional().default(0).describe("Offset for pagination"),
  }),

  execute: async (input) => {
    debug("filterRecords called", { collection: input.collectionName, filter: input.filter });
    
    try {
      const mustConditions: unknown[] = [];
      
      for (const [field, value] of Object.entries(input.filter)) {
        if (typeof value === 'object' && value !== null && ('gte' in value || 'lte' in value || 'gt' in value || 'lt' in value)) {
          mustConditions.push({ key: field, range: value });
        } else if (Array.isArray(value)) {
          // Multiple values - any match
          mustConditions.push({ key: field, match: { any: value } });
        } else {
          mustConditions.push({ key: field, match: { value } });
        }
      }

      const response = await fetch(
        `${process.env.VECTX_URL || 'http://localhost:6333'}/collections/${input.collectionName}/points/scroll`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            filter: { must: mustConditions },
            limit: input.limit,
            offset: input.offset,
            with_payload: true,
          }),
        }
      );

      if (!response.ok) {
        throw new Error(`Filter failed: ${await response.text()}`);
      }

      const data = await response.json();
      const points = data.result?.points || [];
      debug("Filter results", { count: points.length });

      return {
        success: true,
        filter: input.filter,
        resultCount: points.length,
        hasMore: points.length === input.limit,
        results: points.map((p: { id: string | number; payload: Record<string, unknown> }, i: number) => ({
          rank: i + 1,
          id: p.id,
          data: p.payload,
        })),
        message: `Found ${points.length} records matching the filter`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Filter failed",
      };
    }
  },
});

/**
 * Count records matching a filter
 */
export const countRecords = tool({
  description: `Count how many records match a filter condition.

Use this when:
- User asks "how many X are there?"
- User wants totals or statistics`,

  inputSchema: z.object({
    collectionName: z.string().describe("Name of the collection"),
    filter: z.record(z.unknown()).optional().describe("Filter conditions (omit to count all)"),
  }),

  execute: async (input) => {
    debug("countRecords called", { collection: input.collectionName, filter: input.filter });
    
    try {
      const body: Record<string, unknown> = { exact: true };
      
      if (input.filter && Object.keys(input.filter).length > 0) {
        const mustConditions: unknown[] = [];
        for (const [field, value] of Object.entries(input.filter)) {
          if (typeof value === 'object' && value !== null && ('gte' in value || 'lte' in value)) {
            mustConditions.push({ key: field, range: value });
          } else {
            mustConditions.push({ key: field, match: { value } });
          }
        }
        body.filter = { must: mustConditions };
      }

      const response = await fetch(
        `${process.env.VECTX_URL || 'http://localhost:6333'}/collections/${input.collectionName}/points/count`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        }
      );

      if (!response.ok) {
        throw new Error(`Count failed: ${await response.text()}`);
      }

      const data = await response.json();
      const count = data.result?.count || 0;
      debug("Count result", { count });

      const filterDesc = input.filter 
        ? Object.entries(input.filter).map(([k, v]) => `${k}=${v}`).join(', ')
        : 'all records';

      return {
        success: true,
        count,
        filter: input.filter || 'none',
        message: `There are ${count} records matching: ${filterDesc}`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Count failed",
      };
    }
  },
});

/**
 * Delete records by filter
 */
export const deleteRecords = tool({
  description: `Delete records matching a filter condition.

⚠️ This permanently deletes data! Use with caution.

Use this when:
- User explicitly asks to delete/remove records
- User wants to clean up data`,

  inputSchema: z.object({
    collectionName: z.string().describe("Name of the collection"),
    filter: z.record(z.unknown()).describe("Filter for records to delete"),
    confirm: z.boolean().describe("Must be true to confirm deletion"),
  }),

  execute: async (input) => {
    if (!input.confirm) {
      return {
        success: false,
        error: "Deletion not confirmed. Set confirm: true to proceed.",
      };
    }

    debug("deleteRecords called", { collection: input.collectionName, filter: input.filter });
    
    try {
      const mustConditions: unknown[] = [];
      for (const [field, value] of Object.entries(input.filter)) {
        if (typeof value === 'object' && value !== null && ('gte' in value || 'lte' in value)) {
          mustConditions.push({ key: field, range: value });
        } else {
          mustConditions.push({ key: field, match: { value } });
        }
      }

      const response = await fetch(
        `${process.env.VECTX_URL || 'http://localhost:6333'}/collections/${input.collectionName}/points/delete`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            filter: { must: mustConditions },
          }),
        }
      );

      if (!response.ok) {
        throw new Error(`Delete failed: ${await response.text()}`);
      }

      debug("Delete completed");

      return {
        success: true,
        filter: input.filter,
        message: `Successfully deleted records matching the filter`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Delete failed",
      };
    }
  },
});

/**
 * Get a specific record by ID
 */
export const getRecord = tool({
  description: `Get a specific record by its ID.

Use this when:
- User asks about a specific item by ID
- You need to look up a particular record`,

  inputSchema: z.object({
    collectionName: z.string().describe("Name of the collection"),
    id: z.union([z.string(), z.number()]).describe("Record ID to retrieve"),
  }),

  execute: async (input) => {
    debug("getRecord called", { collection: input.collectionName, id: input.id });
    
    try {
      const response = await fetch(
        `${process.env.VECTX_URL || 'http://localhost:6333'}/collections/${input.collectionName}/points/${input.id}`,
        { method: 'GET' }
      );

      if (!response.ok) {
        if (response.status === 404) {
          return {
            success: false,
            error: `Record with ID ${input.id} not found`,
          };
        }
        throw new Error(`Get record failed: ${await response.text()}`);
      }

      const data = await response.json();
      const point = data.result;
      debug("Got record", { id: point?.id });

      return {
        success: true,
        record: {
          id: point.id,
          data: point.payload,
        },
        message: `Retrieved record ${input.id}`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Get record failed",
      };
    }
  },
});
