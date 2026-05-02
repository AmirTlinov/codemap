# Slice 08: Fast Scanner, Ignore Rules, Generated And Vendor Detection

## Intent

Make cold scans fast and trustworthy by avoiding directories and files that
should not be treated as source ownership.

## Scope

Likely files:

```txt
src/repo/*
src/cache/*
src/model/*
tests/fixtures/*
tests/structural_map/*
```

## Implementation Steps

1. Centralize repository walking policy.
2. Respect:
   - `.gitignore`;
   - common generated/vendor dirs;
   - dependency dirs like `node_modules`, `.venv`, `target`, `dist`, `build`;
   - lockfile-specific package manager artifacts;
   - `.ctx.yml` ignore overrides if already supported.
3. Add generated-file detection by hard evidence:
   - generated header comment;
   - known generated path;
   - manifest/tool output directory;
   - source map pairing.
4. Store generated/vendor status on `FileInfo` or equivalent.
5. Ensure generated files may appear as surfaces but are not treated as owners
   when deterministic source exists.
6. Add scanner timing counters:
   - files visited;
   - files skipped;
   - bytes scanned;
   - ignored groups;
   - generated groups.

## Acceptance

- Cold scans skip obvious heavy directories.
- Generated/vendor files do not dominate root maps.
- Ignored/generated counts are visible in doctor or hidden groups.
- The scanner never silently skips source directories without an explainable
  rule.
- Performance improves without losing source facts.

## Load-Bearing Tests

Fixtures must include:

- ignored dependency directory;
- generated client file;
- generated file with source owner;
- vendor directory with source-like files;
- root map where generated files are hidden as a group.

Tests fail if generated/vendor files become primary owner surfaces.

## Live Dogfood

Run cold and warm:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio doctor
codemap --root /Users/amir/Documents/projects/Sillentway-VPN doctor
codemap --root <third-project> ls .
```

Record skipped counts and whether any real source disappeared.

## Reviewer Checklist

Reviewer checks:

```txt
no hidden source loss
ignore rules are explainable
generated ownership is fail-closed
scanner counters are honest
performance gain is real
```

## Done When

Scanning is faster and less noisy without pretending generated artifacts are
the source of truth.

## Closure

Status: closed within boundary.

Implemented:

- Repository walking policy now records scanner stats while keeping common
  ignored dirs out of the indexed file map.
- Config discovery uses visible candidates, so `.ctx.yml` files inside ignored
  build/dependency dirs cannot become semantic anchor errors.
- `doctor` / `status --format json` now expose `scanner` counters and grouped
  ignored/generated facts through `status_report` schema v3.
- Ignored groups count unique ignored roots, not a mixed directory + tracked
  file total.
- Generated path conventions and hard generated header comments mark files with
  the `generated` role while keeping them visible as explicit surfaces.

Boundary:

```txt
closed: fast scanner policy, ignore/vendor/build pruning, generated header
detection, scanner counters, status schema v3, and load-bearing tests.
excluded: generated source-owner tracing, source maps, codegen config ownership,
and cache partial-rescan work.
```

Proof:

```bash
cargo fmt --check
cargo test --quiet scanner_reports_ignored_dirs_and_generated_header_surfaces --test structural_map
cargo test --quiet public_json_reports_validate_against_manifest_schemas --test structural_map
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo run --quiet --bin codemap -- doctor
cargo run --quiet --bin codemap -- doctor --format json
git diff --check
```

Live result:

```txt
spritestudio doctor: status_report v3, 1123 scanned, 135 skipped
Sillentway-VPN doctor: status_report v3, 785 scanned, 199 skipped
Levelly-1 ls: ls_report v3, bounded root map
```

Reviewer: PASS after fixing ignored-root double counting.
