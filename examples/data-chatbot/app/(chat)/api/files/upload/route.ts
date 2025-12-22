import { put } from "@vercel/blob";
import { NextResponse } from "next/server";
import { z } from "zod";

// Supported file types
const IMAGE_TYPES = ["image/jpeg", "image/png", "image/gif", "image/webp"];
const DATA_TYPES = ["text/csv", "text/plain", "application/csv", "application/vnd.ms-excel"];

// Use Blob instead of File since File is not available in Node.js environment
const FileSchema = z.object({
  file: z
    .instanceof(Blob)
    .refine((file) => file.size <= 10 * 1024 * 1024, {
      message: "File size should be less than 10MB",
    })
    // Accept images and data files (CSV)
    .refine(
      (file) => [...IMAGE_TYPES, ...DATA_TYPES].includes(file.type) || file.type === "",
      {
        message: "File type should be an image (JPEG, PNG) or data file (CSV)",
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
        { error: "File too large. Maximum size is 10MB." },
        { status: 413 }
      );
    }

    const file = formData.get("file") as Blob;

    if (!file) {
      return NextResponse.json({ error: "No file uploaded" }, { status: 400 });
    }

    const validatedFile = FileSchema.safeParse({ file });

    if (!validatedFile.success) {
      const errorMessage = validatedFile.error.errors
        .map((error) => error.message)
        .join(", ");

      return NextResponse.json({ error: errorMessage }, { status: 400 });
    }

    // Get filename from formData since Blob doesn't have name property
    const filename = (formData.get("file") as File).name;
    const fileBuffer = await file.arrayBuffer();

    try {
      const data = await put(`${filename}`, fileBuffer, {
        access: "public",
      });

      return NextResponse.json(data);
    } catch (_error) {
      return NextResponse.json({ error: "Upload failed" }, { status: 500 });
    }
  } catch (_error) {
    return NextResponse.json(
      { error: "Failed to process request" },
      { status: 500 }
    );
  }
}
