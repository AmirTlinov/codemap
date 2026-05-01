# codemap

`codemap` is a structural code-map CLI for AI coding agents.

It is not a task router, ranking engine, search replacement, embedding index, or generated architecture document. It reads a repository as data and returns a bounded map at the level you ask for:

- what is here;
- which code imports or references it;
- which tests prove it;
- which package/domain boundaries it crosses;
- what changed and what can break.

The goal is to let an agent choose the exact code lines to read without vacuuming the whole repository with broad `rg`/`grep` passes.

## Contract

```txt
codemap = ls + xref + cone + impact + proof for code
```

Default behavior:

```txt
no repository writes
no required init
no required AGENTS.md
no generated repo maps
no project script execution without --run
no network
no LLM or embeddings in the hard path
```

Cache lives outside target repositories:

```txt
macOS:   ~/Library/Caches/codemap/
Linux:   ~/.cache/codemap/
Windows: %LOCALAPPDATA%/codemap/
```

Use `CODEMAP_CACHE_DIR=/path` to override and `CODEMAP_NO_CACHE=1` to disable cache writes.

## Core Flow

At the repository root:

```bash
codemap ls .
```

This returns a bounded top-level map: domains, packages, scripts, test surfaces, and cross-scope edges. It does not print the whole project galaxy.

At a concrete scope or file:

```bash
codemap ls packages/replay
codemap ls packages/replay/src/session.ts
codemap cone packages/replay/src/session.ts --depth 1
```

Use additional lenses when the intent needs a different map spectrum:

```bash
codemap contract packages/replay/src/types.ts
codemap runtime apps/web
codemap flow apps/web/app/api/login/route.ts
codemap siblings packages/replay/src
codemap place packages/replay --kind test
codemap delete packages/replay/src/legacy-session.ts
```

After edits:

```bash
codemap diff-map --changed
codemap impact --changed
codemap proof-map --changed
codemap proof --changed
```

`proof` prints a plan by default. It runs commands only with explicit `--run`.

## Commands

```bash
codemap doctor
codemap status
codemap files [--path <scope>]
codemap ls <file-or-dir>
codemap cone <file-or-dir> [--depth 1]
codemap impact --changed
codemap impact --staged
codemap impact --since main
codemap impact --files path/a.ts,path/b.ts
codemap diff-map --changed
codemap contract <file-or-manifest>
codemap runtime <scope>
codemap proof <file-or-dir>
codemap proof --changed
codemap proof --changed --run
codemap proof-map <scope>
codemap delete <file-or-symbol-anchor>
codemap boundary-map <scope>
codemap flow <file-or-symbol-anchor>
codemap siblings <scope>
codemap place <scope> --kind route|service|component|test|contract
codemap graph --lens causal --format mermaid
codemap boundaries
codemap anchors validate
codemap schema manifest
codemap schema <kind>
codemap bootstrap --global-instruction
codemap init --print
codemap init --write-minimal
codemap init --agents
```

Markdown is the default agent-facing format. Use `--format json` for strict integrations. Mermaid output is limited to `codemap graph`.

## Agent Integration

One-time global instruction:

```md
For coding tasks, if `codemap` is available in PATH, begin with a bounded structural map:

`codemap ls .`
`codemap ls <scope-or-file>`
`codemap cone <scope-or-file> --depth 1`

Use `contract`, `runtime`, `flow`, `siblings`, `place`, or `delete` only when that lens matches the work. They are deterministic map views, not recommendations.

After edits:

`codemap diff-map --changed`
`codemap impact --changed`
`codemap proof-map --changed`
`codemap proof --changed`

Read code lines after choosing anchors from the map. Use `codemap cone <anchor> --depth 2` only when structural edges, public/package/schema boundaries, or proof surfaces require it.
```

Optional project bootloader:

```bash
codemap init --agents
```

This writes a tiny `AGENTS.md` that tells agents to call `codemap`. It is not a project map, not architecture documentation, and not a Mermaid graph.

## Optional `.ctx.yml`

Zero-config works from files, manifests, imports, tests, scripts, and git diff. Use `.ctx.yml` only for semantic facts code cannot reliably reveal: explicit domains, concepts, forbidden boundaries, and verification defaults.

```yaml
version: 1

domain:
  id: replay
  purpose: deterministic replay state and replay-derived DTOs

concepts:
  replay.timeline:
    role: state_model
    files:
      - src/replay-timeline.ts
    invariants:
      - deterministic_for_same_input
      - no_wall_clock_time

boundaries:
  forbidden:
    - from: src/**
      to: ../renderer/src/**
      reason: replay emits DTOs; renderer consumes DTOs
      recovery:
        - extend replay DTO
        - update renderer adapter
        - update contract tests

verification:
  default:
    - pnpm test domains/replay
```

Unknown `.ctx.yml` fields are rejected. Invalid anchors fail closed for map commands, while `codemap anchors validate` remains available for diagnosis.

## Development

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo run --bin codemap -- doctor
```
