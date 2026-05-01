# Slice 22: Siblings And Place Convention Lenses

## Intent

Help agents avoid duplicate code and follow local conventions without semantic
search or recommendations.

`siblings` answers:

```txt
what nearby things have the same structural role?
what proof/contract/helper patterns repeat here?
```

`place` answers:

```txt
how is this kind of thing organized in this scope?
```

## Scope

Likely files:

```txt
src/map/lenses/siblings.rs
src/map/lenses/place.rs
src/render/siblings.rs
src/render/place.rs
schemas/siblings.schema.json
schemas/place.schema.json
tests/fixtures/*
```

## Deterministic Grouping Rules

Allowed grouping facts:

```txt
same directory
same package/domain
same role
same file suffix/prefix convention
same route convention
same test naming convention
same import target layer
same proof command/container
shared helper imported by group
shared contract imported by group
```

Forbidden:

```txt
semantic similarity
task prompt matching
best example
recommended placement
ranked examples
```

## Implementation Steps

1. Define role taxonomy used by both lenses:
   - route;
   - component;
   - hook;
   - service;
   - contract;
   - test;
   - e2e;
   - config;
   - asset.
2. Build sibling groups from structural facts only.
3. Add paired proof pattern detection:
   - file and test naming;
   - importing tests;
   - package test command.
4. Add shared helper/contract sections from import edges.
5. Add `place <scope> --kind <kind>` requiring explicit kind.
6. `place` shows existing convention; it does not tell the user to create a
   specific file as a recommendation.

## Acceptance

- Siblings group same-role/same-dir/same-convention files.
- Place shows current local convention for a kind.
- Paired tests/proof pattern appears.
- Shared helpers/contracts appear only from structural edges.
- No `best`, `recommended`, or ranking language appears.

## Load-Bearing Tests

Tests fail if:

- semantic name similarity alone creates sibling group;
- place accepts natural-language task prompt;
- output says "best example" or "recommended";
- shared helper appears without import evidence;
- test pairing ignores actual naming/import facts.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio siblings <known-scope>
codemap --root /Users/amir/Documents/projects/spritestudio place <known-scope> --kind test
codemap --root /Users/amir/Documents/projects/Sillentway-VPN place <known-scope> --kind config
codemap --root <third-project> siblings <known-scope>
codemap --root <third-project> place <known-scope> --kind test
```

Record whether it reduces duplication risk without pretending to choose for the
agent.

## Reviewer Checklist

Reviewer checks:

```txt
no semantic search
no ranking/recommendation
structural grouping evidence
proof pattern real
local convention not global advice
```

## Done When

Agents can see local shape before adding code, without asking the tool to think
semantically.
