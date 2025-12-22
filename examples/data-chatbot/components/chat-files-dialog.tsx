"use client";

import { useEffect, useState } from "react";
import { formatDistanceToNow } from "date-fns";
import { FileIcon, DatabaseIcon, FileTextIcon, Loader2Icon } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { getChatFiles, getChatsFromStorage } from "@/lib/storage/chats";
import type { UploadedFile } from "@/lib/types/db";

interface ChatFilesDialogProps {
  chatId: string | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function ChatFilesDialog({ chatId, open, onOpenChange }: ChatFilesDialogProps) {
  const [files, setFiles] = useState<UploadedFile[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [chatTitle, setChatTitle] = useState<string>("");

  useEffect(() => {
    if (open && chatId) {
      setIsLoading(true);
      const chats = getChatsFromStorage();
      const chat = chats.find((c) => c.id === chatId);
      
      if (chat) {
        setChatTitle(chat.title);
        setFiles(chat.uploadedFiles || []);
      } else {
        setFiles([]);
      }
      setIsLoading(false);
    } else {
      setFiles([]);
      setChatTitle("");
    }
  }, [open, chatId]);

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[80vh] overflow-hidden flex flex-col">
        <DialogHeader>
          <DialogTitle>Uploaded Files</DialogTitle>
          <DialogDescription>
            {chatTitle && `Files uploaded to "${chatTitle}"`}
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto mt-4">
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <Loader2Icon className="h-6 w-6 animate-spin text-muted-foreground" />
              <span className="ml-2 text-sm text-muted-foreground">Loading...</span>
            </div>
          ) : files.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-8 text-center">
              <FileIcon className="h-12 w-12 text-muted-foreground/50 mb-4" />
              <p className="text-sm text-muted-foreground">
                No files uploaded yet
              </p>
              <p className="text-xs text-muted-foreground/70 mt-1">
                Upload files via the sidebar or chat attachment
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {files.map((file) => (
                <div
                  key={file.id}
                  className="flex items-start gap-3 p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors"
                >
                  <div className="flex-shrink-0 mt-0.5">
                    {file.type === "data" ? (
                      <DatabaseIcon className="h-5 w-5 text-blue-500" />
                    ) : (
                      <FileTextIcon className="h-5 w-5 text-green-500" />
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium truncate" title={file.filename}>
                          {file.filename}
                        </p>
                        <div className="flex items-center gap-3 mt-1 text-xs text-muted-foreground">
                          <span className="capitalize">{file.type}</span>
                          <span>•</span>
                          <span>{formatFileSize(file.size)}</span>
                          {file.rowCount && (
                            <>
                              <span>•</span>
                              <span>{file.rowCount.toLocaleString()} rows</span>
                            </>
                          )}
                          {file.chunkCount && (
                            <>
                              <span>•</span>
                              <span>{file.chunkCount} chunks</span>
                            </>
                          )}
                        </div>
                      </div>
                      <div className="text-xs text-muted-foreground whitespace-nowrap">
                        {formatDistanceToNow(new Date(file.uploadedAt), { addSuffix: true })}
                      </div>
                    </div>
                    <div className="mt-2 text-xs text-muted-foreground">
                      <span className="font-medium">Collection:</span>{" "}
                      <code className="px-1.5 py-0.5 rounded bg-muted text-xs">
                        {file.collectionName}
                      </code>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
