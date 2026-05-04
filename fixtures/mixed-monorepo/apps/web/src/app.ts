import { renderFrame } from "@codemap-fixture/renderer";
import { refreshSession } from "@codemap-fixture/auth";

export function appTick(token: string): string {
  return `${renderFrame(32)}:${refreshSession(token)}`;
}
