import type { LoginDto, LoginResult } from "../../../lib/auth.schema";

export async function POST(request: Request): Promise<Response> {
  const body = (await request.json()) as LoginDto;
  const result: LoginResult = { userId: body.token };
  return Response.json(result);
}
