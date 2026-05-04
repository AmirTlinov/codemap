# Slice 16: Contract Lens

## Intent

Separate real public/schema/API surfaces from implementation names.

`contract` answers:

```txt
is this public?
who consumes it?
what schema/API/type surface changes here?
what proof exists around this contract?
```

## Scope

Likely files:

```txt
src/map/lenses/contract.rs
src/repo/contracts*
src/model/*
src/render/*
schemas/contract.schema.json
tests/fixtures/*
```

## Hard Evidence Sources

Use hard/high evidence for:

```txt
package.json exports/types/main/module/bin
Cargo lib/bin/public module declarations where deterministic
pyproject entry-points
TS barrel exports
Rust pub use/pub mod/public items
Python __all__
*.schema.json
*.d.ts
OpenAPI/GraphQL schema files
exported DTO/type/interface files by syntax
cross-package consumers
explicit .codemap.yml public anchors if present
```

Do not mark a file public only because its name contains `contract`.

## Implementation Steps

1. Add contract surface kinds:
   - package_export;
   - barrel_export;
   - schema_file;
   - dto_type;
   - public_symbol;
   - api_schema;
   - entrypoint_contract.
2. Add `public_surface` only when evidence is hard/high.
3. Separate:
   - same-package consumers;
   - cross-package consumers;
   - runtime consumers;
   - proof sensors.
4. Add `contract --changed` support from structural events.
5. Add unknowns for generated clients, dynamic schema loading, and unresolved
   public owner.

## Acceptance

- Package exports are visible.
- Barrel reexports are visible.
- DTO/schema/type surfaces are visible with syntax evidence.
- Cross-package consumers are separated.
- Internal helpers are not promoted to contracts by name alone.
- Contract proof sensors are shown.

## Load-Bearing Tests

Tests fail if:

- `contract.rs` or `contracts/foo.ts` becomes public by filename alone;
- package export removal is not detected;
- barrel reexport is ignored;
- cross-package consumer is mixed with same-package consumer;
- schema files lack contract surfaces;
- generated clients become source owners without evidence.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio contract <known-contract-or-package>
codemap --root /Users/amir/Documents/projects/Sillentway-VPN contract <manifest-or-public-surface>
codemap --root <third-project> contract <known-contract-or-package>
```

If no obvious contract exists, record that and use `ls .` / `runtime .` to find
candidate public surfaces.

## Reviewer Checklist

Reviewer checks:

```txt
no fake public claims from names
schema/API surfaces are evidence-backed
cross-package consumers separated
proof around contracts real
unknowns for generated/dynamic cases
```

## Done When

Public/schema/API risk is visible before edits.
