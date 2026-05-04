# Product Shape

`codemap` gives coding agents a current structural map of the code they are about to touch.

It does not maximize context. It minimizes blind navigation.

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
- root `codemap ls .` shows a bounded domain/package map, not every file;
- deeper views require an explicit scope, file, depth, or changed-file input;
- optional `.codemap.yml` supplies only hard semantic anchors code cannot reveal.

## Daily Surface

Primary daily commands:

```bash
codemap ls [scope]
codemap cone <anchor>
codemap changed
codemap proof <anchor|changed>
```

Focused lenses remain public and supported, but they are deep map targets:
`runtime`, `contract`, `flow`, `boundary-map`, `siblings`, `place`, `delete`,
`diff-map`, `impact`, `proof-map`, and `graph`.

`doctor`, `status`, `files`, `schema`, `bootstrap`, `init`, `anchors`, and
`boundaries` are diagnostics or setup surfaces, not primary map commands.

## MapPrelude

Primary map outputs may include a fresh local-only repo/worktree prelude. The
prelude is read-only, non-network, non-actionable, and not cached as structural
map truth.

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

`codemap cone <anchor>` shows a bounded structural cone around one anchor: an X-Ray Card with role, inputs, outputs, state, side effects, consumers, structural flow, nearby implemented surfaces, proof buckets, unknowns, plus outgoing imports, incoming consumers, proof edges, contracts, boundaries, hidden counts, and expand commands.

`codemap impact --changed|--files` clusters changed anchors by structural blast radius. It is edge-first: reverse imports, package consumers, contract/schema/public surfaces, and proof candidates.

`codemap diff-map --changed|--files` shows map-level changes: structural import/export lines, changed exported symbol surfaces, and new unknowns. It does not print textual diff.

`codemap changed` is the daily after-edit map. It uses stable readable sections: Worktree, Boundary Facts, Surface Hints, Coupling, Risks, Observed, Links, Proof, Unknown, Hidden, and expand targets without running commands.

`codemap contract <anchor>` shows exported/schema/package/public surfaces, producers, consumers, cross-package consumers, and proof edges.

`codemap runtime <scope>` shows deterministic runtime surfaces: entrypoints, file-convention routes, static framework route registrations, scripts, env references, workers/jobs, CI, nearby proof, and typed blind spots such as dynamic route strings or env keys.

`codemap proof <anchor|changed>` returns the smallest structural proof surfaces it can justify. It prefers adjacent/importing tests and package-local commands before broad fallbacks. It never runs by default.

`codemap proof-map <scope>|--changed` shows observed proof surfaces around
an area using the shared evidence taxonomy: hard, direct evidence, mediated
evidence, soft evidence, setup/support surfaces, missing direct proof evidence
for important surfaces, and commands.

`codemap delete <anchor>` shows deletion blockers and mechanical cleanup hints from references, reexports, package exports, tests, and runtime refs. It must not say “safe to delete”.

`codemap boundary-map <scope>` is a read-only map of actual package/domain crossings. `codemap boundaries` remains the explicit rule checker.

`codemap flow <anchor>` shows bounded structural steps and deterministic side-effect surfaces. It must stop at unknowns instead of claiming full callgraph or dataflow.

`codemap siblings <scope>` and `codemap place <scope> --kind <kind>` show local structural conventions from same directory/kind/proof patterns, including route/service/test triplets when deterministic names and file surfaces expose them. They are not semantic search or ranking.

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

An agent should start wide only at the current level:

```bash
codemap ls .
```

Then it should move to the relevant scope or file:

```bash
codemap ls <scope-or-file>
codemap cone <scope-or-file> --depth 1
```

After edits:

```bash
codemap changed
codemap proof changed
```

Expand only when structural evidence requires it: empty cone, public/package/schema boundary, missing proof surface, or failure outside the predicted impact.

## Anchor Contract

`.codemap.yml` is allowed because some architecture truth cannot be inferred from code alone. It may declare:

- domains;
- concepts and their exact files;
- invariants;
- forbidden boundaries and recovery paths;
- verification defaults.

It must not declare task routes, ranked file routes, prose architecture maps, or generated summaries.
