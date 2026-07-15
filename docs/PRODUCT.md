# Product Shape

`codemap` gives coding agents a current structural map of the code they are about to touch.

It does not maximize context. It minimizes blind navigation.

This document owns the stable released product shape and public CLI contract. The
[flagship contract](../контракт-спецификация.md) owns the active S00–S17 delivery ledger, target
behavior, and closure receipts. A target behavior becomes part of this stable
product contract only after its owning slice closes.

```txt
codemap = ls + xref + cone + impact + proof for code
```

## Invariants

- one external binary in `PATH`;
- zero target-repository writes by default;
- external cache by default;
- no required init;
- no required `AGENTS.md`;
- no generated architecture documents;
- no project script execution without `--run`;
- no LLM, embeddings, ranking engine, or task router in the hard path;
- exact paths are anchors;
- use the narrowest known anchor; root orientation is only for an unknown scope;
- root `codemap ls .` shows a bounded domain/package map, not every file;
- deeper views require an explicit scope, file, depth, or changed-file input;
- optional `.codemap.yml` supplies only hard semantic anchors code cannot reveal.

## Daily Surface

Primary daily commands:

```bash
codemap ls [scope]
codemap where <symbol>
codemap cone <anchor>
codemap changed
codemap proof <anchor|changed>
```

Global `--brief` (or `CODEMAP_BRIEF=1`) collapses the repo prelude to one line and
drops the repeated provenance disclaimers, for token-tight agent loops. The default
output already omits cache telemetry (state/strategy/location); it stays in `--json`,
`doctor`, and `status` (or `CODEMAP_CACHE_TELEMETRY=1` for debugging).

`changed --since <token>` and `proof changed --since <token>` diff against a prior
agent snapshot. The `Map Snapshot` line prints `snapshot=<token>`; pass it back to
see only what changed since then. A `--since` that is not a known snapshot token is
treated as a git ref; an unknown token fails open to the full worktree set.

Focused lenses remain public and supported, but they are deep map targets:
`runtime`, `contract`, `flow`, `boundary-map`, `siblings`, `place`, `delete`,
`diff-map`, `impact`, `proof-map`, and `graph`.

`doctor`, `status`, `files`, `schema`, `bootstrap`, `init`, `anchors`, and
`boundaries` are diagnostics or setup surfaces, not primary map commands.

## MapPrelude

Primary map outputs may include a fresh local-only repo/worktree prelude. The
prelude is read-only, non-network, non-actionable, and not cached as structural
map truth.

JSON reports also carry a live top-level `build_identity`. Daily commands record
the running executable, version, cache/schema formats, and source provenance but
mark binary hashing as `not_requested`; `doctor` and `status` compute the SHA-256
and compare the running executable with the `codemap` resolved from `PATH`.

It may say:

- branch/head/upstream/ahead/behind from local git refs;
- worktree counts;
- remote URL display;
- local remote refs currentness unknown;
- no network used.

It must not:

- fetch;
- pull;
- prune;
- call `ls-remote`;
- claim the remote is current;
- recommend actions;
- mark operations safe or unsafe.

## Surfaces

`codemap ls <file-or-dir>` shows what exists at that level: file symbols, package/domain surfaces, imports, incoming counts, tests, hidden generic counts, Boundary Facts at the repo root, and the next useful map command.

`codemap cone <anchor>` shows a bounded structural cone around one anchor: an X-Ray Card with role, inputs, outputs, state, side effects, consumers, structural flow, nearby implemented surfaces, verification buckets, unknowns, plus outgoing imports, incoming consumers, verification edges, contracts, boundaries, hidden counts, and expand commands.

`codemap impact --changed|--files` clusters changed anchors by structural blast radius. It is edge-first: reverse imports, package consumers, contract/schema/public surfaces, and verification candidates.

`codemap diff-map --changed|--files` shows map-level changes: structural import/export lines, changed exported symbol surfaces, and new unknowns. It does not print textual diff.

`codemap changed` is the daily after-edit map. It uses stable readable sections: Worktree, Boundary Facts, Surface Hints, Coupling, Risks, Observed, Links, Proof, Unknown, Hidden, and expand targets without running commands. `Proof` is the stable compatibility section name for verification surfaces.

`codemap contract <anchor>` shows exported/schema/package/public surfaces, producers, consumers, cross-package consumers, and verification edges.

`codemap where <symbol>` is a deterministic locator for every exact definition of a
symbol name across the indexed map, with consumers and exact `cone file#symbol`
expand commands. A single definition renders the full cone X-Ray. It is a lookup,
not search: definitions are enumerated by path, never ranked, with no "best file".
A not-found query may show soft substring name matches, explicitly not an answer.

`codemap runtime <scope>` shows deterministic runtime surfaces: entrypoints, file-convention routes, static framework route registrations, scripts, env references, workers/jobs, CI, nearby verification surfaces, and typed blind spots such as dynamic route strings or env keys. At the root scope it still surfaces nested routes (the high-signal, low-volume category) as a bounded top-N instead of hiding them, while keeping high-volume nested entrypoints behind the scoped expand.

`codemap proof <anchor|changed>` returns the smallest verification surface map it can justify. It prefers adjacent/importing tests and package-local command surfaces before broad fallbacks. A `Most-Direct Commands` section lists commands with a direct structural link to the changed files as a fact, not a sufficiency verdict or recommendation. It never runs by default.

`codemap proof-map <scope>|--changed` shows observed verification surfaces
around an area using the shared evidence taxonomy: runnable, direct linked,
mediated linked, soft surface matches, setup/support surfaces, missing direct
links for important surfaces, and runnable command surfaces. The command name
is historical compatibility, not a correctness verdict.

`codemap delete <anchor>` shows deletion blockers and mechanical cleanup hints from references, reexports, package exports, tests, and runtime refs. It must not say “safe to delete”.

`codemap boundary-map <scope>` is a read-only map of actual package/domain crossings. `codemap boundaries` remains the explicit rule checker.

`codemap flow <anchor>` shows bounded structural steps and deterministic side-effect surfaces. It must stop at unknowns instead of claiming full callgraph or dataflow.

`codemap siblings <scope>` and `codemap place <scope> --kind <kind>` show local structural conventions from same directory/kind/verification patterns, including route/service/test triplets when deterministic names and file surfaces expose them. They are not semantic search or ranking.

`codemap graph` is a small lens renderer for humans. It is not the primary product surface and must stay bounded.

## What Must Not Reappear

- task capsules;
- `read_first`;
- guessed canonical ownership;
- global confidence as answer trust;
- query-based route ranking;
- hidden LLM or embedding inference;
- generated `ARCHITECTURE.md` / large generated `AGENTS.md` maps;
- broad project scans as a default answer.

## Agent Rule

An agent chooses one entry using the narrowest anchor already present in the task:

```bash
codemap where <exact-symbol>
codemap cone <file-or-file#symbol> --depth 1
codemap ls <file-or-directory>
```

Only when the relevant scope is unknown should it begin at the current level:

```bash
codemap ls .
```

After edits:

```bash
codemap changed
codemap proof changed
```

Expand only when structural evidence requires it: empty cone, public/package/schema boundary, missing direct verification surface, or failure outside the predicted impact.

## Anchor Contract

`.codemap.yml` is allowed because some architecture truth cannot be inferred from code alone. It may declare:

- domains;
- concepts and their exact files;
- invariants;
- forbidden boundaries and recovery paths;
- verification defaults.

It must not declare task routes, ranked file routes, prose architecture maps, or generated summaries.
