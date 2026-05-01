# Slice 24: Unknown Taxonomy, Scope Repair, And Fail-Closed Traversal

## Intent

Make blind spots useful instead of noisy. Unknowns should tell the agent what
is not structurally known and where to inspect next.

## Required Unknown Kinds

At minimum:

```txt
dynamic_import
js_require_dynamic
unresolved_import
route_string_concat
route_dynamic_path
route_dynamic_method
env_dynamic_lookup
raw_sql_literal
unsupported_framework_route
ambiguous_route_owner
di_token_unresolved
macro_expansion_boundary
generated_source_owner_unknown
dynamic_asset_path
dynamic_event_topic
reflection_boundary
```

## Scope

Likely files:

```txt
src/model/*
src/repo/*
src/map/lenses/helpers.rs
src/render/*
schemas/*
tests/fixtures/*
```

## Implementation Steps

1. Normalize unknown structure across all lenses.
2. Group unknowns by kind and scope.
3. Add budgets:
   - max unknown groups;
   - max examples per group;
   - hidden unknown count.
4. Add expand commands that help:
   - `codemap ls <file>`;
   - `codemap cone <file>`;
   - `codemap runtime <scope>`;
   - `codemap flow <anchor> --show-unknowns`.
5. Add scope repair for anchors:
   - ambiguous path;
   - missing path;
   - unsupported symbol anchor;
   - deleted file from git state.
6. Ensure fail-closed traversal in `flow`, `impact`, `delete`, and `runtime`.

## Acceptance

- Unknowns are typed, grouped, bounded, and actionable.
- Unknowns do not drown root output.
- Dynamic facts never become hard edges.
- Ambiguous anchors produce useful repair suggestions.
- Deleted/renamed files from git state do not look like unresolved user errors.

## Load-Bearing Tests

Tests fail if:

- unknown is rendered as free text only;
- unknown lacks reason/effect/expand;
- root output lists every unknown instance;
- dynamic import appears as resolved edge;
- ambiguous anchor silently picks a file.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio runtime .
codemap --root /Users/amir/Documents/projects/Sillentway-VPN flow <known-anchor>
codemap --root <third-project> ls .
```

Record whether unknowns help decide where to inspect next.

## Reviewer Checklist

Reviewer checks:

```txt
typed taxonomy complete
unknowns bounded
fail-closed behavior
anchor repair clarity
no fake edges
```

## Done When

Blind spots become honest map features rather than omissions or noise.

