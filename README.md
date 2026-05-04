# codemap

A read-only project x-ray for AI coding agents.

`codemap` helps an agent stop guessing where to look. It shows what exists in a
repository, what connects to what, what changed, what can verify it, and what is
still unknown.

It is useful when the agent would otherwise burn time stitching together `pwd`,
`ls`, `rg`, `git status`, imports, manifests, scripts, configs, schemas, and
tests by hand.

## Why It Helps

- The agent sees the current repo, branch, head, dirty files, and untracked files.
- It can find nearby code surfaces: imports, exports, tests, scripts, configs,
  schemas, routes, receipts, and helpers.
- It can see who consumes a file and what the file consumes.
- It can see proof surfaces and proof gaps before claiming something is tested.
- It gets exact next expand commands instead of a giant context dump.
- `Unknown` stays visible when static structure is not enough.

This usually means fewer blind `rg` passes, less repeated work, and faster first
orientation in an unfamiliar repo.

## What It Is Not

`codemap` is not a recommender, semantic search tool, embedding index, LLM
summary, task router, architecture judge, or correctness proof.

It does not say "best file", "safe change", "good architecture", or "this test
proves correctness".

It says: found, linked, missing, soft, proven by this surface, unknown.

## Install

From this repository:

```bash
cargo install --path . --locked --force
codemap --version
codemap doctor
```

`codemap` writes no files into target repositories by default. Its cache lives
outside the repo:

```txt
macOS:   ~/Library/Caches/codemap/
Linux:   ~/.cache/codemap/
Windows: %LOCALAPPDATA%/codemap/
```

Use `CODEMAP_CACHE_DIR=/path` to move the cache, or `CODEMAP_NO_CACHE=1` to
disable cache writes.

## Daily Flow

Start wide:

```bash
codemap ls .
```

Open the relevant folder or file:

```bash
codemap ls <scope-or-file>
codemap cone <scope-or-file> --depth 1
```

After edits:

```bash
codemap changed
codemap proof changed
```

That is the main workflow.

## Main Commands

| Command | What it answers |
| --- | --- |
| `codemap ls [scope]` | What exists here? |
| `codemap cone <anchor> --depth 1` | What surrounds this file, folder, symbol, manifest, config, or schema? |
| `codemap changed` | What is true in the worktree after edits? |
| `codemap proof <anchor\|changed>` | What can verify this, and what proof is missing? |

`proof` prints by default. It runs commands only with explicit `--run`.

## How To Read The Output

Common sections:

- `Repo` / `Worktree`: current local git truth. No network is used.
- `Surface Hints`: what kind of files changed or exist nearby.
- `Coupling`: deterministic relationships, not advice.
- `Risks`: mechanical facts such as conflicts, generated files, lockfile drift,
  or large binary changes. These are not safety verdicts.
- `Proof`: what can verify the anchor or changed files.
- `Unknown`: where the map cannot prove a relationship.
- `Expand`: exact next commands for deeper detail.

If `codemap` says `Unknown`, that is useful. It means the tool did not invent an
answer.

## Drill Down Only When Needed

Most deeper views are discovered through `Expand` lines in the output:

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

Readable text is the default. JSON exists only for integrations:

```bash
codemap proof changed --format json
codemap changed --json
```

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
