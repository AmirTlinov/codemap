# Schema Policy

`ctx` treats JSON output as an integration contract, not as debug text.

## Stable Surfaces

Bundled schemas live in `schemas/` and are exported by:

```bash
ctx schema <kind>
```

The exported schema list is owned by `schemas/manifest.json` and is printable from installed binaries with:

```bash
ctx schema manifest
```

Legacy route output contracts use:

```json
"schema_version": "1"
```

Structural v2 route output contracts use:

```json
"schema_version": "2"
```

Semantic anchor config uses:

```yaml
version: 1
```

## Change Rules

Within the same version, allowed changes are limited to documentation, titles, descriptions, examples, and schema fixes that make the schema match JSON already emitted by the CLI.

These changes require a new schema/config version:

- adding, removing, or renaming emitted fields;
- changing field type, nullability, or meaning;
- changing enum values in a way strict clients must handle differently;
- changing required fields;
- changing output budgets such as max command counts or route sizes;
- changing `.ctx.yml` semantics or accepted field names.

Unknown `.ctx.yml` fields stay rejected. New anchor fields require a config version bump unless the old parser can safely ignore them without changing behavior.

## Release Guard

`tests/schema_policy.rs` verifies that:

- every `*.schema.json` file is listed in `schemas/manifest.json`;
- every manifest entry is printable through `ctx schema <kind>`;
- the manifest itself is printable through `ctx schema manifest`;
- printed schemas match the bundled files;
- route schemas require a manifest-declared `schema_version`;
- legacy route schemas stay on `schema_version: "1"`;
- structural route schemas use `schema_version: "2"`;
- anchor schemas require `version: 1`;
- schemas remain strict at the root.

`scripts/release-check.sh` runs this through `cargo test` and checks that schemas and the manifest are present in the packaged crate.
