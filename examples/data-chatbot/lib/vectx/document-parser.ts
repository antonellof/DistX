/**
 * Document parsing for vectX RAG
 * 
 * Extracts text from various document formats (PDF, Word, text)
 * and prepares them for embedding and storage in vectX.
 * 
 * Uses a local Python FastAPI service for PDF/Word parsing (same approach as fastest-rag-stack).
 * 
 * SERVER-ONLY: This module must not be imported in client components.
 */

import "server-only";

// Get PDF parser service URL from environment
function getParserServiceUrl(): string {
  return process.env.PDF_PARSER_SERVICE_URL || "http://localhost:8000";
}

export interface ParsedDocument {
  text: string;
  filename: string;
  pageCount?: number;
  wordCount: number;
}

/**
 * Extract text from PDF file using Python FastAPI service
 */
export async function parsePDF(fileBuffer: Buffer, filename: string): Promise<ParsedDocument> {
  try {
    const serviceUrl = getParserServiceUrl();
    
    // Create FormData to send file to Python service
    const formData = new FormData();
    const blob = new Blob([fileBuffer], { type: "application/pdf" });
    formData.append("file", blob, filename);

    // Call Python FastAPI service
    const response = await fetch(`${serviceUrl}/parse`, {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`PDF parser service error (${response.status}): ${errorText}`);
    }

    const result = await response.json();

    return {
      text: result.text || "",
      filename,
      pageCount: result.pageCount || undefined,
      wordCount: result.wordCount || 0,
    };
  } catch (error) {
    throw new Error(`Failed to parse PDF: ${error instanceof Error ? error.message : "Unknown error"}`);
  }
}

/**
 * Extract text from Word document (.docx) using Python FastAPI service
 */
export async function parseWord(fileBuffer: Buffer, filename: string): Promise<ParsedDocument> {
  try {
    const serviceUrl = getParserServiceUrl();
    const formData = new FormData();
    const blob = new Blob([fileBuffer], { type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document" });
    formData.append("file", blob, filename);

    const response = await fetch(`${serviceUrl}/parse`, {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`PDF parser service error (${response.status}): ${errorText}`);
    }

    const result = await response.json();

    return {
      text: result.text || "",
      filename,
      pageCount: result.pageCount || undefined,
      wordCount: result.wordCount || 0,
    };
  } catch (error) {
    throw new Error(`Failed to parse Word document: ${error instanceof Error ? error.message : "Unknown error"}`);
  }
}

/**
 * Extract text from plain text file using Python FastAPI service
 */
export async function parseText(fileBuffer: Buffer, filename: string): Promise<ParsedDocument> {
  try {
    const serviceUrl = getParserServiceUrl();
    const formData = new FormData();
    const blob = new Blob([fileBuffer], { type: "text/plain" });
    formData.append("file", blob, filename);

    const response = await fetch(`${serviceUrl}/parse`, {
      method: "POST",
      body: formData,
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`PDF parser service error (${response.status}): ${errorText}`);
    }

    const result = await response.json();

    return {
      text: result.text || "",
      filename,
      pageCount: result.pageCount || undefined,
      wordCount: result.wordCount || 0,
    };
  } catch (error) {
    throw new Error(`Failed to parse text file: ${error instanceof Error ? error.message : "Unknown error"}`);
  }
}

/**
 * Parse document based on file extension
 */
export async function parseDocument(
  fileBuffer: Buffer,
  filename: string
): Promise<ParsedDocument> {
  const ext = filename.split(".").pop()?.toLowerCase();
  
  switch (ext) {
    case "pdf":
      return parsePDF(fileBuffer, filename);
    case "docx":
    case "doc":
      return parseWord(fileBuffer, filename);
    case "txt":
    case "text":
      return parseText(fileBuffer, filename);
    default:
      throw new Error(`Unsupported file type: ${ext}. Supported: PDF, DOCX, TXT`);
  }
}

/**
 * Chunk text into overlapping segments for better retrieval
 */
export function chunkText(
  text: string,
  chunkSize: number = 1000,
  overlap: number = 200
): string[] {
  if (text.length <= chunkSize) {
    return [text];
  }
  
  const chunks: string[] = [];
  let start = 0;
  
  while (start < text.length) {
    let end = start + chunkSize;
    let chunk = text.slice(start, end);
    
    // Try to break at paragraph or sentence boundary
    if (end < text.length) {
      // Look for paragraph break
      const lastPara = chunk.lastIndexOf("\n\n");
      if (lastPara > chunkSize / 2) {
        chunk = chunk.slice(0, lastPara);
        end = start + lastPara;
      } else {
        // Look for sentence break
        const lastPeriod = chunk.lastIndexOf(". ");
        if (lastPeriod > chunkSize / 2) {
          chunk = chunk.slice(0, lastPeriod + 1);
          end = start + lastPeriod + 1;
        }
      }
    }
    
    chunks.push(chunk.trim());
    start = end - overlap; // Overlap for context continuity
    
    if (start >= text.length) {
      break;
    }
  }
  
  return chunks.filter((c) => c.length > 0);
}
