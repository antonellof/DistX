import type { NextRequest } from "next/server";

// Client handles all chat storage via localStorage
// This endpoint is kept for compatibility but returns empty data
export async function GET(request: NextRequest) {
  return Response.json({ chats: [], hasMore: false });
}

export async function DELETE() {
  // Client handles deletion in localStorage
  return Response.json({ success: true }, { status: 200 });
}
