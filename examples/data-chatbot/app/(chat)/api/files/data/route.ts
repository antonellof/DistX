import { NextResponse } from "next/server";
import { z } from "zod";
import { auth } from "@/app/(auth)/auth";
import { 
  getDistXClient, 
  parseCSV, 
  inferSchema, 
  schemaToSummary,
  generatePayloadEmbeddings,
  getEmbeddingDimension,
} from "@/lib/distx";

// Schema for data file validation
const DataFileSchema = z.object({
  file: z
    .instanceof(Blob)
    .refine((file) => file.size <= 10 * 1024 * 1024, {
      message: "File size should be less than 10MB",
    })
    .refine(
      (file) =>
        [
          "text/csv",
          "text/plain",
          "application/csv",
          "application/vnd.ms-excel",
          "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ].includes(file.type) || file.type === "",
      {
        message: "File type should be CSV or Excel",
      }
    ),
});

export async function POST(request: Request) {
  const session = await auth();

  if (!session) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (request.body === null) {
    return new Response("Request body is empty", { status: 400 });
  }

  try {
    const formData = await request.formData();
    const file = formData.get("file") as Blob;
    const collectionName = formData.get("collectionName") as string;

    if (!file) {
      return NextResponse.json({ error: "No file uploaded" }, { status: 400 });
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
    const textContent = new TextDecoder().decode(fileBuffer);

    // For now, only support CSV (Excel would need a library like xlsx)
    if (extension !== "csv") {
      return NextResponse.json(
        { error: "Currently only CSV files are supported. Excel support coming soon." },
        { status: 400 }
      );
    }

    // Parse CSV
    let data;
    try {
      data = parseCSV(textContent);
    } catch (parseError) {
      return NextResponse.json(
        { error: `Failed to parse CSV: ${parseError instanceof Error ? parseError.message : "Unknown error"}` },
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
    const inferred = inferSchema(data);

    // Generate collection name from filename if not provided
    const finalCollectionName = (collectionName || filename.replace(/\.[^/.]+$/, ""))
      .toLowerCase()
      .replace(/[^a-z0-9_-]/g, "_")
      .substring(0, 50);

    // Connect to DistX
    const client = getDistXClient();
    const isConnected = await client.healthCheck();

    if (!isConnected) {
      return NextResponse.json(
        { error: "DistX is not running. Please start DistX on port 6333." },
        { status: 503 }
      );
    }

    // Delete existing collection if it exists
    await client.deleteCollection(finalCollectionName);

    // Generate embeddings for text fields
    const textFields = Object.entries(inferred.fields)
      .filter(([_, config]) => config.type === 'text')
      .map(([name]) => name);

    let embeddings: number[][] = [];
    try {
      embeddings = await generatePayloadEmbeddings(data, textFields);
    } catch (error) {
      console.log("Embedding generation failed, using zero vectors:", error);
      const dim = getEmbeddingDimension();
      embeddings = data.map(() => new Array(dim).fill(0));
    }

    // Create collection with vector size and schema
    await client.createCollection(finalCollectionName, {
      vectorSize: getEmbeddingDimension(),
      distance: 'Cosine',
      schema: { fields: inferred.fields },
    });

    // Prepare points with embeddings
    const points = data.map((row, index) => ({
      id: index + 1,
      payload: row,
      vector: embeddings[index],
    }));

    // Insert in batches
    const batchSize = 100;
    for (let i = 0; i < points.length; i += batchSize) {
      const batch = points.slice(i, i + batchSize);
      await client.upsertPoints(finalCollectionName, batch);
    }

    // Return success with schema info
    return NextResponse.json({
      success: true,
      collection: finalCollectionName,
      filename,
      rowCount: data.length,
      columns: Object.keys(inferred.fields),
      schema: inferred.inferenceDetails.map((d) => ({
        field: d.field,
        type: d.inferredType,
        weight: inferred.fields[d.field].weight,
      })),
      schemaSummary: schemaToSummary({ fields: inferred.fields }),
      sampleRows: inferred.sampleData.slice(0, 3),
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
