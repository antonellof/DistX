/**
 * Type definitions for chat data structures
 * These types are used for localStorage storage (no database)
 */

export type UploadedFile = {
  id: string;
  filename: string;
  type: "data" | "document";
  size: number;
  uploadedAt: Date;
  collectionName: string;
  rowCount?: number; // For data files
  chunkCount?: number; // For document files
};

export type Chat = {
  id: string;
  createdAt: Date;
  title: string;
  userId: string;
  visibility: "public" | "private";
  uploadedFiles?: UploadedFile[];
};

export type Vote = {
  chatId: string;
  messageId: string;
  isUpvoted: boolean;
};

export type Suggestion = {
  id: string;
  documentId: string;
  userId: string;
  originalText: string;
  suggestedText: string;
  description: string;
  isResolved: boolean;
  createdAt: Date;
  documentCreatedAt: Date;
};

export type Document = {
  id: string;
  userId: string;
  title: string;
  content: string;
  kind: "text" | "code" | "sheet" | "image";
  createdAt: Date;
};

// Legacy type for compatibility
export type DBMessage = {
  id: string;
  chatId: string;
  role: string;
  parts: unknown[];
  attachments: unknown[];
  createdAt: Date;
};
