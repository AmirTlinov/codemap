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

