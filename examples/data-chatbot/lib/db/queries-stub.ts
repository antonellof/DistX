/**
 * Stub query functions for compatibility
 * All data is now stored in localStorage, so these return empty/default values
 */

import type { Document, Suggestion } from "@/lib/types/db";

// Stub functions - return empty/default values since we use localStorage
export async function getDocumentById({ id }: { id: string }): Promise<Document | null> {
  // Documents stored in localStorage if needed in the future
  return null;
}

export async function saveDocument({
  id,
  content,
  title,
  kind,
  userId,
}: {
  id: string;
  content: string;
  title: string;
  kind: "text" | "code" | "sheet" | "image";
  userId: string;
}): Promise<Document> {
  // Stub - documents can be stored in localStorage if needed
  return {
    id,
    userId,
    title,
    content,
    kind,
    createdAt: new Date(),
  };
}

export async function getDocumentsById({ id }: { id: string }): Promise<Document[]> {
  return [];
}

export async function deleteDocumentsByIdAfterTimestamp({
  id,
  timestamp,
}: {
  id: string;
  timestamp: Date;
}): Promise<number> {
  return 0;
}

export async function getSuggestionsByDocumentId({
  documentId,
}: {
  documentId: string;
}): Promise<Suggestion[]> {
  return [];
}

export async function saveSuggestions({
  suggestions,
}: {
  suggestions: Suggestion[];
}): Promise<void> {
  // Stub - suggestions can be stored in localStorage if needed
}

export async function createGuestUser(): Promise<Array<{ email: string; id: string }>> {
  // Stub - no user creation needed (auth removed)
  return [{ email: "guest@example.com", id: "guest-user" }];
}

export async function getUser(email: string): Promise<Array<{ id: string; email: string; password: string | null }>> {
  // Stub - auth removed
  return [];
}

export async function createUser(email: string, password: string): Promise<void> {
  // Stub - auth removed
}
