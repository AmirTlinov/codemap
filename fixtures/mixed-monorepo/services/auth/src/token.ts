export function refreshToken(token: string): string {
  return `${token}:fresh`;
}
