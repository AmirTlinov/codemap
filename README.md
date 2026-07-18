# codemap

Read-only project x-ray for AI coding agents.

Give an agent `codemap` before it edits an unfamiliar repo. The agent gets a
small map instead of guessing through `ls`, `rg`, `git status`, manifests,
imports, configs, schemas, scripts, and tests.

## What It Gives The Agent

| Question | Command | What the agent sees |
| --- | --- | --- |
| Where is this symbol? | `codemap where <symbol>` | every exact definition; one definition opens as a bounded X-Ray |
| What is around this anchor? | `codemap cone <file-or-file#symbol> --depth 1` | imports, exports, consumers, state/effects, nearby helpers, verification surfaces, unknowns |
| What exists in this known scope? | `codemap ls <file-or-directory>` | only that file or the current directory level, with structural links and hidden counts |
| Where am I in an unfamiliar repo? | `codemap ls .` | a bounded root map of packages, scripts, configs, tests, and owner containers |
| What did I change? | `codemap changed` | staged/unstaged/untracked files, changed surface types, links, risks, verification gaps |
| How can this be checked? | `codemap proof changed` | tests, build/check commands, linked surfaces, broad fallbacks, missing direct links |

The win: the agent spends less time wandering and is less likely to reimplement
code that already exists nearby.

## Measure The Win

Run a read-only benchmark against one or more repos:

```bash
scripts/benchmark-codemap-value.py . /path/to/another/repo
```

It compares visible repo text tokens with the daily `codemap` map
(`ls`, `changed`, `proof changed`, one `cone`) and reports compression,
path/expand/unknown/proof signals, and captured readable outputs.

Both benchmark harnesses resolve one attributable binary in this order:
`--codemap-bin`, `CODEMAP_BIN`, the local `target/debug` or `target/release`
binary, then `PATH`. Their JSON receipts preserve the exact argv, executable,
version, and SHA-256; an identity disagreement fails the run instead of silently
benchmarking another installation.
`--codemap-bin` and `CODEMAP_BIN` accept one direct executable path. Benchmark
orchestration keeps any interpreter wrapper as an argv array instead of a shell string.

This proves context compression and navigation-signal density. It does not
claim that the model became smarter. Run the paired behavioral benchmark for that:

```bash
scripts/benchmark-codemap-ab.py tasks.jsonl \
  --model gpt-5.6-sol --reasoning-effort high --repetitions 3
```

It gives identical tasks and weighted external completeness criteria to isolated
Codex worktrees with and without codemap. The winner is determined by required
outcomes and verified coverage; time and tokens are reported only as resource cost.
Release-level claims use `scripts/benchmark-codemap-flagship.py`: it freezes the
18-task corpus, repository commits, prompts, verifier bytes, arm order, model,
limits, and binary before execution. Two counterbalanced repetitions produce 72
agent runs; acceptance is computed only from external verifier outcomes and cost.
See [`docs/BENCHMARK_AB.md`](docs/BENCHMARK_AB.md) for the task format, validity
rules, artifacts, and claim boundary.

## Copy-Paste Workflow

Choose **one** entry proportional to what the task already names:

```bash
codemap where <exact-symbol>
codemap cone <file-or-file#symbol> --depth 1
codemap ls <directory>
```

Preserve an exact task-named file instead of widening to its parent directory. Use
root orientation only when the relevant scope is unknown:

```bash
codemap ls .
```

Do not pay for root orientation first when an exact anchor is already known.

After edits:

```bash
codemap changed
codemap proof changed
```

Follow the exact `Expand` commands printed by the output when more detail is
needed.

## What The Sections Mean

| Section | Meaning |
| --- | --- |
| `Repo` / `Worktree` | Current local git truth. No network is used. |
| `Surface Hints` | File types and nearby surfaces: source, tests, docs, config, generated, receipts. |
| `Coupling` | Deterministic relationships: imports, consumers, verification links. |
| `Risks` | Mechanical facts: conflicts, lockfile drift, generated files, large binaries. Not a safety verdict. |
| `Proof` | Historical section/command name for verification surfaces around this file or change set. |
| `Unknown` | What `codemap` could not prove statically. |
| `Expand` | Exact next command for deeper detail. |

`Unknown` is useful. It means the tool did not pretend to know.

## Install

Published releases, checksums, supported targets, Homebrew, and compatibility policy are in
[`docs/DISTRIBUTION.md`](docs/DISTRIBUTION.md).

```bash
cargo install --path . --locked --force
codemap --version
codemap doctor
```

`codemap` does not write into target repos by default. Cache is outside the repo:

```txt
macOS:   ~/Library/Caches/codemap/
Linux:   ~/.cache/codemap/
Windows: %LOCALAPPDATA%/codemap/
```

Cache contents, privacy, retention, corruption fallback, and explicit
`cache status|gc|clear` maintenance are documented in
[`docs/CACHE.md`](docs/CACHE.md).

Versioned ecosystem tiers, support cells, and typed unsupported boundaries are
documented in [`docs/ECOSYSTEM_SUPPORT.md`](docs/ECOSYSTEM_SUPPORT.md).
Security reports use the private process in [`SECURITY.md`](SECURITY.md); release changes are
recorded in [`CHANGELOG.md`](CHANGELOG.md).

## Agent Integration

JSON consumers use the stable agent envelope and exit taxonomy from
[`docs/AGENT_PROTOCOL.md`](docs/AGENT_PROTOCOL.md). Discover exact schemas and
the machine contract without loading a repository:

```bash
codemap schema manifest
codemap schema cone
codemap completions zsh
```

Machine expansions are argv arrays under `agent.expands`; unlike readable
`Expand` lines, they already request schema-backed JSON.

## Boundaries

`codemap` does not choose the best file, recommend fixes, judge architecture,
prove correctness, use embeddings, use an LLM in the hard path, fetch from the
network, or run project commands unless you explicitly use `proof --run`.

It reports: found, linked, missing, soft match, verification surface, unknown.

`proof` and `proof-map` are compatibility command names. Their readable output
is a verification surface map, not a correctness verdict or a claim that a test
set is sufficient.

## Deeper Detail

Do not memorize extra commands. Read the `Expand` lines in the output and run
the exact command shown there.

Readable text is the default. JSON exists for integrations, not for daily agent
use.

## Development

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
python3 scripts/check-version-bump.py
cargo run --bin codemap -- doctor
```
