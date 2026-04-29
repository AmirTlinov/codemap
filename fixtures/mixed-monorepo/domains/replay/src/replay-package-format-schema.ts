export interface ReplayPackageFormat {
  version: 1;
  events: Array<{ timeMs: number; frame: number }>;
}

export function parseReplayPackage(input: ReplayPackageFormat): ReplayPackageFormat {
  return input;
}
