# Slice 19: Delete Lens Without Safety Claims

## Intent

Give a deletion blocker map without saying deletion is safe.

`delete <anchor>` answers:

```txt
who uses this?
is it exported or reexported?
is it in package public surface?
is it referenced by runtime/proof?
what dynamic blind spots could hide users?
what mechanical cleanup follows from evidence?
```

## Scope

Likely files:

```txt
src/map/lenses/delete.rs
src/render/delete.rs
schemas/delete.schema.json
tests/structural_map/*
```

## Implementation Steps

1. Support file and symbol anchors.
2. Show blocker groups:
   - direct users;
   - symbol users;
   - reexports;
   - package exports;
   - runtime refs;
   - tests/proof sensors;
   - unknown dynamic refs.
3. Generate mechanical checklist only from evidence:
   - remove barrel export;
   - remove package export;
   - update route registration;
   - update direct test import;
   - inspect dynamic import unknown.
4. Ban wording:
   - safe;
   - probably unused;
   - recommended deletion;
   - best.
5. Add expand commands to `cone`, `contract`, `runtime`, and `proof-map`.

## Acceptance

- Reverse imports and symbol refs appear.
- Package/barrel exports appear.
- Runtime refs appear where deterministic.
- Tests appear as sensors/users.
- Dynamic blind spots are unknowns.
- No safe-to-delete language exists.

## Load-Bearing Tests

Tests fail if:

- delete output contains `safe`;
- barrel reexport blocker is missed;
- package export blocker is missed;
- dynamic import is ignored;
- symbol delete only checks file-level imports;
- checklist includes unsupported advice.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio delete <known-file>
codemap --root /Users/amir/Documents/projects/Sillentway-VPN delete <known-anchor>
codemap --root <third-project> delete <known-anchor>
```

Do not delete files during dogfood. Record whether blockers are useful.

## Reviewer Checklist

Reviewer checks:

```txt
no safety claims
blockers evidence-backed
symbol-level support honest
unknown dynamic refs included
checklist is mechanical only
```

## Done When

Deletion planning becomes factual and cautious without claiming certainty.
