import { tool } from "ai";
import { z } from "zod";
import { getVectXClient, generateEmbedding, createTextForEmbedding } from "@/lib/vectx";
import type { PointPayload } from "@/lib/vectx";

// Debug logging helper
const debug = (action: string, data?: any) => {
  const msg = data 
    ? `[vectX] 🔍 ${action}: ${JSON.stringify(data)}`
    : `[vectX] 🔍 ${action}`;
  console.warn(msg);
};

/**
 * Tool to find similar records using vector search.
 * 
 * This uses standard Qdrant vector search with embeddings.
 */
export const findSimilar = tool({
  description: `Find records similar to a given example using vector search.

Use this when user wants to find items SIMILAR TO something (mentions a product name, description, or reference item).

The tool will:
1. Extract text from the example to create an embedding
2. Search for similar vectors
3. Optionally apply filters for exact matches (category, status, etc.)`,

  inputSchema: z.object({
    collectionName: z
      .string()
      .describe("Name of the collection to search"),
    example: z
      .record(z.union([z.string(), z.number(), z.boolean(), z.null()]))
      .describe("Example record to find similar matches for. Can be partial."),
    limit: z
      .number()
      .optional()
      .default(10)
      .describe("Maximum number of results to return"),
    filter: z
      .record(z.any())
      .optional()
      .describe("Optional Qdrant filter for exact matches"),
  }),

  execute: async (input) => {
    debug("findSimilar called", {
      collection: input.collectionName,
      example: input.example,
      limit: input.limit,
    });
    
    const client = getVectXClient();

    try {
      // Verify collection exists
      const collection = await client.getCollection(input.collectionName);
      if (!collection) {
        return {
          success: false,
          error: `Collection "${input.collectionName}" not found.`,
        };
      }

      // Create text from example for embedding
      const queryText = createTextForEmbedding(input.example, Object.keys(input.example));
      
      if (!queryText.trim()) {
        return {
          success: false,
          error: "Example must contain at least one text field for semantic search.",
        };
      }

      // Generate embedding
      const queryVector = await generateEmbedding(queryText);
      debug("Generated embedding", { queryText: queryText.substring(0, 100), vectorDim: queryVector.length });

      // Perform vector search
      const results = await client.search(
        input.collectionName,
        queryVector,
        {
          limit: input.limit || 10,
          filter: input.filter,
        }
      );

      debug("Search results", { resultCount: results.length });

      return {
        success: true,
        results: results.map((r: any) => ({
          id: r.id,
          score: r.score,
          payload: r.payload,
        })),
        query_example: input.example,
        collection: input.collectionName,
      };
    } catch (error: any) {
      debug("Error in findSimilar", { error: error.message });
      return {
        success: false,
        error: error.message || "Failed to find similar records",
      };
    }
  },
});
