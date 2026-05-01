# Slice 13: `ls`, `graph --lens causal`, And Root Map Quality

## Intent

Make the two orientation commands genuinely useful in any repo:

```txt
codemap ls .
codemap graph --lens causal
```

## Scope

Likely files:

```txt
src/map/lenses/ls.rs
src/map/lenses/graph.rs
src/render/*
schemas/ls.schema.json
schemas/graph.schema.json
tests/structural_map/*
```

## Implementation Steps

1. Make `ls .` show current-level surfaces:
   - packages;
   - domains;
   - runtime containers;
   - contract containers;
   - proof containers;
   - generated/vendor hidden groups;
   - scripts/build/CI surfaces.
2. Make `ls <file>` show:
   - file kind;
   - package;
   - symbols;
   - exports;
   - imports;
   - imported_by count;
   - direct proof sensors;
   - unknowns;
   - expand.
3. Make `graph --lens causal` show current-level edges only:
   - package dependencies;
   - domain crossings;
   - runtime-to-package surfaces;
   - contract consumers;
   - proof containers;
   - hidden deeper edges.
4. Ensure graph is not a giant Mermaid or recursive edge dump.

## Acceptance

- Root `ls` is compact and actionable.
- Root graph is current-level causal map, not all imports.
- Exact file `ls` gives enough structure to choose read targets.
- Hidden counts and expand commands are present.
- Generic UI/source files do not dominate root output.

## Load-Bearing Tests

Tests fail if:

- root `ls` includes all fixture files;
- root graph includes all file-level imports;
- exact file `ls` omits symbol/import/export sections;
- hidden groups lack counts;
- graph output has no causal relation evidence.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio ls .
codemap --root /Users/amir/Documents/projects/spritestudio graph --lens causal
codemap --root /Users/amir/Documents/projects/Sillentway-VPN ls .
codemap --root /Users/amir/Documents/projects/Sillentway-VPN graph --lens causal
codemap --root <third-project> ls .
codemap --root <third-project> graph --lens causal
```

Record whether root output is less work than manual `ls` plus package reading.

## Reviewer Checklist

Reviewer checks:

```txt
root current-level only
no recursive file galaxy
graph edges have evidence
exact files remain detailed
output line budget holds
```

## Done When

`ls .` and `graph --lens causal` become reliable first moves in unfamiliar
repos.
