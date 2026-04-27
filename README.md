# ctx

`ctx` is a zero-footprint context-kernel CLI for coding agents.

It is installed once in `PATH`, runs inside any project, keeps generated cache outside the repository by default, and returns a task-specific route instead of a project encyclopedia.

## Product Contract

`ctx` should answer:

- what to read first;
- what not to inspect yet;
- what looks risky;
- what to verify;
- when to widen;
- when to stop.

It should not:

- generate repository files by default;
- require `ctx init` before being useful;
- use embeddings or an LLM in the hard routing path;
- treat `AGENTS.md` or README prose as hard architecture truth;
- run project scripts unless explicitly requested with `--run`.

## Commands

Planned stable commands:

```bash
ctx doctor
ctx status
ctx init
ctx scan
ctx locate --task "fix broken save"
ctx start --task "fix broken save" --path src
ctx impact --changed
ctx verify --changed
ctx verify --changed --run
ctx explain src/lib.rs
ctx graph --lens boundary --format mermaid
ctx boundaries
ctx widen --reason "read-first set did not contain the bug"
```

## Architecture

The global binary owns generic algorithms:

- repo discovery;
- file inventory;
- language adapters;
- graph building;
- task routing;
- impact analysis;
- verification planning;
- boundary checks;
- markdown/json/mermaid rendering.

Project-specific state is optional and local:

```txt
.ctx.yml        # optional semantic anchors
AGENTS.md       # optional tiny bootloader
```

Generated cache belongs outside the repo:

```txt
macOS: ~/Library/Caches/agent-context/
Linux: ~/.cache/agent-context/
```

## Development

```bash
cargo fmt --check
cargo test
cargo run --bin ctx -- doctor
```
