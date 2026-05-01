# Slice 05: Scope Model, Current-Level Root Maps, And Expand Protocol

## Intent

Make root and directory output behave like a real map, not a recursive dump.

Root answers:

```txt
what are the current-level domains/packages/surfaces?
```

Exact anchors answer:

```txt
what is inside and connected to this thing?
```

## Scope

Likely files:

```txt
src/repo/*
src/map/lenses/helpers.rs
src/map/lenses/*
src/render/*
schemas/*
tests/structural_map/*
```

## Implementation Steps

1. Add a `Scope` model:
   - repo root;
   - package root;
   - directory;
   - file;
   - symbol anchor;
   - virtual surface like route or package.
2. Add current-level child grouping:
   - package/workspace;
   - domain directory;
   - source container;
   - test container;
   - runtime container;
   - contract container;
   - generated/vendor hidden group.
3. Add root budgets:
   - max current-level surfaces;
   - max relation groups;
   - max unknown groups;
   - max expand commands.
4. Define expand command format:
   - `codemap ls <child>`;
   - `codemap cone <anchor> --depth 1`;
   - `codemap runtime <scope>`;
   - `codemap proof-map <scope>`;
   - `codemap <lens> <scope> --show-hidden <kind>`.
5. Ensure hidden groups include reason, count, examples, and expand.

## Acceptance

- `codemap ls .` does not print every file.
- `codemap graph --lens causal` does not print every edge.
- Root output shows hidden recursive counts.
- Directory output aggregates before expanding to files.
- Exact file output shows symbols/imports/exports/reverse-import count.

## Load-Bearing Tests

Tests fail if:

- root fixture output contains all nested files;
- hidden group count is missing when truncating;
- expand command points to the wrong scope;
- directory cone emits file galaxy by default;
- exact file ls omits imports/exports/symbols.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio ls .
codemap --root /Users/amir/Documents/projects/spritestudio ls src
codemap --root /Users/amir/Documents/projects/Sillentway-VPN ls .
codemap --root <third-project> ls .
```

Record whether the output feels like `ls` with relationships, not a dump.

## Reviewer Checklist

Reviewer checks:

```txt
root boundedness
no path spam
hidden counts accurate enough
expand commands usable
exact anchors still detailed
```

## Done When

The tool has a clear zoom model: root map, scope map, file map, symbol map.
