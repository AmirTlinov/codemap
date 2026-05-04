# codemap

Read-only project x-ray for AI coding agents.

Give an agent `codemap` when you want it to understand a repository before
editing code. It replaces a messy first pass of `pwd`, `ls`, `rg`, `git status`,
manifest hunting, import chasing, and test guessing with one compact structural
map.

## What The Agent Gets

| Need | Command | Result |
| --- | --- | --- |
| Understand the repo shape | `codemap ls .` | Packages, folders, scripts, configs, schemas, tests, boundary facts |
| Understand one area or file | `codemap cone <anchor> --depth 1` | Inputs, outputs, consumers, state, effects, nearby helpers, proof, unknowns |
| Understand current edits | `codemap changed` | Live branch/worktree, changed surfaces, coupling, mechanical risks, proof links |
| Understand verification | `codemap proof changed` | Runnable proof surfaces, evidence-only surfaces, broad fallbacks, proof gaps |

The useful part is not "AI magic". The useful part is that the agent sees the
repo wires before it starts reading random files.

## Daily Copy-Paste Flow

At repo start:

```bash
codemap ls .
```

Before touching a file or folder:

```bash
codemap ls <scope-or-file>
codemap cone <scope-or-file> --depth 1
```

After edits:

```bash
codemap changed
codemap proof changed
```

That is the normal workflow.

## What It Shows

- where the agent is: root, cwd, branch, head, upstream, dirty state;
- what exists nearby: files, packages, public surfaces, scripts, configs,
  schemas, routes, tests, receipts;
- what connects: imports, exports, consumers, sibling surfaces, proof surfaces;
- what changed: staged, unstaged, untracked, generated files, lockfiles,
  docs/config/source/test surfaces;
- what can verify it: tests, build/check scripts, proof runners, receipts,
  fallbacks;
- what is missing: no direct test import, unresolved runner, missing proof
  surface, unknown consumer;
- where to go next: exact `Expand` commands.

## How To Read The Output

| Section | Meaning |
| --- | --- |
| `Repo` / `Worktree` | Current local git truth. No network is used. |
| `Surface Hints` | What kind of files exist or changed. Not intent. |
| `Coupling` | Deterministic relationships. Not advice. |
| `Risks` | Mechanical facts such as conflicts, generated files, lockfile drift, large binaries. Not a safety verdict. |
| `Proof` | What can verify this anchor or changed set. |
| `Unknown` | What `codemap` could not prove statically. |
| `Expand` | Exact next command for deeper detail. |

`Unknown` is a feature. It means the tool did not invent certainty.

## Install

From this repository:

```bash
cargo install --path . --locked --force
codemap --version
codemap doctor
```

`codemap` does not write into target repositories by default. Its cache lives
outside the repo:

```txt
macOS:   ~/Library/Caches/codemap/
Linux:   ~/.cache/codemap/
Windows: %LOCALAPPDATA%/codemap/
```

Use `CODEMAP_CACHE_DIR=/path` to move the cache, or `CODEMAP_NO_CACHE=1` to
disable cache writes.

## Focused Drill-Downs

You usually do not need to memorize these. Main commands print them as `Expand`
targets when they matter.

```bash
codemap changed --section proof
codemap changed --section unknown
codemap proof-map <scope>
codemap diff-map --changed
codemap impact --changed
codemap runtime <scope>
codemap flow <anchor>
codemap siblings <scope>
codemap place <scope> --kind test
```

Readable text is the default. JSON exists for integrations:

```bash
codemap proof changed --format json
codemap changed --json
```

## Boundaries

`codemap` does not rank files, recommend fixes, judge architecture, prove
correctness, use embeddings, use an LLM in the hard path, fetch from the network,
or run project commands unless you explicitly use `proof --run`.

It reports facts with provenance: found, linked, missing, soft, proven by this
surface, unknown.

## Optional Project Hints

Zero-config mode works from files, manifests, imports, tests, scripts, schemas,
and git state.

Use `.ctx.yml` only for semantic facts code cannot cheaply reveal: explicit
domains, concepts, forbidden boundaries, or custom proof commands.

```bash
codemap init --print
codemap anchors validate
```

## Development

Before finishing code changes:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
scripts/check-version-bump.sh
cargo run --bin codemap -- doctor
```
