export interface AuthRequest {
  token: string;
}

export interface AuthResponse {
  userId: string;
}

export function authContractVersion(): number {
  return 1;
}
