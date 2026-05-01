# Slice 07: Schema Rail And Golden Report Validation

## Intent

Make every public JSON output contract explicit and tested.

## Required Schemas

The manifest must include:

```txt
doctor
ls
cone
graph
runtime
contract
flow
boundary-map
siblings
place
delete
changed
diff-map
impact
proof-map
proof
```

## Scope

Likely files:

```txt
schemas/*.schema.json
schemas/manifest.json
src/cli/schema_and_roots.rs
src/model/lens_reports.rs
tests/schema*
tests/golden*
```

## Implementation Steps

1. Add missing schemas for every public report.
2. Add `schema_version` to reports that do not have it.
3. Bump schema versions where `locations`, typed unknowns, or surfaces break
   prior shape.
4. Add `codemap schema <kind>` tests for every manifest item.
5. Add golden JSON reports for representative fixtures.
6. Validate golden JSON against schemas in tests.
7. Add a manifest parity test:
   - every schema file is listed;
   - every listed schema exists;
   - every public report has a schema;
   - no legacy router schema appears as primary.

## Acceptance

- Schema command prints bundled schemas.
- All public JSON reports validate against schemas.
- Schema manifest has no stale entries.
- Breaking changes are versioned.
- Markdown changes do not require JSON consumers to guess shape.

## Load-Bearing Tests

Tests fail if:

- a new public command lacks schema;
- schema manifest points to missing file;
- golden JSON does not validate;
- unknowns/locations are omitted from schemas;
- legacy router reports re-enter the manifest as primary.

## Live Dogfood

Run:

```bash
codemap schema ls
codemap schema cone
codemap schema changed
codemap --root /Users/amir/Documents/projects/spritestudio ls . --format json
codemap --root /Users/amir/Documents/projects/Sillentway-VPN ls . --format json
codemap --root <third-project> ls . --format json
```

Record whether JSON is complete enough for integration use.

## Reviewer Checklist

Reviewer checks:

```txt
schema parity
version bump correctness
unknown/location requirements
legacy schema quarantine
goldens are representative, not cosmetic
```

## Done When

Schemas are a real public contract, not documentation by hope.
