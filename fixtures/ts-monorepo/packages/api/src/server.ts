import type { AuthRequest, AuthResponse } from "@fixture/contracts";

const tenantPrefix = "/tenant";

export function loginHandler(input: AuthRequest): AuthResponse {
  return { userId: input.token };
}

export function attachRoutes(app: { post(path: string, handler: unknown): void; get(path: string, handler: unknown): void }) {
  app.post("/auth/login", loginHandler);
  app.get(tenantPrefix + "/profile", loginHandler);
}
