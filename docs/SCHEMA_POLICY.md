# Schema Policy

`codemap` treats JSON output as an integration contract, not as debug text.

## Stable Surfaces

Bundled schemas live in `schemas/` and are exported by:

```bash
codemap schema <kind>
codemap schema manifest
```

Structural output contracts use:

```json
"schema_version": "2"
```

Semantic anchor config uses:

```yaml
version: 1
```

## Change Rules

Within the same schema version, allowed changes are limited to documentation, examples, and schema fixes that make schemas match JSON already emitted by the CLI.

These changes require a new schema/config version:

- adding, removing, or renaming emitted fields;
- changing field type, nullability, enum values, or meaning;
- changing required fields;
- changing output budgets;
- changing `.ctx.yml` semantics or accepted field names.

Unknown `.ctx.yml` fields stay rejected. New anchor fields require a config version bump unless the old parser can safely ignore them without changing behavior.

## Guard

Tests verify that:

- every `*.schema.json` file is listed in `schemas/manifest.json`;
- every manifest entry is printable through `codemap schema <kind>`;
- printed schemas match bundled files;
- structural schemas require `schema_version: "2"`;
- anchor schemas require `version: 1`;
- schema commands do not write cache.
