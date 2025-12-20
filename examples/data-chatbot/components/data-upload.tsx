"use client";

import { useCallback, useRef, useState } from "react";
import { toast } from "sonner";
import { Button } from "./ui/button";
import { DatabaseIcon, FileSpreadsheetIcon, Loader2Icon, CheckCircleIcon } from "lucide-react";

interface DataUploadResult {
  success: boolean;
  collection?: string;
  filename?: string;
  rowCount?: number;
  columns?: string[];
  schema?: Array<{
    field: string;
    type: string;
    weight: number;
  }>;
  schemaSummary?: string;
  message?: string;
  error?: string;
}

interface DataUploadProps {
  onUploadComplete?: (result: DataUploadResult) => void;
}

export function DataUpload({ onUploadComplete }: DataUploadProps) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [lastUpload, setLastUpload] = useState<DataUploadResult | null>(null);

  const handleUpload = useCallback(
    async (file: File) => {
      setIsUploading(true);
      setLastUpload(null);

      const formData = new FormData();
      formData.append("file", file);

      try {
        const response = await fetch("/api/files/data", {
          method: "POST",
          body: formData,
        });

        const result: DataUploadResult = await response.json();

        if (response.ok && result.success) {
          setLastUpload(result);
          toast.success(
            `Imported ${result.rowCount} rows into "${result.collection}"`,
            {
              description: `${result.columns?.length} fields detected`,
            }
          );
          onUploadComplete?.(result);
        } else {
          toast.error(result.error || "Upload failed");
        }
      } catch (error) {
        toast.error("Failed to upload file");
        console.error("Upload error:", error);
      } finally {
        setIsUploading(false);
      }
    },
    [onUploadComplete]
  );

  const handleFileChange = useCallback(
    async (event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0];
      if (file) {
        await handleUpload(file);
      }
      // Reset input
      if (fileInputRef.current) {
        fileInputRef.current.value = "";
      }
    },
    [handleUpload]
  );

  const handleDrop = useCallback(
    async (event: React.DragEvent) => {
      event.preventDefault();
      const file = event.dataTransfer.files?.[0];
      if (file) {
        const ext = file.name.split(".").pop()?.toLowerCase();
        if (ext === "csv") {
          await handleUpload(file);
        } else {
          toast.error("Please upload a CSV file");
        }
      }
    },
    [handleUpload]
  );

  const handleDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
  }, []);

  return (
    <div className="p-3 border-b">
      <div className="flex items-center gap-2 mb-2">
        <DatabaseIcon className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-medium">Data Import</span>
      </div>

      <input
        ref={fileInputRef}
        type="file"
        accept=".csv"
        onChange={handleFileChange}
        className="hidden"
      />

      <div
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        className="border-2 border-dashed rounded-lg p-4 text-center hover:border-primary/50 transition-colors cursor-pointer"
        onClick={() => fileInputRef.current?.click()}
      >
        {isUploading ? (
          <div className="flex flex-col items-center gap-2">
            <Loader2Icon className="h-6 w-6 animate-spin text-muted-foreground" />
            <span className="text-sm text-muted-foreground">Importing...</span>
          </div>
        ) : lastUpload ? (
          <div className="flex flex-col items-center gap-2">
            <CheckCircleIcon className="h-6 w-6 text-green-500" />
            <span className="text-sm font-medium">{lastUpload.collection}</span>
            <span className="text-xs text-muted-foreground">
              {lastUpload.rowCount} rows · {lastUpload.columns?.length} fields
            </span>
            <Button
              variant="ghost"
              size="sm"
              className="text-xs"
              onClick={(e) => {
                e.stopPropagation();
                setLastUpload(null);
              }}
            >
              Upload another
            </Button>
          </div>
        ) : (
          <div className="flex flex-col items-center gap-2">
            <FileSpreadsheetIcon className="h-6 w-6 text-muted-foreground" />
            <span className="text-sm text-muted-foreground">
              Drop CSV here or click to upload
            </span>
            <span className="text-xs text-muted-foreground">
              Auto-detects schema for similarity queries
            </span>
          </div>
        )}
      </div>

      {lastUpload && lastUpload.schema && (
        <div className="mt-3 text-xs">
          <div className="font-medium mb-1">Detected Fields:</div>
          <div className="space-y-1 max-h-32 overflow-y-auto">
            {lastUpload.schema.slice(0, 6).map((field) => (
              <div
                key={field.field}
                className="flex justify-between text-muted-foreground"
              >
                <span>{field.field}</span>
                <span className="text-xs">
                  {field.type} ({Math.round(field.weight * 100)}%)
                </span>
              </div>
            ))}
            {lastUpload.schema.length > 6 && (
              <div className="text-muted-foreground">
                +{lastUpload.schema.length - 6} more...
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
