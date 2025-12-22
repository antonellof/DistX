/**
 * vectX - Vector Database
 * 
 * vectX is a Qdrant-compatible vector database. The client generates embeddings
 * using OpenAI and stores them in vectX for semantic search.
 * 
 * This client uses the official Qdrant JS SDK for standard vector operations.
 * 
 * @see https://github.com/qdrant/qdrant-js
 */

export { vectXClient, getVectXClient, QdrantClient } from './client';
export { parseCSV } from './schema-inference';
export { 
  generateEmbedding, 
  generateEmbeddings, 
  generatePayloadEmbeddings,
  createTextForEmbedding,
  getEmbeddingDimension,
} from './embeddings';
// Document parser is server-only - import directly from './document-parser' in API routes
// export { parseDocument, parsePDF, parseWord, parseText, chunkText } from './document-parser';
// export type { ParsedDocument } from './document-parser';
export type {
  CollectionInfo,
  Point,
  PointPayload,
} from './types';
