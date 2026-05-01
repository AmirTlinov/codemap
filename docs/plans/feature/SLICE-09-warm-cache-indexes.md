# Slice 09: Warm Cache, Indexes, And Incremental Freshness

## Intent

Make warm `codemap` calls fast enough that agents use them reflexively.

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

- Warm `ls .` and `cone <file>` avoid full repo scan.
- Dirty repos rescan changed files and affected indexes only.
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

