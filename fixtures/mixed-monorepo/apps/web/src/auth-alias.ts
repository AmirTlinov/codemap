import { refreshSession } from "@auth/session";

export function aliasTick(token: string): string {
  return refreshSession(token);
}
