"use client";

import type { Chat } from "@/lib/types/db";
import type { ChatMessage } from "@/lib/types";
import { getVectXClient } from "@/lib/vectx";

const CHATS_STORAGE_KEY = "vectx-chats";
const MESSAGES_STORAGE_KEY = "vectx-messages";

// Extended type for stored messages (includes chatId)
type StoredMessage = ChatMessage & { chatId: string };

/**
 * Get all chats from localStorage
 */
export function getChatsFromStorage(): Chat[] {
  if (typeof window === "undefined") {
    return [];
  }

  try {
    const stored = localStorage.getItem(CHATS_STORAGE_KEY);
    if (!stored) return [];
    const parsed = JSON.parse(stored) as any[];
    // Convert createdAt strings back to Date objects
    return parsed.map((chat) => ({
      ...chat,
      createdAt: new Date(chat.createdAt),
    })) as Chat[];
  } catch {
    return [];
  }
}

/**
 * Save a chat to localStorage
 */
export function saveChatToStorage(chat: Chat): void {
  if (typeof window === "undefined") return;

  const chats = getChatsFromStorage();
  const existingIndex = chats.findIndex((c) => c.id === chat.id);

  if (existingIndex >= 0) {
    chats[existingIndex] = chat;
  } else {
    chats.unshift(chat); // Add to beginning
  }

  // Sort by createdAt descending
  chats.sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime());

  // Serialize dates to ISO strings for localStorage
  const serialized = chats.map((chat) => ({
    ...chat,
    createdAt: chat.createdAt.toISOString(),
  }));
  localStorage.setItem(CHATS_STORAGE_KEY, JSON.stringify(serialized));
}

/**
 * Get collection name from chatId (matches the format used in upload routes)
 */
function getCollectionNameFromChatId(chatId: string): string {
  return `chat_${chatId}`.toLowerCase().replace(/[^a-z0-9_-]/g, "_");
}

/**
 * Delete a chat from localStorage and its associated vectX collection
 */
export async function deleteChatFromStorage(chatId: string): Promise<void> {
  if (typeof window === "undefined") return;

  const chats = getChatsFromStorage();
  const filtered = chats.filter((c) => c.id !== chatId);
  localStorage.setItem(CHATS_STORAGE_KEY, JSON.stringify(filtered));

  // Also delete messages for this chat
  try {
    const stored = localStorage.getItem(MESSAGES_STORAGE_KEY);
    if (stored) {
      const allMessages = JSON.parse(stored) as StoredMessage[];
      const filteredMessages = allMessages.filter((m) => m.chatId !== chatId);
      localStorage.setItem(MESSAGES_STORAGE_KEY, JSON.stringify(filteredMessages));
    }
  } catch {
    // Ignore errors
  }

  // Delete the associated vectX collection
  try {
    const client = getVectXClient();
    const collectionName = getCollectionNameFromChatId(chatId);
    await client.deleteCollection(collectionName);
    console.log(`Deleted vectX collection: ${collectionName}`);
  } catch (error) {
    // Log but don't fail - collection might not exist
    console.warn(`Failed to delete vectX collection for chat ${chatId}:`, error);
  }
}

/**
 * Delete all chats from localStorage and their associated vectX collections
 */
export async function deleteAllChatsFromStorage(): Promise<void> {
  if (typeof window === "undefined") return;

  // Get all chats before deleting to know which collections to delete
  const chats = getChatsFromStorage();
  
  // Delete all vectX collections for these chats
  try {
    const client = getVectXClient();
    const deletePromises = chats.map(async (chat) => {
      const collectionName = getCollectionNameFromChatId(chat.id);
      try {
        await client.deleteCollection(collectionName);
        console.log(`Deleted vectX collection: ${collectionName}`);
      } catch (error) {
        // Log but continue - collection might not exist
        console.warn(`Failed to delete vectX collection ${collectionName}:`, error);
      }
    });
    
    await Promise.all(deletePromises);
  } catch (error) {
    console.warn("Error deleting vectX collections:", error);
  }

  // Delete from localStorage
  localStorage.removeItem(CHATS_STORAGE_KEY);
  localStorage.removeItem(MESSAGES_STORAGE_KEY);
}

/**
 * Get messages for a chat from localStorage
 */
export function getMessagesFromStorage(chatId?: string): ChatMessage[] {
  if (typeof window === "undefined") {
    return [];
  }

  try {
    const stored = localStorage.getItem(MESSAGES_STORAGE_KEY);
    if (!stored) return [];
    const allMessages = JSON.parse(stored) as StoredMessage[];
    
    if (chatId) {
      return allMessages.filter((m) => m.chatId === chatId).map(({ chatId, ...msg }) => msg);
    }
    return allMessages.map(({ chatId, ...msg }) => msg);
  } catch {
    return [];
  }
}

/**
 * Save messages to localStorage
 * Messages should include chatId when calling this function
 */
export function saveMessagesToStorage(messages: (ChatMessage & { chatId?: string })[]): void {
  if (typeof window === "undefined") return;

  const existingMessages = getMessagesFromStorage() as StoredMessage[];
  const messageMap = new Map<string, StoredMessage>();
  
  // Load existing messages with chatId
  for (const msg of existingMessages) {
    if ('chatId' in msg) {
      messageMap.set(msg.id, msg as StoredMessage);
    }
  }

  // Update or add messages (with chatId)
  for (const message of messages) {
    if (message.chatId) {
      messageMap.set(message.id, message as StoredMessage);
    }
  }

  const allMessages = Array.from(messageMap.values());
  localStorage.setItem(MESSAGES_STORAGE_KEY, JSON.stringify(allMessages));
}

/**
 * Get paginated chats (for compatibility with existing API)
 */
export function getChatsPaginated(options: {
  limit: number;
  startingAfter?: string | null;
  endingBefore?: string | null;
}): { chats: Chat[]; hasMore: boolean } {
  const allChats = getChatsFromStorage();
  let filtered = [...allChats];

  // Apply pagination
  if (options.endingBefore) {
    const index = filtered.findIndex((c) => c.id === options.endingBefore);
    if (index >= 0) {
      filtered = filtered.slice(index + 1);
    }
  } else if (options.startingAfter) {
    const index = filtered.findIndex((c) => c.id === options.startingAfter);
    if (index >= 0) {
      filtered = filtered.slice(0, index);
    }
  }

  const chats = filtered.slice(0, options.limit);
  const hasMore = filtered.length > options.limit;

  return { chats, hasMore };
}

/**
 * Add uploaded file to a chat
 */
export function addFileToChat(chatId: string, file: import("@/lib/types/db").UploadedFile): void {
  if (typeof window === "undefined") return;

  const chats = getChatsFromStorage();
  const chatIndex = chats.findIndex((c) => c.id === chatId);
  
  if (chatIndex >= 0) {
    const chat = chats[chatIndex];
    const uploadedFiles = chat.uploadedFiles || [];
    
    // Check if file already exists (by filename and upload time)
    const existingIndex = uploadedFiles.findIndex(
      (f) => f.filename === file.filename && f.uploadedAt === file.uploadedAt
    );
    
    if (existingIndex >= 0) {
      uploadedFiles[existingIndex] = file;
    } else {
      uploadedFiles.push(file);
    }
    
    chat.uploadedFiles = uploadedFiles;
    saveChatToStorage(chat);
  }
}

/**
 * Get uploaded files for a chat
 */
export function getChatFiles(chatId: string): import("@/lib/types/db").UploadedFile[] {
  const chats = getChatsFromStorage();
  const chat = chats.find((c) => c.id === chatId);
  return chat?.uploadedFiles || [];
}
