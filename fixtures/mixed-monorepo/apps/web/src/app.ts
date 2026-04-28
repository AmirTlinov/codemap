import { renderFrame } from "@ctx-fixture/renderer";
import { refreshSession } from "@ctx-fixture/auth";

export function appTick(token: string): string {
  return `${renderFrame(32)}:${refreshSession(token)}`;
}
