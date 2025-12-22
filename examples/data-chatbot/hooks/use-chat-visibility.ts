"use client";

import { useEffect, useMemo, useState } from "react";
import { getChatsFromStorage, saveChatToStorage } from "@/lib/storage/chats";
import type { VisibilityType } from "@/components/visibility-selector";

export function useChatVisibility({
  chatId,
  initialVisibilityType,
}: {
  chatId: string;
  initialVisibilityType: VisibilityType;
}) {
  const [visibilityType, setVisibilityTypeState] = useState<VisibilityType>(initialVisibilityType);

  // Load from localStorage on mount
  useEffect(() => {
    const chats = getChatsFromStorage();
    const chat = chats.find((c) => c.id === chatId);
    if (chat) {
      setVisibilityTypeState(chat.visibility);
    }
  }, [chatId]);

  // Listen for storage changes
  useEffect(() => {
    const handleStorageChange = () => {
      const chats = getChatsFromStorage();
      const chat = chats.find((c) => c.id === chatId);
      if (chat) {
        setVisibilityTypeState(chat.visibility);
      }
    };
    window.addEventListener("storage", handleStorageChange);
    window.addEventListener("chat-updated", handleStorageChange);
    return () => {
      window.removeEventListener("storage", handleStorageChange);
      window.removeEventListener("chat-updated", handleStorageChange);
    };
  }, [chatId]);

  const setVisibilityType = (updatedVisibilityType: VisibilityType) => {
    setVisibilityTypeState(updatedVisibilityType);
    
    // Update in localStorage
    const chats = getChatsFromStorage();
    const chat = chats.find((c) => c.id === chatId);
    if (chat) {
      saveChatToStorage({
        ...chat,
        visibility: updatedVisibilityType,
      });
      // Dispatch event to update other components
      window.dispatchEvent(new Event("chat-updated"));
    }
  };

  return { visibilityType, setVisibilityType };
}
