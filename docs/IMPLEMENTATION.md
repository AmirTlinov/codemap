# Implementation Notes

The current Rust implementation ports the useful behavior from `ctx-kernel` while preserving the stricter product invariant: no project writes by default and no generated project maps.

V2 shifts the primary product surface from task routing to structural lenses:

```txt
ctx = ls + xref + cone + impact + proof for code
```

The existing task router remains a legacy compatibility layer; structural commands are the primary v2 surface.

## Implemented Slice

- `ctx doctor` / `ctx status`
- external cache under the platform cache directory
- `CTX_CACHE_DIR` and `CTX_NO_CACHE`
- cache artifacts for repo status, inventory, graph edges, and file fingerprints
- cache artifact observability in `ctx status` / `ctx doctor`, including cold/warm/stale/disabled state and fingerprint matching without self-warming first
- git-root-first repo resolution
- explicit global `--root <dir>` uses that directory as the project root instead of climbing to an enclosing git root
- nested `AGENTS.md` detection without treating it as root
- file inventory from `git ls-files -co --exclude-standard`
- filesystem fallback for non-git directories
- common build/cache/vendor ignores
- `ctx files --path` supports both directory scopes and exact indexed file paths
- lightweight role classification
- lightweight JS/TS, Python, Rust, and Go import extraction
- JS/TS import resolution for relative imports, local workspace package imports, package entrypoints, and simple `tsconfig.json` path aliases
- Go import resolution for local workspace module paths
- Python import resolution for local workspace src-layout packages
- reverse import graph
- domain discovery from common workspace folders
- `ctx locate`
- `ctx find`
- `ctx start`
- `ctx impact`
- `ctx verify` print-only by default
- explicit `ctx verify --run`
- `ctx explain`
- `ctx widen`
- `ctx graph`
- `ctx graph --lens verification` renders changed/impacted files, related tests, and verification commands as a bounded verification graph
- `ctx graph --lens impact --changed` and `ctx graph --lens verification --changed` preserve an explicitly empty changed set instead of falling back to unrelated general context
- `ctx graph --lens impact` stays empty when no changed-file input is provided, because impact evidence must come from a diff or explicit file set
- Mermaid output is accepted only by `ctx graph`; route/status/report commands expose Markdown and JSON only
- `ctx graph --lens boundaries` renders forbidden file/package findings as graph edges, not only as loose nodes
- `ctx boundaries`
- `ctx init --print`
- `ctx init --write-minimal`
- `ctx init --agents`
- `ctx bootstrap --global-instruction`
- `ctx schema manifest`
- `ctx schema <status|files|find|ls|cone|proof|capsule|impact|verify|anchors|locate|explain|widen|graph|boundaries>`
- `ctx anchors validate`
- stable JSON schemas for agent-facing route JSON outputs and `.ctx.yml` semantic anchors under `schemas/`
- v2 schema contracts for structural `find`, `ls`, `cone`, `impact --structural`, and `proof` outputs
- stable JSON schemas for `status` and `files` JSON reports
- schema evolution policy in `docs/SCHEMA_POLICY.md`, with exported schema ownership in `schemas/manifest.json`
- bundled schema and schema-manifest printing from the installed binary, without loading a project or writing cache
- root and nested `.ctx.yml` semantic anchor loading
- YAML anchor parsing through maintained `yaml_serde`
- unknown `.ctx.yml` fields are rejected instead of being silently ignored
- absolute path-bearing arguments for `start`, `files`, `graph`, `explain`, `widen --path`, `widen --already`, `init --path`, `impact`, and `verify` select and normalize to the owning repo
- safe `ctx init --write-minimal`: creates requested domain directories, refuses writes outside the repo, and writes no fake placeholder concepts
- invalid `.ctx.yml` files are reported by `ctx anchors validate` and block routing commands instead of being silently ignored
- semantically invalid `.ctx.yml` anchors fail closed before routing when `version: 1`, exact concept files, route read-first files, boundary reasons, or route match/read declarations are missing
- `ctx verify --run` fails closed when the plan contains only a non-runnable placeholder
- `ctx verify --changed` and `ctx verify --files` reuse the same impact traversal and `--depth`/`--limit` controls as `ctx impact`
- legacy `ctx impact --changed`, structural `ctx impact --changed --structural`, and `ctx verify --changed` keep verification empty when the changed set is empty instead of inferring a project-wide check
- `ctx impact --structural` returns v2 structural clusters over changed anchors, with direct consumers, cross-boundary/package consumers, contract risks, proof edges, hidden edge counts, and exact follow-up commands
- `ctx proof <path|--changed|--files>` returns v2 proof plans from structural test evidence and remains print-only unless `--run` is explicit
- `ctx find "<query>"` returns v2 anchor candidates plus separated weak matches and points to `ctx ls` / `ctx cone`, not `ctx start`
- legacy `locate`, file `explain`, `verify`, and path-scoped `widen` Markdown now bridge to structural `find`, `ls`, `proof`, and `cone`; their JSON contracts remain v1 compatibility outputs
- verification planning prefers the single affected package owner when a scoped nested package is clearer than the repository root runner
- `ctx impact` names public-boundary, schema/DTO, source-of-truth, unclassified-source, generated-file, and cross-domain expansion triggers explicitly
- workspace domain discovery from root `package.json` workspaces, `pnpm-workspace.yaml`, Cargo workspace members, `go.work`, and simple Python workspace/member arrays
- boundary checks include explicit forbidden file imports and local JS/Cargo/Go/Python package-manifest dependency edges
- `impact` expands public-boundary/package changes through local package-manifest consumer edges
- Cargo package graph extraction covers inline path dependencies and `[dependencies.<crate>] path = ...` table dependencies
- Go package graph extraction covers `require` plus local/module `replace` edges
- Python package graph extraction covers local `pyproject.toml` path dependencies from common uv/Poetry source tables
- mixed-monorepo, materialized Rust-workspace, materialized Go-workspace, and Python-workspace golden fixtures cover replay/auth routing, bounded impact, JS/TS package/alias imports, language package imports, and package-consumer traversal
- release packaging check script verifies tests, clippy, doctor, bundled schemas, and crate package contents
- `scripts/package-release.sh` builds a target-specific tarball with the `ctx` binary, `README.md`, `LICENSE`, and a sha256 sidecar
- `npm/agent-context-cli` is a thin npm installer wrapper that downloads and verifies those release archives instead of shipping a JS implementation
- `scripts/package-npm-wrapper.sh` builds the packed npm wrapper tarball for GitHub Release assets without bundling a native binary
- `scripts/generate-homebrew-formula.sh` derives a Homebrew formula from release archive checksum sidecars, so formula sha256 values are never guessed before artifacts exist
- `scripts/update-homebrew-tap.sh` updates a local Homebrew tap checkout from a release formula asset, with local commit support but no push
- GitHub Actions runs the release check on Linux and macOS
- version tags matching the Cargo package version and belonging to `main` publish Linux x64 and macOS arm64 archives, the npm wrapper tarball, and a generated Homebrew formula to GitHub Releases after asset verification
- README install guidance separates public Homebrew formula URLs from private-release installs, which should use native archives or the npm wrapper with GitHub API authentication
- printed global agent bootstrap does not advertise a separate `--for-agent` mode; Markdown is already the agent-facing default

## Structural V2 Contract

The v2 implementation must keep these surfaces separate from the legacy router:

- `find` is weak discovery and returns anchor candidates, not a ranked route;
- `ls` explains one exact file or directory surface;
- `cone` traverses real outgoing, incoming, proof, contract, and boundary edges around an exact anchor;
- `impact` clusters changed anchors by structural blast radius;
- `proof` derives checks from cone/impact evidence and remains print-only unless `--run` is explicit.

Do not call `select_read_first` from v2 commands. Do not infer source-of-truth ownership for v2. Per-edge evidence strength replaces global confidence.

Legacy v1 commands may keep their published route fields in JSON. Markdown compatibility wrappers should not send agents back to `ctx start` when a structural command can answer the same question.

## Core Files

```txt
src/main.rs
src/cli.rs
src/model.rs
src/repo.rs
src/route.rs
src/render.rs
src/cache.rs
src/route/graph_lens.rs
```

This is intentionally flatter than the final large-tree design. The next split should happen only when a module becomes hard to change, not as ceremony.

## Behavioral Tests

`tests/cli_smoke.rs` protects the load-bearing contract:

- `doctor` exposes zero-footprint default;
- `status` exposes cold/warm/stale/disabled external cache artifacts without project writes or self-warming;
- `start` routes a persistence task and writes nothing to the target repo;
- nested `AGENTS.md` does not replace git root;
- `verify --changed` prints a plan and does not run scripts without `--run`;
- plain `ctx init` writes nothing;
- domain-local `.ctx.yml` paths such as `src/replay-session.ts` resolve under `domain.path`, not repo root;
- nested domain `.ctx.yml` files are loaded and normalized to repo-relative paths;
- task keywords alone do not create high-confidence capsules without matching files or anchors;
- broad low-confidence general tasks still receive a bounded orientation route instead of an empty read-first set;
- dogfood `ctx` implementation tasks route to context-routing implementation owners before output schemas;
- task keyword/domain overlap matching respects token boundaries, so short keywords such as `ui` do not match inside unrelated words like `build`;
- build/CI tasks route to build surfaces such as manifests, entrypoints, common CI files, task runners, Docker build files, and workflow files instead of returning empty read-first context;
- top-level fixtures/examples/samples are excluded from normal task routing unless the task explicitly asks for them;
- explicit `--path` scopes into support artifact containers can narrow to the nested package whose manifest/path matches the task, and explicit file paths inside nested packages resolve to that package owner;
- support artifact roots such as `fixtures/**` and `examples/**` appear in negative context when they are not task owners;
- default graph lenses exclude support artifacts unless the command is scoped into them;
- absolute path-bearing commands work from outside the repo and normalize paths to repo-relative output;
- exact file scopes for `ctx files --path` return that file instead of an empty directory-style listing;
- `ctx init --write-minimal` writes a valid skeletal `.ctx.yml` and refuses absolute paths outside the repo.
- invalid semantic anchors block `start`/`impact`/`verify` until fixed, while `ctx anchors validate` stays available for diagnosis.
- semantic anchor validation catches missing/unsupported config versions, unknown fields, missing exact concept files, missing route read-first files, missing boundary reasons, and empty route declarations.
- `verify --run` returns non-zero when no concrete command can be inferred.
- empty changed-file reports do not invent project-wide verification commands.
- scoped nested package tasks do not inherit unrelated root verification runners.
- explicit forbidden boundary edges have regression coverage.
- package-manifest boundary edges have regression coverage.
- `verify` recommends checks discovered through impacted files, not only directly changed files, including bounded multi-hop traversal when `--depth` is raised.
- `ctx schema` exposes bundled schemas without repo/cache side effects.
- workspace globs outside the built-in `apps/`, `domains/`, `services/`, `packages/`, and `crates/` shapes become routeable domains.
- impact reports expose specific expansion triggers for schema/DTO, public boundary, and source-of-truth changes.
- schema files are valid JSON, pinned to JSON Schema draft 2020-12, and validate real `status`/`files`/`locate`/`start`/`impact`/`verify`/`explain`/`widen`/`graph`/`boundaries` JSON outputs in tests.
- `tests/schema_policy.rs` guards schema manifest coverage, `ctx schema <kind>` parity, route `schema_version`, anchor `version`, and root strictness.
- `tests/golden_routing.rs` protects mixed-monorepo, Rust-workspace, Go-workspace, and Python-workspace routing quality.
- graph golden tests protect package-boundary graph edges and changed-file verification lenses.
- graph golden tests protect explicitly empty `--changed` graph lenses from inventing fallback context.
- graph golden tests protect `impact` from silently falling back to causal context when no changed input exists.
- smoke tests protect the output-format contract: Mermaid is graph-only, while agent reports remain Markdown/JSON.
- root `--path` bootloader calls still use task routing, while narrower explicit paths constrain the route.
- `tests/e2e_workflow.rs` protects the full agent loop: `start -> impact -> verify -> verify --run -> boundaries -> explain`.

## Remaining Non-Goals Before Registry Publication

1. Publish `agent-context-cli` to npm and/or crates.io after registry credentials are available; until then, GitHub Releases carry the native archives and npm tarball.
2. Add a dedicated Homebrew tap publish workflow only after there is a real tap repository and credentials; until then, use `scripts/update-homebrew-tap.sh` locally.
3. Split large route/repo modules only after the next behavior slice makes them hard to change safely.
4. Add an MCP wrapper only after the CLI contract remains stable through real release use.
