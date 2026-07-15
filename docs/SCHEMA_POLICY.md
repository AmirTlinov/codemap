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

## Flagship draft migration: complete runtime group horizons

Manifest version 6 advances `runtime` from schema 4 to 5. The single
`RuntimeReport.observations` ledger now contains exactly one horizon and one
group-specific certificate for each of `entrypoints`, `routes`, `scripts`,
`env`, `workers`, `ci`, `proof`, and `unknowns`. JSON is the full projection:
every horizon has `shown=observed`, `hidden=0`, and no expansion handle.
Readable output remains bounded; its `shown/hidden` values are projection
metadata, while observation counts, closure reasons, and certificates remain
independent of that presentation limit.

The root runtime cache keeps the same trusted-local checksum boundary but now
rejects wrong report kind/schema/scope, missing or extra group horizons,
list-to-horizon mismatches, stale certificate snapshots, and legacy duplicate
hidden groups before serving a body. No new runtime extractor is introduced by
this migration: incomplete entrypoint/env grammars, non-exhaustive root-only
script catalogs, dynamic env lookups, partial verification relations, and
detector gaps remain typed open coverage rather than false zeroes.

## Flagship draft migration: root inventory horizons

Manifest version 7 extends the observation contract to the root `ls .`
inventory: `ls` advances from schema 7 to 8 and carries an `observations`
ledger with exactly one certificate-backed horizon per group —
`directory_surfaces`, `packages`, `scripts` and `test_surfaces`. Readable and
JSON projections resolve the same certificates; `shown`/`hidden` belong to the
projection and converge exactly against visible surface members. JSON ignores
the readable display limit and serializes every observed member; aggregate
rows retain all examples and report `hidden_count=0`. The detached
`directory surfaces hidden by limit` and
`support packages hidden below support scopes` groups are removed at the root
scope because the horizons own that accounting. Both root owners certify their
own extractor truth: the full-index owner may close groups under exact
candidate-inventory accounting, while the bounded cold inventory fast path
keeps role-dependent groups typed `open` (`unsupported_construct`) and its
root-only script catalog keeps nested manifests as exact
`incomplete_traversal` exclusions. Malformed/unreadable package manifests and
unavailable test-role candidates remain typed unsupported gaps in their
certificates instead of disappearing behind a closed zero. Lens artifact
format 16 stores the four-group ledger and a body checksum; a corrupt warm
artifact is rebuilt rather than served. File, symbol and nested-directory `ls`
anchors stay outside this boundary.

## Flagship draft migration: exact-symbol LS horizons

Manifest version 8 advances `ls` from schema 8 to 9. An exact `file#symbol`
anchor now carries exactly two certificate-backed horizons, `consumers` and
`verification`. Readable output remains bounded, but its per-group
`shown`/`hidden` values resolve the same certificates as the complete JSON
edge projection. The former detached `symbol edges hidden by limit` group is
removed because the horizons own that accounting. Missing symbols and symbols
under unavailable files keep both groups explicit as
`unavailable(anchor_not_indexed)` rather than returning an empty ledger.

This migration reuses the existing static-consumer and structural-verification
candidate universes; it adds no extractor and does not upgrade open coverage to
closed. Lens artifact format 17 prevents pre-migration exact-symbol LS bodies
without the required ledgers from being served. Ordinary file and nested
directory `ls` anchors remain outside this propagation boundary.

## Flagship draft migration: exact-file relationship horizons

Manifest version 9 advances `ls` from schema 9 to 10. An exact file report now
owns three relationship horizons: `imports`, `consumers`, and `verification`.
Readable output retains its global edge budget, while complete JSON serializes
every observed relationship; both projections resolve the same group
certificates. Test imports are represented by the verification relation rather
than being duplicated in the consumer group. The detached
`edges hidden by limit` row is removed because per-group horizons now own the
remainder and expansion handle.

The import certificate names dynamic and unresolved targets as typed open
stops. Consumer and verification groups reuse their existing candidate
universes. An indexed file whose body is unavailable exposes all three groups
as unavailable instead of presenting an empty relationship map. Lens artifact
format 18 invalidates exact-file bodies without the required ledger. File
symbol-catalog visibility and nested-directory reports remain outside this
propagation boundary.


## Flagship draft migration: exact-file symbol-catalog horizon

Manifest version 10 advances `ls` from schema 10 to 11. The exact-file ledger
adds a `symbols` horizon beside the three relationship horizons. Readable output
keeps its symbol budget, while JSON serializes the complete indexed symbol
catalog; both projections resolve one certificate and reconcile
`observed`/`shown`/`hidden`. The detached `nested symbols hidden by default` and
`symbols hidden by limit` rows are removed from exact-file reports because the
new horizon owns that visibility accounting.

Readable supported source files with no indexed symbols can now state
`proven-zero`; unsupported languages and indexed files without a readable body
remain unavailable rather than presenting an unqualified empty catalog. Lens
artifact format 19 renames the exact-file cache projection key and prevents
pre-migration bounded symbol bodies from being served. Exact-symbol, root,
nested-directory and cone projections remain outside this propagation boundary.


## Flagship draft migration: nested-directory relationship horizon

Manifest version 11 advances `ls` from schema 11 to 12. Every non-root
directory report now owns one certificate-backed `relations` horizon. Readable
output retains its balanced edge budget, while JSON serializes all observed
aggregate directory relations; both projections resolve the same candidate
universe and certificate. The detached `directory edges hidden by limit` row is
removed from nested-directory reports because the horizon owns the remainder.

The relation certificate covers project-wide static source candidates that can
create incoming crossings plus scoped package, script, CI, env, schema and
lockfile owners. Dynamic or unresolved imports, unsupported source languages,
malformed manifests and unavailable candidate bodies remain typed open stops.
Lens artifact format 20 records the complete-directory-relations projection and
rejects pre-migration bounded edge bodies. Root directory relations, nested
surface inventory and cone projections remain outside this propagation
boundary.

## Flagship draft migration: nested-directory surface horizons

Manifest version 12 advances `ls` from schema 12 to 13. Every non-root
directory report adds `surface_groups` and `surface_members` beside its
`relations` horizon. Readable output keeps its bounded group and example
projection, while JSON serializes every classified surface group and member;
each projection reconciles separately against the same candidate basis. The
detached nested-directory rows for hidden surfaces, generic files, support
packages, support artifacts and recursive files are removed because the two
surface horizons now own that visibility accounting.

The shared surface basis audits every indexed file below the directory and the
package manifests among them. Unavailable bodies and malformed manifests keep
both horizons typed open with exact unsupported carriers rather than silently
closing the inventory. Lens artifact format 21 renames the complete nested
projection key, persists the three-horizon ledger and rejects bounded
pre-migration surface bodies. Root, exact-anchor and cone projections remain
outside this migration boundary.

## Flagship draft migration: directory-cone relationship horizons

Manifest version 13 advances `cone` from schema 11 to 12. A directory anchor
now carries exactly five relationship horizons: `outgoing`, `incoming`,
`verification`, `contracts` and `boundary`. Readable output retains its
per-section edge budget, while JSON serializes every observed aggregate edge;
both projections resolve the same group certificates. The five detached
`directory ... edges hidden by limit` rows are removed because their horizons
own the typed remainders and expansion handles.

Outgoing and incoming groups reuse the nested-directory static relation
candidate universe. Verification, contract and boundary groups audit their
indexed scoped and external carriers independently. Dynamic or unresolved
flows, malformed manifests and unavailable candidate bodies remain typed open;
a zero closes only when its declared candidate inventory was fully visited.
Lens artifact format 22 rejects cached directory cones without the five-group
ledger. Symbol/file cone and X-Ray surface projections remain outside this
migration boundary.

## Flagship draft migration: exact-file cone relationship horizons

Manifest version 14 advances `cone` from schema 12 to 13. Every indexed
exact-file anchor now carries the same five relationship horizons as a
directory cone: `outgoing`, `incoming`, `verification`, `contracts` and
`boundary`. Readable output keeps its per-section edge budget, JSON retains the
complete relation lists, and the horizon projection alone owns shown/hidden
accounting; the former detached relationship hidden rows are no longer emitted.

Each group declares the candidate inventory used by its existing collector.
Static outgoing and contract traversal follow the requested depth, incoming
relations retain the file-consumer audit, verification includes test and
consumer carriers, and boundary relations include the parsed semantic config.
Dynamic and unresolved imports, re-exports, Rust includes, malformed manifests,
unavailable bodies and config parse failures keep affected certificates open.
Lens artifact format 23 rejects cached exact-file cones without the five-group
ledger. Symbol catalogs, symbol cones and the remaining X-Ray surface groups
stay outside this migration boundary.

## Flagship draft migration: exact-file cone symbol catalog

Manifest version 15 advances `cone` from schema 13 to 14. An indexed
exact-file cone adds `symbols` beside its five relationship horizons. Readable
output keeps the bounded anchor catalog, JSON serializes every indexed symbol,
and both projections resolve the same catalog certificate. The detached rows
for nested symbols hidden by default and symbols hidden by limit are removed
because the catalog horizon now owns that remainder.

The certificate is shared with exact-file `ls`: supported source extractors can
close an empty catalog, while unsupported languages and unavailable bodies stay
typed unavailable. Lens artifact format 24 rejects cached exact-file cones
without the sixth horizon. Symbol anchors and the remaining X-Ray surface
groups stay outside this migration boundary.

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
