# Slice 06: Compact Markdown Grammar And Renderer

## Intent

Replace many bespoke markdown shapes with one compact grammar agents can parse
quickly by eye.

This is not a separate "pretty" renderer. It is the default agent-facing map.

## Required Sections

Reports may omit irrelevant sections, but must preserve this order:

```txt
scope
summary
map
relations
runtime
contracts
proof
unknowns
hidden
expand
```

## Scope

Likely files:

```txt
src/render/*
src/render/lenses.rs
src/model/lens_reports.rs
tests/golden*
tests/structural_map/*
```

## Implementation Steps

1. Add shared render helpers for:
   - scoped path prefix;
   - grouped surfaces;
   - grouped edges;
   - compact locations;
   - unknown groups;
   - hidden groups;
   - expand commands.
2. Update `ls`, `cone`, `runtime`, `contract`, `impact`, `proof-map`, `proof`,
   and `changed` to use the same grammar.
3. Remove duplicate full path repetition inside a scope.
4. Render evidence as compact provenance:
   - `import src/a.ts:12`;
   - `manifest package.json`;
   - `route app/api/x/route.ts`;
   - `test_import foo.test.ts:3`.
5. Keep JSON unchanged except for schema-versioned structural fields.
6. Add markdown line-budget tests.

## Acceptance

- Markdown is compact enough for an agent to read without scrolling through
  duplicate paths.
- JSON remains complete.
- Same concept has same name across lenses.
- Root markdown normally stays under 150 lines.
- Exact anchors include enough line hints to open code immediately.

## Load-Bearing Tests

Tests fail if:

- a golden output repeats the same directory prefix more than budget;
- root output exceeds line budget on fixture;
- `hidden` appears without `expand`;
- evidence disappears from primary relations;
- JSON and markdown disagree on counts.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio ls .
codemap --root /Users/amir/Documents/projects/spritestudio cone <known-anchor>
codemap --root /Users/amir/Documents/projects/Sillentway-VPN proof-map .
```

Record whether the output is easier than manual `find`/`rg` and whether any
section feels ceremonial.

## Reviewer Checklist

Reviewer checks:

```txt
no 100 renderer dialects
no table spam where grouped lists are clearer
no lost evidence
no hidden detail without expand
line budget holds
```

## Done When

Markdown becomes a stable map grammar, not per-command prose.

