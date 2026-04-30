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
src/model.rs
src/repo.rs
src/map.rs
src/map/graph_lens.rs
src/render.rs
src/cache.rs
```

Keep this flat until a module is hard to change safely.

## Implemented Surfaces

- `codemap doctor` and `codemap status`;
- external cache with `CODEMAP_CACHE_DIR` and `CODEMAP_NO_CACHE`;
- git-root-first repo resolution with non-git fallback;
- file inventory through git tracked/untracked files or ignored filesystem scan;
- common build/cache/vendor ignores;
- lightweight role classification;
- lightweight symbol extraction for JS/TS, Rust, Python, and Go;
- UI/test-facing surface phrase/token extraction from selectors, test ids, aria labels, and routes;
- JS/TS, Rust, Python, and Go import extraction;
- JS/TS relative import, workspace package import, package entrypoint, and simple `tsconfig.json` alias resolution;
- Go local module import resolution;
- Python src-layout import resolution;
- SwiftPM `Package.swift` package detection and local `.package(path:)` dependency resolution;
- reverse import graph;
- package/workspace detection for package.json, pnpm, Cargo, Go, Python, SwiftPM, Make, and just surfaces;
- domain discovery from common workspace folders and explicit `.ctx.yml`;
- root/directory `ls` surfaces with bounded domain/package/script/test map;
- file `ls` with symbols, exports, imports, incoming count, adjacent tests, and next command;
- `cone` with outgoing, incoming, proof, contract, boundary, hidden, unknown, and expand sections;
- structural `impact` by changed anchors, direct consumers, cross-boundary consumers, contract risks, and proof edges;
- structural `proof` from adjacent/importing tests and package-local commands;
- boundary checks from explicit `.ctx.yml` forbidden rules plus resolved imports/package edges;
- anchor validation with resolved domain/concept/boundary/verification details;
- graph lenses for causal, impact, proof, and boundaries;
- schema printing without loading a project or writing cache;
- optional `codemap init --agents`, `--print`, and `--write-minimal`.

## Data Rules

Hard evidence:

- resolved imports;
- reverse imports;
- package manifests;
- tests that import an anchor;
- explicit semantic anchors;
- git changed-file inputs;
- schema/config file roles.

Soft evidence:

- file names;
- UI/test-facing string surfaces such as selectors, test ids, aria labels, and routes;
- nearby test names;
- directory roles;
- script names.

Soft evidence may explain hidden or secondary surfaces. It must not become a ranked task route.

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
- `.ctx.yml` parse/validation errors fail closed for map commands;
- symlinks outside the repo are not followed by default.

## Tests

`tests/structural_map.rs` protects the current product contract:

- help exposes only map-first commands;
- root `ls .` is a bounded domain/package map;
- file `ls` and `cone` expose symbols, edges, proof, and boundaries;
- `impact` and `proof` are structural without extra flags;
- e2e proof links can use shared exact selector/test-id surfaces, not only test path names;
- e2e proof can follow spec -> test support/page object -> source anchor import chains;
- Python test files can receive file-level `pytest <path>` proof commands even without a package manifest;
- schema manifest has no removed task-router contracts;
- schema printing is side-effect free.

Add new tests when a new structural edge type becomes product-relevant. Do not add tests that preserve removed router behavior.
