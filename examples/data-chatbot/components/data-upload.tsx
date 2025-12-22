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

        let result: DataUploadResult;
        try {
          result = await response.json();
        } catch {
          // JSON parse failed - create error from status
          if (response.status === 413) {
            result = { success: false, error: "File too large. Maximum size is 10MB." };
          } else {
            result = { success: false, error: `Upload failed (${response.status})` };
          }
        }

        if (response.ok && result.success) {
          setLastUpload(result);
          toast.success(
            `Imported ${result.rowCount} rows into "${result.collection}"`,
            {
              description: `${result.columns?.length} fields detected`,
            }
          );
          // Dispatch event to refresh collections list
          window.dispatchEvent(new Event("collection-updated"));
          onUploadComplete?.(result);
        } else {
          toast.error(result.error || "Upload failed", {
            description: response.status === 413 ? "Try a smaller file" : undefined,
            duration: 5000,
          });
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : "Failed to upload file";
        toast.error(message, { duration: 5000 });
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
        if (["csv", "xlsx", "xls"].includes(ext || "")) {
          await handleUpload(file);
        } else {
          toast.error("Please upload a CSV or Excel file (.csv, .xlsx, .xls)");
        }
      }
    },
    [handleUpload]
  );

  const handleDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
  }, []);

  return (
    <div className="p-2 border-b">
      <div className="flex items-center gap-2 mb-1.5">
        <DatabaseIcon className="h-3.5 w-3.5 text-muted-foreground" />
        <span className="text-xs font-medium">Data Import</span>
      </div>

      <input
        ref={fileInputRef}
        type="file"
        accept=".csv,.xlsx,.xls"
        onChange={handleFileChange}
        className="hidden"
      />

      <div
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        className="border-2 border-dashed rounded-lg p-2 text-center hover:border-primary/50 transition-colors cursor-pointer"
        onClick={() => fileInputRef.current?.click()}
      >
        {isUploading ? (
          <div className="flex flex-col items-center gap-1">
            <Loader2Icon className="h-4 w-4 animate-spin text-muted-foreground" />
            <span className="text-xs text-muted-foreground">Importing...</span>
          </div>
        ) : lastUpload ? (
          <div className="flex flex-col items-center gap-1">
            <CheckCircleIcon className="h-4 w-4 text-green-500" />
            <span className="text-xs font-medium truncate w-full">{lastUpload.collection}</span>
            <span className="text-xs text-muted-foreground">
              {lastUpload.rowCount} rows
            </span>
            <Button
              variant="ghost"
              size="sm"
              className="text-xs h-6"
              onClick={(e) => {
                e.stopPropagation();
                setLastUpload(null);
              }}
            >
              Upload another
            </Button>
          </div>
        ) : (
          <div className="flex flex-col items-center gap-1">
            <FileSpreadsheetIcon className="h-4 w-4 text-muted-foreground" />
            <span className="text-xs text-muted-foreground">
              Drop CSV or click
            </span>
          </div>
        )}
      </div>

      {lastUpload && lastUpload.schema && (
        <div className="mt-2 text-xs">
          <div className="font-medium mb-1 text-xs">Fields:</div>
          <div className="space-y-0.5 max-h-20 overflow-y-auto">
            {lastUpload.schema.slice(0, 4).map((field) => (
              <div
                key={field.field}
                className="flex justify-between text-muted-foreground text-xs"
              >
                <span className="truncate">{field.field}</span>
                <span className="text-xs ml-1">
                  {field.type}
                </span>
              </div>
            ))}
            {lastUpload.schema.length > 4 && (
              <div className="text-muted-foreground text-xs">
                +{lastUpload.schema.length - 4} more...
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
