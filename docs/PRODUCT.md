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
- optional `.ctx.yml` supplies only hard semantic anchors code cannot reveal.

## Surfaces

`codemap ls <file-or-dir>` shows what exists at that level: file symbols, package/domain surfaces, imports, incoming counts, tests, hidden generic counts, and the next useful map command.

`codemap cone <anchor>` shows a bounded structural cone around one anchor: outgoing imports, incoming consumers, proof edges, contracts, boundaries, hidden counts, and unknowns.

`codemap impact --changed|--files` clusters changed anchors by structural blast radius. It is edge-first: reverse imports, package consumers, contract/schema/public surfaces, and proof candidates.

`codemap proof <anchor>|--changed` returns the smallest structural proof surfaces it can justify. It prefers adjacent/importing tests and package-local commands before broad fallbacks. It never runs by default.

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
codemap impact --changed
codemap proof --changed
```

Expand only when structural evidence requires it: empty cone, public/package/schema boundary, missing proof surface, or failure outside the predicted impact.

## Anchor Contract

`.ctx.yml` is allowed because some architecture truth cannot be inferred from code alone. It may declare:

- domains;
- concepts and their exact files;
- invariants;
- forbidden boundaries and recovery paths;
- verification defaults.

It must not declare task routes, ranked file routes, prose architecture maps, or generated summaries.
