"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { Chat } from "@/components/chat";
import { DataStreamHandler } from "@/components/data-stream-handler";
import { DEFAULT_CHAT_MODEL } from "@/lib/ai/models";
import type { ChatMessage } from "@/lib/types";
import { getMessagesFromStorage } from "@/lib/storage/chats";
import { getChatsFromStorage } from "@/lib/storage/chats";

export default function Page() {
  const params = useParams();
  const id = params.id as string;
  const [initialMessages, setInitialMessages] = useState<ChatMessage[]>([]);
  const [initialChatModel] = useState(DEFAULT_CHAT_MODEL);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Load messages from localStorage
    const messages = getMessagesFromStorage(id);
    setInitialMessages(messages);
    setIsLoading(false);
  }, [id]);

  if (isLoading) {
    return <div className="flex h-dvh" />;
  }

  // Check if chat exists
  const chats = getChatsFromStorage();
  const chat = chats.find((c) => c.id === id);

  if (!chat) {
    // Chat doesn't exist, redirect to home
    window.location.href = "/";
    return null;
  }

  return (
    <>
      <Chat
        autoResume={true}
        id={id}
        initialChatModel={initialChatModel}
        initialMessages={initialMessages}
        initialVisibilityType={chat.visibility}
        isReadonly={false}
      />
      <DataStreamHandler />
    </>
  );
}
