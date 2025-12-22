/**
 * vectX TypeScript Types
 * 
 * Types for Qdrant-compatible vector database operations.
 */

export interface CollectionInfo {
  name: string;
  points_count: number;
  vectors_count: number;
  status: string;
}

export interface PointPayload {
  [key: string]: string | number | boolean | null;
}

export interface Point {
  id: number | string;
  vector?: number[];
  payload: PointPayload;
}
