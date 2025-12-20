import { tool } from "ai";
import { z } from "zod";
import { 
  getDistXClient, 
  parseCSV, 
  inferSchema, 
  schemaToSummary,
  generatePayloadEmbeddings,
  getEmbeddingDimension,
} from "@/lib/distx";
import type { PointPayload, SimilaritySchema } from "@/lib/distx";

// Debug logging helper - using console.warn for visibility in Next.js
const debug = (action: string, data?: any) => {
  const msg = data 
    ? `[DistX] 🔧 ${action}: ${JSON.stringify(data)}`
    : `[DistX] 🔧 ${action}`;
  console.warn(msg);
};

// In-memory store for active data collections (in production, use Redis or DB)
const activeCollections = new Map<string, {
  name: string;
  schema: SimilaritySchema;
  rowCount: number;
  columns: string[];
}>();

export function getActiveCollections() {
  return Array.from(activeCollections.values());
}

export function getActiveCollection(name: string) {
  return activeCollections.get(name);
}

/**
 * Tool to import CSV data into DistX with automatic schema inference.
 * 
 * This creates a Similarity Contract that governs how the data
 * will be queried for similarity.
 */
export const importData = tool({
  description: `Import tabular data (CSV format) into DistX for similarity queries. 
The system will automatically:
1. Analyze the data structure
2. Infer field types (text, number, categorical, boolean)
3. Create a Similarity Contract with appropriate weights
4. Index all rows for similarity search

Use this when the user wants to:
- Upload a CSV file
- Import data for analysis
- Set up a dataset for similarity queries
- Find similar records in their data`,

  inputSchema: z.object({
    csvContent: z
      .string()
      .describe("The CSV content to import (including header row)"),
    collectionName: z
      .string()
      .describe("A name for this data collection (e.g., 'products', 'customers', 'transactions')"),
    description: z
      .string()
      .optional()
      .describe("Optional description of what this data represents"),
  }),

  execute: async (input) => {
    const client = getDistXClient();

    // Check DistX connection
    const isConnected = await client.healthCheck();
    if (!isConnected) {
      return {
        success: false,
        error: "DistX is not running. Please start DistX on port 6333.",
        hint: "Run: docker run -p 6333:6333 distx/distx",
      };
    }

    try {
      // Parse CSV
      const data = parseCSV(input.csvContent);
      if (data.length === 0) {
        return {
          success: false,
          error: "No data rows found in CSV",
        };
      }

      // Infer schema
      const inferred = inferSchema(data, {
        excludeFields: [], // Could add 'id' exclusion logic
      });

      // Create collection with similarity schema
      const collectionName = input.collectionName
        .toLowerCase()
        .replace(/[^a-z0-9_-]/g, "_");

      // Delete existing collection if it exists
      await client.deleteCollection(collectionName);

      // Identify text fields for embedding generation
      const textFields = Object.entries(inferred.fields)
        .filter(([_, config]) => config.type === 'text')
        .map(([name, _]) => name);

      debug("Generating embeddings for text fields", { textFields, rowCount: data.length });

      // Generate embeddings for all payloads
      let embeddings: number[][] = [];
      try {
        embeddings = await generatePayloadEmbeddings(data, textFields);
        debug("Embeddings generated", { count: embeddings.length, dimension: embeddings[0]?.length });
      } catch (error) {
        debug("Embedding generation failed, using empty vectors", { error: String(error) });
        // Fall back to empty vectors if OpenAI fails
        const dim = getEmbeddingDimension();
        embeddings = data.map(() => new Array(dim).fill(0));
      }

      // Create new collection with vector size and schema
      await client.createCollection(collectionName, {
        vectorSize: getEmbeddingDimension(),
        distance: 'Cosine',
        schema: { fields: inferred.fields },
      });

      // Prepare points with embeddings
      const points = data.map((row, index) => ({
        id: index + 1,
        payload: row,
        vector: embeddings[index],
      }));

      // Insert data in batches
      const batchSize = 100;
      for (let i = 0; i < points.length; i += batchSize) {
        const batch = points.slice(i, i + batchSize);
        await client.upsertPoints(collectionName, batch);
        debug("Inserted batch", { batch: i / batchSize + 1, total: Math.ceil(points.length / batchSize) });
      }

      // Store in active collections
      activeCollections.set(collectionName, {
        name: collectionName,
        schema: { fields: inferred.fields },
        rowCount: data.length,
        columns: Object.keys(inferred.fields),
      });

      return {
        success: true,
        collection: collectionName,
        rowCount: data.length,
        columns: Object.keys(inferred.fields),
        schemaInference: inferred.inferenceDetails.map((d) => ({
          field: d.field,
          type: d.inferredType,
          weight: inferred.fields[d.field].weight,
          reason: d.reason,
        })),
        schemaSummary: schemaToSummary({ fields: inferred.fields }),
        sampleRows: inferred.sampleData.slice(0, 3),
        message: `Successfully imported ${data.length} rows into "${collectionName}". You can now query for similar records.`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Import failed",
      };
    }
  },
});

/**
 * Tool to list available data collections
 */
export const listDataCollections = tool({
  description: `List all data collections available for similarity queries.
Shows collection names, row counts, and schema information.`,

  inputSchema: z.object({}),

  execute: async () => {
    debug("listDataCollections called");
    const client = getDistXClient();

    const isConnected = await client.healthCheck();
    debug("DistX health check", { connected: isConnected });
    
    if (!isConnected) {
      return {
        success: false,
        error: "DistX is not running",
        collections: [],
      };
    }

    try {
      const collectionNames = await client.listCollections();
      debug("Found collections", { collections: collectionNames });
      const collections = [];

      for (const name of collectionNames) {
        const info = await client.getCollection(name);
        const schema = await client.getSimilaritySchema(name);

        if (info) {
          collections.push({
            name,
            rowCount: info.points_count,
            hasSchema: !!schema,
            fields: schema ? Object.keys(schema.fields) : [],
          });
        }
      }

      debug("Returning collections", { count: collections.length, collections });
      return {
        success: true,
        collections,
        message:
          collections.length > 0
            ? `Found ${collections.length} collection(s) ready for similarity queries.`
            : "No collections found. Import some data first!",
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Failed to list collections",
        collections: [],
      };
    }
  },
});

/**
 * Tool to explore a data collection's schema and sample data
 */
export const exploreData = tool({
  description: `Explore a data collection to understand its structure.
Shows the Similarity Contract (field types, weights, distances) and sample rows.
Use this before running similarity queries to understand what fields are available.`,

  inputSchema: z.object({
    collectionName: z.string().describe("Name of the collection to explore"),
    sampleCount: z
      .number()
      .optional()
      .default(5)
      .describe("Number of sample rows to return"),
  }),

  execute: async (input) => {
    debug("exploreData called", { collection: input.collectionName, sampleCount: input.sampleCount });
    const client = getDistXClient();

    try {
      const info = await client.getCollection(input.collectionName);
      debug("Collection info", { collection: input.collectionName, found: !!info, points_count: info?.points_count });
      
      if (!info) {
        return {
          success: false,
          error: `Collection "${input.collectionName}" not found`,
        };
      }

      const schema = await client.getSimilaritySchema(input.collectionName);
      const { points } = await client.scrollPoints(input.collectionName, {
        limit: input.sampleCount,
      });
      
      debug("Schema and sample data", { 
        hasSchema: !!schema, 
        fields: schema ? Object.keys(schema.fields) : [],
        sampleCount: points.length 
      });

      const schemaDetails = schema
        ? Object.entries(schema.fields)
            .sort((a, b) => b[1].weight - a[1].weight)
            .map(([field, config]) => ({
              field,
              type: config.type,
              weight: config.weight,
              distance: config.distance || "default",
            }))
        : null;

      return {
        success: true,
        collection: input.collectionName,
        rowCount: info.points_count,
        schema: schemaDetails,
        schemaSummary: schema ? schemaToSummary(schema) : "No schema defined",
        sampleData: points.map((p) => p.payload),
        message: schema
          ? `Collection has ${info.points_count} rows with ${Object.keys(schema.fields).length} fields defined in the Similarity Contract.`
          : `Collection has ${info.points_count} rows but no Similarity Schema (standard vector search only).`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Failed to explore collection",
      };
    }
  },
});
