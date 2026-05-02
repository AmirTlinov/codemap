# Slice 14: `cone` Exact Anchors, Directory Aggregation, And Symbol Anchors

## Intent

Make `cone` the agent's answer to "what touches this?" without dumping the
project.

## Anchor Types

Support:

```txt
file path
directory path
path#symbol
package anchor
runtime route anchor where available
contract surface anchor where available
```

## Scope

Likely files:

```txt
src/map/lenses/cone.rs
src/map/lenses/helpers.rs
src/render/*
schemas/cone.schema.json
tests/structural_map/*
```

## Implementation Steps

1. Normalize anchor parsing and exact-path precedence.
2. For file anchors show:
   - outgoing imports;
   - incoming imports;
   - exported symbols;
   - symbol references where deterministic;
   - direct tests/proof sensors;
   - runtime refs;
   - contract refs;
   - unknowns.
3. For directory anchors aggregate:
   - internal surfaces;
   - inbound/outbound edges by current-level child;
   - proof containers;
   - runtime/contract surfaces;
   - hidden deeper edges.
4. For symbol anchors show:
   - declaration location;
   - local references;
   - exported/public status;
   - tests that reference/import owner;
   - unknowns.
5. Add `--depth` semantics:
   - depth 0 = anchor only;
   - depth 1 = direct edges;
   - depth 2 = second-order grouped edges;
   - no unbounded recursion.

## Acceptance

- Exact paths never get re-guessed.
- Directory cone aggregates before file detail.
- Symbol anchors work where supported and fail clearly where not.
- Incoming/outgoing edges have evidence and locations.
- `cone` output remains bounded by default.

## Load-Bearing Tests

Tests fail if:

- exact file anchor is treated as a query;
- directory cone lists every nested file;
- symbol anchor cannot find a supported symbol;
- dynamic reference becomes fake symbol use;
- `--depth` changes are unbounded.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio cone <domain-dir> --depth 1
codemap --root /Users/amir/Documents/projects/spritestudio cone <known-file>
codemap --root /Users/amir/Documents/projects/Sillentway-VPN cone <known-anchor>
```

Record whether `cone` replaces the first round of manual `rg`.

## Reviewer Checklist

Reviewer checks:

```txt
anchor exactness
directory aggregation
symbol support honesty
bounded depth
locations and evidence
```

## Done When

`cone` is the reliable local relationship map for files, dirs, and supported
symbols.

## First Closure Boundary

Do not hold this slice open until every anchor type is perfect. The first
load-bearing boundary is exact symbol-anchor honesty:

```txt
closed: `cone file#MissingSymbol` returns an anchor with kind
`missing_symbol`, emits typed unknown `missing_symbol_anchor`, and leaves
outgoing/incoming/proof/contract/boundary sections empty instead of falling
back to a whole-file cone or a generic unindexed path.
excluded: package anchors, runtime route anchors, contract surface anchors,
and broader directory/symbol depth improvements.
proof: controlled fixture for an indexed file with a missing symbol plus local
gate.
review: PASS.
live: not required; live repos add noise to an exact fail-closed case.
```

Next Slice 14 work should start only from a concrete bad cone behavior, such as
an exact anchor being re-guessed, a directory cone leaking a file galaxy, or a
supported symbol anchor losing evidence.
