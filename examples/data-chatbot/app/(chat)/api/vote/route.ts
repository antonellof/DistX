// Client handles all chat storage via localStorage
// Votes can be stored in localStorage if needed in the future
export async function GET(request: Request) {
  // Return empty votes for now
  return Response.json([], { status: 200 });
}

export async function PATCH(request: Request) {
  // Votes stored client-side if needed
  return new Response("Message voted", { status: 200 });
}
