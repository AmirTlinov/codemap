# codemap

Read-only project x-ray for AI coding agents.

Give an agent `codemap` before it edits an unfamiliar repo. The agent gets a
small map instead of guessing through `ls`, `rg`, `git status`, manifests,
imports, configs, schemas, scripts, and tests.

## What It Gives The Agent

| Question | Command | What the agent sees |
| --- | --- | --- |
| Where am I? | `codemap ls .` | repo root, branch, dirty state, packages, scripts, configs, tests |
| What is around this file? | `codemap cone <file> --depth 1` | imports, exports, consumers, state/effects, nearby helpers, proof, unknowns |
| What did I change? | `codemap changed` | staged/unstaged/untracked files, changed surface types, links, risks, proof gaps |
| How can this be checked? | `codemap proof changed` | tests, build/check commands, evidence-only surfaces, broad fallbacks, missing proof |

The win: the agent spends less time wandering and is less likely to reimplement
code that already exists nearby.

## Copy-Paste Workflow

Start in a repo:

```bash
codemap ls .
```

Before editing a file or folder:

```bash
codemap cone <path> --depth 1
```

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
| `Coupling` | Deterministic relationships: imports, consumers, proof links. |
| `Risks` | Mechanical facts: conflicts, lockfile drift, generated files, large binaries. Not a safety verdict. |
| `Proof` | What can verify this file or change set. |
| `Unknown` | What `codemap` could not prove statically. |
| `Expand` | Exact next command for deeper detail. |

`Unknown` is useful. It means the tool did not pretend to know.

## Install

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

## Boundaries

`codemap` does not choose the best file, recommend fixes, judge architecture,
prove correctness, use embeddings, use an LLM in the hard path, fetch from the
network, or run project commands unless you explicitly use `proof --run`.

It reports: found, linked, missing, soft, proof surface, unknown.

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
scripts/check-version-bump.sh
cargo run --bin codemap -- doctor
```
