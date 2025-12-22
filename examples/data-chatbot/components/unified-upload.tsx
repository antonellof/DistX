"use client";

import { useCallback, useRef, useState } from "react";
import { usePathname, useRouter } from "next/navigation";
import { toast } from "sonner";
import { Button } from "./ui/button";
import { FileIcon, Loader2Icon, CheckCircleIcon, DatabaseIcon, FileTextIcon } from "lucide-react";
import { addFileToChat, saveChatToStorage } from "@/lib/storage/chats";
import { getOrCreateUserId } from "@/lib/storage/user-id";
import { generateUUID } from "@/lib/utils";
import type { UploadedFile } from "@/lib/types/db";

interface UploadResult {
  success: boolean;
  collection?: string;
  filename?: string;
  rowCount?: number;
  chunks?: number;
  wordCount?: number;
  columns?: string[];
  schema?: Array<{
    field: string;
    type: string;
    weight: number;
  }>;
  schemaSummary?: string;
  message?: string;
  error?: string;
  type?: "data" | "document";
  file?: {
    id: string;
    filename: string;
    type: "data" | "document";
    size: number;
    uploadedAt: string;
    collectionName: string;
    rowCount?: number;
    chunkCount?: number;
  };
}

interface UnifiedUploadProps {
  onUploadComplete?: (result: UploadResult) => void;
}

export function UnifiedUpload({ onUploadComplete }: UnifiedUploadProps) {
  const pathname = usePathname();
  const router = useRouter();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [lastUpload, setLastUpload] = useState<UploadResult | null>(null);

  // Extract chatId from pathname (e.g., /chat/123 -> 123)
  const chatId = pathname?.startsWith("/chat/") ? pathname.split("/")[2] : null;

  // Create a new chat if one doesn't exist
  const getOrCreateChatId = useCallback((): string => {
    if (chatId) {
      return chatId;
    }

    // Create a new chat
    const newChatId = generateUUID();
    const userId = getOrCreateUserId();
    
    saveChatToStorage({
      id: newChatId,
      title: "New chat",
      userId,
      visibility: "private",
      createdAt: new Date(),
      uploadedFiles: [],
    });

    // Navigate to the new chat
    router.push(`/chat/${newChatId}`);
    
    return newChatId;
  }, [chatId, router]);

  const isDataFile = (ext: string | undefined): boolean => {
    return ["csv", "xlsx", "xls"].includes(ext || "");
  };

  const isDocumentFile = (ext: string | undefined): boolean => {
    return ["pdf", "docx", "doc", "txt"].includes(ext || "");
  };

  const handleUpload = useCallback(
    async (file: File) => {
      setIsUploading(true);
      setLastUpload(null);

      const ext = file.name.split(".").pop()?.toLowerCase();
      const isData = isDataFile(ext);
      const isDoc = isDocumentFile(ext);

      if (!isData && !isDoc) {
        toast.error("Unsupported file type. Please upload CSV, Excel, PDF, Word, or text files.");
        setIsUploading(false);
        return;
      }

      // Get or create chat ID
      const currentChatId = getOrCreateChatId();

      const formData = new FormData();
      formData.append("file", file);
      formData.append("chatId", currentChatId);

      try {
        const endpoint = isData ? "/api/files/data" : "/api/files/documents";
        const response = await fetch(endpoint, {
          method: "POST",
          body: formData,
        });

        let result: UploadResult;
        try {
          result = await response.json();
        } catch {
          if (response.status === 413) {
            result = { 
              success: false, 
              error: `File too large. Maximum size is ${isData ? "10MB" : "50MB"}.` 
            };
          } else {
            result = { success: false, error: `Upload failed (${response.status})` };
          }
        }

        if (response.ok && result.success) {
          result.type = isData ? "data" : "document";
          setLastUpload(result);
          
          // Save file metadata to chat
          if (result.file && currentChatId) {
            const uploadedFile: UploadedFile = {
              id: result.file.id,
              filename: result.file.filename,
              type: result.file.type,
              size: result.file.size,
              uploadedAt: new Date(result.file.uploadedAt),
              collectionName: result.file.collectionName,
              rowCount: result.file.rowCount,
              chunkCount: result.file.chunkCount,
            };
            addFileToChat(currentChatId, uploadedFile);
            // Dispatch event to refresh chat list
            window.dispatchEvent(new Event("chat-updated"));
          }
          
          if (isData) {
            toast.success(
              `Imported ${result.rowCount} rows into "${result.collection}"`,
              {
                description: `${result.columns?.length} fields detected`,
              }
            );
          } else {
            toast.success(
              `Imported "${result.filename}" with ${result.chunks} chunks`,
              {
                description: `${result.wordCount} words extracted`,
              }
            );
          }
          
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
        await handleUpload(file);
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
        <span className="text-xs font-medium">Import Files</span>
      </div>

      <input
        ref={fileInputRef}
        type="file"
        accept=".csv,.xlsx,.xls,.pdf,.docx,.doc,.txt"
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
            <span className="text-xs font-medium truncate w-full">
              {lastUpload.collection || lastUpload.filename}
            </span>
            <span className="text-xs text-muted-foreground">
              {lastUpload.type === "data" 
                ? `${lastUpload.rowCount} rows`
                : `${lastUpload.chunks} chunks`}
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
              Drop files or click
            </span>
            <span className="text-xs text-muted-foreground/70">
              CSV, Excel, PDF, Word, TXT
            </span>
          </div>
        )}
      </div>

      {lastUpload && lastUpload.type === "data" && lastUpload.schema && (
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
