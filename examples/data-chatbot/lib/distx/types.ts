/**
 * DistX TypeScript Types
 * 
 * Types for the Similarity Contract engine.
 * DistX stores objects and derives vectors from their structure.
 */

export type FieldType = 'text' | 'number' | 'categorical' | 'boolean';

// DistX distance types for the Similarity Contract
// - semantic: For text fields (trigram-based similarity)
// - relative: For numbers (percentage difference)
// - absolute: For numbers (absolute difference)
// - exact: For categorical/boolean (exact match)
// - overlap: For sets/arrays
export type DistanceType = 'semantic' | 'relative' | 'absolute' | 'exact' | 'overlap';

export interface FieldConfig {
  type: FieldType;
  weight: number;
  distance?: DistanceType;
}

export interface SimilaritySchema {
  fields: Record<string, FieldConfig>;
}

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

export interface FieldContribution {
  field: string;
  weight: number;
  similarity: number;
  contribution: number;
}

export interface SimilarResult {
  id: number | string;
  score: number;
  payload: PointPayload;
  contributions?: FieldContribution[];
}

export interface SimilarResponse {
  results: SimilarResult[];
  query_example: PointPayload;
  collection: string;
}

export interface DataSourceConfig {
  type: 'csv' | 'excel' | 'postgres' | 'mysql';
  name: string;
  // CSV/Excel
  content?: string;
  // Database
  connectionString?: string;
  table?: string;
}

export interface InferredSchema {
  fields: Record<string, FieldConfig>;
  sampleData: PointPayload[];
  totalRows: number;
  inferenceDetails: {
    field: string;
    inferredType: FieldType;
    uniqueValues: number;
    nullCount: number;
    reason: string;
  }[];
}

export interface DataCollection {
  name: string;
  schema: SimilaritySchema;
  rowCount: number;
  columns: string[];
  createdAt: Date;
}
