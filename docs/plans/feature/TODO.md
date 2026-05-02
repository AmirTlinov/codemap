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
| 01 Product lock, baseline inventory, invariant guards | todo | focused | Product invariant guards and baseline truth are explicit. |
| 02 Daily command surface and alias cleanup | todo | focused/live | Daily surface is `ls`, `cone`, `changed`, `proof`, `doctor`; focused lenses remain discoverable. |
| 03 Unified fact model and constructors | todo | full | Shared constructors prevent lenses from inventing separate fact logic. |
| 04 Evidence locations and typed unknowns | todo | full | Important claims carry provenance; blind spots are typed. |
| 05 Scope model, root maps, expand protocol | todo | focused/live | Root is current-level; exact anchors drill down; expand is reproducible. |
| 06 Compact markdown grammar and renderer | closed | live | Daily/focused markdown was compacted within the declared boundary. |
| 07 Schema rail and golden validation | closed | full | Public JSON reports validate against manifest-selected schemas. |
| 08 Fast scanner, ignore/generated/vendor detection | closed | live | Shared scanner policy and generated/ignored stats are visible. |
| 09 Warm cache, indexes, incremental freshness | closed | full | Honest cache diagnostics landed; real warm reuse remains excluded. |
| 10 Git structural events, changed, diff-map | closed | full | Comment-only edits no longer create false structural deltas. |
| 11 Symbol/import/export extraction matrix | closed | full | Exact file cones surface unresolved local imports as typed unknowns. |
| 12 Package/workspace graph and boundaries | closed first boundary | full | Deterministic dependency kind on package edges. |
| 13 `ls`, `graph --lens causal`, root map quality | closed first boundary | focused | Root causal graph normalizes workspace package coordinates. |
| 14 `cone` exact anchors and directory aggregation | closed first boundary | focused | Missing symbol anchors fail closed without whole-file fallback. |
| 15 Runtime lens | todo | focused/live | Deterministic execution entrypoints stitch to code where known. |
| 16 Contract lens | todo | focused/live | Public/schema/API surfaces are separated from implementation names. |
| 17 Proof-map and proof safety | todo | focused/live | Proof is a sensor map; `--run` stays safe by default. |
| 18 Impact lens | todo | focused/live | Blast radius is derived from structural edges and grouped compactly. |
| 19 Delete lens | todo | focused | Deletion blockers are mapped without `safe`/`probably unused` claims. |
| 20 Boundary-map lens | todo | focused/live | Crossings are read-only facts; forbidden findings require explicit config. |
| 21 Flow lens | todo | focused/live | Bounded structural paths stop at typed unknowns. |
| 22 Siblings and place lenses | todo | focused | Local conventions are shown without ranking or recommendations. |
| 23 Non-code, assets, data, events, generated ownership | todo | focused/live | CSS/assets/config/data/events/generated files become first-class surfaces. |
| 24 Unknown taxonomy, scope repair, fail-closed traversal | todo | full | Empty scopes and unsupported traversal fail closed with useful expand. |
| 25 Performance, path stability, cognitive gates | todo | live | Warm path, stable ordering, path normalization, and output budgets are guarded. |
| 26 Fixture matrix across stacks | todo | full | Fixtures cover supported stacks and public lenses without overfitting. |
| 27 Live adoption harness and PATH ergonomics | todo | live | Read-only live probes and local install/PATH ergonomics are usable. |
| 28 Final audit, cleanup, TODO closure | todo | final | Full system audit passes; TODO is honestly closed. |

## Closed Boundaries

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
