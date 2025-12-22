import { NextResponse } from "next/server";
import { z } from "zod";
import * as XLSX from "xlsx";
import {
  getVectXClient,
  parseCSV,
  generatePayloadEmbeddings,
  getEmbeddingDimension,
  type PointPayload,
} from "@/lib/vectx";

// Schema for data file validation
const DataFileSchema = z.object({
  file: z
    .instanceof(Blob)
    .refine((file) => file.size <= 10 * 1024 * 1024, {
      message: "File size should be less than 10MB",
    })
        .refine(
      (file) => {
        const ext = (file as File).name.split(".").pop()?.toLowerCase();
        const validTypes = [
          "text/csv",
          "text/plain",
          "application/csv",
          "application/vnd.ms-excel",
          "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ];
        return validTypes.includes(file.type) || file.type === "" || ["csv", "xlsx", "xls"].includes(ext || "");
      },
      {
        message: "File type should be CSV or Excel (.csv, .xlsx, .xls)",
      }
    ),
});

export async function POST(request: Request) {
  // No auth required

  if (request.body === null) {
    return new Response("Request body is empty", { status: 400 });
  }

  try {
    // Parse form data with error handling for large files
    let formData: FormData;
    try {
      formData = await request.formData();
    } catch (formError) {
      console.error("FormData parse error:", formError);
      return NextResponse.json(
        { error: "File too large. Maximum size is 10MB. Please use a smaller file." },
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
    if (file.size > 10 * 1024 * 1024) {
      return NextResponse.json(
        { error: `File too large (${Math.round(file.size / 1024 / 1024)}MB). Maximum size is 10MB.` },
        { status: 413 }
      );
    }

    // Get filename
    const filename = (formData.get("file") as File).name;
    const extension = filename.split(".").pop()?.toLowerCase();

    // Validate file type by extension if MIME type is not reliable
    if (!["csv", "xlsx", "xls"].includes(extension || "")) {
      return NextResponse.json(
        { error: "File must be CSV or Excel format" },
        { status: 400 }
      );
    }

    // Read file content
    const fileBuffer = await file.arrayBuffer();
    let data: Record<string, unknown>[];

    // Parse based on file type
    if (extension === "csv") {
      const textContent = new TextDecoder().decode(fileBuffer);
      try {
        data = parseCSV(textContent);
      } catch (parseError) {
        return NextResponse.json(
          { error: `Failed to parse CSV: ${parseError instanceof Error ? parseError.message : "Unknown error"}` },
          { status: 400 }
        );
      }
    } else if (extension === "xlsx" || extension === "xls") {
      // Parse Excel file
      try {
        const workbook = XLSX.read(fileBuffer, { type: "array" });
        
        // Get the first sheet
        const firstSheetName = workbook.SheetNames[0];
        if (!firstSheetName) {
          return NextResponse.json(
            { error: "Excel file has no sheets" },
            { status: 400 }
          );
        }
        
        const worksheet = workbook.Sheets[firstSheetName];
        
        // Convert to JSON (array of objects)
        data = XLSX.utils.sheet_to_json(worksheet, {
          raw: false, // Convert all values to strings for consistency
          defval: null, // Use null for empty cells
        }) as Record<string, unknown>[];
        
        // Convert all values to strings/numbers/booleans (XLSX may return mixed types)
        data = data.map((row) => {
          const normalized: Record<string, unknown> = {};
          for (const [key, value] of Object.entries(row)) {
            // Convert null/undefined to empty string for consistency
            if (value === null || value === undefined) {
              normalized[key] = "";
            } else {
              normalized[key] = value;
            }
          }
          return normalized;
        });
      } catch (parseError) {
        return NextResponse.json(
          { error: `Failed to parse Excel file: ${parseError instanceof Error ? parseError.message : "Unknown error"}` },
          { status: 400 }
        );
      }
    } else {
      return NextResponse.json(
        { error: `Unsupported file format: ${extension}. Supported formats: CSV, XLSX, XLS` },
        { status: 400 }
      );
    }

    if (data.length === 0) {
      return NextResponse.json(
        { error: "No data rows found in CSV" },
        { status: 400 }
      );
    }

    // Infer schema
    // Schema inference removed - using standard vector embeddings

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

    // Get or create collection (don't delete, allow multiple uploads to same chat)
    const existingCollection = await client.getCollection(finalCollectionName);

    // Generate embeddings for all data
    let embeddings: number[][] = [];
    try {
      // Get all field names from the first row as text fields
      const textFields = data.length > 0 ? Object.keys(data[0]) : [];
      embeddings = await generatePayloadEmbeddings(data, textFields);
    } catch (error) {
      console.log("Embedding generation failed, using zero vectors:", error);
      const dim = getEmbeddingDimension();
      embeddings = data.map(() => new Array(dim).fill(0));
    }

    // Create collection if it doesn't exist
    if (!existingCollection) {
      await client.createCollection(finalCollectionName, {
        vectorSize: getEmbeddingDimension(),
        distance: 'Cosine',
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
            .map((p: any) => (typeof p.id === 'number' ? p.id : 0))
            .filter((id: number) => id > 0);
          if (ids.length > 0) {
            maxId = Math.max(...ids, Date.now());
          }
        }
      } catch (e) {
        // Ignore errors, use timestamp
      }
    }

    // Prepare points with embeddings
    const points = data.map((row, index) => ({
      id: maxId + index + 1,
      payload: {
        ...(row as PointPayload),
        _uploaded_file: filename, // Track which file this came from
        _uploaded_file_type: 'data', // Mark as data file
        _uploaded_at: new Date().toISOString(),
      } as PointPayload,
      vector: embeddings[index],
    }));

    // Insert in batches (smaller batches due to large vectors)
    const batchSize = 20;
    for (let i = 0; i < points.length; i += batchSize) {
      const batch = points.slice(i, i + batchSize);
      await client.upsertPoints(finalCollectionName, batch);
    }

    // Return success with schema info and file metadata
    return NextResponse.json({
      success: true,
      collection: finalCollectionName,
      chatId,
      file: {
        id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        filename,
        type: "data" as const,
        size: file.size,
        uploadedAt: new Date().toISOString(),
        collectionName: finalCollectionName,
        rowCount: data.length,
      },
      rowCount: data.length,
      columns: Object.keys(data[0] || {}),
      schema: Object.keys(data[0] || {}).map((field) => ({
        field,
        type: "text",
      })),
      schemaSummary: "Standard vector embeddings",
      sampleRows: data.slice(0, 3),
      message: `Successfully imported ${data.length} rows into "${finalCollectionName}". You can now query for similar records.`,
    });
  } catch (error) {
    console.error("Data upload error:", error);
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Failed to process data file" },
      { status: 500 }
    );
  }
}
