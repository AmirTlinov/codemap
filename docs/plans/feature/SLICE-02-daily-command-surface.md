# Slice 02: Daily Command Surface And Alias Cleanup

## Intent

Make the tool cheap to remember. An agent should not need a menu of 30 commands
to start useful work.

The daily entrypoints are:

```txt
ls
cone
changed
proof
```

Focused lenses and diagnostics stay available as expand or diagnostic targets,
not as the primary ritual.

## Scope

Likely files:

```txt
src/cli/args.rs
src/cli/run.rs
src/cli/schema_and_roots.rs
src/render/*
README.md
docs/PRODUCT.md
tests/cli_smoke.rs
tests/structural_map/*
```

## Implementation Steps

1. Rework CLI help grouping:
   - four primary map commands visible in `Commands`;
   - focused lenses listed as exact expand targets;
   - diagnostics/schema commands listed outside the primary command list;
   - compat aliases hidden or explicitly compat.
2. Add `codemap changed` as a vertical daily MVP if only `diff-map` exists.
   The MVP may compose existing `diff-map`, `impact`, `proof-map`, and `proof`
   facts, but it must already be useful enough to replace the after-edit ritual.
3. Ensure `changed` explains its relationship to:
   - `diff-map --changed`;
   - `impact --changed`;
   - `proof-map --changed`;
   - `proof changed`.
4. Ensure `proof` defaults to plan-only mode and only executes with `--run`.
5. Make every daily command support:
   - `--format markdown`;
   - `--format json`;
   - `--limit` or shared budget where relevant;
   - `--root`.
6. Add expand suggestions from daily commands to focused lenses.

## Acceptance

- `codemap --help` makes the four primary map commands obvious.
- No command asks for a natural-language task prompt.
- `start`, `locate`, `verify`, `widen`, or old names do not appear as primary
  UX. If kept, they are compatibility only.
- The user can discover deeper lenses from `expand`.
- The daily surface is enough for normal orientation/change/proof work.
- `changed` exists as a real overview command before the deeper structural-event
  hardening in Slice 10.

## Load-Bearing Tests

Tests fail if:

- help order puts focused lenses before daily commands;
- proof runs commands without `--run`;
- `changed` is missing or only prints generic help;
- compat commands emit `read_first` or `source_of_truth`;
- `expand` lacks a next command on daily reports.

## Live Dogfood

Run on each live repo:

```bash
codemap --root <repo> --help
codemap --root <repo> ls .
codemap --root <repo> changed
codemap --root <repo> proof changed
```

Record whether you needed to remember any non-obvious command to continue.

## Reviewer Checklist

Reviewer checks:

```txt
daily surface is small
focused lenses are discoverable
old router ghost is not primary
help text is not noisy
aliases do not hide behavior
```

## Done When

Daily command use is obvious from help and live use, with reviewer PASS.
