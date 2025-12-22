"use client";

const USER_ID_KEY = "vectx-user-id";

/**
 * Generates or retrieves a user ID from localStorage
 * The ID persists across browser sessions
 */
export function getOrCreateUserId(): string {
  if (typeof window === "undefined") {
    // Server-side: return a temporary ID (won't be used)
    return "temp-user-id";
  }

  let userId = localStorage.getItem(USER_ID_KEY);

  if (!userId) {
    // Generate a UUID-like ID
    userId = `user-${Date.now()}-${Math.random().toString(36).substring(2, 15)}`;
    localStorage.setItem(USER_ID_KEY, userId);
  }

  return userId;
}

/**
 * Gets the current user ID (doesn't create if missing)
 */
export function getUserId(): string | null {
  if (typeof window === "undefined") {
    return null;
  }
  return localStorage.getItem(USER_ID_KEY);
}
