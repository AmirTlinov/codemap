# ctx

`ctx` is an external context-kernel CLI for AI coding agents.

It does not store context in projects, generate repo maps, require `AGENTS.md`, run project scripts without `--run`, or use an LLM as source of truth. It reads the project as data, keeps a lightweight index in the user cache, and returns a task-specific route:

- what to read first;
- what not to read yet;
- what is risky;
- what to verify;
- when to widen;
- when to stop.

## Product Contract

`ctx` is installed once in `PATH` and works in any project:

```bash
ctx start --task "fix replay jumping to wrong frame after seek"
ctx impact --changed
ctx verify --changed
ctx explain replay.timeline
ctx graph --lens causal
```

Default behavior:

```txt
no project writes
no required init
no required AGENTS.md
no generated artifacts in git
no project script execution without --run
no network
```

Generated cache belongs outside the repo:

```txt
macOS:   ~/Library/Caches/agent-context/
Linux:   ~/.cache/agent-context/
Windows: %LOCALAPPDATA%/agent-context/
```

Use `CTX_CACHE_DIR=/path` to override and `CTX_NO_CACHE=1` to disable cache writes.
`ctx status --format json` reports the observed external cache state (`cold`, `warm`, `stale`, or `disabled`) plus expected cache artifacts and whether their fingerprints match the current project scan. `status` and `doctor` observe cache without warming it first.

## Commands

```bash
ctx doctor
ctx status
ctx files
ctx schema status
ctx schema files
ctx schema capsule
ctx schema anchors
ctx schema graph
ctx locate --task "fix auth token refresh"
ctx start --task "fix broken save" --path src
ctx impact --changed
ctx impact --staged
ctx impact --since main
ctx impact --files /abs/path/to/file.ts
ctx verify --changed
ctx verify --changed --depth 2
ctx verify --changed --run
ctx explain src/lib.rs
ctx widen --reason "read-first set did not contain the cause"
ctx graph --lens causal --format mermaid
ctx boundaries
ctx init --print
ctx init --write-minimal
ctx init --agents
ctx bootstrap --global-instruction
ctx anchors validate
```

Markdown is the default agent-facing format. JSON is available with `--format json`.
Stable JSON schemas live under `schemas/` for agent-facing route outputs, status/files reports, and `.ctx.yml` semantic anchors.
Schema-backed outputs include `schema_version: "1"`.
Installed binaries can print bundled schemas with `ctx schema <kind>`, including `status`, `files`, `capsule`, `impact`, `verify`, `anchors`, `locate`, `explain`, `widen`, `graph`, and `boundaries`.
Schema evolution rules are documented in `docs/SCHEMA_POLICY.md` and guarded by `tests/schema_policy.rs`.

## Agent Integration

Preferred global instruction:

```md
For coding tasks, if `ctx` is available in PATH, begin with:

`ctx start --task "<user task>" --path "$PWD"`

After edits:

`ctx impact --changed`
`ctx verify --changed`

Do not manually scan the repository before using `ctx` unless ctx confidence is low or an expansion trigger fires.
```

Optional project bootloader:

```bash
ctx init --agents
```

This writes a tiny `AGENTS.md` that tells agents to call `ctx`. It is not a project map, not generated architecture documentation, and not a Mermaid graph. Nested `AGENTS.md` files are treated as relevant local instructions, not as project-root markers.

## Install

Release archives contain one `ctx` binary plus `README.md` and `LICENSE`.

```bash
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum -c ctx-v*.tar.gz.sha256
else
  shasum -a 256 -c ctx-v*.tar.gz.sha256
fi
tar -xzf ctx-v*.tar.gz
mkdir -p ~/.local/bin
install ctx-v*/ctx ~/.local/bin/ctx
~/.local/bin/ctx doctor
```

From a source checkout, build the same archive with:

```bash
./scripts/package-release.sh
```

The release script writes `dist/ctx-v<version>-<target>.tar.gz` and a `.sha256` sidecar.
Pushing a version tag from `main`, for example `v0.1.1`, publishes Linux x64 and macOS arm64 archives to GitHub Releases after asset verification.

The npm package is a thin installer wrapper around those same release archives:

```bash
npm install -g agent-context-cli
ctx doctor
```

It downloads and verifies the matching archive during install. It does not make Node a project dependency for repositories where `ctx` is used.
For private GitHub releases, run install with `GH_TOKEN`, `GITHUB_TOKEN`, or `CTX_NPM_GITHUB_TOKEN` available so the wrapper can download assets through the GitHub API.

The release workflow also publishes a generated Homebrew formula asset with checksums derived from the release archives:

```bash
brew install --formula https://github.com/AmirTlinov/ctx/releases/download/v0.1.1/ctx.rb
```

## Optional `.ctx.yml`

Zero-config works from files, manifests, tests, imports, scripts, and git diff. Use `.ctx.yml` only for semantic facts code cannot reliably reveal. A config can live at the repo root or inside a domain directory; nested config paths are treated as domain-local and normalized to repo-relative paths:

`ctx init --write-minimal --path domains/replay` writes a valid skeletal config only. It does not invent placeholder concepts, source-of-truth files, or boundaries.

If a `.ctx.yml` exists but cannot be parsed or contains invalid semantic anchors, routing commands fail closed. Use `ctx anchors validate` to see the exact problem.

```yaml
version: 1

domain:
  id: replay
  purpose: deterministic replay truth and replay-derived DTOs

concepts:
  replay.timeline:
    role: source_of_truth
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

task_routes:
  playback_session:
    match:
      - frame
      - seek
      - cursor
      - playback
    read_first:
      - src/replay-session.ts
      - src/replay-timeline.ts
      - tests/replay-session.test.ts
    verify:
      - pnpm test domains/replay -- session
```

## Development

```bash
cargo fmt --check
cargo test
cargo run --bin ctx -- doctor
./scripts/release-check.sh
```

CI runs the release check on Linux and macOS. Version tags publish Linux x64 and macOS arm64 archives plus a generated Homebrew formula after confirming the tag matches the crate version and belongs to `main`.
