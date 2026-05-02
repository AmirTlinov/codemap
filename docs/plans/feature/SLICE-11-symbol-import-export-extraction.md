# Slice 11: Symbol, Import, And Export Extraction Matrix

## Intent

Give exact anchors enough structure that agents can read only needed lines.

## Language Matrix

Support deterministic extraction for:

```txt
TypeScript / TSX
JavaScript / JSX
Rust
Python
Go
Swift where currently feasible
```

## Required Facts

For each supported file where syntax allows:

```txt
symbols with kind and line range
exports
imports
resolved imports
reverse imports
symbol references where deterministic
barrel reexports
module declarations
```

## Scope

Likely files:

```txt
src/repo/*
src/repo/preamble.rs
src/model/*
tests/fixtures/*
tests/structural_map/*
```

## Implementation Steps

1. Define supported symbol kinds:
   - function;
   - class;
   - component;
   - hook;
   - type/interface;
   - struct/enum/trait/impl;
   - method;
   - route handler;
   - test.
2. Improve line range extraction without claiming AST precision if using line
   scanners.
3. Extract exports and reexports with locations.
4. Resolve imports with package/workspace awareness.
5. Record unresolved imports as typed unknowns.
6. Add symbol-anchor grammar:
   - `path#symbol`;
   - route anchors where supported later.
7. Add fixtures per language.

## Acceptance

- `codemap ls <file>` shows meaningful symbols with line ranges.
- Imports/exports/reverse imports remain correct.
- Barrels/reexports are first-class surfaces/edges.
- Unsupported syntax becomes unknown or omitted, not fake symbol data.
- Symbol anchors work where deterministic.

## Load-Bearing Tests

Tests fail if:

- TS component/function/hook ranges are missing;
- Rust `pub use`, `mod`, `impl`, and function symbols disappear;
- Python class/function imports are missed;
- Go exported functions and imports are missed;
- barrel reexports are just rendered strings without edges;
- unresolved import does not produce unknown.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio ls <known-ts-file>
codemap --root /Users/amir/Documents/projects/Sillentway-VPN ls <known-rust-or-swift-file>
codemap --root <third-project> ls <known-file>
```

Record whether line ranges are good enough to read surgically.

## Reviewer Checklist

Reviewer checks:

```txt
line ranges are not decorative
no language overclaim
exports/reexports are modeled
symbol anchors are deterministic
fixtures cover each claimed language
```

## Done When

Exact files become useful maps, not just file metadata.

## First Closure

Status: closed within the unresolved-local-import boundary.

Implemented:

- `FileInfo` now records `unresolved_imports` from the shared import resolver.
- Only local-looking unresolved imports are reported as blind spots:
  - relative or absolute source imports;
  - Rust `crate::`, `self::`, `super::`, and `.rs` include-style specs.
- External package imports are not treated as unresolved local structural facts.
- `cone <file>` emits typed `unresolved_import` unknowns with line provenance
  where the import statement can be located.

Boundary:

```txt
closed: exact file cones no longer silently drop unresolved local imports.
excluded: full base/source-owner import matrix, non-code import resolution,
path-alias repair beyond existing resolver support, and unresolved external
package dependency diagnostics.
```

Proof:

```bash
cargo fmt --check
cargo test --quiet cone_reports_unresolved_local_imports_as_typed_unknowns --test structural_map
cargo test --quiet public_json_reports_validate_against_manifest_schemas --test structural_map
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo run --quiet --bin codemap -- doctor
git diff --check
```

Live decision:

```txt
not required for this boundary; the fixture isolates the false omission better
than ambient live repos, and no broad command orientation behavior changed.
```

Reviewer: PASS.
