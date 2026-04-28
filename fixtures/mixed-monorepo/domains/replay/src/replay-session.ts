import { frameAt } from "./replay-timeline";

export function seekFrame(timeMs: number): number {
  return frameAt(timeMs);
}
