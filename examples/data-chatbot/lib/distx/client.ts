/**
 * DistX Client for TypeScript
 * 
 * Uses the official Qdrant JS client for standard vector operations,
 * with extensions for DistX-specific Similarity Contract features.
 * 
 * Key concept: DistX does not store vectors that represent objects.
 * It stores objects, and derives vectors from their structure.
 * 
 * @see https://github.com/qdrant/qdrant-js
 */

import { QdrantClient } from '@qdrant/js-client-rest';
import type {
  SimilaritySchema,
  PointPayload,
  SimilarResult,
  SimilarResponse,
} from './types';

/**
 * Extended DistX client that wraps QdrantClient and adds
 * Similarity Contract features.
 */
export class DistXClient {
  private baseUrl: string;
  public qdrant: QdrantClient;

  constructor(baseUrl: string = 'http://localhost:6333') {
    this.baseUrl = baseUrl;
    
    // Use the official Qdrant JS client for standard operations
    // DistX is 100% Qdrant API compatible
    this.qdrant = new QdrantClient({ 
      url: baseUrl,
      // Skip version check since DistX doesn't report Qdrant version
      checkCompatibility: false,
    });
  }

  // ============================================================
  // Standard Qdrant Operations (delegated to official client)
  // ============================================================

  /**
   * Check if DistX is connected and healthy
   */
  async healthCheck(): Promise<boolean> {
    try {
      // Use a simple fetch since QdrantClient doesn't expose health check
      const response = await fetch(`${this.baseUrl}/`);
      return response.ok;
    } catch {
      return false;
    }
  }

  /**
   * List all collections
   */
  async listCollections(): Promise<string[]> {
    const result = await this.qdrant.getCollections();
    return result.collections.map((c) => c.name);
  }

  /**
   * Get collection info
   */
  async getCollection(name: string) {
    try {
      return await this.qdrant.getCollection(name);
    } catch {
      return null;
    }
  }

  /**
   * Delete a collection
   */
  async deleteCollection(name: string): Promise<boolean> {
    try {
      await this.qdrant.deleteCollection(name);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Get a specific point by ID
   */
  async getPoint(collectionName: string, pointId: number | string) {
    try {
      const results = await this.qdrant.retrieve(collectionName, {
        ids: [pointId],
        with_payload: true,
      });
      return results[0] || null;
    } catch {
      return null;
    }
  }

  /**
   * Count points in a collection
   */
  async countPoints(collectionName: string): Promise<number> {
    try {
      const result = await this.qdrant.count(collectionName);
      return result.count;
    } catch {
      return 0;
    }
  }

  /**
   * Scroll through points in a collection
   */
  async scrollPoints(
    collectionName: string,
    options: {
      limit?: number;
      offset?: number | string;
      withPayload?: boolean;
    } = {}
  ) {
    const { limit = 100, offset, withPayload = true } = options;

    const result = await this.qdrant.scroll(collectionName, {
      limit,
      offset: offset as number | undefined,
      with_payload: withPayload,
    });

    return {
      points: result.points.map((p) => ({
        id: p.id,
        payload: p.payload as PointPayload,
      })),
      nextOffset: result.next_page_offset,
    };
  }

  // ============================================================
  // DistX-Specific: Similarity Contract Features
  // These endpoints are DistX extensions to the Qdrant API
  // ============================================================

  /**
   * Create a collection with vector configuration and optional Similarity Schema
   * 
   * @param name - Collection name
   * @param options - Vector size, distance metric, and optional similarity schema
   */
  async createCollection(
    name: string,
    options: {
      vectorSize: number;
      distance?: 'Cosine' | 'Euclidean' | 'Dot';
      schema?: SimilaritySchema;
    }
  ): Promise<boolean> {
    const { vectorSize, distance = 'Cosine', schema } = options;
    
    // Use DistX's collection creation API
    const body: Record<string, unknown> = {
      vectors: {
        size: vectorSize,
        distance: distance,
      },
    };
    
    if (schema) {
      body.similarity_schema = schema;
    }
    
    const response = await fetch(`${this.baseUrl}/collections/${name}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to create collection: ${error}`);
    }

    return true;
  }

  /**
   * Get the Similarity Schema for a collection
   * (DistX-specific endpoint)
   */
  async getSimilaritySchema(name: string): Promise<SimilaritySchema | null> {
    try {
      const response = await fetch(
        `${this.baseUrl}/collections/${name}/similarity-schema`
      );
      if (!response.ok) {
        return null;
      }
      const data = await response.json();
      return data.result;
    } catch {
      return null;
    }
  }

  /**
   * Set/Update the Similarity Schema for a collection
   * (DistX-specific endpoint)
   */
  async setSimilaritySchema(
    name: string,
    schema: SimilaritySchema
  ): Promise<boolean> {
    const response = await fetch(
      `${this.baseUrl}/collections/${name}/similarity-schema`,
      {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(schema),
      }
    );
    return response.ok;
  }

  /**
   * Delete the Similarity Schema for a collection
   * (DistX-specific endpoint)
   */
  async deleteSimilaritySchema(name: string): Promise<boolean> {
    const response = await fetch(
      `${this.baseUrl}/collections/${name}/similarity-schema`,
      { method: 'DELETE' }
    );
    return response.ok;
  }

  /**
   * Insert points into a collection
   * 
   * With a Similarity Schema, vectors are auto-generated from the payload.
   * No need to provide embeddings - DistX derives them from structure.
   */
  async upsertPoints(
    collectionName: string,
    points: { id: number | string; payload: PointPayload; vector?: number[] }[]
  ): Promise<boolean> {
    // Use Qdrant client's upsert - DistX will auto-embed if schema exists
    await this.qdrant.upsert(collectionName, {
      points: points.map((p) => ({
        id: p.id,
        payload: p.payload,
        // If no vector provided, DistX auto-generates from payload
        vector: p.vector || [],
      })),
    });
    return true;
  }

  /**
   * Query by example - find similar records
   * (DistX-specific endpoint)
   * 
   * This is the core of the Similarity Contract:
   * - Provide an example object
   * - Get back similar objects with explainable scores
   * - Optionally override field weights for different intents
   */
  async findSimilar(
    collectionName: string,
    example: PointPayload,
    options: {
      limit?: number;
      weights?: Record<string, number>;
      explain?: boolean;
    } = {}
  ): Promise<SimilarResponse> {
    const { limit = 10, weights, explain = true } = options;

    const body: Record<string, unknown> = {
      example,
      limit,
      explain,
    };

    if (weights) {
      body.weights = weights;
    }

    const response = await fetch(
      `${this.baseUrl}/collections/${collectionName}/similar`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      }
    );

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Similarity query failed: ${error}`);
    }

    const data = await response.json();
    // DistX returns result.result (not result.results)
    const results = data.result?.result || data.result?.results || [];
    return {
      results: results.map((r: any) => ({
        id: r.id,
        score: r.score,
        payload: r.payload,
        contributions: r.explain ? Object.entries(r.explain).map(([field, contribution]) => ({
          field,
          contribution: contribution as number,
        })) : undefined,
      })),
      query_example: example,
      collection: collectionName,
    };
  }
}

// Singleton instance
let distxClient: DistXClient | null = null;

export function getDistXClient(): DistXClient {
  if (!distxClient) {
    const url = process.env.DISTX_URL || 'http://localhost:6333';
    distxClient = new DistXClient(url);
  }
  return distxClient;
}

// Re-export QdrantClient for direct use when needed
export { QdrantClient } from '@qdrant/js-client-rest';
