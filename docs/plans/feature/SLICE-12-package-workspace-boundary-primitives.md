# Slice 12: Package, Workspace Graph, And Boundary Primitives

## Intent

Make package/domain relationships first-class facts used by `ls`, `impact`,
`boundary-map`, `contract`, and `proof`.

## Scope

Likely files:

```txt
src/repo/packages.rs
src/repo/*
src/map/*
src/model/*
schemas/*
tests/fixtures/*
```

## Implementation Steps

1. Detect workspaces:
   - pnpm/yarn/npm workspaces;
   - Cargo workspace;
   - Go modules/workspaces;
   - Python packages/pyproject;
   - Swift Package if feasible.
2. Model package surfaces:
   - manifest path;
   - package name;
   - root path;
   - scripts/bin;
   - public exports;
   - dependencies/dev dependencies.
3. Build package dependency edges from manifests.
4. Build cross-package import edges from resolved imports.
5. Distinguish:
   - runtime dependency;
   - dev/test dependency;
   - workspace/internal dependency;
   - external package dependency.
6. Add initial boundary primitives:
   - package crossing;
   - domain crossing;
   - test-only crossing;
   - public-boundary file.

## Acceptance

- Root map shows packages/workspaces as primary surfaces.
- Cross-package edges exist without reading task prompts.
- Test-only crossings are separated from runtime crossings.
- Package manifest facts are hard evidence.
- Missing manifests do not create fake package boundaries.

## Load-Bearing Tests

Fixtures must include:

- pnpm monorepo;
- Cargo workspace;
- Python package;
- Go module;
- cross-package import;
- test-only cross-package import.

Tests fail if package graph is inferred only from directory names when manifests
exist.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio ls .
codemap --root /Users/amir/Documents/projects/Sillentway-VPN ls .
codemap --root <third-project> graph --lens causal
```

Record whether package/domain boundaries became clearer.

## Reviewer Checklist

Reviewer checks:

```txt
manifest evidence is hard
cross-package edges are real
test-only crossings separated
external deps do not flood root map
boundary primitives reusable
```

## Done When

Package and workspace structure becomes part of the shared map, not per-lens
heuristics.

