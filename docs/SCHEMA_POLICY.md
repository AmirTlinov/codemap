# Schema Policy

`codemap` treats JSON output as an integration contract, not as debug text.

## Stable Surfaces

Bundled schemas live in `schemas/` and are exported by:

```bash
codemap schema <kind>
codemap schema manifest
```

Each manifest entry is the authority for that output's exact version. Individual
outputs advance independently when their emitted JSON contract changes.
The schema manifest is the source of truth for mixed-version surfaces such as
`ls`, `changed`, `proof`, `proof-map`, `siblings`, and `place`.

## Flagship draft migration: build identity

Manifest version 3 adds a live `build_identity` overlay to every structural JSON
report. The already-migrated daily surfaces are `ls` 7, `cone` 9, `changed` 10,
and `proof` 10; every other affected report advances one schema version, while
`status`/`doctor` 6 embeds the diagnostic form. The overlay identifies the
running executable, package/cache/schema versions, and compile-time source
provenance. Non-diagnostic reports mark `binary_sha256` as `not_requested`;
diagnostics compute it. Cached structural report bodies do not store this live
process identity.

## Flagship draft migration: evidence horizon pilot

Manifest version 4 introduces report-local observation ledgers for the S03.a
consumer-horizon pilot. `where` advances to schema 4 and `cone` advances to
schema 10. Their observed counts resolve deterministic certificate ids inside
the same report and keep lower bounds when traversal remains open. A unique
`where` definition also serializes the incoming and verification relations
named by its horizons, so machine consumers never receive a count without its
observed fact list.

The migration is deliberately confined to `where` and `cone`; the legacy
`FileSummary.imported_by` shape and schemas embedding it do not change. The
shared lens artifact format advances from 13 to 15, so old artifacts are
rebuilt; cached symbol-cone report bodies are content-hashed and their ledgers
are validated before serving, so corrupted facts or certificate/horizon
registries become a cache miss. Project inventory,
fingerprint, and non-cone lens report data models are unchanged.

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
- changing `.codemap.yml` semantics or accepted field names.

Unknown `.codemap.yml` fields stay rejected. New anchor fields require a config version bump unless the old parser can safely ignore them without changing behavior.

## Guard

Tests verify that:

- every `*.schema.json` file is listed in `schemas/manifest.json`;
- every manifest entry is printable through `codemap schema <kind>`;
- printed schemas match bundled files;
- structural schemas declare the `schema_version` listed for their manifest entry;
- anchor schemas require `version: 1`;
- schema commands do not write cache.
