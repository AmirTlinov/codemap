# External Cache Truth

`codemap` keeps reusable facts outside the inspected repository. The cache is an
acceleration surface, never an authority that may override the current files or
Git state.

## Location and contents

The default base directory is the OS cache directory (`~/Library/Caches/codemap`
on macOS and `~/.cache/codemap` on Linux). `CODEMAP_CACHE_DIR` may select another
external base; `CODEMAP_NO_CACHE=1` disables reuse. Each repository/version gets
a hashed directory containing:

- `inventory.json`: per-file extracted structural facts;
- `fingerprints.json`: file identity, content hash, Git HEAD and dirty-set facts;
- `reverse-imports.json`: integrity-checked reverse-import index;
- `graph.json` and `runtime-root.json`: derived structural artifacts;
- bounded lens artifacts for exact `ls`, `cone`, `changed`, and verification paths;
- `snapshots/`: up to 32 `--since` manifests and content-addressed text blobs;
- `quarantine/` and `events/`: corruption receipts and read/write diagnostics.

Snapshot blobs may contain the text of indexed repository files. `codemap` does
not upload or synchronize cache content. Atomic artifact files are created with
owner-only permissions on Unix; access to the cache base remains governed by the
OS account and parent-directory permissions.

## Freshness and failure behavior

The Git status/HEAD probe first selects the changed and removed paths. Unchanged
per-file facts are reused; changed files alone are parsed again. The existing
reverse-import owner is updated only for targets whose source relation changed.
`doctor` exposes both counts and the `cache_probe`, `scan`, `facts`,
`reverse_index`, and `cache_write` phases.

Every artifact is written to a same-directory temporary file, synced, and
renamed atomically. `status.json` is published last as the transaction marker.
An interrupted refresh therefore becomes a miss, not a falsely warm map.
Unreadable, malformed, identity-mismatched, or integrity-mismatched artifacts
are quarantined with a receipt; the command rebuilds from repository truth.
Read, parse, quarantine, and write failures remain visible in `doctor` and
`codemap cache status`.

## Explicit maintenance

The maintenance surface is hidden from the primary map-first help because it is
diagnostic, not a navigation entry:

```bash
codemap cache status
codemap cache status --format json
codemap cache gc
codemap cache clear --yes
```

`status` is read-only. `gc` removes expired quarantine entries, excess diagnostic
events, abandoned temporary files, and quarantines invalid top-level JSON.
`clear --yes` deletes only the current repository/version cache and writes an
external deletion receipt. Mutating cache commands refuse to run when the chosen
cache directory resolves inside the target repository.

Retention is intentionally bounded but conservative: 32 snapshot manifests,
32 recent diagnostics, seven days of quarantine, and project facts until an
explicit clear or OS cache cleanup.

## Performance contract

`scripts/cache-performance-gate.py` creates a deterministic 1,200-file fixture,
changes 100 files, checks snapshot-output parity, verifies exactly 100 rebuilt
file-fact records and only affected reverse targets, and enforces the S12 cold
and warm latency budgets. CI runs the gate in release mode with `--strict`; an
over-budget path is a failing check rather than advisory telemetry. The JSON
receipt records OS/hardware, binary SHA-256, repo scale, cold/warm state, project
phases, command latency, budgets, and semantic parity.
