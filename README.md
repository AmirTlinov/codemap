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

## Local Install

For daily use, install the binary once so agents can call `codemap` from any
repository without paying the `cargo run` tax:

```bash
cargo install --path .
codemap doctor
```

For local dogfooding before installing, point the harness at a built binary:

```bash
cargo build --bin codemap
CODEMAP_BIN=./target/debug/codemap scripts/dogfood-codemap.sh /path/to/repo
```

## Daily Flow

The daily surface is intentionally small:

```bash
codemap ls [scope]
codemap cone <anchor>
codemap changed
codemap proof <anchor|changed>
```

Focused lenses remain available as deterministic drill-down targets, but the
agent should normally discover them through `expand` instead of memorizing a
large ritual.

`codemap doctor` remains available for diagnostics, but it is not part of the
primary map workflow.

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

Run focused map views when the current map points at that deeper spectrum:

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
codemap changed
codemap proof changed
```

`proof` prints a plan by default. It runs commands only with explicit `--run`.

## Primary Commands

```bash
codemap ls [scope]
codemap cone <file-or-dir> [--depth 1]
codemap changed
codemap changed --section observed
codemap changed --section links
codemap changed --section roles
codemap changed --section proof
codemap changed --section unknown
codemap changed --section hidden
codemap proof <file-or-dir>
codemap proof changed
codemap proof changed --section proof
codemap proof changed --section unknown
codemap proof changed --section hidden
codemap proof changed --run
```

`proof --section ...` is display-only; combine `proof` with `--run` only when
you want the full proof plan to execute.

Focused expand targets:

```bash
codemap impact --changed
codemap impact --staged
codemap impact --since main
codemap impact --files path/a.ts,path/b.ts
codemap diff-map --changed
codemap contract <file-or-manifest>
codemap runtime <scope>
codemap proof-map <scope>
codemap delete <file-or-symbol-anchor>
codemap boundary-map <scope>
codemap flow <file-or-symbol-anchor>
codemap siblings <scope>
codemap place <scope> --kind route|service|component|test|contract|lens
codemap graph --lens causal --format mermaid
```

Diagnostics and schema surfaces:

```bash
codemap doctor
codemap status
codemap files [--path <scope>]
codemap boundaries
codemap anchors validate
codemap schema manifest
codemap schema <kind>
codemap bootstrap --global-instruction
codemap init --print
codemap init --write-minimal
codemap init --agents
```

Markdown is the default agent-facing format. Use `CODEMAP_FORMAT=json` or the
hidden `--format json` flag for strict integrations. Mermaid output is limited
to `codemap graph`.

## Agent Integration

One-time global instruction:

```md
For coding tasks, if `codemap` is available in PATH, begin with the small daily structural map surface:

`codemap ls .`
`codemap ls <scope-or-file>`
`codemap cone <scope-or-file> --depth 1`

After edits:

`codemap changed`
`codemap proof changed`

Follow exact expand commands from the output for focused lenses such as `runtime`, `contract`, `flow`, `boundary-map`, `siblings`, `place`, `delete`, `diff-map`, `impact`, `proof-map`, or `graph`. Read code lines after choosing anchors from the map.
```

Optional project bootloader:

```bash
codemap init --agents
```

This writes a tiny `AGENTS.md` that tells agents to call `codemap`. It is not a project map, not architecture documentation, and not a Mermaid graph.

## Optional `.ctx.yml`

Zero-config works from files, manifests, imports, tests, scripts, and git diff. Use `.ctx.yml` only for semantic facts code cannot reliably reveal: explicit domains, concepts, role patterns, forbidden boundaries, and proof commands for custom repos.

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

roles:
  "experiments/receipts/*.json": receipt
  "tools/run_*.py": proof_runner

proof:
  changed:
    - make validate-receipts
    - make doctor
```

Unknown `.ctx.yml` fields are rejected. Invalid anchors fail closed for map commands, while `codemap anchors validate` remains available for diagnosis. `codemap teach` prints a read-only dialect draft from deterministic patterns; it does not write config.

## Development

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo run --bin codemap -- doctor
```
