"use client";

import { useEffect, useState } from "react";
import { Loader2Icon } from "lucide-react";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { getVectXClient } from "@/lib/vectx";

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

interface CollectionInfoDialogProps {
  chatId: string | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function CollectionInfoDialog({ chatId, open, onOpenChange }: CollectionInfoDialogProps) {
  const [selectedCollection, setSelectedCollection] = useState<CollectionDetails | null>(null);
  const [loadingInfo, setLoadingInfo] = useState(false);

  useEffect(() => {
    if (open && chatId) {
      setLoadingInfo(true);
      setSelectedCollection(null);
      
      // Collection name is chat_{chatId}
      const collectionName = `chat_${chatId}`.toLowerCase().replace(/[^a-z0-9_-]/g, "_");
      
      const handleShowInfo = async () => {
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
            toast.error("Collection not found. Upload files to create a collection for this chat.");
            onOpenChange(false);
          }
        } catch (error) {
          console.error("Error fetching collection info:", error);
          toast.error(`Error fetching collection info: ${error instanceof Error ? error.message : "Unknown error"}`);
          onOpenChange(false);
        } finally {
          setLoadingInfo(false);
        }
      };

      handleShowInfo();
    } else {
      setSelectedCollection(null);
    }
  }, [open, chatId, onOpenChange]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Collection Information</DialogTitle>
          <DialogDescription>
            Detailed information about the collection for this chat
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
        ) : (
          <div className="flex flex-col items-center justify-center py-8 text-center">
            <p className="text-sm text-muted-foreground">
              No collection found for this chat
            </p>
            <p className="text-xs text-muted-foreground/70 mt-1">
              Upload files to create a collection
            </p>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
