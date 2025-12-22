import { geolocation } from "@vercel/functions";
import {
  convertToModelMessages,
  createUIMessageStream,
  JsonToSseTransformStream,
  smoothStream,
  stepCountIs,
  streamText,
} from "ai";
import { after } from "next/server";
import {
  createResumableStreamContext,
  type ResumableStreamContext,
} from "resumable-stream";
// Auth removed - userId comes from client
import { type RequestHints, type CollectionSchema, systemPrompt } from "@/lib/ai/prompts";
import { getVectXClient } from "@/lib/vectx";
import { getLanguageModel } from "@/lib/ai/providers";
import { createDocument } from "@/lib/ai/tools/create-document";
import { getWeather } from "@/lib/ai/tools/get-weather";
import { requestSuggestions } from "@/lib/ai/tools/request-suggestions";
import { updateDocument } from "@/lib/ai/tools/update-document";
// vectX data query tools
import { importData, listDataCollections, exploreData } from "@/lib/ai/tools/import-data";
import { findSimilar } from "@/lib/ai/tools/find-similar";
import { 
  vectorSearch, 
  textSearch, 
  getFacets, 
  recommend, 
  filterRecords, 
  countRecords, 
  deleteRecords, 
  getRecord 
} from "@/lib/ai/tools/vector-search";

// Debug wrapper for tools
const wrapToolWithLogging = (name: string, tool: any): any => {
  const originalExecute = tool.execute;
  if (originalExecute) {
    return {
      ...tool,
      execute: async (...args: any[]) => {
        console.warn(`\n========================================`);
        console.warn(`[vectX TOOL CALL] ${name}`);
        console.warn(`Input: ${JSON.stringify(args[0], null, 2)}`);
        console.warn(`========================================`);
        const result = await originalExecute(...args);
        console.warn(`[vectX TOOL RESULT] ${name}: ${result.success ? 'SUCCESS' : 'FAILED'}`);
        if (!result.success && result.error) {
          console.warn(`Error: ${result.error}`);
        }
        return result;
      },
    };
  }
  return tool;
};

// Wrap vectX tools with logging
const loggedListDataCollections = wrapToolWithLogging('listDataCollections', listDataCollections);
const loggedFindSimilar = wrapToolWithLogging('findSimilar', findSimilar);
const loggedExploreData = wrapToolWithLogging('exploreData', exploreData);
const loggedImportData = wrapToolWithLogging('importData', importData);
// Vector search tools
const loggedVectorSearch = wrapToolWithLogging('vectorSearch', vectorSearch);
const loggedTextSearch = wrapToolWithLogging('textSearch', textSearch);
const loggedGetFacets = wrapToolWithLogging('getFacets', getFacets);
const loggedRecommend = wrapToolWithLogging('recommend', recommend);
const loggedFilterRecords = wrapToolWithLogging('filterRecords', filterRecords);
const loggedCountRecords = wrapToolWithLogging('countRecords', countRecords);
const loggedDeleteRecords = wrapToolWithLogging('deleteRecords', deleteRecords);
const loggedGetRecord = wrapToolWithLogging('getRecord', getRecord);
import { isProductionEnvironment } from "@/lib/constants";
// PostgreSQL removed - using localStorage on client side
import { ChatSDKError } from "@/lib/errors";
import type { ChatMessage } from "@/lib/types";
import { convertToUIMessages, generateUUID } from "@/lib/utils";
import { generateTitleFromUserMessage } from "../../actions";
import { type PostRequestBody, postRequestBodySchema } from "./schema";

export const maxDuration = 60;

let globalStreamContext: ResumableStreamContext | null = null;

export function getStreamContext() {
  if (!globalStreamContext) {
    try {
      globalStreamContext = createResumableStreamContext({
        waitUntil: after,
      });
    } catch (error: any) {
      if (error.message.includes("REDIS_URL")) {
        console.log(
          " > Resumable streams are disabled due to missing REDIS_URL"
        );
      } else {
        console.error(error);
      }
    }
  }

  return globalStreamContext;
}

export async function POST(request: Request) {
  let requestBody: PostRequestBody;

  try {
    const json = await request.json();
    requestBody = postRequestBodySchema.parse(json);
  } catch (_) {
    return new ChatSDKError("bad_request:api").toResponse();
  }

  try {
    const { id, message, messages, selectedChatModel, selectedVisibilityType, existingMessages } =
      requestBody;

    // Check if this is a tool approval flow (all messages sent)
    const isToolApprovalFlow = Boolean(messages);

    // Client provides existing messages from localStorage
    // For new chats, client sends just the new message
    const uiMessages: ChatMessage[] = isToolApprovalFlow
      ? (messages as ChatMessage[])
      : existingMessages && existingMessages.length > 0
        ? [...(existingMessages as ChatMessage[]), message as ChatMessage]
        : message
          ? [message as ChatMessage]
          : [];

    // Generate title for new chats (client will save it)
    let titlePromise: Promise<string> | null = null;
    if (message?.role === "user" && (!existingMessages || existingMessages.length === 0)) {
      titlePromise = generateTitleFromUserMessage({ message });
    }

    const { longitude, latitude, city, country } = geolocation(request);

    const requestHints: RequestHints = {
      longitude,
      latitude,
      city,
      country,
    };

    // Client handles all persistence in localStorage
    // No database operations needed

    const stream = createUIMessageStream({
      // Pass original messages for tool approval continuation
      originalMessages: isToolApprovalFlow ? uiMessages : undefined,
      execute: async ({ writer: dataStream }) => {
        // Handle title generation in parallel
        // Client will save the title to localStorage
        if (titlePromise) {
          titlePromise.then((title) => {
            dataStream.write({ type: "data-chat-title", data: title });
          });
        }

        const isReasoningModel =
          selectedChatModel.includes("reasoning") ||
          selectedChatModel.includes("thinking");

        // Fetch available collections to include schema in system prompt
        let collections: CollectionSchema[] = [];
        try {
          const client = getVectXClient();
          const isConnected = await client.healthCheck();
          if (isConnected) {
            const collectionNames = await client.listCollections();
            for (const name of collectionNames) {
              const info = await client.getCollection(name);
              if (info) {
                // Get sample data to infer field types
                const fields: Record<string, { type: string }> = {};
                try {
                  const { points } = await client.scrollPoints(name, { limit: 1 });
                  if (points.length > 0 && points[0].payload) {
                    for (const [fieldName, value] of Object.entries(points[0].payload)) {
                      if (fieldName.startsWith('_')) continue; // Skip metadata fields
                      fields[fieldName] = {
                        type: typeof value === 'number' ? 'number' : typeof value === 'boolean' ? 'boolean' : 'text',
                      };
                    }
                  }
                } catch (e) {
                  // Ignore errors
                }
                collections.push({ name, rowCount: info.points_count || 0, fields });
              }
            }
          }
        } catch (e) {
          // Silently continue without schema if vectX not available
          console.warn("Could not fetch vectX collections for system prompt:", e);
        }

        const result = streamText({
          model: getLanguageModel(selectedChatModel),
          system: systemPrompt({ selectedChatModel, requestHints, collections }),
          messages: await convertToModelMessages(uiMessages),
          stopWhen: stepCountIs(5),
          experimental_activeTools: isReasoningModel
            ? []
            : [
                "getWeather",
                "createDocument",
                "updateDocument",
                "requestSuggestions",
                // vectX data query tools
                "importData",
                "listDataCollections",
                "exploreData",
                "findSimilar",
                // Vector search tools
                "vectorSearch",
                "textSearch",
                "getFacets",
                "recommend",
                "filterRecords",
                "countRecords",
                "deleteRecords",
                "getRecord",
              ],
          experimental_transform: isReasoningModel
            ? undefined
            : smoothStream({ chunking: "word" }),
          providerOptions: isReasoningModel
            ? {
                anthropic: {
                  thinking: { type: "enabled", budgetTokens: 10_000 },
                },
              }
            : undefined,
          tools: {
            getWeather,
            createDocument: createDocument({ session: null, dataStream }),
            updateDocument: updateDocument({ session: null, dataStream }),
            requestSuggestions: requestSuggestions({
              session: null,
              dataStream,
            }),
            // vectX data query tools (with logging)
            importData: loggedImportData,
            listDataCollections: loggedListDataCollections,
            exploreData: loggedExploreData,
            findSimilar: loggedFindSimilar,
            // Vector search tools
            vectorSearch: loggedVectorSearch,
            textSearch: loggedTextSearch,
            getFacets: loggedGetFacets,
            recommend: loggedRecommend,
            filterRecords: loggedFilterRecords,
            countRecords: loggedCountRecords,
            deleteRecords: loggedDeleteRecords,
            getRecord: loggedGetRecord,
          },
          experimental_telemetry: {
            isEnabled: isProductionEnvironment,
            functionId: "stream-text",
          },
        });

        result.consumeStream();

        dataStream.merge(
          result.toUIMessageStream({
            sendReasoning: true,
          })
        );
      },
      generateId: generateUUID,
      onFinish: async ({ messages: finishedMessages }) => {
        // Client handles all persistence in localStorage
        // No database operations needed
      },
      onError: () => {
        return "Oops, an error occurred!";
      },
    });

    // Resumable streams disabled (no Redis) - just return the stream
    return new Response(stream.pipeThrough(new JsonToSseTransformStream()));
  } catch (error) {
    const vercelId = request.headers.get("x-vercel-id");

    if (error instanceof ChatSDKError) {
      return error.toResponse();
    }

    // Check for Vercel AI Gateway credit card error
    if (
      error instanceof Error &&
      error.message?.includes(
        "AI Gateway requires a valid credit card on file to service requests"
      )
    ) {
      return new ChatSDKError("bad_request:activate_gateway").toResponse();
    }

    console.error("Unhandled error in chat API:", error, { vercelId });
    return new ChatSDKError("offline:chat").toResponse();
  }
}

export async function DELETE(request: Request) {
  // Client handles deletion in localStorage
  // This endpoint is kept for compatibility but does nothing
  return Response.json({ success: true }, { status: 200 });
}
