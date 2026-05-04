# Slice 20: Boundary-Map Read-Only Crossing Lens

## Intent

Show package/domain crossing edges as a map. Keep enforcement separate from
read-only mapping.

`boundary-map` answers:

```txt
what domains/packages cross here?
which crossings are runtime versus test-only?
which public boundaries are used?
where are internal leaks?
which violations come from explicit config?
```

## Scope

Likely files:

```txt
src/map/lenses/boundary_map.rs
src/map/lenses/boundaries.rs
src/render/boundary_map.rs
schemas/boundary-map.schema.json
tests/structural_map/*
```

## Implementation Steps

1. Build read-only crossing facts from package/domain edges.
2. Classify crossings:
   - runtime;
   - test-only;
   - proof-only;
   - contract/public;
   - internal/private.
3. Detect public boundary files from contract/package evidence.
4. Detect internal leaks as facts, not moral failures.
5. Only emit forbidden findings from explicit `.codemap.yml` or configured rules.
6. Keep old `boundaries` check behavior separate if it exists.

## Acceptance

- Boundary-map works without config as a map.
- Forbidden/violation language appears only with explicit rules.
- Test-only crossings are separated.
- Package dependency edges are visible.
- Public boundary surfaces are evidence-backed.
- Root boundary-map is bounded.

## Load-Bearing Tests

Tests fail if:

- no-config boundary-map calls crossings violations;
- test-only imports are mixed with runtime imports;
- internal leak claim lacks edge evidence;
- package dependency edge is missing;
- explicit forbidden config is ignored.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio boundary-map .
codemap --root /Users/amir/Documents/projects/Sillentway-VPN boundary-map .
codemap --root <third-project> boundary-map .
```

Record whether boundary view explains architecture without judgmental noise.

## Reviewer Checklist

Reviewer checks:

```txt
map versus enforcement separation
explicit config required for violations
test-only separated
no moral language
bounded output
```

## Done When

Boundary relationships are visible without turning every crossing into a
warning.

