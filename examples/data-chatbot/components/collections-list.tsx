"use client";

import { useEffect, useState } from "react";
import { toast } from "sonner";
import { DatabaseIcon, TrashIcon, Loader2Icon, InfoIcon } from "lucide-react";
import { MoreHorizontalIcon } from "./icons";
import { getVectXClient } from "@/lib/vectx";
import { Button } from "./ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "./ui/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuAction,
  SidebarMenuButton,
} from "./ui/sidebar";

interface CollectionDetails {
  name: string;
  status: string;
  points_count: number;
  segments_count?: number;
  shards_count?: number;
  vectors_config?: {
    size: number;
    distance: string;
  };
}

export function CollectionsList() {
  const [collections, setCollections] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [deleteCollectionName, setDeleteCollectionName] = useState<string | null>(null);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [selectedCollection, setSelectedCollection] = useState<CollectionDetails | null>(null);
  const [showInfoDialog, setShowInfoDialog] = useState(false);
  const [loadingInfo, setLoadingInfo] = useState(false);

  const loadCollections = async () => {
    try {
      const client = getVectXClient();
      const isConnected = await client.healthCheck();
      if (isConnected) {
        const names = await client.listCollections();
        setCollections(names);
      } else {
        setCollections([]);
      }
    } catch (error) {
      console.error("Failed to load collections:", error);
      setCollections([]);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadCollections();
    
    // Refresh collections periodically
    const interval = setInterval(loadCollections, 10000); // Every 10 seconds
    
    // Also listen for custom events (e.g., after upload)
    const handleCollectionUpdate = () => {
      loadCollections();
    };
    window.addEventListener("collection-updated", handleCollectionUpdate);
    
    return () => {
      clearInterval(interval);
      window.removeEventListener("collection-updated", handleCollectionUpdate);
    };
  }, []);

  const handleDelete = async () => {
    if (!deleteCollectionName) return;

    try {
      const client = getVectXClient();
      const success = await client.deleteCollection(deleteCollectionName);
      
      if (success) {
        toast.success(`Collection "${deleteCollectionName}" deleted`);
        setCollections((prev) => prev.filter((name) => name !== deleteCollectionName));
        setShowDeleteDialog(false);
        setDeleteCollectionName(null);
      } else {
        toast.error(`Failed to delete collection "${deleteCollectionName}"`);
      }
    } catch (error) {
      toast.error(`Error deleting collection: ${error instanceof Error ? error.message : "Unknown error"}`);
    }
  };

  const handleShowInfo = async (collectionName: string) => {
    setLoadingInfo(true);
    setShowInfoDialog(true);
    
    try {
      const client = getVectXClient();
      const info = await client.getCollection(collectionName);
      
      if (info) {
        // Extract details from Qdrant collection info
        // Qdrant returns: { result: { status, points_count, segments_count, config: { params: { vectors: { size, distance } } } } }
        const result = (info as any).result || info;
        const config = result.config || {};
        const params = config.params || {};
        const vectors = params.vectors || {};
        
        // Handle both single vector config and named vectors
        let vectorSize = 1536;
        let vectorDistance = "Cosine";
        
        if (typeof vectors === 'object' && vectors !== null) {
          // Could be { size: X, distance: Y } or { default: { size: X, distance: Y } }
          if ('size' in vectors) {
            vectorSize = vectors.size as number;
            vectorDistance = (vectors.distance as string) || "Cosine";
          } else if ('default' in vectors) {
            const defaultVec = vectors.default as any;
            vectorSize = defaultVec?.size || 1536;
            vectorDistance = defaultVec?.distance || "Cosine";
          }
        }
        
        const details: CollectionDetails = {
          name: collectionName,
          status: result.status || "unknown",
          points_count: result.points_count || 0,
          segments_count: result.segments_count,
          shards_count: result.shards_count || 1,
          vectors_config: {
            size: vectorSize,
            distance: vectorDistance,
          },
        };
        setSelectedCollection(details);
      } else {
        toast.error("Failed to fetch collection information");
        setShowInfoDialog(false);
      }
    } catch (error) {
      console.error("Error fetching collection info:", error);
      toast.error(`Error fetching collection info: ${error instanceof Error ? error.message : "Unknown error"}`);
      setShowInfoDialog(false);
    } finally {
      setLoadingInfo(false);
    }
  };

  if (isLoading) {
    return (
      <SidebarGroup>
        <SidebarGroupContent>
          <div className="flex items-center gap-2 px-2 py-1">
            <Loader2Icon className="h-4 w-4 animate-spin text-muted-foreground" />
            <span className="text-sm text-muted-foreground">Loading collections...</span>
          </div>
        </SidebarGroupContent>
      </SidebarGroup>
    );
  }

  if (collections.length === 0) {
    return (
      <SidebarGroup>
        <SidebarGroupContent>
          <div className="px-2 py-1 text-xs text-muted-foreground">
            No collections yet. Upload a CSV to create one.
          </div>
        </SidebarGroupContent>
      </SidebarGroup>
    );
  }

  return (
    <>
      <SidebarGroup>
        <div className="flex items-center gap-2 px-2 py-1">
          <DatabaseIcon className="h-4 w-4 text-muted-foreground" />
          <span className="text-xs font-medium text-sidebar-foreground/70">vectX Collections</span>
        </div>
        <SidebarGroupContent>
          <SidebarMenu className="gap-0.5">
            {collections.map((name) => (
              <SidebarMenuItem key={name}>
                <SidebarMenuButton size="default" className="flex-1" title={name}>
                  <span className="text-sm truncate">{name}</span>
                </SidebarMenuButton>
                <DropdownMenu modal={true}>
                  <DropdownMenuTrigger asChild>
                    <SidebarMenuAction
                      className="mr-0.5 data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
                      showOnHover={true}
                    >
                      <MoreHorizontalIcon size={16} />
                      <span className="sr-only">More</span>
                    </SidebarMenuAction>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end" side="bottom">
                    <DropdownMenuItem
                      className="cursor-pointer"
                      onSelect={() => handleShowInfo(name)}
                    >
                      <InfoIcon className="h-4 w-4 mr-2" />
                      <span>Show Information</span>
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      className="cursor-pointer text-destructive focus:bg-destructive/15 focus:text-destructive dark:text-red-500"
                      onSelect={() => {
                        setDeleteCollectionName(name);
                        setShowDeleteDialog(true);
                      }}
                    >
                      <TrashIcon className="h-4 w-4 mr-2" />
                      <span>Delete</span>
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>

      <AlertDialog
        onOpenChange={setShowDeleteDialog}
        open={showDeleteDialog}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete collection?</AlertDialogTitle>
            <AlertDialogDescription>
              This will permanently delete the collection "{deleteCollectionName}" and all its data.
              This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={handleDelete}>
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      <Dialog open={showInfoDialog} onOpenChange={setShowInfoDialog}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Collection Information</DialogTitle>
            <DialogDescription>
              Detailed information about the collection
            </DialogDescription>
          </DialogHeader>
          
          {loadingInfo ? (
            <div className="flex items-center justify-center py-8">
              <Loader2Icon className="h-6 w-6 animate-spin text-muted-foreground" />
              <span className="ml-2 text-sm text-muted-foreground">Loading...</span>
            </div>
          ) : selectedCollection ? (
            <div className="mt-4">
              <div className="overflow-x-auto">
                <table className="w-full border-collapse">
                  <thead>
                    <tr className="border-b">
                      <th className="text-left py-2 px-4 font-medium">Property</th>
                      <th className="text-left py-2 px-4 font-medium">Value</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr className="border-b">
                      <td className="py-2 px-4 font-medium">Name</td>
                      <td className="py-2 px-4">{selectedCollection.name}</td>
                    </tr>
                    <tr className="border-b">
                      <td className="py-2 px-4 font-medium">Status</td>
                      <td className="py-2 px-4">
                        <span className="inline-flex items-center gap-1">
                          <span className="h-2 w-2 rounded-full bg-green-500"></span>
                          {selectedCollection.status}
                        </span>
                      </td>
                    </tr>
                    <tr className="border-b">
                      <td className="py-2 px-4 font-medium">Points (Approx)</td>
                      <td className="py-2 px-4">{selectedCollection.points_count.toLocaleString()}</td>
                    </tr>
                    <tr className="border-b">
                      <td className="py-2 px-4 font-medium">Segments</td>
                      <td className="py-2 px-4">{selectedCollection.segments_count ?? "N/A"}</td>
                    </tr>
                    <tr className="border-b">
                      <td className="py-2 px-4 font-medium">Shards</td>
                      <td className="py-2 px-4">{selectedCollection.shards_count ?? 1}</td>
                    </tr>
                    <tr className="border-b">
                      <td className="py-2 px-4 font-medium">Vectors Config</td>
                      <td className="py-2 px-4">
                        {selectedCollection.vectors_config ? (
                          <div className="space-y-1">
                            <div>
                              <span className="font-medium">Size:</span> {selectedCollection.vectors_config.size}
                            </div>
                            <div>
                              <span className="font-medium">Distance:</span> {selectedCollection.vectors_config.distance}
                            </div>
                          </div>
                        ) : (
                          <span className="text-muted-foreground">Default</span>
                        )}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          ) : null}
        </DialogContent>
      </Dialog>
    </>
  );
}
