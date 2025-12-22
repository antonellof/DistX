// Client handles all chat storage via localStorage
// Resumable streams disabled (no Redis) - return empty response
export async function GET(
  _: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  return new Response(null, { status: 204 });
}
