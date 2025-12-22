import Link from "next/link";
import { memo } from "react";
import type { Chat } from "@/lib/types/db";
import {
  MoreHorizontalIcon,
  TrashIcon,
  FileIcon,
  InfoIcon,
} from "./icons";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import {
  SidebarMenuAction,
  SidebarMenuButton,
  SidebarMenuItem,
} from "./ui/sidebar";

const PureChatItem = ({
  chat,
  isActive,
  onDelete,
  onShowFiles,
  onShowCollectionInfo,
  setOpenMobile,
}: {
  chat: Chat;
  isActive: boolean;
  onDelete: (chatId: string) => void;
  onShowFiles?: (chatId: string) => void;
  onShowCollectionInfo?: (chatId: string) => void;
  setOpenMobile: (open: boolean) => void;
}) => {
  const fileCount = chat.uploadedFiles?.length || 0;

  return (
    <SidebarMenuItem>
      <SidebarMenuButton asChild isActive={isActive}>
        <Link href={`/chat/${chat.id}`} onClick={() => setOpenMobile(false)}>
          <span className="flex-1 truncate">{chat.title}</span>
          {fileCount > 0 && (
            <span className="ml-2 flex items-center gap-1 text-xs text-muted-foreground">
              <FileIcon size={12} />
              <span>{fileCount}</span>
            </span>
          )}
        </Link>
      </SidebarMenuButton>

      <DropdownMenu modal={true}>
        <DropdownMenuTrigger asChild>
          <SidebarMenuAction
            className="mr-0.5 data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
            showOnHover={!isActive}
          >
            <MoreHorizontalIcon />
            <span className="sr-only">More</span>
          </SidebarMenuAction>
        </DropdownMenuTrigger>

        <DropdownMenuContent align="end" side="bottom">
          {onShowCollectionInfo && (
            <DropdownMenuItem
              className="cursor-pointer"
              onSelect={() => onShowCollectionInfo(chat.id)}
            >
              <InfoIcon size={16} className="mr-2" />
              <span>Collection Info</span>
            </DropdownMenuItem>
          )}
          {fileCount > 0 && onShowFiles && (
            <DropdownMenuItem
              className="cursor-pointer"
              onSelect={() => onShowFiles(chat.id)}
            >
              <FileIcon size={16} className="mr-2" />
              <span>Show Files ({fileCount})</span>
            </DropdownMenuItem>
          )}
          <DropdownMenuItem
            className="cursor-pointer text-destructive focus:bg-destructive/15 focus:text-destructive dark:text-red-500"
            onSelect={() => onDelete(chat.id)}
          >
            <TrashIcon />
            <span>Delete</span>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </SidebarMenuItem>
  );
};

export const ChatItem = memo(PureChatItem, (prevProps, nextProps) => {
  // Re-render if active state changes
  if (prevProps.isActive !== nextProps.isActive) {
    return false;
  }
  // Re-render if chat ID changes
  if (prevProps.chat.id !== nextProps.chat.id) {
    return false;
  }
  // Re-render if file count changes
  const prevFileCount = prevProps.chat.uploadedFiles?.length || 0;
  const nextFileCount = nextProps.chat.uploadedFiles?.length || 0;
  if (prevFileCount !== nextFileCount) {
    return false;
  }
  // Re-render if chat title changes
  if (prevProps.chat.title !== nextProps.chat.title) {
    return false;
  }
  // Props are equal, skip re-render
  return true;
});
