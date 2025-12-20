import { tool } from "ai";
import { z } from "zod";
import { getDistXClient, schemaToSummary } from "@/lib/distx";
import type { PointPayload } from "@/lib/distx";

// Debug logging helper - using console.warn for visibility in Next.js
const debug = (action: string, data?: any) => {
  const msg = data 
    ? `[DistX] 🔍 ${action}: ${JSON.stringify(data)}`
    : `[DistX] 🔍 ${action}`;
  console.warn(msg);
};

/**
 * Tool to find similar records using DistX's Similarity Contract.
 * 
 * This is the core query capability:
 * - Query by example (provide a partial or full example)
 * - Get results with explainable per-field scores
 * - Override weights for different search intents
 */
export const findSimilar = tool({
  description: `Find records similar to a given example using the Similarity Contract.

This tool enables "query by example" - provide some field values and find matching records.

Key features:
- Partial examples work: just provide the fields you care about
- Results include per-field contribution breakdown (explainability)
- Override weights to change what "similar" means at query time

Examples:
- "Find products similar to iPhone" → example: { name: "iPhone" }
- "Find cheaper alternatives to product X" → example + weights: { price: 0.8 }
- "Find customers in the same industry" → example: { industry: "tech" }`,

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
    weights: z
      .record(z.number())
      .optional()
      .describe("Override field weights for this query. Higher weight = more importance. Example: { 'price': 0.8, 'name': 0.2 }"),
    explain: z
      .boolean()
      .optional()
      .default(true)
      .describe("Include per-field contribution breakdown"),
  }),

  execute: async (input) => {
    debug("findSimilar called", {
      collection: input.collectionName,
      example: input.example,
      limit: input.limit,
      weights: input.weights,
    });
    
    const client = getDistXClient();

    try {
      // Verify collection exists and has a schema
      const schema = await client.getSimilaritySchema(input.collectionName);
      debug("Schema check", { collection: input.collectionName, hasSchema: !!schema });
      
      if (!schema) {
        return {
          success: false,
          error: `Collection "${input.collectionName}" has no Similarity Schema. Import data with automatic schema inference first.`,
        };
      }

      // Execute similarity query
      debug("Executing DistX similarity query", {
        url: `POST /collections/${input.collectionName}/similar`,
        body: { example: input.example, limit: input.limit, weights: input.weights }
      });
      
      const response = await client.findSimilar(
        input.collectionName,
        input.example as PointPayload,
        {
          limit: input.limit,
          weights: input.weights,
          explain: input.explain,
        }
      );
      
      debug("DistX response", { resultCount: response.results.length });

      // Format results for display
      const formattedResults = response.results.map((result, index) => {
        const base: Record<string, unknown> = {
          rank: index + 1,
          score: Math.round(result.score * 1000) / 1000,
          data: result.payload,
        };

        if (result.contributions && result.contributions.length > 0) {
          base.contributions = result.contributions
            .sort((a, b) => b.contribution - a.contribution)
            .slice(0, 5) // Top 5 contributors
            .map((c) => ({
              field: c.field,
              contribution: Math.round(c.contribution * 1000) / 1000,
            }));
        }

        return base;
      });

      // Build response message
      let message = `Found ${response.results.length} similar records`;
      if (input.weights) {
        const weightDesc = Object.entries(input.weights)
          .map(([k, v]) => `${k}: ${v}`)
          .join(", ");
        message += ` with custom weights (${weightDesc})`;
      }
      message += ".";

      return {
        success: true,
        query: {
          collection: input.collectionName,
          example: input.example,
          weights: input.weights || "default (from schema)",
        },
        resultCount: response.results.length,
        results: formattedResults,
        message,
        hint:
          response.results.length === 0
            ? "Try a different example or check if the collection has data."
            : "Use 'weights' parameter to adjust what fields matter most for your search.",
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Similarity search failed",
      };
    }
  },
});

/**
 * Tool to find records similar to an existing record by ID
 */
export const findSimilarById = tool({
  description: `Find records similar to an existing record in the collection.
Provide a record ID and get back similar records with explainability.`,

  inputSchema: z.object({
    collectionName: z.string().describe("Name of the collection"),
    recordId: z.union([z.string(), z.number()]).describe("ID of the record to find similar matches for"),
    limit: z.number().optional().default(10).describe("Maximum results"),
    weights: z.record(z.number()).optional().describe("Override field weights"),
  }),

  execute: async (input) => {
    const client = getDistXClient();

    try {
      // Get the reference record
      const point = await client.getPoint(input.collectionName, input.recordId);
      if (!point) {
        return {
          success: false,
          error: `Record with ID ${input.recordId} not found in collection "${input.collectionName}"`,
        };
      }

      // Use the payload as the example
      const response = await client.findSimilar(
        input.collectionName,
        point.payload,
        {
          limit: input.limit + 1, // +1 to exclude self
          weights: input.weights,
          explain: true,
        }
      );

      // Filter out the query record itself
      const filteredResults = response.results
        .filter((r) => r.id !== input.recordId)
        .slice(0, input.limit);

      const formattedResults = filteredResults.map((result, index) => ({
        rank: index + 1,
        id: result.id,
        score: Math.round(result.score * 1000) / 1000,
        data: result.payload,
        topContributors: result.contributions
          ?.sort((a, b) => b.contribution - a.contribution)
          .slice(0, 3)
          .map((c) => `${c.field}: ${Math.round(c.contribution * 100)}%`),
      }));

      return {
        success: true,
        referenceRecord: {
          id: input.recordId,
          data: point.payload,
        },
        resultCount: filteredResults.length,
        results: formattedResults,
        message: `Found ${filteredResults.length} records similar to record #${input.recordId}`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Search failed",
      };
    }
  },
});

/**
 * Tool to analyze what makes records similar or different
 */
export const compareSimilarity = tool({
  description: `Compare two records and explain their similarity.
Shows which fields contribute most to their similarity/difference.`,

  inputSchema: z.object({
    collectionName: z.string().describe("Name of the collection"),
    recordId1: z.union([z.string(), z.number()]).describe("First record ID"),
    recordId2: z.union([z.string(), z.number()]).describe("Second record ID"),
  }),

  execute: async (input) => {
    const client = getDistXClient();

    try {
      // Get both records
      const [point1, point2] = await Promise.all([
        client.getPoint(input.collectionName, input.recordId1),
        client.getPoint(input.collectionName, input.recordId2),
      ]);

      if (!point1 || !point2) {
        return {
          success: false,
          error: "One or both records not found",
        };
      }

      // Get schema for field types
      const schema = await client.getSimilaritySchema(input.collectionName);
      if (!schema) {
        return {
          success: false,
          error: "Collection has no Similarity Schema",
        };
      }

      // Find similarity of record2 using record1 as example
      const response = await client.findSimilar(
        input.collectionName,
        point1.payload,
        {
          limit: 100, // Get enough to find record2
          explain: true,
        }
      );

      // Find record2 in results
      const match = response.results.find(
        (r) => String(r.id) === String(input.recordId2)
      );

      if (!match) {
        return {
          success: false,
          error: "Could not compute similarity between records",
        };
      }

      // Build comparison
      const comparison = Object.keys(schema.fields).map((field) => {
        const val1 = point1.payload[field];
        const val2 = point2.payload[field];
        const contribution = match.contributions?.find(
          (c) => c.field === field
        );

        return {
          field,
          type: schema.fields[field].type,
          record1Value: val1,
          record2Value: val2,
          same: val1 === val2,
          contribution: contribution
            ? Math.round(contribution.contribution * 1000) / 1000
            : 0,
        };
      });

      // Sort by contribution
      comparison.sort((a, b) => b.contribution - a.contribution);

      return {
        success: true,
        overallSimilarity: Math.round(match.score * 1000) / 1000,
        record1: { id: input.recordId1, data: point1.payload },
        record2: { id: input.recordId2, data: point2.payload },
        fieldComparison: comparison,
        summary: {
          sameFields: comparison.filter((c) => c.same).map((c) => c.field),
          differentFields: comparison.filter((c) => !c.same).map((c) => c.field),
          topContributors: comparison.slice(0, 3).map((c) => c.field),
        },
        message: `Records have ${Math.round(match.score * 100)}% similarity. Top contributing fields: ${comparison
          .slice(0, 3)
          .map((c) => c.field)
          .join(", ")}.`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Comparison failed",
      };
    }
  },
});
