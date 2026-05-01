# Slice 25: Performance, Path Stability, And Cognitive Regression Gates

## Intent

Prevent the tool from becoming slow, ritualistic, or noisy as capabilities grow.

This slice makes speed and cognitive cost executable gates. It also protects
stable path normalization so fast output does not become stale, duplicated, or
randomly ordered output.

## Scope

Likely files:

```txt
scripts/perf-smoke.sh
scripts/cognitive-smoke.sh
tests/perf*
tests/golden*
src/render/*
src/cache/*
src/repo/path*
src/repo/resolver*
```

## Implementation Steps

1. Add perf smoke script with representative commands:
   - `ls .`;
   - `ls <file>`;
   - `cone <file>`;
   - `changed`;
   - `proof --changed`;
   - `runtime .`.
2. Add fixture timing tests where stable enough.
3. Add output budget tests:
   - root line count;
   - repeated path prefix count;
   - hidden-without-expand forbidden;
   - max proof command duplicates;
   - max unknown examples per group.
4. Add `doctor` performance diagnostics:
   - cold/warm;
   - cache hit;
   - scan/load/query/render timings;
   - hottest extraction group if known.
5. Add path normalization and stability probes for:
   - symlinks;
   - pnpm/workspace symlinks;
   - case-insensitive filesystems;
   - Unicode paths;
   - paths with spaces;
   - nested git repos/submodules;
   - path aliases such as tsconfig paths.
6. Add deterministic ordering policy:
   - structural class;
   - path;
   - symbol line;
   - evidence strength.
7. Add regression threshold policy:
   - hard fail for fixture explosions;
   - warning for local machine timing variance;
   - live dogfood required for subjective speed.

## Acceptance

- Slowdown is visible.
- The gate policy explicitly checks the targets from `PLAN.md`:
  - small warm `ls`/`cone`: under 200ms;
  - medium warm `ls`/`cone`: under 500ms;
  - large warm `ls`/`cone`: under 1s;
  - medium warm `changed`/`proof`: under 1s;
  - large warm `changed`/`proof`: under 2s;
  - cold small repo scan: under 1s;
  - cold medium repo scan: under 5s.
- Root output cannot accidentally become a file dump.
- Repeated path spam is testable.
- Proof command duplicates are testable.
- `doctor` explains performance enough to debug.
- Same repo and same facts produce stable ordering.
- Symlink/case/path-alias normalization does not create duplicate anchors.

## Load-Bearing Tests

Tests fail if:

- root golden exceeds line budget;
- output repeats same path prefix beyond budget;
- hidden appears without expand;
- proof repeats same command many times;
- fixture warm query exceeds hard threshold where deterministic;
- repeated runs produce different ordering without fact changes;
- symlink/case/path-alias fixtures create duplicate anchors.

## Live Dogfood

Run:

```bash
scripts/perf-smoke.sh /Users/amir/Documents/projects/spritestudio
scripts/perf-smoke.sh /Users/amir/Documents/projects/Sillentway-VPN
scripts/perf-smoke.sh <third-project>
```

Record whether speed is good enough to use voluntarily.

## Reviewer Checklist

Reviewer checks:

```txt
gates protect real user intent
no brittle timing-only CI
path normalization correctness
line budgets meaningful
doctor diagnostics useful
tool beats manual habits
```

## Done When

Performance, readability, and path stability are protected by gates, not good
intentions.
