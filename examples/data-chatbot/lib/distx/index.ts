/**
 * DistX - Vector Database for Structured Similarity
 * 
 * DistX is a vector database. The client generates embeddings
 * using OpenAI and stores them in DistX for semantic search.
 * 
 * This client uses the official Qdrant JS SDK for standard vector operations
 * and extends it with DistX-specific Similarity Schema features.
 * 
 * @see https://github.com/qdrant/qdrant-js
 */

export { DistXClient, getDistXClient, QdrantClient } from './client';
export { inferSchema, parseCSV, schemaToSummary } from './schema-inference';
export { 
  generateEmbedding, 
  generateEmbeddings, 
  generatePayloadEmbeddings,
  createTextForEmbedding,
  getEmbeddingDimension,
} from './embeddings';
export type {
  FieldType,
  DistanceType,
  FieldConfig,
  SimilaritySchema,
  CollectionInfo,
  Point,
  PointPayload,
  SimilarResult,
  SimilarResponse,
  DataSourceConfig,
  InferredSchema,
  DataCollection,
  FieldContribution,
} from './types';
