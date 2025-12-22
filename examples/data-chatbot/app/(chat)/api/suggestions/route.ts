import { getSuggestionsByDocumentId } from "@/lib/db/queries-stub";
import { ChatSDKError } from "@/lib/errors";

export async function GET(request: Request) {
  const { searchParams } = new URL(request.url);
  const documentId = searchParams.get("documentId");

  if (!documentId) {
    return new ChatSDKError(
      "bad_request:api",
      "Parameter documentId is required."
    ).toResponse();
  }

  // No auth - allow access to all suggestions
  const suggestions = await getSuggestionsByDocumentId({
    documentId,
  });

  return Response.json(suggestions, { status: 200 });
}
