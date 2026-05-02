# Feature Plan: Codemap As The Daily Structural Map For Agents

This is the implementation plan for making `codemap` a deterministic structural
code-map CLI that an AI coding agent chooses before ad-hoc `ls`, `rg`,
`git diff`, and manual test discovery.

The target is not a smarter search engine. The target is a trustworthy map:

```txt
current-level orientation at the root
deeper structure for exact scopes and anchors
structural edges with evidence and locations
typed unknowns where certainty stops
compact markdown for agents
complete JSON for integrations
warm queries fast enough to become a reflex
```

The tool should become the agent's default lens for answering:

```txt
what is here?
what is connected to this?
where does execution enter?
what is public/schema/contract surface?
where are proof/test/e2e sensors?
what changed structurally?
what may break?
where are blind spots?
how do I expand one level deeper without dumping the repo?
```

## Product Invariants

These are hard constraints for every slice:

- no task router;
- no ranking engine;
- no embeddings;
- no semantic search;
- no LLM in the hard path;
- no generated architecture documents;
- no repo writes by default;
- no broad recursive root dumps by default;
- no fake `safe`, `best`, `recommended`, or `probably unused` language;
- no global confidence score as a trust answer;
- no pretending full callgraph or full dataflow exists;
- no docs/comments as hard evidence for dependencies or proof.

`codemap` may be incomplete. It must not be confidently false.

## Process Invariant

This plan must not turn into a ceremony engine. Validation scales with risk:

```txt
docs-only / planning cleanup:
  prove the document diff is coherent and whitespace-clean

small renderer or copy cleanup:
  run focused tests or command snapshots for the touched output

model, schema, cache, scanner, git-state, or lens semantics:
  run the full local gate and use an independent reviewer

final product closure:
  run full gates, fixture coverage, live dogfood, and final reviewer
```

Do not keep a slice open until every future edge case in the roadmap is solved.
Close the declared boundary once the load-bearing acceptance is true, record
excluded work explicitly, then move to the next higher-value slice.

Do not convert every roadmap item into five equal checkboxes. Track a slice by
its boundary, proof tier, review decision, live decision, exclusions, and next
move. If the proof or review does not protect the boundary, skip it and say why.

Do not spawn a reviewer or run the full live dogfood harness for cosmetic
markdown cleanup unless the cleanup changes agent-facing behavior, line-budget
contracts, or a false claim risk. The process is here to lower future repair
cost, not to become another product surface.

When a slice starts producing mostly polish commits, stop and run a closure
audit:

```txt
what is now true?
what is still false or excluded?
does another renderer tweak reduce false claims, or only taste?
what fact/schema/cache/lens correctness issue should move next?
```

For the current feature wave, the compact renderer boundary is closed. Further
renderer work needs a concrete confusing output or false structural claim.

## End-State Command Shape

The daily surface must stay small:

```txt
codemap ls <scope>              # where am I?
codemap cone <anchor>           # what touches this?
codemap changed                 # what changed in the structural map?
codemap proof <scope|--changed> # how can I prove this?
codemap doctor                  # is the tool/cache healthy and fast?
```

Focused lenses remain available, but the agent should discover them from
`expand`, not memorize a 30-command menu:

```txt
codemap graph --lens causal
codemap runtime <scope>
codemap contract <anchor>
codemap flow <anchor>
codemap boundary-map <scope>
codemap siblings <scope>
codemap place <scope> --kind <kind>
codemap delete <anchor>
codemap diff-map --changed
codemap impact --changed
codemap proof-map --changed
```

Aliases may exist only when they reduce ritual without hiding meaning. They
must not resurrect `start`, `read_first`, `source_of_truth`, or task routing.

Execution priority matters:

```txt
first make the daily surface usable
then compact the map language
then harden shared facts and deep lenses
```

`changed` should exist as a vertical daily MVP early. Slice 10 later turns it
into a complete git structural-event engine; it is not the first moment the
command becomes visible.

Once the daily surface and compact renderer have enough closure to be useful,
prefer work that improves factual correctness over additional presentation
polish:

```txt
package/workspace dependency facts
git structural events that remove false deltas
runtime-to-code stitching
proof sensor provenance
cache correctness and timing truth
```

## Unified Fact Layer

All lenses must be projections over one indexed structural fact layer:

```txt
FileInfo
SymbolInfo
Surface
StructuralEdge
EvidenceLocation
Unknown
PackageInfo
PackageDependency
RuntimeRoute
ProofSurface
GitStructuralEvent
```

No lens should have its own private parser for facts already represented in the
shared model. New extraction work should add facts first, then render them
through lenses.

Required fact rules:

- edges carry `from`, `to`, `type`, `evidence`, `strength`, and `locations`;
- surfaces carry `id`, `kind`, `path`, `role`, `evidence`, `strength`,
  `locations`, `count`, `hidden_count`, and examples where useful;
- unknowns carry `kind`, `path`, `line`, `reason`, `effect`, and `expand`;
- every hard/high claim has a source location where the file format allows it;
- dynamic or ambiguous constructs become unknowns, not invented edges.
- non-code files, generated outputs, data/config surfaces, and event channels
  are first-class `Surface` facts, not fake source-code symbols.

## Output Grammar

Markdown must be one compact map language across all lenses:

```txt
scope
summary
map
relations
runtime
contracts
proof
unknowns
hidden
expand
```

The sections are optional per lens, but their order and meaning are stable.

Rendering rules:

- print common path prefixes once;
- show children relative to the active scope;
- group edges by source or current-level container;
- group proof sensors by command or proof container;
- show symbol names under their file instead of repeating full paths;
- use location hints like `file.ts:42`, not repeated evidence columns;
- hide detail behind explicit `expand` commands;
- include `hidden` count whenever output is truncated;
- keep root markdown normally under 150 lines;
- keep JSON complete even when markdown is compact.

## Root Versus Exact Anchor Behavior

Root-level commands show the current-level map:

```txt
packages
domains
top-level dirs
scripts
runtime containers
contract containers
proof containers
boundary crossings
hidden recursive counts
explicit expand commands
```

Root-level commands must not dump every file, test, route, or import.

Exact file/scope commands may go deeper:

```txt
symbols
exports
imports
reverse imports
direct tests
runtime refs
contract refs
unknowns
line locations
```

Directory anchors aggregate at the directory level first. They should only
expand to file-level details when explicitly asked.

## Trust Model

The trust model is evidence-first:

- `Hard`: manifest, exact syntax, exact file convention, explicit config.
- `High`: resolved imports, exported symbols, static route registrations,
  static env lookups, static test imports.
- `Medium`: naming convention plus structural corroboration.
- `Low`: useful soft surface, never primary proof.

Comments, README prose, and docs are soft surfaces only. They can help orient
but cannot prove imports, runtime paths, proof sensors, or public contracts.

Typed unknowns are first-class output. Required kinds include:

```txt
dynamic_import
js_require_dynamic
unresolved_import
route_string_concat
route_dynamic_path
route_dynamic_method
env_dynamic_lookup
raw_sql_literal
unsupported_framework_route
ambiguous_route_owner
di_token_unresolved
macro_expansion_boundary
generated_source_owner_unknown
dynamic_asset_path
dynamic_event_topic
reflection_boundary
ambiguous_symbol_anchor
cache_freshness_uncertain
```

## Performance Model

The agent will ignore `codemap` if it is slower than manual shell habits.

Target paths:

```txt
cold:
  walk repo
  detect ignore/generated/vendor boundaries
  extract facts
  build indexes
  write external cache

warm:
  cheap fingerprint
  load indexed facts
  query scoped index
  render compact map

dirty:
  read git state
  rescan changed files only
  update affected indexes
  compute structural events
```

Cache correctness must cover deleted and renamed files, branch changes, changed
manifests/config, symlinks, path aliases, case-insensitive filesystems, Unicode
paths, and paths with spaces. If correctness is uncertain, the cache must be
invalidated or reported as suspect; stale facts are worse than slow output.

Targets for live dogfood:

```txt
small warm ls/cone:       < 200ms
medium warm ls/cone:      < 500ms
large warm ls/cone:       < 1s
medium changed/proof:     < 1s
large changed/proof:      < 2s
cold small repo scan:     < 1s
cold medium repo scan:    < 5s
```

Silent slowdown is a blocker. If a target is missed, `doctor` must explain why:
cache miss, repo size, ignored directories, unsupported file type, git state,
or extraction hotspot.

## Schemas And Compatibility

Every public JSON report must have a schema and manifest entry:

```txt
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
doctor
status
files
```

Focused and diagnostic commands that expose public JSON must either use one of
these schemas or add their own manifest entry. Breaking report changes bump
schema versions. Legacy/router schemas must be
absent unless explicitly quarantined as compatibility and not used by structural
lenses.

## Validation Loop

Each slice declares its validation tier before closure.

Full structural slices use:

```txt
cargo fmt --check
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo run --quiet --bin codemap -- doctor
git diff --check
```

Focused code slices may replace the full suite with targeted tests plus the
smallest command probe that proves the changed behavior, unless the change
touches shared model/schema/cache/scanner/git-state code.

Docs-only slices use:

```txt
git diff --check -- docs/plans/feature
```

Reviewer subagents are required for:

- model/schema/cache/scanner/git-state changes;
- new or materially changed public lens behavior;
- performance/correctness claims;
- final audit and TODO closure.

They are optional for:

- typo fixes;
- plan/TODO cleanup;
- narrow renderer polish inside an already closed boundary;
- test-only additions that do not change behavior.

When used, the reviewer returns:

```txt
PASS
CHANGES
BLOCK
```

The reviewer must inspect:

- router/ranking/semantic leakage;
- fake structural claims;
- dynamic code turned into hard facts;
- schema mismatch;
- root recursive dumps;
- stale cache risk;
- unbounded output;
- missing hidden/expand;
- missing evidence/location;
- duplicate path spam;
- fake proof sensors;
- overfitted fixtures;
- whether the agent would still prefer manual `rg`.

No slice is done until its declared validation tier passes. If reviewer or live
dogfood is intentionally skipped, the slice notes must say why.

## Live Dogfooding

Use live dogfood when the change affects user-facing command behavior,
performance, cache freshness, git state, scanner visibility, or final adoption.
The required live repo set is:

```txt
/Users/amir/Documents/projects/spritestudio
/Users/amir/Documents/projects/Sillentway-VPN
one additional repo under /Users/amir/Documents/projects
```

For docs-only, schema-only parity tests, or narrow internal refactors, use a
focused local proof and record `live dogfood not required` with the reason. If a
slice's `Live Dogfood` section lists slice-specific examples, run those examples
only when they prove the slice boundary better than the full harness.

Live dogfood scripts must be read-only with respect to target repositories. They
may write summaries only under this repo's `target/` or another explicitly named
artifact directory outside the probed repo.

For each live probe record:

- exact commands run;
- runtime numbers where relevant;
- what became clearer than manual shell work;
- where manual `ls`/`rg` was still needed;
- whether output was noisy or duplicated;
- whether any claim looked false;
- whether the agent would voluntarily use it again.

## Slice Sequence

This sequence is a roadmap, not a demand to polish every slice to world-perfect
completion before moving on. Each slice needs a closure boundary:

```txt
closed: what is now genuinely true
excluded: what remains later work
next: the next highest-value slice
```

If a slice starts producing only cosmetic micro-commits, stop and run a closure
audit. The next work should move back to facts, schemas, cache, or lens
correctness unless the cosmetic issue causes real agent confusion.

1. Product lock, baseline inventory, and invariant guards.
2. Daily command surface and alias cleanup.
3. Unified fact model and constructors.
4. Evidence locations and typed unknowns.
5. Scope model, current-level root maps, and expand protocol.
6. Compact markdown grammar and renderer.
7. Schema rail and golden report validation.
8. Fast scanner, ignore rules, generated/vendor detection.
9. Warm cache, indexes, and incremental freshness.
10. Git structural events, `changed`, and `diff-map`.
11. Symbol/import/export extraction matrix.
12. Package/workspace graph and boundary primitives.
13. `ls`, `graph --lens causal`, and root map quality.
14. `cone` exact anchors, directory aggregation, and symbol anchors.
15. Runtime lens: scripts, entrypoints, routes, env, CI, jobs.
16. Contract lens: public exports, schemas, DTOs, API surfaces.
17. Proof-map and proof command safety.
18. Impact lens from structural edges.
19. Delete lens without safety claims.
20. Boundary-map read-only crossing lens.
21. Flow lens as bounded structural path.
22. Siblings and place convention lenses.
23. Non-code, UI assets, data, events, and generated-code ownership.
24. Unknown taxonomy, scope repair, and fail-closed traversal.
25. Performance gates, cognitive output gates, path normalization, and
    regression budget.
26. Fixture matrix across stacks.
27. Live adoption harness and local PATH ergonomics across real projects.
28. Final audit, cleanup, and TODO closure.

## Final Stop Condition

The plan is complete only when:

- all public lenses exist and have schemas;
- daily commands are enough for most agent work;
- root views are bounded current-level maps;
- exact anchors expose useful local detail;
- dynamic blind spots are typed unknowns;
- warm speed is good enough to beat manual habits;
- markdown is compact and non-duplicative;
- JSON remains complete;
- proof is a sensor map, not a fallback command dump;
- no known false structural claim remains unfixed;
- live dogfood on multiple real repos is acceptable;
- a local `codemap` binary is installable or linked into `PATH` for daily use;
- final reviewer returns PASS;
- `TODO.md` is honestly checked.
