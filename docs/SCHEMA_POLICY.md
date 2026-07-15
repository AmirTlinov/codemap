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

## Flagship draft migration: runtime route horizon

Manifest version 5 extends the same certificate-backed observation contract to
the `routes` group of `runtime`. Runtime advances from schema 3 to 4 and now
requires an `observations` ledger. Bounded readable output may show a subset of
routes, while JSON and `--all` return the complete observed route list; both
projections resolve the same certificate because presentation limits are not
part of the observation basis.

The shared coverage vocabulary adds `dynamic_runtime_registration`. Because
that enum is embedded in the strict `where` and `cone` schemas, those reports
advance mechanically from 4 to 5 and from 10 to 11 even though their emitted
horizons and certificate identities do not otherwise change. The root runtime
cache persists the ledger with a report-body hash and rejects any body or
ledger/list mismatch before serving it. Other runtime groups retain their
legacy count semantics until a separately activated S03 propagation slice.
The adjacent hash is a corruption/stale-body checksum for a trusted local
cache, not an authenticity or tamper-resistance claim.

## Flagship draft migration: remaining runtime-group horizons

Manifest version 6 completes the runtime observation contract: the
`entrypoints`, `scripts`, `env`, `workers`, `ci`, `proof` and `unknowns` groups
each carry exactly one certificate-backed horizon next to `routes`, and
runtime advances from schema 4 to 5. The detached per-group
`… hidden by limit` groups are removed because `shown`/`hidden` now belong to
the horizons; readable and JSON projections resolve the same per-group
certificates. Zero-fact groups close only under exact candidate-inventory
accounting (`eligible_files == visited_files + exact disjoint exclusions` with
no unresolved/dynamic/external stops); scope or extractor gaps stay typed
`open` instead of becoming proven-zero.

The shared coverage vocabulary adds `dynamic_env_lookup`. Because that enum is
embedded in the strict `where` and `cone` schemas, those reports advance
mechanically from 5 to 6 and from 11 to 12 even though their emitted horizons
and certificate identities do not otherwise change. The root runtime cache
validates the full eight-group ledger before serving a warm artifact.

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
