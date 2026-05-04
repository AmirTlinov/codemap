import { seekFrame } from "@codemap-fixture/replay";

export function renderFrame(timeMs: number): string {
  return `frame:${seekFrame(timeMs)}`;
}
