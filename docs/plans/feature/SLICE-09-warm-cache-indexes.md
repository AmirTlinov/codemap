# Slice 09: Warm Cache, Indexes, And Incremental Freshness

## Intent

Make warm `codemap` calls fast enough that agents use them reflexively.

## Closure Boundary

This slice must be correctness-first. Do not claim a warm fast path if the code
still walks and rescans the whole repo before reading cache artifacts.

Acceptable first closure:

```txt
closed: honest cache/timing diagnostics and freshness state in doctor/status
excluded: partial-rescan reuse and full indexed Project reconstruction
```

Full closure:

```txt
closed: safe warm load or partial rescan for selected commands, with stale-cache
tests for changed imports/exports and deleted files
```

Pick the smaller boundary that can be proven without stale facts. A slower true
map is better than a fast false map.

## Required Indexes

Build or normalize indexes for:

```txt
path -> FileInfo
symbol -> anchors
export -> anchors
import target -> importers
package -> files
runtime route -> handler/surface
proof sensor -> covered surfaces
surface kind -> surfaces
unknown kind -> unknowns
```

## Scope

Likely files:

```txt
src/cache/*
src/repo/*
src/map/*
src/model/*
tests/cache*
tests/structural_map/*
```

## Implementation Steps

1. Define cache key:
   - repo root;
   - codemap version;
   - schema/fact version;
   - config fingerprint;
   - relevant lockfile/manifests fingerprint.
2. Add cheap freshness check:
   - mtimes/size for files;
   - git index state where available;
   - config/schema version mismatch.
3. Rescan changed files only where safe.
4. Rebuild dependent indexes incrementally:
   - reverse imports;
   - package edges;
   - proof coverage;
   - runtime routes;
   - unknown groups.
5. If freshness is uncertain, rescan or mark cache suspect in `doctor`.
6. Add timing output for load/fingerprint/query/render.

## Acceptance

- Cache behavior is honest: full scan, warm load, or suspect cache are clearly
  distinguishable in `doctor` / `status`.
- If warm `ls .` or `cone <file>` avoid full repo scan, tests prove stale facts
  are not rendered as fresh.
- If partial rescan is not implemented yet, output does not imply that it is.
- Cache uncertainty is visible and never produces a fresh claim.
- Cache stays external by default and does not write repo files.
- Performance targets in `PLAN.md` are measured.

## Load-Bearing Tests

Tests fail if:

- stale cache hides a changed import/export;
- deleting a file leaves reverse imports pointing to it as fresh;
- config/schema version mismatch reuses old cache silently;
- cache writes inside target repo by default;
- warm timing fields disappear.

## Live Dogfood

Run:

```bash
time codemap --root /Users/amir/Documents/projects/spritestudio ls .
time codemap --root /Users/amir/Documents/projects/spritestudio ls .
time codemap --root /Users/amir/Documents/projects/Sillentway-VPN cone <known-anchor>
```

Record cold/warm delta and whether speed is good enough to beat shell habits.

## Reviewer Checklist

Reviewer checks:

```txt
cache invalidation correctness
external cache default
no stale facts rendered as fresh
indexes support future lenses
timing is not fake precision
```

## Done When

Warm map queries feel instant enough for daily use.

## First Closure

Status: closed within the honest diagnostics boundary.

Implemented:

- `status_report` schema v4 now exposes `cache_strategy`, `files_reused`, and
  `timings`.
- `doctor` / `status` markdown shows cache strategy, reused files, and project
  timing phases.
- Current strategy is explicit: `full_scan` with `files_reused=0`. A warm
  artifact state no longer implies the command loaded facts from cache.
- Tests guard the distinction between warm artifacts and full-scan execution.

Boundary:

```txt
closed: honest cache/timing diagnostics and freshness state in doctor/status.
excluded: warm Project reconstruction, partial-rescan reuse, and stale-cache
guards for reused facts because no reused facts are rendered yet.
```

Proof:

```bash
cargo fmt --check
cargo test --quiet doctor_distinguishes_warm_artifacts_from_full_scan_strategy --test structural_map
cargo test --quiet public_json_reports_validate_against_manifest_schemas --test structural_map
cargo test --quiet scanner_reports_ignored_dirs_and_generated_header_surfaces --test structural_map
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo run --quiet --bin codemap -- doctor
git diff --check
```

Live probe:

```txt
spritestudio doctor: status_report v4, stale artifacts, full_scan, 1123 scanned, 0 reused, total 3116ms
Sillentway-VPN doctor: status_report v4, warm artifacts, full_scan, 785 scanned, 0 reused, total 9613ms
Levelly-1 doctor: status_report v4, stale artifacts, full_scan, 445 scanned, 0 reused, total 2166ms
```

Reviewer: PASS. The reviewed boundary is diagnostics only, not real warm-cache
reuse.
