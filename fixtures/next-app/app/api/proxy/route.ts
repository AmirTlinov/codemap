const routeName = "auth";

export async function GET(): Promise<Response> {
  const plugin = await import("../plugins/" + routeName);
  return Response.json({ loaded: Boolean(plugin) });
}
