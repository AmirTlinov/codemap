# Product Shape

`ctx` minimizes uncertainty for the next coding action.

It never tries to maximize context. In v2 the primary contract is structural, not router-first:

```txt
ctx = ls + xref + cone + impact + proof for code
```

The tool should answer:

- what is here;
- what has a real edge to it;
- what can break;
- what proves the slice;
- where an explicit anchor may be needed.

## Invariants

- one external binary in `PATH`;
- zero project writes by default;
- external cache by default;
- no required `init`;
- no required `AGENTS.md`;
- no generated repo maps;
- no project script execution without `--run`;
- no LLM in the hard routing path;
- no embeddings or ranking engine in the hard routing path;
- exact paths are anchors, not query hints;
- optional `.ctx.yml` only for semantic anchors code cannot reveal.

## Primary Structural Output

The v2 primary flow is:

```bash
ctx find "<query>"      # weak discovery, anchor candidates only
ctx ls <path>          # what is here
ctx cone <path>        # incoming/outgoing/proof/contract edges
ctx impact --changed   # changed clusters and blast radius
ctx proof --changed    # proof plan, print-only by default
```

`ctx find` may use query terms to discover anchor candidates. `ctx ls`, `ctx cone`, `ctx impact`, and `ctx proof` must be edge-first and must not call the legacy task router.

V2 outputs must not use:

- `read_first` as the primary answer;
- guessed source-of-truth ownership;
- global confidence as answer trust.

Every primary structural item should carry local evidence: import edge, reverse import edge, test edge, package edge, explicit anchor, or bounded filesystem fact.

## Legacy Router

`ctx start`, `ctx locate`, `ctx explain`, `ctx verify`, `ctx widen`, and existing graph lenses remain supported as v1 compatibility surfaces while v2 lands.

Legacy task capsules may still expose `read_first`, negative context, stop rules, and confidence because that is their published schema. New structural surfaces should not copy those fields.

Fixture/example/sample code is not normal ownership evidence. It should not become the route owner unless the task explicitly asks for that support artifact or the command is scoped into it. When an explicit support-container scope contains nested packages, the task route should narrow to the matching package and keep sibling packages in `do_not_read_yet`.

Hard legacy output budgets:

- `read_first`: max 7 files;
- `do_not_read_yet`: max 8 entries;
- `forbidden_moves`: max 7;
- `invariants`: max 7;
- verification commands: max 3 per tier;
- markdown capsule: target max 150 lines.

## AGENTS.md

`AGENTS.md` is allowed only as a tiny bootloader:

```md
# Agent Bootstrap

For coding tasks in this repository, start with:

`ctx find "<user task>"`
```

Nested `AGENTS.md` files are local instruction surfaces. They are not project-root markers and they are not hard architecture truth for `ctx`.

## Stop Rule

Structural v2 should stop expanding after the smallest proof set passes unless:

- a public boundary changed;
- package, contract, DTO, or schema surface changed;
- a test failure points outside predicted impact;
- an unclassified or generated file participates;
- the cone has no edge that explains the observed failure;
- the user explicitly asks for a wider cross-domain view.

## Distribution Contract

The installed binary must be enough to operate the agent contract:

- `ctx schema <kind>` prints bundled schemas for route outputs and anchors without loading a project;
- `status`, `files`, and structural JSON reports are schema-backed because they are common integration entrypoints;
- legacy route output schemas are versioned with `schema_version: "1"`;
- structural output schemas are versioned with `schema_version: "2"`;
- `.ctx.yml` anchors are versioned with `version: 1`;
- schema evolution is governed by `docs/SCHEMA_POLICY.md` and `schemas/manifest.json`;
- release checks must prove the crate contains `schemas/`, `fixtures/`, and the end-to-end workflow test.
- GitHub Releases must carry enough install artifacts to work without a registry publish: native archives, checksum sidecars, generated Homebrew formula, and the packed npm wrapper.
- Private or restricted GitHub Releases must document authenticated install paths explicitly; plain Homebrew release URLs are only a public-release or public-mirror path.
- Homebrew tap mutation stays outside the release workflow until a real tap remote exists; local tap updates are explicit and never push by default.

## Anchor Contract

`.ctx.yml` is allowed to supply hard semantic anchors only when it is internally valid:

- config version must be `version: 1`;
- unknown fields are rejected instead of ignored;
- exact concept and route file references must exist;
- forbidden boundaries must include `from`, `to`, and `reason`;
- task routes must declare either `match` terms or `read_first`;
- `ctx schema anchors` exposes the structural contract for external validators;
- routing fails closed when semantic anchors are invalid.

`task_routes.read_first` remains a v1 compatibility field. V2 may read explicit anchors and boundaries, but must not turn `.ctx.yml` into generated prose or guessed architecture truth.
