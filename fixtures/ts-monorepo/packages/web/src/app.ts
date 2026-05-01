import type { AuthResponse } from "@fixture/contracts";

export function renderUser(response: AuthResponse): string {
  return response.userId;
}
