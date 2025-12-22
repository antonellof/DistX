"use client";

import { useCallback, useRef, useState } from "react";
import { toast } from "sonner";
import { Button } from "./ui/button";
import { FileTextIcon, FileIcon, Loader2Icon, CheckCircleIcon } from "lucide-react";

interface DocumentUploadResult {
  success: boolean;
  collection?: string;
  filename?: string;
  chunks?: number;
  wordCount?: number;
  message?: string;
  error?: string;
}

interface DocumentUploadProps {
  onUploadComplete?: (result: DocumentUploadResult) => void;
}

export function DocumentUpload({ onUploadComplete }: DocumentUploadProps) {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [lastUpload, setLastUpload] = useState<DocumentUploadResult | null>(null);

  const handleUpload = useCallback(
    async (file: File) => {
      setIsUploading(true);
      setLastUpload(null);

      const formData = new FormData();
      formData.append("file", file);

      try {
        const response = await fetch("/api/files/documents", {
          method: "POST",
          body: formData,
        });

        let result: DocumentUploadResult;
        try {
          result = await response.json();
        } catch {
          if (response.status === 413) {
            result = { success: false, error: "File too large. Maximum size is 50MB." };
          } else {
            result = { success: false, error: `Upload failed (${response.status})` };
          }
        }

        if (response.ok && result.success) {
          setLastUpload(result);
          toast.success(
            `Imported "${result.filename}" with ${result.chunks} chunks`,
            {
              description: `${result.wordCount} words extracted`,
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
        if (["pdf", "docx", "doc", "txt"].includes(ext || "")) {
          await handleUpload(file);
        } else {
          toast.error("Please upload a PDF, Word, or text file");
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
        <FileTextIcon className="h-3.5 w-3.5 text-muted-foreground" />
        <span className="text-xs font-medium">Document Import</span>
      </div>

      <input
        ref={fileInputRef}
        type="file"
        accept=".pdf,.docx,.doc,.txt"
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
            <span className="text-xs text-muted-foreground">Processing...</span>
          </div>
        ) : lastUpload ? (
          <div className="flex flex-col items-center gap-1">
            <CheckCircleIcon className="h-4 w-4 text-green-500" />
            <span className="text-xs font-medium truncate w-full">{lastUpload.filename}</span>
            <span className="text-xs text-muted-foreground">
              {lastUpload.chunks} chunks
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
            <FileIcon className="h-4 w-4 text-muted-foreground" />
            <span className="text-xs text-muted-foreground">
              Drop PDF/Word or click
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
