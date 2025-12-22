"use client";

import { useEffect } from "react";
import { usePathname } from "next/navigation";
import { initialArtifactData, useArtifact } from "@/hooks/use-artifact";
import { getChatsFromStorage, saveChatToStorage } from "@/lib/storage/chats";
import { artifactDefinitions } from "./artifact";
import { useDataStream } from "./data-stream-provider";

export function DataStreamHandler() {
  const { dataStream, setDataStream } = useDataStream();
  const pathname = usePathname();
  // Extract chat ID from pathname (e.g., "/chat/123" -> "123")
  const chatId = pathname?.startsWith("/chat/") ? pathname.split("/")[2] : undefined;

  const { artifact, setArtifact, setMetadata } = useArtifact();

  useEffect(() => {
    if (!dataStream?.length) {
      return;
    }

    const newDeltas = dataStream.slice();
    setDataStream([]);

    for (const delta of newDeltas) {
      // Handle chat title updates
      if (delta.type === "data-chat-title" && chatId) {
        // Save title to localStorage
        const chats = getChatsFromStorage();
        const chat = chats.find((c) => c.id === chatId);
        if (chat) {
          saveChatToStorage({
            ...chat,
            title: delta.data,
          });
          // Dispatch event to update sidebar
          window.dispatchEvent(new Event("chat-updated"));
        }
        continue;
      }
      const artifactDefinition = artifactDefinitions.find(
        (currentArtifactDefinition) =>
          currentArtifactDefinition.kind === artifact.kind
      );

      if (artifactDefinition?.onStreamPart) {
        artifactDefinition.onStreamPart({
          streamPart: delta,
          setArtifact,
          setMetadata,
        });
      }

      setArtifact((draftArtifact) => {
        if (!draftArtifact) {
          return { ...initialArtifactData, status: "streaming" };
        }

        switch (delta.type) {
          case "data-id":
            return {
              ...draftArtifact,
              documentId: delta.data,
              status: "streaming",
            };

          case "data-title":
            return {
              ...draftArtifact,
              title: delta.data,
              status: "streaming",
            };

          case "data-kind":
            return {
              ...draftArtifact,
              kind: delta.data,
              status: "streaming",
            };

          case "data-clear":
            return {
              ...draftArtifact,
              content: "",
              status: "streaming",
            };

          case "data-finish":
            return {
              ...draftArtifact,
              status: "idle",
            };

          default:
            return draftArtifact;
        }
      });
    }
  }, [dataStream, setArtifact, setMetadata, artifact, setDataStream, chatId]);

  return null;
}
