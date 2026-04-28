import { seekFrame } from "@ctx-fixture/replay";

export function renderFrame(timeMs: number): string {
  return `frame:${seekFrame(timeMs)}`;
}
