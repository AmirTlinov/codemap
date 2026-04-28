import { refreshToken } from "./token";

export function refreshSession(token: string): string {
  return refreshToken(token);
}
