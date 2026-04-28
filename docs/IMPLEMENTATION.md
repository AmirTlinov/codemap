# Implementation Notes

The current Rust implementation ports the useful behavior from `ctx-kernel` while preserving the stricter product invariant: no project writes by default and no generated project maps.

## Implemented Slice

- `ctx doctor` / `ctx status`
- external cache under the platform cache directory
- `CTX_CACHE_DIR` and `CTX_NO_CACHE`
- git-root-first repo resolution
- nested `AGENTS.md` detection without treating it as root
- file inventory from `git ls-files -co --exclude-standard`
- filesystem fallback for non-git directories
- common build/cache/vendor ignores
- lightweight role classification
- lightweight JS/TS, Python, Rust, and Go import extraction
- reverse import graph
- domain discovery from common workspace folders
- `ctx locate`
- `ctx start`
- `ctx impact`
- `ctx verify` print-only by default
- explicit `ctx verify --run`
- `ctx explain`
- `ctx widen`
- `ctx graph`
- `ctx boundaries`
- `ctx init --print`
- `ctx init --write-minimal`
- `ctx init --agents`
- `ctx bootstrap --global-instruction`
- `ctx schema <capsule|impact|verify|anchors|locate|explain|widen|graph|boundaries>`
- `ctx anchors validate`
- stable JSON schemas for agent-facing route JSON outputs and `.ctx.yml` semantic anchors under `schemas/`
- schema evolution policy in `docs/SCHEMA_POLICY.md`, with exported schema ownership in `schemas/manifest.json`
- bundled schema printing from the installed binary, without loading a project or writing cache
- root and nested `.ctx.yml` semantic anchor loading
- YAML anchor parsing through `serde_yml`
- unknown `.ctx.yml` fields are rejected instead of being silently ignored
- absolute `--path` and `--files` arguments normalized to the owning repo
- safe `ctx init --write-minimal`: creates requested domain directories, refuses writes outside the repo, and writes no fake placeholder concepts
- invalid `.ctx.yml` files are reported by `ctx anchors validate` and block routing commands instead of being silently ignored
- semantically invalid `.ctx.yml` anchors fail closed before routing when `version: 1`, exact concept files, route read-first files, boundary reasons, or route match/read declarations are missing
- `ctx verify --run` fails closed when the plan contains only a non-runnable placeholder
- `ctx verify --changed` and `ctx verify --files` reuse the same impact traversal and `--depth`/`--limit` controls as `ctx impact`
- `ctx impact` names public-boundary, schema/DTO, source-of-truth, unclassified-source, generated-file, and cross-domain expansion triggers explicitly
- workspace domain discovery from root `package.json` workspaces, `pnpm-workspace.yaml`, Cargo workspace members, `go.work`, and simple Python workspace/member arrays
- boundary checks include explicit forbidden file imports and local package-manifest dependency edges
- `impact` expands public-boundary/package changes through local package-manifest consumer edges
- mixed-monorepo golden fixtures cover replay/auth routing and bounded replay impact
- release packaging check script verifies tests, clippy, doctor, bundled schemas, and crate package contents
- printed global agent bootstrap does not advertise a separate `--for-agent` mode; Markdown is already the agent-facing default

## Core Files

```txt
src/main.rs
src/cli.rs
src/model.rs
src/repo.rs
src/route.rs
src/render.rs
src/cache.rs
```

This is intentionally flatter than the final large-tree design. The next split should happen only when a module becomes hard to change, not as ceremony.

## Behavioral Tests

`tests/cli_smoke.rs` protects the load-bearing contract:

- `doctor` exposes zero-footprint default;
- `start` routes a persistence task and writes nothing to the target repo;
- nested `AGENTS.md` does not replace git root;
- `verify --changed` prints a plan and does not run scripts without `--run`;
- plain `ctx init` writes nothing;
- domain-local `.ctx.yml` paths such as `src/replay-session.ts` resolve under `domain.path`, not repo root;
- nested domain `.ctx.yml` files are loaded and normalized to repo-relative paths;
- task keywords alone do not create high-confidence capsules without matching files or anchors;
- broad low-confidence general tasks still receive a bounded orientation route instead of an empty read-first set;
- top-level fixtures/examples/samples are excluded from normal task routing unless the task explicitly asks for them;
- explicit `--path` scopes into support artifact containers can narrow to the nested package whose manifest/path matches the task, and explicit file paths inside nested packages resolve to that package owner;
- support artifact roots such as `fixtures/**` and `examples/**` appear in negative context when they are not task owners;
- default graph lenses exclude support artifacts unless the command is scoped into them;
- absolute start paths and file arguments work from outside the repo;
- `ctx init --write-minimal` writes a valid skeletal `.ctx.yml` and refuses absolute paths outside the repo.
- invalid semantic anchors block `start`/`impact`/`verify` until fixed, while `ctx anchors validate` stays available for diagnosis.
- semantic anchor validation catches missing/unsupported config versions, unknown fields, missing exact concept files, missing route read-first files, missing boundary reasons, and empty route declarations.
- `verify --run` returns non-zero when no concrete command can be inferred.
- explicit forbidden boundary edges have regression coverage.
- package-manifest boundary edges have regression coverage.
- `verify` recommends checks discovered through impacted files, not only directly changed files, including bounded multi-hop traversal when `--depth` is raised.
- `ctx schema` exposes bundled schemas without repo/cache side effects.
- workspace globs outside the built-in `apps/`, `domains/`, `services/`, `packages/`, and `crates/` shapes become routeable domains.
- impact reports expose specific expansion triggers for schema/DTO, public boundary, and source-of-truth changes.
- schema files are valid JSON, pinned to JSON Schema draft 2020-12, and validate real `locate`/`start`/`impact`/`verify`/`explain`/`widen`/`graph`/`boundaries` JSON outputs in tests.
- `tests/schema_policy.rs` guards schema manifest coverage, `ctx schema <kind>` parity, route `schema_version`, anchor `version`, and root strictness.
- `tests/golden_routing.rs` protects mixed-monorepo routing quality.
- `tests/e2e_workflow.rs` protects the full agent loop: `start -> impact -> verify -> verify --run -> boundaries -> explain`.

## Next Useful Slices

1. Add deeper per-language adapters once JS/TS, Rust, Python, or Go extraction logic becomes hard to change in-place.
2. Add Homebrew and npm wrapper distribution after the cargo package shape stabilizes.
3. Split large route/repo modules only after the next behavior slice makes them hard to change safely.
