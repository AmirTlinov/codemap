# Slice 01: Product Lock, Baseline Inventory, And Invariant Guards

## Intent

Freeze the product direction before new code can drift back into router,
ranking, or broad search behavior.

This slice answers:

```txt
what commands exist today?
what reports and schemas exist today?
what legacy language is still visible?
what baseline speed/noise does the current tool have?
which invariants are executable guards now?
```

## Scope

Likely files:

```txt
README.md
docs/PRODUCT.md
docs/IMPLEMENTATION.md
src/cli/*
src/render/*
schemas/manifest.json
tests/*
docs/plans/feature/TODO.md
```

Do not change lens behavior except to remove or quarantine misleading product
language. This slice is a guardrail and inventory slice.

## Implementation Steps

1. Inventory all public commands from CLI help and schema manifest.
2. Classify commands as daily, focused lens, compat, or internal.
3. Add tests that fail if public help/docs contain:
   - `task router`;
   - `read_first`;
   - `source_of_truth`;
   - global `confidence`;
   - `best`;
   - `recommended`;
   - `safe to delete`;
   - embedding/semantic/LLM promises.
4. Add a baseline `doctor` output snapshot or JSON assertion for:
   - cache state;
   - repo root;
   - file count;
   - ignored count if available;
   - scan/load timing if already exposed.
5. Record current fixture coverage and missing lenses in a test-visible list.
6. Update product docs so `codemap ls`, `cone`, `changed`, `proof`, and
   `doctor` are the primary daily flow.

## Acceptance

- Public help no longer presents legacy routing as the main UX.
- Existing compat commands, if any, are explicitly marked compat.
- Product invariant tests are load-bearing and fail on forbidden language.
- The current baseline is measurable enough to compare later slices.
- No schema is removed without a compatibility decision.

## Load-Bearing Tests

Add tests that would fail if:

- `codemap --help` advertises task routing as primary;
- markdown output contains `best`, `recommended`, or `safe`;
- docs claim full dataflow/callgraph;
- schema manifest references absent schemas;
- `doctor` stops exposing enough fields to compare baseline.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio doctor
codemap --root /Users/amir/Documents/projects/Sillentway-VPN doctor
codemap --root <third-project> doctor
codemap --root /Users/amir/Documents/projects/spritestudio --help
```

Record whether current help makes it obvious what to run first.

## Reviewer Checklist

Ask reviewer to look specifically for:

```txt
legacy router language
ranking/recommendation language
false publishing/release scope
tests that only check strings but miss behavior
baseline output that is too vague to compare later
```

## Done When

- gates pass;
- reviewer returns PASS;
- live baseline notes exist;
- `TODO.md` Slice 01 boxes can be checked honestly.

