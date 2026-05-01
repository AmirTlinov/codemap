# Slice 17: Proof-Map And Proof Command Safety

## Intent

Make proof a sensor map, not a broad fallback command dump.

`proof-map` answers:

```txt
what tests/e2e/contract checks observe this area?
what important surfaces lack direct sensors?
```

`proof` answers:

```txt
what should I run to prove this slice?
```

## Scope

Likely files:

```txt
src/map/lenses/proof_map.rs
src/map/lenses/proof.rs
src/repo/proof*
src/render/proof*
schemas/proof-map.schema.json
schemas/proof.schema.json
tests/fixtures/*
```

## Sensor Types

Track:

```txt
direct_import
symbol_reference
route_e2e
contract_test
schema_test
snapshot_fixture
name_match
scope_container
package_command
root_fallback_command
ci_job
```

Non-proof:

```txt
README
markdown docs
comments
placeholder scripts
empty test commands
```

## Implementation Steps

1. Build `ProofSurface` facts from test files, imports, route visits, scripts,
   and manifests.
2. Add proof coverage links to important surfaces:
   - changed files;
   - exported contracts;
   - routes;
   - package boundaries;
   - runtime entrypoints.
3. Add `missing_direct` only for important surfaces, not every private helper.
4. Deduplicate commands by package and command string.
5. Add line locations where deterministic:
   - test import line;
   - test function line;
   - symbol reference line;
   - route visit line;
   - snapshot/fixture reference line.
6. Keep raw sensor dumps behind an explicit `--raw-sensors` mode; root and
   changed views should show grouped containers by default.
7. Visually separate hard/direct sensors from medium name/token/container
   sensors.
8. Prefer:
   - adjacent/importing tests;
   - package-local command;
   - focused e2e;
   - root-wide fallback last.
9. Classify command safety:
   - test;
   - typecheck;
   - build;
   - e2e_needs_server;
   - docker;
   - migration;
   - deploy;
   - unknown_or_mutating.
10. Keep `proof --run` fail-closed:
   - refuse placeholder scripts;
   - refuse deploy/migration/unknown mutating commands by default;
   - show exact commands before running;
   - no repo writes by default beyond tool/test side effects.

## Acceptance

- Root `proof-map .` shows proof containers, not every test file.
- `proof-map <scope>` shows direct/indirect/e2e/contract sensors.
- `proof --changed` uses structural changes, not task keywords.
- Broad fallback is clearly marked.
- Direct/import/symbol/e2e/contract/snapshot/name/container/fallback proof is
  visibly separated.
- Direct proof locations usually have line numbers.
- Markdown/docs are never proof sensors.
- Placeholder-only commands are refused with `--run`.
- Raw sensors require `--raw-sensors`.

## Load-Bearing Tests

Tests fail if:

- README is counted as proof;
- internal private helper without test creates noisy missing_direct;
- route e2e visit is not linked to route surface;
- direct import proof lacks a line when the import line exists;
- medium name-match proof renders like hard direct proof;
- raw sensors leak into default root output;
- package-local test command is outranked by root fallback;
- proof executes without `--run`;
- placeholder command runs;
- migration/deploy/unknown command runs by default.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio proof-map .
codemap --root /Users/amir/Documents/projects/spritestudio proof --changed
codemap --root /Users/amir/Documents/projects/Sillentway-VPN proof-map .
codemap --root /Users/amir/Documents/projects/Sillentway-VPN proof --changed
codemap --root <third-project> proof-map .
codemap --root <third-project> proof --changed
```

Record whether it finds better proof than manual package script reading.

## Reviewer Checklist

Reviewer checks:

```txt
real sensors only
root boundedness
missing_direct not noisy
commands deduped
--run safety
proof taxonomy and line locations
proof from structural inputs
```

## Done When

An agent can ask "how do I prove this?" and get a compact, trustworthy sensor
map plus focused command plan.
