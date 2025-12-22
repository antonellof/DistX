/**
 * Server-side embedding generation for vectX
 * 
 * vectX is a vector database - it stores and queries vectors.
 * This module generates embeddings using OpenAI that are then
 * stored in vectX for semantic search.
 * 
 * NOTE: This module is server-only. Embeddings are generated
 * on the server to keep API keys secure.
 */

import OpenAI from 'openai';

const EMBEDDING_MODEL = 'text-embedding-3-small';
const EMBEDDING_DIMENSION = 1536;

// Lazy initialization of OpenAI client (server-side only)
let openaiClient: OpenAI | null = null;

const getOpenAIClient = (): OpenAI => {
  // Prevent client-side usage
  if (typeof window !== 'undefined') {
    throw new Error('Embeddings can only be generated on the server side');
  }
  
  if (!openaiClient) {
    const apiKey = process.env.OPENAI_API_KEY;
    if (!apiKey) {
      throw new Error("OPENAI_API_KEY environment variable is not set");
    }
    openaiClient = new OpenAI({ apiKey });
  }
  return openaiClient;
};

/**
 * Generate embedding for a single text
 */
export async function generateEmbedding(text: string): Promise<number[]> {
  const openaiClient = getOpenAIClient();
  const response = await openaiClient.embeddings.create({
    model: EMBEDDING_MODEL,
    input: text,
  });
  return response.data[0].embedding;
}

/**
 * Generate embeddings for multiple texts (batched for efficiency)
 */
export async function generateEmbeddings(texts: string[]): Promise<number[][]> {
  if (texts.length === 0) return [];
  
  // Process in batches of 100 (OpenAI limit)
  const batchSize = 100;
  const allEmbeddings: number[][] = [];
  
  const openaiClient = getOpenAIClient();
  for (let i = 0; i < texts.length; i += batchSize) {
    const batch = texts.slice(i, i + batchSize);
    const response = await openaiClient.embeddings.create({
      model: EMBEDDING_MODEL,
      input: batch,
    });
    
    // Sort by index to maintain order
    const sorted = response.data.sort((a, b) => a.index - b.index);
    allEmbeddings.push(...sorted.map(d => d.embedding));
  }
  
  return allEmbeddings;
}

/**
 * Create a composite text from payload fields for embedding
 * 
 * This combines relevant text fields into a single string
 * that captures the semantic meaning of the record.
 */
export function createTextForEmbedding(
  payload: Record<string, unknown>,
  textFields: string[]
): string {
  const parts: string[] = [];
  
  for (const field of textFields) {
    const value = payload[field];
    if (value != null && typeof value === 'string' && value.trim()) {
      // Include field name for context
      parts.push(`${field}: ${value}`);
    }
  }
  
  return parts.join('. ');
}

/**
 * Generate embeddings for a batch of payloads
 * 
 * @param payloads - Array of record payloads
 * @param textFields - Fields to include in the embedding
 * @returns Array of embedding vectors
 */
export async function generatePayloadEmbeddings(
  payloads: Record<string, unknown>[],
  textFields: string[]
): Promise<number[][]> {
  // Create composite text for each payload
  const texts = payloads.map(payload => 
    createTextForEmbedding(payload, textFields)
  );
  
  // Filter out empty texts and track indices
  const nonEmptyTexts: { index: number; text: string }[] = [];
  texts.forEach((text, index) => {
    if (text.trim()) {
      nonEmptyTexts.push({ index, text });
    }
  });
  
  if (nonEmptyTexts.length === 0) {
    // All texts are empty, return zero vectors
    const zeroVector = new Array(EMBEDDING_DIMENSION).fill(0);
    return payloads.map(() => [...zeroVector]);
  }
  
  // Generate embeddings for non-empty texts
  const embeddings = await generateEmbeddings(
    nonEmptyTexts.map(t => t.text)
  );
  
  // Map back to original indices, using zero vector for empty
  const zeroVector = new Array(EMBEDDING_DIMENSION).fill(0);
  const result: number[][] = new Array(payloads.length).fill(null).map(() => [...zeroVector]);
  
  nonEmptyTexts.forEach((item, i) => {
    result[item.index] = embeddings[i];
  });
  
  return result;
}

/**
 * Get the embedding dimension
 */
export function getEmbeddingDimension(): number {
  return EMBEDDING_DIMENSION;
}
