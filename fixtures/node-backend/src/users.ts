import type { UserDto } from "./contracts/user";

export function loadUser(id: string): UserDto {
  return { id };
}
