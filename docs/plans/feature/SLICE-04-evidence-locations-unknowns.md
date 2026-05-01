# Slice 04: Evidence Locations And Typed Unknowns

## Intent

Make every important claim actionable. A map edge should tell the agent where it
was proven, and blind spots should be explicit.

## Model Requirements

`EvidenceLocation`:

```txt
path
line_start
line_end
kind
```

`Unknown`:

```txt
kind
path
line_start
reason
effect
expand
```

## Scope

Likely files:

```txt
src/model.rs
src/repo/*
src/map/lenses/*
src/render/*
schemas/*
tests/fixtures/*
tests/structural_map/*
```

## Implementation Steps

1. Add line-aware capture for:
   - import statements;
   - export statements;
   - route registrations;
   - env lookups;
   - test imports;
   - e2e route visits;
   - package manifest keys where feasible.
2. Attach locations to edges and surfaces where deterministic.
3. Add typed unknown detection for:
   - dynamic import;
   - dynamic require;
   - unresolved import;
   - route string concatenation;
   - dynamic route method;
   - dynamic env lookup;
   - raw SQL literal;
   - unresolved DI token;
   - unsupported framework route.
4. Ensure dynamic constructs do not become hard edges.
5. Render unknowns compactly with one-line effect and expand command.
6. Add schema coverage for typed unknowns and locations.

## Acceptance

- Import/reverse-import edges point to real lines.
- Static env/route/test facts point to real lines where possible.
- Dynamic facts become unknowns, not fake relations.
- Unknowns are grouped and limited by budget.
- Markdown gives enough location to open the right file immediately.

## Load-Bearing Tests

Fixture tests must include:

- static import with location;
- unresolved import unknown;
- `import(name)` unknown;
- `require(prefix + name)` unknown;
- Express/FastAPI route concatenation unknown;
- `process.env[name]` unknown;
- raw SQL literal unknown.

Tests fail if any dynamic case becomes a hard/high edge.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio cone <known-anchor>
codemap --root /Users/amir/Documents/projects/Sillentway-VPN runtime .
codemap --root <third-project> cone <known-anchor>
```

Record whether locations reduced manual searching.

## Reviewer Checklist

Reviewer checks:

```txt
no primary edge without evidence
unknowns have path/reason/effect
dynamic facts do not become fake edges
locations point to real lines
schemas require new fields
```

## Done When

The map is not just a graph; it is a graph with source-backed handles.
