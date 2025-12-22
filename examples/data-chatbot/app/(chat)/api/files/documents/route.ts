import { NextResponse } from "next/server";
import { z } from "zod";
import { parseDocument, chunkText } from "@/lib/vectx/document-parser";
import { getVectXClient, generateEmbeddings, getEmbeddingDimension } from "@/lib/vectx";

// Schema for document file validation
const DocumentFileSchema = z.object({
  file: z
    .instanceof(Blob)
    .refine((file) => file.size <= 50 * 1024 * 1024, {
      message: "File size should be less than 50MB",
    })
    .refine(
      (file) => {
        const filename = (file as File).name;
        const ext = filename.split(".").pop()?.toLowerCase();
        return ["pdf", "docx", "doc", "txt", "text"].includes(ext || "");
      },
      {
        message: "File type should be PDF, Word, or Text",
      }
    ),
});

export async function POST(request: Request) {
  if (request.body === null) {
    return new Response("Request body is empty", { status: 400 });
  }

  try {
    // Parse form data
    let formData: FormData;
    try {
      formData = await request.formData();
    } catch (formError) {
      console.error("FormData parse error:", formError);
      return NextResponse.json(
        { error: "File too large. Maximum size is 50MB." },
        { status: 413 }
      );
    }

    const file = formData.get("file") as Blob;
    const chatId = formData.get("chatId") as string | null;

    if (!file) {
      return NextResponse.json({ error: "No file uploaded" }, { status: 400 });
    }

    if (!chatId) {
      return NextResponse.json({ error: "chatId is required" }, { status: 400 });
    }

    // Validate file size
    if (file.size > 50 * 1024 * 1024) {
      return NextResponse.json(
        { error: `File too large (${Math.round(file.size / 1024 / 1024)}MB). Maximum size is 50MB.` },
        { status: 413 }
      );
    }

    // Get filename
    const filename = (formData.get("file") as File).name;
    const extension = filename.split(".").pop()?.toLowerCase();

    // Read file content
    const fileBuffer = Buffer.from(await file.arrayBuffer());

    // Parse document
    let parsedDoc;
    try {
      parsedDoc = await parseDocument(fileBuffer, filename);
    } catch (parseError) {
      return NextResponse.json(
        { error: `Failed to parse document: ${parseError instanceof Error ? parseError.message : "Unknown error"}` },
        { status: 400 }
      );
    }

    if (!parsedDoc.text || parsedDoc.text.trim().length < 10) {
      return NextResponse.json(
        { error: "No text content extracted from document" },
        { status: 400 }
      );
    }

    // Use chatId as collection name (sanitized)
    const finalCollectionName = `chat_${chatId}`.toLowerCase().replace(/[^a-z0-9_-]/g, "_");

    // Connect to vectX
    const client = getVectXClient();
    const isConnected = await client.healthCheck();

    if (!isConnected) {
      return NextResponse.json(
        { error: "vectX is not running. Please start vectX on port 6333." },
        { status: 503 }
      );
    }

    // Chunk the document text
    const chunks = chunkText(parsedDoc.text, 1000, 200);

    // Generate embeddings for chunks
    let embeddings: number[][];
    try {
      embeddings = await generateEmbeddings(chunks);
    } catch (error) {
      console.error("Embedding generation failed:", error);
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      return NextResponse.json(
        { error: `Failed to generate embeddings: ${errorMessage}. Please check OPENAI_API_KEY.` },
        { status: 500 }
      );
    }

    // Create collection if it doesn't exist
    const existingCollection = await client.getCollection(finalCollectionName);
    if (!existingCollection) {
      // Collection doesn't exist, create it
      await client.createCollection(finalCollectionName, {
        vectorSize: getEmbeddingDimension(),
        distance: "Cosine",
      });
    }

    // Get current max ID to append new points
    let maxId = Date.now();
    if (existingCollection) {
      try {
        // Scroll to get points and find max ID
        const scrollResult = await client.scrollPoints(finalCollectionName, { limit: 100 });
        if (scrollResult?.points && Array.isArray(scrollResult.points) && scrollResult.points.length > 0) {
          const ids = scrollResult.points
            .map((p: any) => (typeof p.id === 'number' ? p.id : Date.now()))
            .filter((id: number) => id > 0);
          if (ids.length > 0) {
            maxId = Math.max(...ids, Date.now());
          }
        }
      } catch (e) {
        // Ignore errors, use timestamp
      }
    }

    // Prepare points with embeddings and metadata
    const points = chunks.map((chunk, index) => ({
      id: maxId + index + 1,
      payload: {
        text: chunk,
        document: filename,
        chunk_index: index,
        document_type: extension || null,
        word_count: chunk.split(/\s+/).length,
        _uploaded_file: filename, // Track which file this came from
        _uploaded_file_type: 'document', // Mark as document file
        _uploaded_at: new Date().toISOString(),
      },
      vector: embeddings[index],
    }));

    // Insert in batches
    const batchSize = 20;
    for (let i = 0; i < points.length; i += batchSize) {
      const batch = points.slice(i, i + batchSize);
      await client.upsertPoints(finalCollectionName, batch);
    }

    // Return success with file metadata
    return NextResponse.json({
      success: true,
      collection: finalCollectionName,
      chatId,
      file: {
        id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        filename,
        type: "document" as const,
        size: file.size,
        uploadedAt: new Date().toISOString(),
        collectionName: finalCollectionName,
        chunkCount: chunks.length,
      },
      chunks: chunks.length,
      wordCount: parsedDoc.wordCount,
      message: `Successfully imported "${filename}" with ${chunks.length} chunks into "${finalCollectionName}". You can now query the document.`,
    });
  } catch (error) {
    console.error("Document upload error:", error);
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Failed to process document" },
      { status: 500 }
    );
  }
}
