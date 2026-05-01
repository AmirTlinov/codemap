# Slice 03: Unified Fact Model And Constructors

## Intent

Stop each lens from inventing its own data model. Build one shared structural
fact layer that all reports use.

## Required Types

The shared model must include or normalize:

```txt
FileInfo
SymbolInfo
Surface
StructuralEdge
EvidenceLocation
Unknown
PackageInfo
PackageDependency
RuntimeRoute
ProofSurface
GitStructuralEvent
```

## Scope

Likely files:

```txt
src/model.rs
src/model/*
src/map.rs
src/map/lenses/helpers.rs
src/repo/*
src/render/*
schemas/*
tests/structural_map/*
```

## Implementation Steps

1. Add first-class constructors for:
   - `surface(...)`;
   - `edge(...)`;
   - `edge_with_location(...)`;
   - `unknown(...)`;
   - `proof_surface(...)`;
   - `runtime_route(...)`.
2. Replace direct `StructuralEdge { ... }` construction in lenses with helpers.
3. Give each constructor a default `EvidenceStrength`.
4. Add helper variants for common facts:
   - import edge;
   - reverse import edge;
   - package dependency edge;
   - barrel export edge;
   - package export edge;
   - test import edge;
   - e2e route edge;
   - runtime route edge;
   - boundary crossing edge.
5. Add a shared `LensBudget` struct for all bounded reports.
6. Add model tests that assert required fields are never silently omitted.

## Acceptance

- New lenses can be written as fact queries, not private parsers.
- Most edges and surfaces are created through shared helpers.
- The model distinguishes fact, evidence, location, and rendering.
- Existing reports still serialize successfully.
- No v2 lens calls legacy candidate/ranking types.

## Load-Bearing Tests

Tests fail if:

- an edge can be serialized without evidence;
- an important surface lacks evidence strength;
- a typed unknown lacks reason/effect;
- direct edge construction reappears outside allowed model tests;
- a lens creates proof/runtime/contract facts as ad-hoc strings only.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio ls . --format json
codemap --root /Users/amir/Documents/projects/Sillentway-VPN cone . --format json
codemap --root <third-project> ls . --format json
```

Inspect whether JSON shows shared fact shapes instead of per-lens one-offs.

## Reviewer Checklist

Reviewer checks:

```txt
fact model is not renderer-shaped
helpers do not hide fake evidence
legacy ranking/candidate types are not used by structural lenses
schemas match model
```

## Done When

The shared model is the default path for new facts and reports.
