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
- `ctx anchors validate`
- root and nested `.ctx.yml` semantic anchor loading
- absolute `--path` and `--files` arguments normalized to the owning repo
- safe `ctx init --write-minimal`: creates requested domain directories, refuses writes outside the repo, and writes no fake placeholder concepts
- invalid `.ctx.yml` files are reported by `ctx anchors validate` and block routing commands instead of being silently ignored
- `ctx verify --run` fails closed when the plan contains only a non-runnable placeholder
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
- absolute start paths and file arguments work from outside the repo;
- `ctx init --write-minimal` writes a valid skeletal `.ctx.yml` and refuses absolute paths outside the repo.
- invalid semantic anchors block `start`/`impact`/`verify` until fixed, while `ctx anchors validate` stays available for diagnosis.
- `verify --run` returns non-zero when no concrete command can be inferred.
- explicit forbidden boundary edges have regression coverage.

## Next Useful Slices

1. Add golden fixtures for mixed monorepos and workspace package detection.
2. Add public-boundary and DTO/schema impact rules with stronger tests.
3. Split JS/TS, Rust, Python, and Go adapters once their extraction logic grows.
4. Publish stable JSON schemas for task capsules and impact reports.
5. Replace deprecated `serde_yaml` before distribution hardening.
