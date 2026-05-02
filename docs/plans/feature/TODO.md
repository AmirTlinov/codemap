# Feature TODO: Structural Map Completion Ledger

This is the execution ledger for `docs/plans/feature/PLAN.md`.

It is deliberately not a five-checkbox-per-slice ritual. A slice is tracked by
its real boundary:

```txt
status: todo | active | closed | parked | blocked
tier: docs | focused | full | live | final
proof: the smallest check that proves the boundary
review: required only when risk warrants it
live: required only when real-repo behavior is the point
```

Closure format:

```txt
closed: what is now genuinely true
excluded: what remains later work
proof: commands or notes that proved this boundary
review: PASS, not required, or blocker summary
live: commands/results, or not required with reason
next: the next highest-value move
```

## Proportional Validation

Use the lightest proof that can catch the real failure:

```txt
docs:
  git diff --check -- docs/plans/feature

focused:
  targeted tests or command probes for the touched behavior

full:
  cargo fmt --check
  cargo test --quiet
  cargo clippy --all-targets -- -D warnings
  cargo run --quiet --bin codemap -- doctor
  git diff --check

live:
  full proof plus read-only probes on live repos where the behavior matters

final:
  full proof, fixture matrix, live adoption harness, final reviewer
```

Reviewer is required for model/schema/cache/scanner/git-state changes, material
public-lens behavior, performance/correctness claims, and final audit. Reviewer
is not required for plan cleanup, typos, narrow renderer polish inside an
already closed renderer boundary, or test-only changes that do not alter
behavior.

Live dogfood is required for user-facing command behavior, performance/cache,
git state, scanner visibility, and final adoption. It is not required for
docs-only changes, schema parity tests, or controlled fixture fixes where live
repos would add noise instead of proof.

## Current Boundary

Slice 06 compact renderer is closed. Do not continue renderer micro-polish
unless a specific output still confuses the agent or creates a false claim.

Slice 09 fourth boundary is closed:

```txt
closed: cache avoids the full candidate-list fallback for committed HEAD
changes when git can prove the changed/removed path set. It derives parser work
from cached HEAD -> current HEAD `git diff --name-status`, merges live status
mismatches, and falls back to cached-index stat/hash recheck when a prior dirty
cache did not have a valid status probe.
proof: committed HEAD delta fixture, exact subdir-root stale-symbol regression,
existing dirty/untracked/delete/rename cache diagnostics, full gate.
review: first review BLOCK found subdir-root path normalization risk; final
review PASS after `--relative`/root-prefix normalization and regression test.
live: spritestudio and Sillentway-VPN read-only doctor probes stayed warm_load
with files_visited=0 and files_scanned=0.
next: keep cache parked unless daily use shows stale derived facts, config
dependency invalidation gaps, or path-normalization cases outside git diff.
```

Slice 09 fifth boundary is closed:

```txt
closed: when git status cannot provide a reliable mismatch set, such as an
active conflict cache written without a valid status probe, the fallback
rechecks cached file fingerprints and rescans only mismatched files instead of
falling through to broad parser work.
proof: active conflict cache regression plus focused dirty/status/head/delete
cache tests and full gate.
review: not required; narrow fallback correction inside the existing cache
delta model with a direct regression.
live: spritestudio and Sillentway-VPN read-only doctor probes stayed warm_load
with files_visited=0 and files_scanned=0.
next: keep cache parked unless daily use exposes stale derived facts or config
dependency invalidation gaps.
```

Slice 25 first boundary is closed:

```txt
closed: primary daily markdown now has an executable cognitive regression
gate. The guard runs `ls .`, `cone <anchor>`, `changed`, and
`proof --changed` on the fixture, enforcing line budgets, no table spam, no
forbidden product language, hidden sections with executable expand, and proof
command grouping.
proof: focused daily workflow cognitive gate plus line-budget test.
review: not required; test-only guard over existing public behavior.
live: not required for this boundary; the fixture captures daily workflow
shape without adding live-repo noise.
next: add live perf smoke only when changing runtime/cache behavior or before
final adoption closure.
```

Slice 27 first boundary is closed:

```txt
closed: `scripts/dogfood-codemap.sh` is now a real read-only live adoption
harness for daily plus focused probes, and README documents the local
`cargo install --path .` path plus `CODEMAP_BIN` dogfood override. The harness
auto-discovers one source anchor and one contract anchor, runs
cone/flow/delete/siblings/place/contract where possible, records elapsed time,
line count, line budget, and budget status in JSONL, and keeps target repos
clean.
proof: harness unit fixture verifies daily/focused labels, timing/budget
summary fields, and clean target `git status`; existing unsafe-output tests
remain green.
review: not required; script behavior is guarded by focused tests and does not
change public report semantics.
live: spritestudio, Sillentway-VPN, and Levelly-1 all ran read-only with zero
command failures and zero line-budget failures. Warm ls/cone/proof are
daily-fast; runtime/proof-map/changed/flow/delete/siblings still show ~1s+
latency on some live repos and remain the next performance gap.
next: do not add another live script; use this harness for final adoption and
future perf/cache work.
```

Slice 02 first boundary is closed:

```txt
closed: root help now makes the daily workflow visible before the flat command
list: `ls`, `cone`, `changed`, `proof`, `doctor`. Focused lenses are explicitly
described as drill-down targets from expand output, with diagnostics/integration
called out separately. Command behavior and schemas are unchanged.
proof: focused help test plus direct `codemap --help` output inspection.
review: not required; narrow help/UX contract change with existing product
invariant guards.
live: not required; `--help` is repo-independent and does not inspect target
repos.
next: only revisit Slice 02 if live daily workflow still needs non-obvious
commands before orientation/change/proof closure.
```

Slice 12 first boundary is closed:

```txt
closed: package/workspace dependency edges carry deterministic dependency_kind
from JavaScript and Cargo manifest sections.
proof: focused package-edge fixture, schema validation, then full gate because
the shared model/schema changes.
review: PASS.
live: not required; controlled manifest fixtures prove this boundary better
than dirty live repos.
next: move to the next boundary instead of expanding this slice into full
package/workspace perfection.
```

Slice 24 first boundary is closed:

```txt
closed: empty but existing narrow proof/test scopes no longer look like a dead
end. `proof <scope>` and `place <scope> --kind test` emit typed
`nearest_proof_scope` unknowns with exact expand commands to the nearest parent
scope that has deterministic proof sensors.
proof: focused scope-repair fixtures for proof, place, and missing-path
negative cases; proof schema validation bumped to v5.
review: required before commit because this changes public proof JSON shape and
daily proof/place semantics.
live: not required for first boundary; controlled fixture proves the false
"nothing here" behavior directly.
next: extend the same repair pattern to proof-map/place parent hints only if a
real daily workflow still leaves the agent stuck at an empty exact scope.
```

Slice 24 second boundary is closed:

```txt
closed: proof-map now shares the same nearest parent proof-scope repair, and
`changed` inherits that typed unknown through proof-map facts for changed files
with no direct sensors. The expand stays daily-oriented as `codemap proof
<nearest-scope>`, with proof-map also exposing a broader proof-map drill-down
for explicit scopes.
proof: focused fixtures for empty exact proof-map scope and changed new narrow
anchor without direct proof sensors; schema parity fixture remains green.
review: required before commit because this changes public-lens behavior for
proof-map and changed.
live: not required for this boundary; controlled dirty fixture isolates the
daily after-edit false dead-end better than current live repo dirt.
next: keep scope repair parked unless real daily probes still produce an empty
proof dead-end.
```

Slice 23 first boundary is closed:

```txt
closed: CSS-family files now extract static `@import` dependencies as
deterministic style edges, so CSS barrels connect to imported CSS anchors in
`ls`/`cone` instead of appearing as isolated non-code files. Commented
`@import` text is ignored.
proof: focused CSS import fixture with line evidence and comment-negative
case; live read-only SpriteStudio probe with fresh cache showed
`app/landing-shell.css -> app/landing-hero.css` at line 4.
review: required before commit because parser/cache semantics changed.
live: SpriteStudio exact CSS anchor was the motivating live case.
next: keep non-code work bounded to deterministic imports/references; do not
guess CSS class ownership from names.
```

Slice 21 first boundary is closed:

```txt
closed: Rust CLI flow now stitches explicit Cargo bin entrypoints to their
manifest line, to the exact `main` symbol when present, and to same-file
top-level functions that `main` directly calls. The path is bounded:
Cargo.toml bin target -> file -> `#main` -> direct entry calls -> direct
structural imports.
proof: runtime CLI entrypoint fixture asserts Cargo.toml provenance and
`entry_symbol` plus a same-file `entry_call`; live SilentWay
`flow src/masque-core/src/bin/vpn_server.rs` shows `cargo_bin_target` at
`src/masque-core/Cargo.toml:96`, `vpn_server.rs#main` at lines 136-153, and
direct entry calls to `run_diagnostics` / `run_vpn_server`.
review: PASS after tightening direct-call evidence against method/qualified
false positives.
live: Sillentway-VPN is the motivating Rust/native repo.
next: do not expand into callgraph; only add more runtime-to-code stitching
when a deterministic owner edge is visible.
```

Slice 13 first boundary is closed:

```txt
closed: root causal graph uses one directory coordinate for workspace package
nodes and package-internal edges, avoiding slash/no-slash duplicate anchors.
proof: controlled workspace graph fixture plus full local gate.
review: not required unless the fix expands beyond coordinate normalization.
live: not required; a controlled workspace fixture proves the coordinate bug
more directly than live repo output.
next: stop graph cleanup unless a remaining root-map claim is false or
confusing.
```

Slice 14 first boundary is closed:

```txt
closed: `cone file#MissingSymbol` fails closed as `missing_symbol_anchor`
instead of pretending the whole anchor is an unindexed/missing file path.
proof: controlled missing-symbol cone fixture plus full local gate.
review: PASS.
live: not required; a synthetic exact-symbol miss proves the false-claim case.
next: stop Slice 14 unless a concrete cone anchor still falls back to a broader
or misleading map.
```

## Slice Status

| Slice | Status | Tier | Boundary |
| --- | --- | --- | --- |
| 01 Product lock, baseline inventory, invariant guards | closed first boundary | focused | Public JSON outputs reject router/trust fields. |
| 02 Daily command surface and alias cleanup | closed first boundary | focused | Root help exposes daily workflow before focused lenses and diagnostics. |
| 03 Unified fact model and constructors | todo | full | Shared constructors prevent lenses from inventing separate fact logic. |
| 04 Evidence locations and typed unknowns | todo | full | Important claims carry provenance; blind spots are typed. |
| 05 Scope model, root maps, expand protocol | todo | focused/live | Root is current-level; exact anchors drill down; expand is reproducible. |
| 06 Compact markdown grammar and renderer | closed | live | Daily/focused markdown was compacted within the declared boundary. |
| 07 Schema rail and golden validation | closed | full | Public JSON reports validate against manifest-selected schemas. |
| 08 Fast scanner, ignore/generated/vendor detection | closed | live | Shared scanner policy and generated/ignored stats are visible. |
| 09 Warm cache, indexes, incremental freshness | closed | live | Git-tracked warm path uses status mismatch set, not a full candidate walk. |
| 10 Git structural events, changed, diff-map | closed | full | Comment-only edits no longer create false structural deltas. |
| 11 Symbol/import/export extraction matrix | closed | full | Exact file cones surface unresolved local imports as typed unknowns. |
| 12 Package/workspace graph and boundaries | closed first boundary | full | Deterministic dependency kind on package edges. |
| 13 `ls`, `graph --lens causal`, root map quality | closed first boundary | focused | Root causal graph normalizes workspace package coordinates. |
| 14 `cone` exact anchors and directory aggregation | closed first boundary | focused | Missing symbol anchors fail closed without whole-file fallback. |
| 15 Runtime lens | closed first boundary | focused/live | Static JS/Go/Next routes stitch to handler symbols where deterministic. |
| 16 Contract lens | closed first boundary | full | Package manifest exports are first-class contract evidence. |
| 17 Proof-map and proof safety | closed first boundary | focused | `proof --run` refuses deploy/migrate/unknown shell commands by default. |
| 18 Impact lens | closed first boundary | focused | Package export surfaces participate in contract impact risk. |
| 19 Delete lens | closed first boundary | focused | Direct-user blockers feed mechanical cleanup without deletion-safety claims. |
| 20 Boundary-map lens | closed first boundary | focused/live | Scoped boundary maps do not leak unrelated explicit forbidden findings. |
| 21 Flow lens | closed first boundary | focused/live | Rust CLI runtime entrypoints stitch to manifest provenance and exact `main` symbols. |
| 22 Siblings and place lenses | closed first boundary | focused | `place` expand preserves required kind arguments. |
| 23 Non-code, assets, data, events, generated ownership | closed first boundary | focused/live | CSS @import barrels create deterministic style edges with line evidence. |
| 24 Unknown taxonomy, scope repair, fail-closed traversal | closed first boundary | full | Empty proof/test scopes point to nearest parent proof scope with typed unknowns. |
| 25 Performance, path stability, cognitive gates | closed first boundary | live | Daily workflow cognitive budgets are guarded. |
| 26 Fixture matrix across stacks | todo | full | Fixtures cover supported stacks and public lenses without overfitting. |
| 27 Live adoption harness and PATH ergonomics | closed first boundary | live | Read-only daily/focused live harness is usable and records budgets. |
| 28 Final audit, cleanup, TODO closure | todo | final | Full system audit passes; TODO is honestly closed. |

## Closed Boundaries

### Slice 01: Product Lock, Baseline Inventory, Invariant Guards

```txt
closed: public JSON outputs across the daily/high-risk reports are guarded
against legacy router/trust fields such as `read_first`, `source_of_truth`,
`confidence`, `score`, `rank`, and `safe_to_delete`.
excluded: full docs wording audit and final product-contract audit remain
later work.
proof: focused invariant test plus existing help/bootstrap router guards.
review: not required; test-only product invariant guard.
live: not required; invariant is contract-level and fixture-backed.
```

### Slice 06: Compact Markdown Grammar And Renderer

```txt
closed: daily/focused compact map grammar for ls, cone, changed, impact,
proof, proof-map, runtime, hidden/unknown, surfaces, and contract exports.
excluded: doctor/status diagnostics, graph/boundary diagnostics, tiny metadata
headers that are not repeated path spam.
proof: full gate and live dogfood were run at the time.
review: PASS.
live: completed at the time.
next: stop renderer micro-polish and return to facts/schemas/cache/lens depth.
```

### Slice 07: Schema Rail And Golden Validation

```txt
closed: manifest/schema-command parity for every listed schema, public JSON
report validation against manifest-selected schemas, json_kind/schema_version
parity for structural outputs, doctor schema alias, and legacy/router schema
absence guards.
interpreted: "golden JSON" is covered by real fixture command outputs validated
against schemas, not committed snapshot files.
excluded: a separate golden snapshot framework and exhaustive live lens behavior
dogfood; those would add process without stronger schema-contract proof.
proof: full gate.
review: PASS.
live: not needed beyond the recorded closure proof.
next: move to scanner/cache/facts depth, not more schema ceremony unless a new
public JSON command or report shape appears.
```

### Slice 08: Fast Scanner, Ignore Rules, Generated And Vendor Detection

```txt
closed: shared scanner policy returns scan stats, common ignored dirs are
excluded from inventory and config discovery, ignored stats count unique
ignored roots, generated headers/path conventions mark files as generated,
doctor/status expose scanner groups; introduced in status_report v3 and
preserved by later status schemas.
excluded: full generated source-owner resolution, source-map pairing, codegen
config tracing, and partial-rescan cache. Those belong to later generated/cache
slices, not scanner-policy closure.
proof: full gate and live scanner probes at the time.
review: PASS.
live: spritestudio and Sillentway-VPN scanner stats were recorded at closure.
next: warm cache/index freshness.
```

### Slice 09: Warm Cache, Indexes, And Incremental Freshness

```txt
closed: status_report v4 exposes cache_strategy, files_reused, and project
timings; doctor markdown shows cache strategy, reused count, and timing phases;
cache_state=warm no longer implies cache reuse because cache_strategy remains
full_scan and files_reused=0.
excluded: warm Project reconstruction, partial-rescan cache, and reused index
facts. Those remain later cache work and must not be claimed yet.
proof: full gate.
review: PASS.
live: not required for this first diagnostics boundary.
next: decide whether to make a safe warm load for selected commands or move to
git structural events if timing diagnostics show changed/doctor pain first.
```

```txt
closed next boundary: cache now reuses unchanged cached FileInfo facts, rescans
only changed or added candidate files, removes deleted files from cached facts,
and rebuilds derived packages/imports/reverse-imports/domains over the mixed
cached plus rescanned map. Warm exact hits report scan_ms=0 and
scanner.files_visited=0.
proof next: incremental changed/added/deleted/reverse-import fixtures, full
local gate, and live warm probes on ctx, spritestudio, Sillentway-VPN, and
Levelly-1.
review next: not run for this continuation; full gate plus live cache probes
covered the boundary.
live next: warm_load with scan_ms=0 on all four probed repos.
```

```txt
closed third boundary: for committed git repos with matching cached HEAD, cache
freshness now uses `git status` as the mismatch set before parser work. Cached
untracked files are stored per path and probed directly, so deleted/modified
untracked source does not require a full candidate delta.
proof third: `doctor_uses_git_status_mismatch_set_for_committed_repos`,
`doctor_uses_git_status_mismatch_set_for_untracked_paths_with_spaces`,
`doctor_uses_cached_untracked_probe_for_deleted_untracked_files`,
`doctor_removes_cached_untracked_file_that_becomes_git_ignored`,
`doctor_uses_cached_untracked_probe_for_modified_untracked_files`,
`doctor_removes_cached_source_renamed_into_ignored_directory`, full gate, and
live warm sanity on ctx/spritestudio/Sillentway-VPN.
review third: PASS after the cache-specific status parser preserved old-path
removals for renames into ignored directories.
```

```txt
closed fourth boundary: when cached HEAD differs from current HEAD, cache now
uses git's committed name-status delta instead of building the full candidate
file list first. It rescans only changed/added cache candidates, removes deleted
paths, merges live status mismatches, and rechecks cached fingerprints directly
after invalid dirty-cache probes. Exact `--root <subdir>` paths are normalized
against git top-level output to avoid stale cached symbols.
proof fourth: `doctor_uses_head_delta_after_committed_change`,
`head_delta_normalizes_paths_for_exact_subdir_root`, existing cache dirty status
tests, full gate, and read-only live warm probes on spritestudio/Sillentway-VPN.
review fourth: first reviewer BLOCK on subdir-root stale cache risk; final
review PASS after root-aware diff normalization and regression coverage.
```

### Slice 10: Git Structural Events, `changed`, And `diff-map`

```txt
closed: comment-only edits no longer create changed_symbols, runtime routes, or
proof surfaces; exported symbols are marked changed only when changed
current non-comment code intersects the current symbol line range; removed
import/export lines remain removed_edges/removed_exports instead of false symbol
body deltas.
excluded: full git structural event matrix for deleted/renamed/typechanged/
conflicted/lockfile/generated ownership cases, plus removed-line symbol body
detection until base symbol ranges exist.
proof: controlled fixture and full gate.
review: PASS after blockers were fixed.
live: not required for this boundary; controlled fixture proves the false-claim
case more directly than dirty live repos.
```

```txt
closed next boundary: `changed` now exposes deterministic git structural events
for deleted and renamed paths, with parser support for typechanged/conflicted
events but no full closure claim for those states. Deleted files surface as
`removed_anchor` and expand to `diff-map` for removed edges/exports; renamed
files preserve `old_path -> path` as `renamed_anchor`; each event carries
`git_status` evidence, provenance location, effect text, and an exact expand
command. Dirty git-state parsing now uses NUL-delimited porcelain output so
paths with spaces are preserved. Renames into ignored directories degrade to a
removed old source anchor instead of a false clean changed report. The changed
JSON schema is bumped to v2 for the new public `structural_events` field.
excluded next: lockfile/generated ownership, base-symbol removed-line analysis,
and committed typechanged/conflicted regression coverage remain later work.
proof next: controlled deletion+rename dirty fixture, rename-with-spaces
fixture, dirty and staged rename-into-ignored-dir fixtures, changed schema
validation, schema manifest parity, line budget, and full gate.
review next: PASS after fixing porcelain path quoting, deleted-file expand, and
staged/since rename-into-ignored false-clean behavior.
live next: read-only changed probes on ctx, spritestudio, and Sillentway-VPN
confirmed shape without mutating live repos; deletion/rename truth is proven by
controlled dirty/staged fixtures.
```

### Slice 11: Symbol, Import, And Export Extraction Matrix

```txt
closed: unresolved local imports are recorded in FileInfo and surfaced as typed
unresolved_import unknowns in exact file cones, with line provenance where
available.
excluded: full extraction matrix closure, non-code import resolution,
unresolved external dependency diagnostics, and source-owner/codegen import
repair.
proof: controlled fixture and full gate.
review: PASS.
live: not required for this boundary; a controlled fixture proves the false
omission directly.
```

### Slice 15: Runtime Lens

```txt
closed: static JavaScript direct/chained/object route registrations, Go
HandleFunc registrations, and Next route.ts exported HTTP methods carry
handler_symbol when the handler is a simple deterministic symbol. `flow` adds a
route_handler step only when that symbol exists in the route owner file.
excluded: Rust axum/tower route handler extraction, imported handler ownership,
and multi-line route registration parsing.
proof: runtime extractor fixtures, Next route flow fixture, schema validation,
and full local gate.
review: not required for this first bounded stitching boundary unless the route
model expands again.
live: spritestudio `runtime app/api/health/route.ts` reports GET handler_symbol
GET, and `flow 'GET /api/health'` reaches `app/api/health/route.ts#GET`.
closed next boundary: Rust axum/tower single-line `.route("/path",
get(handler).post(other))` registrations now produce exact runtime routes and
handler_symbol where the handler is a simple local symbol.
excluded next: imported handler ownership and multi-line route registration
parsing.
proof next: `rust_axum` structural tests plus live SilentWay `runtime
src/doh-gateway/src/main.rs` and `flow 'POST /dns-query'` reaching
`src/doh-gateway/src/main.rs#handle_post`.
```

### Slice 17: Proof-map And Proof Safety

```txt
closed: `proof --run` now fails closed before execution for placeholder,
deploy/release/publish, migrate/database mutation, cluster/infra mutation,
remote shell/copy/sync, broad network, destructive file, service-startup, and
unknown shell commands. It only runs a bounded set of known test/check/build/lint
commands and read-only codemap checks by default.
excluded: full public JSON safety classification and proof-map taxonomy closure;
this boundary only protects execution safety for `--run`.
proof: unit tests for placeholder, deploy, unknown shell, shell-control suffix,
unsafe script name, cd scope escape, dedupe, scoped safe test commands, generated
JS test runners, and codemap self-command resolution; full gate required because
this changes command execution behavior.
review: PASS.
live: not required; controlled non-execution tests prove the safety boundary
without touching live repos.
```

### Slice 16: Contract Lens

```txt
closed: `contract <anchor>` now exposes package manifest exports as
`package_exports` structural edges with `package_manifest` evidence, and
`public_surface` is true when a package export points at the anchor. The contract
schema is bumped to v2 for the new public JSON field.
excluded: full public/schema/API taxonomy, cross-language public entry surfaces,
and contract-proof taxonomy closure.
proof: package export fixture validates delete and contract lenses share the
same package export fact; schema manifest/schema command parity fixture.
review: not required for this narrow shared-fact/schema boundary unless the
contract taxonomy expands further.
live: not required; controlled package manifest fixture proves the public export
edge directly.
```

### Slice 18: Impact Lens

```txt
closed: `impact --files <package-export-target>` now reuses package manifest
export evidence as a contract risk, so changed/impact does not understate public
package entrypoint edits as local implementation changes.
excluded: full impact grouping closure, deleted/renamed structural event matrix,
runtime/data/generated impact taxonomy, and broader live output audit.
proof: package export fixture asserts contract impact risk and exact
`package_export` / `package_manifest` edges in `contract_risks` for both the
export target and package manifest; changed overview fixture proves the daily
`codemap changed` path inherits the same package export contract impact.
review: required before commit because this changes public lens semantics.
live: not required; controlled package export fixture proves the false-negative
directly.
```

### Slice 19: Delete Lens

```txt
closed: delete checklist now includes direct-user cleanup when direct consumer
edges exist, so the blocker map does not omit the first mechanical deletion
step. The lens still avoids `safe` / `probably unused` language.
excluded: full symbol-user checklist fixtures, generated ownership, and
dynamic-reference closure remain later work.
proof: focused delete fixture validates direct users and checklist item; schema
unchanged.
review: not required; narrow checklist content derived from existing evidence.
live: not required; controlled fixture proves the missing checklist case.
```

### Slice 20: Boundary-Map Lens

```txt
closed: scoped `boundary-map <scope>` now filters explicit forbidden findings
to findings whose `from` or `to` path is inside the requested scope. This keeps
the lens read-only and current-scope instead of dumping global config findings.
excluded: full boundary taxonomy, domain ownership semantics, and live
cross-repo boundary audit remain later work.
proof: focused fixture verifies an unrelated test scope does not receive the
app-to-replay forbidden finding; schema unchanged.
review: not required for this narrow scope-filter fix.
live: not required; controlled semantic-anchor fixture proves the leak.
```

### Slice 22: Siblings And Place Lenses

```txt
closed: `place <scope> --kind <kind>` now emits executable expand commands that
preserve the required `--kind` argument, including include-hidden drill-downs.
excluded: full siblings/place convention taxonomy, semantic similarity, and
recommendation behavior remain out of scope.
proof: focused place fixture validates the expand command; schema remains
unchanged because this is command content, not report shape.
review: not required; this is a narrow expand reproducibility fix.
live: not required; a fixture catches the exact invalid-command regression.
```

## Live Probe Set

Use this only for live-relevant slices:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio ls .
codemap --root /Users/amir/Documents/projects/spritestudio changed
codemap --root /Users/amir/Documents/projects/spritestudio proof --changed

codemap --root /Users/amir/Documents/projects/Sillentway-VPN ls .
codemap --root /Users/amir/Documents/projects/Sillentway-VPN changed
codemap --root /Users/amir/Documents/projects/Sillentway-VPN proof --changed

codemap --root <third-project> ls .
codemap --root <third-project> changed
codemap --root <third-project> proof --changed
```

Prefer `scripts/dogfood-codemap.sh` when the whole harness is relevant. For a
specific slice, run only the commands that prove that slice boundary.

Record:

```txt
Repo:
Commands:
Runtime:
What became clearer:
Where manual ls/rg was still needed:
Noise or duplication:
Questionable or false claim:
Would I voluntarily use it again:
Decision:
```
