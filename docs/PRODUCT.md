# Product Shape

`ctx` minimizes uncertainty for the next coding action.

It never tries to maximize context. It returns the smallest useful task-specific route an agent can act on now.

## Invariants

- one external binary in `PATH`;
- zero project writes by default;
- external cache by default;
- no required `init`;
- no required `AGENTS.md`;
- no generated repo maps;
- no project script execution without `--run`;
- no LLM in the hard routing path;
- optional `.ctx.yml` only for semantic anchors code cannot reveal.

## Agent Output

A task capsule must answer:

- what to read first;
- what not to read yet;
- what is dangerous;
- what source-of-truth files may be involved;
- what to verify;
- when to widen;
- when to stop.

Hard output budgets:

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

`ctx start --task "<user task>" --path "$PWD"`
```

Nested `AGENTS.md` files are local instruction surfaces. They are not project-root markers and they are not hard architecture truth for `ctx`.

## Stop Rule

Agents should stop after minimal verification passes unless:

- a public boundary changed;
- source-of-truth or DTO/schema changed;
- a test failure points outside predicted impact;
- confidence drops below medium;
- an unclassified or generated file participates;
- the read-first set did not contain the cause.

## Distribution Contract

The installed binary must be enough to operate the agent contract:

- `ctx schema capsule`, `ctx schema impact`, `ctx schema verify`, and `ctx schema anchors` print the bundled schemas without loading a project;
- output schemas are versioned with `schema_version`, and `.ctx.yml` anchors are versioned with `version: 1`;
- release checks must prove the crate contains `schemas/`, `fixtures/`, and the end-to-end workflow test.

## Anchor Contract

`.ctx.yml` is allowed to raise confidence only when it is internally valid:

- config version must be `version: 1`;
- unknown fields are rejected instead of ignored;
- exact concept and route file references must exist;
- forbidden boundaries must include `from`, `to`, and `reason`;
- task routes must declare either `match` terms or `read_first`;
- `ctx schema anchors` exposes the structural contract for external validators;
- routing fails closed when semantic anchors are invalid.
