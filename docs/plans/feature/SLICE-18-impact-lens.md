# Slice 18: Impact Lens From Structural Edges

## Intent

Show blast radius from structural facts, not task terms or guessed ownership.

`impact --changed` answers:

```txt
what changed?
who directly consumes it?
what package/domain/runtime/contract/proof surfaces may be affected?
where should I inspect next?
```

## Scope

Likely files:

```txt
src/map/lenses/impact.rs
src/map/lenses/helpers.rs
src/render/impact.rs
schemas/impact.schema.json
tests/structural_map/*
```

## Implementation Steps

1. Use `GitStructuralEvent` as the primary input for `--changed`.
2. Build impact from:
   - reverse imports;
   - package dependencies;
   - contract consumers;
   - runtime references;
   - proof surfaces;
   - boundary crossings.
3. Group impact by structural area:
   - same file/scope;
   - direct consumers;
   - package/domain consumers;
   - public/contract link;
   - runtime risk;
   - proof candidates;
   - unknowns.
4. Remove any source-of-truth terminology.
5. Do not compute global risk score. Use structural reasons.
6. Limit output and add hidden/expand.

## Acceptance

- Impact is explainable from edges.
- No task prompt or semantic query affects impact.
- Contract/public changes are separated.
- Runtime risks are separated.
- Proof candidates are structural.
- Hidden impacted consumers have count and expand.

## Load-Bearing Tests

Tests fail if:

- impact changes when task wording changes;
- reverse importer is omitted;
- cross-package consumer is not grouped;
- package export removal lacks contract link;
- runtime route removal lacks runtime risk;
- output includes source_of_truth/confidence language.

## Live Dogfood

Run on dirty repos:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio impact --changed
codemap --root /Users/amir/Documents/projects/Sillentway-VPN impact --changed
codemap --root <third-project> impact --changed
```

If a repo is clean, create no edits; record clean-state behavior.

## Reviewer Checklist

Reviewer checks:

```txt
structural reasons only
no ranking/confidence
no task terms
bounded output
proof candidates connected to sensors
```

## Done When

Impact explains blast radius without pretending to know project intent.
