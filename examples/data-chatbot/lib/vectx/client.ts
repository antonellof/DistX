/**
 * vectX Client for TypeScript
 * 
 * Uses the official Qdrant JS client for standard vector operations.
 * vectX is 100% Qdrant API compatible.
 * 
 * @see https://github.com/qdrant/qdrant-js
 */

import { QdrantClient } from '@qdrant/js-client-rest';
import type {
  PointPayload,
} from './types';

/**
 * vectX client that wraps QdrantClient for Qdrant-compatible operations.
 */
export class vectXClient {
  private baseUrl: string;
  public qdrant: QdrantClient;

  constructor(baseUrl: string = 'http://localhost:6333') {
    this.baseUrl = baseUrl;
    
    // Use the official Qdrant JS client for standard operations
    // vectX is 100% Qdrant API compatible
    this.qdrant = new QdrantClient({ 
      url: baseUrl,
      // Skip version check since vectX doesn't report Qdrant version
      checkCompatibility: false,
    });
  }

  // ============================================================
  // Standard Qdrant Operations (delegated to official client)
  // ============================================================

  /**
   * Check if vectX is connected and healthy
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
  // vectX-Specific: Similarity Contract Features
  // These endpoints are vectX extensions to the Qdrant API
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
    const { vectorSize, distance = 'Cosine' } = options;
    
    // Use Qdrant-compatible collection creation API
    const body: Record<string, unknown> = {
      vectors: {
        size: vectorSize,
        distance: distance,
      },
    };
    
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
   * Insert points into a collection
   * Points must include vectors (embeddings)
   */
  async upsertPoints(
    collectionName: string,
    points: { id: number | string; payload: PointPayload; vector: number[] }[]
  ): Promise<boolean> {
    await this.qdrant.upsert(collectionName, {
      points: points.map((p) => ({
        id: p.id,
        payload: p.payload,
        vector: p.vector,
      })),
    });
    return true;
  }

  /**
   * Search for similar vectors
   * Standard Qdrant vector search
   */
  async search(
    collectionName: string,
    vector: number[],
    options: {
      limit?: number;
      filter?: any;
    } = {}
  ) {
    const { limit = 10, filter } = options;
    return await this.qdrant.search(collectionName, {
      vector,
      limit,
      filter,
    });
  }
}

// Singleton instance
let vectxClient: vectXClient | null = null;

export function getVectXClient(): vectXClient {
  if (!vectxClient) {
    const url = process.env.VECTX_URL || 'http://localhost:6333';
    vectxClient = new vectXClient(url);
  }
  return vectxClient;
}

// Re-export QdrantClient for direct use when needed
export { QdrantClient } from '@qdrant/js-client-rest';
