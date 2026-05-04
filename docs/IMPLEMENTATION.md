# Implementation Notes

The Rust implementation is intentionally small and map-first:

```txt
repo discovery -> inventory -> imports/packages/tests -> structural map reports
```

There is no task router. Commands either inspect an exact anchor, a directory level, or a changed-file set.

## Current Modules

```txt
src/main.rs
src/cli.rs
src/cli/*
src/model.rs
src/repo.rs
src/repo/*
src/map.rs
src/map/*
src/render.rs
src/render/*
src/cache.rs
```

Keep new implementation under the existing owner folders. Do not create a second router/search layer beside the structural map engine.

## Implemented Surfaces

- `codemap doctor` and `codemap status`;
- external cache with `CODEMAP_CACHE_DIR` and `CODEMAP_NO_CACHE`;
- git-root-first repo resolution with non-git fallback;
- fresh local `MapPrelude` from one `git status --porcelain=v2 --branch -z`
  snapshot source over cacheable structural report bodies;
- file inventory through git tracked/untracked files or ignored filesystem scan;
- common build/cache/vendor ignores;
- lightweight surface hint classification;
- lightweight symbol extraction for JS/TS, Rust, Python, and Go;
- UI/test-facing surface phrase/token extraction from selectors, test ids, aria labels, and routes;
- JS/TS, Rust, Python, and Go import extraction;
- JS/TS relative import, workspace package import, package entrypoint, and simple `tsconfig.json` alias resolution;
- Go local module import resolution;
- Python src-layout import resolution;
- SwiftPM `Package.swift` package detection and local `.package(path:)` dependency resolution;
- reverse import graph;
- package/workspace detection for package.json, pnpm, Cargo, Go, Python, SwiftPM, Make, and just surfaces;
- domain discovery from common workspace folders and explicit `.codemap.yml`;
- root/directory `ls` surfaces with bounded domain/package/script/test map;
- file `ls` with symbols, exports, imports, incoming count, adjacent tests, and next command;
- `ls .` and `changed` Boundary Facts for instruction files, repo-local guard files, and protected-looking paths;
- `cone` with X-Ray Card role, inputs, outputs, state, side effects, consumers, structural flow, nearby surfaces, proof buckets, unknowns, plus outgoing/incoming/proof/contract/boundary links;
- first-class edge evidence locations and typed unknowns;
- structural `impact` by changed anchors, direct consumers, cross-boundary consumers, contract links, and proof edges;
- `diff-map` for map-level changed structural lines, exported symbol surfaces, and new unknowns;
- `contract` for exported/schema/package/public surfaces and their consumers/proof;
- `runtime` for deterministic entrypoints, Next file-convention routes, static JS/Python/Go route registrations, scripts, env references, workers/jobs, CI, typed runtime unknowns, and proof;
- structural `proof` from adjacent/importing tests and package-local commands;
- `proof changed` coverage summary for runnable, evidence-only, setup/support, soft-only, and missing direct proof buckets;
- `proof-map` for direct/indirect/e2e/contract proof surfaces and typed blind spots around a scope or diff;
- `delete` for deletion blockers, dynamic-reference blind spots, and cleanup hints without safety claims;
- `boundary-map` as read-only package/domain crossing map separate from boundary checks;
- `flow` as bounded structural steps, side-effect surfaces, and unknown stops;
- `siblings` and `place` for local structural conventions and route/service/test triplets without semantic ranking;
- boundary checks from explicit `.codemap.yml` forbidden rules plus resolved imports/package edges;
- anchor validation with resolved domain/concept/boundary/verification details;
- graph lenses for causal, impact, proof, and boundaries;
- schema printing without loading a project or writing cache;
- optional `codemap init --agents`, `--print`, and `--write-minimal`.

## Data Rules

Hard evidence:

- resolved imports;
- reverse imports;
- package manifests;
- exact file route conventions;
- static JS/Python/Go route registrations;
- tests that import an anchor;
- explicit semantic anchors;
- git changed-file inputs;
- local git branch/head/worktree state from porcelain v2;
- schema/config file identification.

Soft evidence:

- file names;
- UI/test-facing string surfaces such as selectors, test ids, aria labels, and routes;
- nearby test names;
- directory naming hints;
- script names.

Soft evidence may explain hidden or secondary surfaces. It must not become a ranked task route.

Typed unknowns are facts too. Dynamic imports, dynamic env lookups, composed route strings, and raw SQL are reported as unresolved structural gaps instead of being converted into guessed edges.

## Output Budgets

- `ls` and `cone` default limit: 20 structural items per section;
- `impact` default limit: 30 structural items;
- `proof` default limit: 12 proof surfaces;
- graph lens default limit: 12 nodes;
- root `ls .` must remain a top-level map;
- hidden counts must be explicit when output is capped.

## Safety Rules

- no target-repository writes except explicit `codemap init --agents` or `codemap init --write-minimal`;
- `proof` runs commands only with `--run`;
- placeholder commands must not run;
- schema commands must not touch repo cache;
- `.codemap.yml` parse/validation errors fail closed for map commands;
- symlinks outside the repo are not followed by default.

## Tests

`tests/structural_map.rs` protects the current product contract:

- help exposes only map-first commands;
- root `ls .` is a bounded domain/package map;
- file `ls` and `cone` expose symbols, edges, proof, and boundaries;
- `impact` and `proof` are structural without extra flags;
- e2e proof links can use shared exact selector/test-id surfaces and static/dynamic Next route visits in root or nested monorepo app layouts, not only test path names;
- e2e proof can follow spec -> test support/page object -> source anchor import chains;
- Python test files can receive file-level `pytest <path>` proof commands even without a package manifest;
- schema manifest has no removed task-router contracts;
- schema printing is side-effect free.

Add new tests when a new structural edge type becomes product-relevant. Do not add tests that preserve removed router behavior.
