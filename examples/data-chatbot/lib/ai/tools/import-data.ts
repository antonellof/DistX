import { tool } from "ai";
import { z } from "zod";
import { 
  getVectXClient, 
  parseCSV, 
  generatePayloadEmbeddings,
  getEmbeddingDimension,
  createTextForEmbedding,
} from "@/lib/vectx";
import type { PointPayload } from "@/lib/vectx";

// Debug logging helper
const debug = (action: string, data?: any) => {
  const msg = data 
    ? `[vectX] 🔧 ${action}: ${JSON.stringify(data)}`
    : `[vectX] 🔧 ${action}`;
  console.warn(msg);
};

/**
 * Tool to import CSV data into vectX with vector embeddings.
 * 
 * This creates embeddings from the data and stores them for vector search.
 */
export const importData = tool({
  description: `Import tabular data (CSV format) into vectX for vector search. 
The system will:
1. Parse the CSV data
2. Generate embeddings for each row
3. Store vectors and payloads in vectX
4. Enable semantic search over the data

Use this when the user wants to:
- Upload a CSV file
- Import data for analysis
- Set up a dataset for similarity queries`,

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
    const client = getVectXClient();

    // Check vectX connection
    const isConnected = await client.healthCheck();
    if (!isConnected) {
      return {
        success: false,
        error: "vectX is not running. Please start vectX on port 6333.",
        hint: "Run: docker run -p 6333:6333 antonellofratepietro/vectx",
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

      debug("Parsed CSV", { rowCount: data.length, columns: Object.keys(data[0] || {}) });

      // Get embedding dimension
      const embeddingDim = getEmbeddingDimension();

      // Check if collection exists, create if not
      const collectionName = input.collectionName.toLowerCase().replace(/[^a-z0-9_]/g, '_');
      let collection = await client.getCollection(collectionName);
      
      if (!collection) {
        await client.createCollection(collectionName, {
          vectorSize: embeddingDim,
          distance: 'Cosine',
        });
        debug("Created collection", { name: collectionName, vectorSize: embeddingDim });
      }

      // Generate embeddings and prepare points
      debug("Generating embeddings", { count: data.length });
      // Get all field names from the first row as text fields
      const textFields = data.length > 0 ? Object.keys(data[0]) : [];
      const embeddings = await generatePayloadEmbeddings(data, textFields);

      // Prepare points with embeddings
      const points = data.map((row, index) => ({
        id: index + 1,
        payload: row as PointPayload,
        vector: embeddings[index],
      }));

      // Insert points in batches
      const batchSize = 100;
      let inserted = 0;
      for (let i = 0; i < points.length; i += batchSize) {
        const batch = points.slice(i, i + batchSize);
        await client.upsertPoints(collectionName, batch);
        inserted += batch.length;
        debug("Inserted batch", { inserted, total: points.length });
      }

      debug("Import complete", { collection: collectionName, points: inserted });

      return {
        success: true,
        collection: collectionName,
        rowCount: inserted,
        columns: Object.keys(data[0] || {}),
        message: `Successfully imported ${inserted} rows into collection "${collectionName}"`,
      };
    } catch (error: any) {
      debug("Import error", { error: error.message });
      return {
        success: false,
        error: error.message || "Failed to import data",
      };
    }
  },
});

/**
 * Tool to list available data collections
 */
export const listDataCollections = tool({
  description: `List all data collections available for vector search.
Shows collection names and row counts.`,

  inputSchema: z.object({}),

  execute: async () => {
    debug("listDataCollections called");
    const client = getVectXClient();

    const isConnected = await client.healthCheck();
    debug("vectX health check", { connected: isConnected });
    
    if (!isConnected) {
      return {
        success: false,
        error: "vectX is not running",
        collections: [],
      };
    }

    try {
      const collectionNames = await client.listCollections();
      debug("Found collections", { collections: collectionNames });
      const collections = [];

      for (const name of collectionNames) {
        const info = await client.getCollection(name);
        if (info) {
          collections.push({
            name,
            rowCount: info.points_count || 0,
          });
        }
      }

      debug("Returning collections", { count: collections.length, collections });
      
      let message = "";
      if (collections.length > 0) {
        message = `You have ${collections.length} collection(s):\n\n`;
        for (const col of collections) {
          message += `**${col.name}** (${col.rowCount} rows)\n`;
        }
      } else {
        message = "No collections found. Upload a CSV file to get started!";
      }

      return {
        success: true,
        collections,
        message,
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
 * Tool to explore a data collection's structure and sample data
 */
export const exploreData = tool({
  description: `Explore a data collection to understand its structure.
Shows field names and sample rows.
Use this before running vector search to understand what fields are available.`,

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
    const client = getVectXClient();

    try {
      const info = await client.getCollection(input.collectionName);
      debug("Collection info", { collection: input.collectionName, found: !!info, points_count: info?.points_count });
      
      if (!info) {
        return {
          success: false,
          error: `Collection "${input.collectionName}" not found`,
        };
      }

      const { points } = await client.scrollPoints(input.collectionName, {
        limit: input.sampleCount,
      });
      
      debug("Sample data", { sampleCount: points.length });

      return {
        success: true,
        collection: input.collectionName,
        rowCount: info.points_count || 0,
        columns: points.length > 0 ? Object.keys(points[0].payload || {}) : [],
        sampleData: points.map((p) => p.payload),
        message: `Collection has ${info.points_count || 0} rows.`,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Failed to explore collection",
      };
    }
  },
});
