# Codemap Live Dogfood Notes

## 2026-05-04

- Issue: X-Ray cone output still inherits the compact snapshot header instead of the full product snapshot
  contract.
  - Evidence: live local build after the X-Ray slice, repo `/Users/amir/Documents/projects/tools/cli/ctx`,
    command `cargo run --quiet --bin codemap -- cone src/map/cone_xray.rs --limit 8`.
  - Observed: header shows `Map Snapshot: root=...; head=...; fingerprint=...`, but not dirty count,
    branch, cache status/location, schema version, or repo footprint.
  - Why it hurts trust: the new X-Ray Card is meant to be compared across live repo states; without dirty/cache
    metadata in the same output, an agent can still compare maps from different states or cache paths as if
    they were the same reality.
  - Boundary: this is not a false edge in the new card. It is remaining snapshot/provenance debt shared by
    map renderers.
  - Follow-up: extend `map_snapshot_line` to include branch/dirty/cache/schema/repo_footprint in every primary
    map output, while keeping it one compact line.
  - Status: tightened in `0.2.20`; live local `cone` now renders branch, dirty count, cache state/strategy,
    external cache location, schema, and `repo_footprint=zero` in the snapshot line.

- Issue: `changed` still renders JSON Schema additions as generic config-key mutations.
  - Evidence: live local build after the X-Ray slice, repo `/Users/amir/Documents/projects/tools/cli/ctx`,
    command `cargo run --quiet --bin codemap -- changed --section observed --limit 12`.
  - Observed: `schemas/cone.schema.json` reports `added_config_key` events for keys such as
    `direct_consumers`, `examples`, `flow_step`, and `flow`.
  - Why it hurts trust: these keys belong to a schema contract surface, not runtime config. Calling them
    config keys makes the structural delta sound more like app behavior/config drift than schema-shape drift.
  - Boundary: the key additions are real source-backed facts; the public mutation role label is too generic.
  - Follow-up: classify JSON Schema/OpenAPI/schema-contract files before generic JSON config-key diffing, or
    render these events as schema-field/schema-shape delta.
  - Status: fixed in `0.2.20`; schema-contract JSON key additions now render as `added_schema_field` /
    `git_diff_schema_field`. Regression:
    `changed_schema_contract_json_keys_are_schema_fields_not_config_keys`.

- Issue: cold/inventory root fast paths still render cache provenance as unknown.
  - Evidence: installed PATH `codemap 0.2.20`, real repos `/Users/amir/Documents/projects/Sillentway-VPN`
    and `/Users/amir/Documents/projects/main_cluster`, command `codemap --root <repo> ls .`.
  - Observed: snapshot line includes `cache=unknown strategy=unknown location=unknown` even though the
    command knows the resolved root and can derive the external cache path.
  - Why it hurts trust: snapshot provenance becomes uneven between full-load maps and cold fast-path maps,
    so an agent cannot tell whether the root map came from an inventory fast path, cache hit, or unavailable
    cache.
  - Boundary: the map content remains source-backed; the misleading part is the snapshot/cache header.
  - Follow-up: make inventory fast paths use a cache-aware snapshot setter with `cache=inventory` or `cold`
    and the derived external cache path.
  - Status: fixed in `0.2.20`; inventory fast paths now render `cache=cold/stale`, `strategy=inventory_fast_path`,
    external cache location, and `repo_footprint=zero`. Regression:
    `cold_large_root_ls_uses_bounded_inventory_map`.

- Status: fixed in `0.2.19`; `proof changed --all` no longer writes the bounded `proof changed`
  lens cache.
  - Evidence: regression `proof_changed_all_does_not_poison_default_lens_cache`.
  - Why it mattered: an expanded one-off read could previously make the normal preflight look less bounded
    and hide its default `Hidden` accounting.

- Status: fixed in `0.2.19`; clean fast-path snapshot headers use a fresh bounded inventory fingerprint
  instead of trusting a stale cached `status.json` fingerprint.
  - Evidence: regression `clean_changed_fast_path_has_snapshot_fingerprint`; reviewer repro addressed.
  - Boundary: cached lens fast paths still use cached fingerprints only after cache/current-state validation.

- Issue: `place <file>` no longer fails without `--kind`, but the default file map can still be nearly empty.
  - Evidence: live PATH `codemap 0.2.16`, repo `/Users/amir/Documents/projects/tools/cli/ctx`, command
    `codemap place src/map/proof_entry.rs`.
  - Observed: output shows snapshot, scope, default `Kind: source`, and expand commands, but no existing
    surfaces or placement-neighborhood facts for the concrete file.
  - Why it hurts trust: this is not a false edge, but it keeps a quick map command from helping when an agent
    naturally asks "where does this file fit?".
  - Boundary: `place` is an expand lens, not one of the four primary map commands.
  - Follow-up: for file anchors, render same-kind siblings, package/domain surface, and local convention
    evidence before falling back to a generic placement expansion.

- Issue: `proof` and `proof-map --raw-sensors` can still disagree on fail-open Unknown for the same source
  anchor.
  - Evidence: live PATH `codemap 0.2.17`, repo `/Users/amir/Documents/projects/Sillentway-VPN`, anchor
    `src/masque-core/src/client/routing.rs`.
  - Observed: `codemap proof ...routing.rs` prints `direct_test_import_not_found`, while
    `codemap proof-map ...routing.rs --raw-sensors` shows only Mediated Evidence and Soft Token Evidence with
    no `## Unknown`.
  - Why it hurts trust: the proof-map text says mediated/soft evidence does not remove Unknown entries, but
    the lens omits the missing-direct Unknown that `proof` reports for the same anchor.
  - Follow-up: make proof-map source anchors emit the same missing-direct deterministic proof Unknown when only
    mediated/soft proof sensors exist.
  - Status: fixed in `0.2.19`; exact source anchors now keep proof-map fail-open Unknown when no runnable or
    direct deterministic proof sensor exists. Regression extends
    `soft_token_proof_does_not_hide_missing_deterministic_proof_or_fallback`.

- Issue: `changed --section unknown --all` can still hide repeated Unknown rows.
  - Evidence: reviewer repro on live `0.2.18` diff in `/Users/amir/Documents/projects/tools/cli/ctx`.
  - Observed: `target/debug/codemap changed --section unknown --all` still printed grouped rows such as
    `direct_test_import_not_found ... hidden: 22 unknowns`.
  - Why it hurts trust: `--all` must mean the current map lens is expanded. Collapsing fail-open Unknowns after
    the agent explicitly asks for all makes missing evidence look partially withheld.
  - Status: fixed in `0.2.19`; Unknown auto-compaction is disabled for expanded display limits. Regression:
    `compact_unknown_samples_use_single_code_spans` now also checks `changed --section unknown --all`.

- Status: fixed in `0.2.19`; map snapshot headers now include current git `head` plus map fingerprint.
  - Evidence: markdown outputs render `Map Snapshot: root=...; head=...; fingerprint=...`.
  - Boundary: cache strategy/warm/stale diagnostics remain in `status`/`doctor`, not every map header.

- Status: fixed in `0.2.19`; schema/manifest/env owner surfaces no longer inherit source-centric
  `direct_test_import_not_found` Unknowns.
  - Evidence: regressions `schema_proof_unknowns_are_schema_specific_not_source_direct_test` and
    `changed_unknown_is_fail_open_for_owner_surfaces`.
  - Boundary: source files still get missing direct-test Unknowns; owner surfaces get role-specific missing
    schema/script/CI/env/consumer evidence.

- Status: fixed in `0.2.19`; proof-runner cones now show bounded runner/receipt/doc/script neighbor rails as
  `Soft Evidence`, not direct proof.
  - Evidence: regression `proof_runner_cone_shows_soft_neighbor_rails_without_calling_them_proof`.
  - Boundary: these are token/path/name neighbor links with provenance; they do not close Unknown or prove a
    receipt is honest.

- Status: fixed in `0.2.19`; deeper cone imports are labeled `resolved_import_via_cone_depth` with medium
  strength instead of looking like direct anchor imports.
  - Evidence: regression `cone_depth_edges_are_marked_mediated_after_the_anchor_layer`.
  - Boundary: direct anchor imports remain `resolved_import`.

- Status: fixed in `0.2.19`; per-surface sample overflow now says `additional examples` instead of `hidden`
  so `--all` does not look self-collapsed.
  - Evidence: regression `root_ls_all_does_not_emit_self_referential_hidden_expand`.
  - Boundary: real report-level hidden groups still render under `Hidden` with exact expand commands.

- Issue: compact Unknown samples render with doubled code backticks.
  - Evidence: live PATH `codemap 0.2.16`, repo `/Users/amir/Documents/projects/tools/cli/ctx`, command
    `codemap changed --section unknown`.
  - Observed: sample paths appeared as ``src/cache.rs`` instead of `src/cache.rs`.
  - Why it hurts trust: this is not a false edge, but it makes the compact fail-open section look sloppy and
    harder to scan during preflight.
  - Follow-up: reuse the already-formatted `unknown_where` string directly in compact Unknown renderers.
  - Status: fixed in `0.2.19`; regression `compact_unknown_samples_use_single_code_spans`.

- Issue: `ls . --all` can still hide link edges inside the `Links` section.
  - Evidence: live PATH `codemap 0.2.16`, repo `/Users/amir/Documents/projects/PABG`, command
    `codemap --root /Users/amir/Documents/projects/PABG ls . --all`.
  - Observed: top-level `Hidden` was gone, but `Links` still printed `hidden: 244 links edges` without an
    exact expand command.
  - Why it hurts trust: after `--all`, hidden wording inside a stable section makes the agent doubt whether
    it is seeing the requested expanded map.
  - Follow-up: when an `ls` report has no hidden groups because it is already expanded, render all link edges
    instead of applying the default 20-edge display cap.
  - Status: fixed in `0.2.19`; regression `root_ls_all_does_not_emit_self_referential_hidden_expand`.

- Issue: `codemap changed` can still become too slow and too noisy on real mixed repos with a small dirty state.
  - Evidence: live PATH `codemap 0.2.10`, dogfood run `target/dogfood-live-wide-2026-05-04`, repo `/Users/amir/Documents/projects/PABG`, probe `changed`.
  - Observed: status 0, but `elapsed_ms=5542` against the 3000 ms dogfood budget, and `lines=135` against the 120-line budget.
  - Why it hurts trust: `changed` is a primary preflight command; when it exceeds both latency and compactness budgets, an agent may treat a noisy map as complete repo truth or skip reading the real diff.
  - Boundary: this is not a correctness verdict on PABG. It is a codemap UX/performance defect for the primary changed map on a real repository.
  - Follow-up: reduce default changed output for mixed repos by collapsing lower-signal proof/unknown/detail groups earlier, while preserving explicit Unknown and exact expand commands.

- Issue: non-primary local lenses can still block the agent flow on real mixed repos.
  - Evidence: same live dogfood run, repo `/Users/amir/Documents/projects/PABG`, anchor `apps/web/.storybook/main.ts`.
  - Observed: `delete apps/web/.storybook/main.ts` completed with status 0 but took `2807ms` against the 2000 ms budget; `siblings apps/web/.storybook` took `2770ms` against the 2000 ms budget.
  - Why it hurts trust: these commands are expand targets, not primary commands, but slow local probes make the tool feel unpredictable after the primary map points an agent at them.
  - Boundary: this is a performance/ergonomics defect, not proof that the reported edges are wrong.
  - Follow-up: add or reuse bounded inventory/cache-aware paths for focused local lenses, especially when the anchor is a config/support directory.

- Issue: large monorepos still have slow diagnostic/expand probes even after fast root `ls` and root causal `graph`.
  - Evidence: live PATH dogfood run `target/dogfood-live-wide-2026-05-04`, repo `/Users/amir/Documents/projects/main_cluster`.
  - Observed: `doctor` took `5488ms` against the 3000 ms budget, `runtime .` took `5189ms` against the 3000 ms budget, `proof-map .` took `2247ms` against the 2000 ms budget, and `siblings agents` took `2087ms` against the 2000 ms budget.
  - Why it hurts trust: the primary commands are now fast, but their expand targets can still feel like a sudden full-repo tax, which weakens codemap as a before-read preflight tool.
  - Boundary: this is not a false edge/provenance finding; it is a live efficiency gap on a real large repo.
  - Follow-up: prioritize bounded fast paths for root `runtime`, root `proof-map`, and scoped `siblings`, and make `doctor` expose expensive phases without requiring the whole scan for basic health.

- Issue: some agent-facing outputs still leak `role:*` labels after the trust-boundary cleanup.
  - Evidence: live output grep over `target/dogfood-live-wide-2026-05-04` and `target/dogfood-live-tail-2026-05-04`.
  - Observed examples: `runtime .` prints entries such as `Makefile [build_ci; build_ci; role:build_ci; high]`; `contract package.json` prints `Contract kind | role:public_boundary`; `siblings` / `place` can print `contract -> src/ [role:public_boundary:3; high]`.
  - Why it hurts trust: `role:*` looks like semantic classification rather than a deterministic surface hint. This weakens the product boundary that codemap shows repository facts and provenance instead of interpreting ownership/intent.
  - Boundary: the underlying paths may be useful and source-backed; the problem is the public vocabulary and evidence label shape.
  - Follow-up: remove or rename public `role:*` rendering in runtime/contract/siblings/place outputs to deterministic hint/provenance wording, and expand dogfood `trust_violations` to catch `role:` as well as `role=`.

- Issue: `changed` can misclassify proof/artifact JSON as config-key mutation.
  - Evidence: live `codemap --root /Users/amir/Documents/projects/PABG changed --section observed`.
  - Observed: files under `artifacts/147/20260503T223955863568000Z-147-proof/` such as `doctor.json`, `metrics.json`, `proof.json`, `reviewer_verdict.json`, and `try_to_end.json` are reported as `added_config_key` with effects like "config key `base_ref` was added".
  - Why it hurts trust: these are proof/receipt/artifact payloads, not runtime config knobs. Treating them as config mutations makes the map look more structurally meaningful than the repository evidence supports.
  - Boundary: reporting that JSON keys exist is source-backed, but the public role "config key" is too strong for artifact/witness paths.
  - Follow-up: classify artifact/receipt/witness JSON before generic JSON config-key diffing, or render this as artifact payload delta rather than config/runtime knob delta.

- Issue: changed surface grouping can contradict its own file hints.
  - Evidence: live `codemap --root /Users/amir/Documents/projects/Economy changed --section roles`.
  - Observed: `.obsidian/graph.json` and `.obsidian/workspace.json` are printed under Surface Hints `unknown`, while each row itself shows `[config; config]`.
  - Why it hurts trust: an agent cannot tell whether codemap knows these files are config or does not know how to classify them.
  - Boundary: this is not a missing file or missing evidence issue; it is an internal classification/rendering consistency bug.
  - Follow-up: make changed grouping use the same deterministic kind/hint pair that the row renderer uses, or explicitly label `.obsidian` as editor_config/support_config.

- Issue: changed compact anchors can say existing untracked files are `missing`.
  - Evidence: live dogfood output `target/dogfood-live-wide-2026-05-04/PABG_.changed_.md` plus filesystem check in `/Users/amir/Documents/projects/PABG`.
  - Observed: `banner_semantics_py.out`, `banner_semantics_unit.out`, and `banner_semantics_witness.out` were printed as `[missing; unknown]`, while `test -e` confirmed the files exist.
  - Why it hurts trust: "missing" reads as a git/filesystem fact, but the real fact is "unindexed by codemap". That can send an agent down the wrong diagnostic path.
  - Boundary: codemap correctly exposes `unindexed_anchor` later; the compact row wording is the misleading part.
  - Follow-up: render existing-but-unindexed files as `unindexed` / `unmapped`, and reserve `missing` for paths that do not exist in git or filesystem.

- Issue: fallback proof can render a non-executable sentence as a bash command.
  - Evidence: live output `target/dogfood-live-wide-2026-05-04/Economy_.changed_.md`.
  - Observed: the fallback block contains `run the nearest domain tests for the changed files` inside a `bash` code block.
  - Why it hurts trust: codemap's proof contract says it prints conservative runnable proof plans by default. A natural-language placeholder in a command block looks executable but is not.
  - Boundary: it is acceptable to say no deterministic proof command was found; it is not acceptable to disguise that as a runnable fallback.
  - Follow-up: replace non-executable fallback placeholders with explicit Unknown/no-command wording, or emit a real repo-local command only when it is source-backed.

- Issue: `cone` and `proof` can disagree on deterministic proof for the same anchor.
  - Evidence: live probes on `/Users/amir/Documents/projects/Sillentway-VPN`, anchor `src/masque-core/src/client/routing.rs`.
  - Observed: `codemap cone ...routing.rs --depth 1` lists proof edges including `test_import_via_direct_consumer; high`, while `codemap proof ...routing.rs` lists only soft token evidence and Unknown `direct_test_import_not_found`.
  - Why it hurts trust: agents use `cone` and `proof` together. If one command presents high proof and the other says direct proof is missing, the map contract is inconsistent.
  - Boundary: via-consumer evidence may be useful, but it must not look equivalent to direct proof for the anchor.
  - Follow-up: align proof sensors across `cone` and `proof`, or separate "via direct consumer" under soft/indirect evidence with an explicit Unknown that direct anchor proof was not found.

- Issue: Rust `include!` blind spot is not surfaced as Unknown.
  - Evidence: `/Users/amir/Documents/projects/Sillentway-VPN/src/silentway-app/src/bin/silentway-macos-lab.rs` contains multiple `include!(concat!(...))` calls, but `codemap cone src/silentway-app/src/bin/silentway-macos-lab.rs --depth 1` prints no Unknown about included fragments.
  - Observed: cone reports normal imports/contracts/proof only, which can look like a complete Rust dependency cone.
  - Why it hurts trust: included Rust fragments share lexical scope. If codemap does not resolve internal dependencies fully, it must say so instead of presenting a normal-looking cone.
  - Boundary: this is a missing Unknown, not necessarily a wrong edge.
  - Follow-up: detect Rust `include!` / `include_str!` / `include_bytes!` separately; for `include!` emit Unknown that included fragments may contain dependencies not represented in the cone.

- Issue: proof/artifact anchors can get unrelated fallback proof commands.
  - Evidence: live `codemap --root /Users/amir/Documents/projects/PABG proof artifacts/147/20260503T223955863568000Z-147-proof/proof.json`.
  - Observed: codemap prints fallback `cargo test` for a proof artifact JSON file, while also saying no deterministic proof sensors were found at that exact scope.
  - Why it hurts trust: root `cargo test` is a broad ritual fallback for an artifact payload, not a source-backed proof connection to that artifact.
  - Boundary: the Unknown is good; the fallback command is the misleading part.
  - Follow-up: suppress generic package fallback for artifact/witness/receipt/build-output anchors unless a producer/check script or CI edge is source-backed.

- Issue: artifact/witness cones still have inconsistent hints.
  - Evidence: live PABG cones for `artifacts/147/.../proof.json` and `artifacts/147/.../witnesses/.../banner_elimination_semantics_report.json`.
  - Observed: `proof.json` is `kind: config` but Surface Hints says `unknown`; the witness report is `kind: witness` but surface hints include `adapter, witness`.
  - Why it hurts trust: support artifacts must be clearly separated from source/adapter/config surfaces so agents do not edit or validate the wrong layer.
  - Boundary: codemap does identify one witness file as `kind: witness`; the issue is inconsistent secondary hinting and artifact JSON fallback to generic config.
  - Follow-up: normalize artifact/receipt/witness/build-output classification before generic config/source hints and make cone observed/hints use the same classifier result.
  - Status: tightened in `0.2.19`; support artifact JSON no longer renders public Surface Hints as `config`
    solely because the file language is JSON/config. Regression:
    `support_artifact_json_surface_hints_do_not_fall_back_to_config`.

- Issue: `codemap changed` can be very slow on a dirty large monorepo even when only a few files are changed.
  - Evidence: live `codemap --root /Users/amir/Documents/projects/main_cluster changed` after main_cluster had 6 dirty status rows.
  - Observed: command completed successfully but took about 18.6 seconds while rendering a 6-file changed map.
  - Why it hurts trust: `changed` is a primary command and should be cheap enough to run before normal file reads. A 18s preflight makes agents skip the map or over-rely on stale mental context.
  - Boundary: output content was mostly useful; this is a primary-command performance failure on a real repo.
  - Follow-up: add a dirty-state fast path for `changed` that starts from git paths and bounded owner-surface extraction before full project graph work.

- Issue: path/name classifiers can still over-semanticize source rows.
  - Evidence: same live `main_cluster changed` output.
  - Observed: `apps/control-center/lib/prosteq/local-preview-orders.ts` and `apps/control-center/lib/prosteq/local-preview.ts` are printed as `[renderer_ui; javascript/typescript; hints=application, renderer_ui]`.
  - Why it hurts trust: the path `lib/prosteq/local-preview*.ts` does not prove renderer/UI ownership. This is the kind of label drift that makes codemap look like it is interpreting intent.
  - Boundary: a weak hint such as `source` or `application` would be acceptable; `renderer_ui` is too specific without stronger evidence.
  - Follow-up: tighten `renderer_ui` classification to UI-framework/component paths or explicit JSX/TSX/component evidence, and keep ambiguous service/lib files as generic source/application hints.

- Issue: `ls --section unknown` is mostly non-functional.
  - Evidence: live `codemap ls . --section unknown` on `main_cluster`, `Levelly-1`, and `Sillentway-VPN`.
  - Observed: output only says `Typed unknowns are not computed by ls.`
  - Why it hurts trust: `unknown` is an advertised section. A generic "not computed" message does not tell the agent what classes were not checked for the repository shape.
  - Boundary: this is not false confidence; it is an underpowered section that fails to help the next read.
  - Follow-up: make root/scope `ls` emit bounded map-quality unknowns such as no detected package consumer graph, no contract registry, no schema proof rails, no env readers, or explicitly say which detector classes are not run for `ls`.

- Issue: some package-level edges use `aggregate` instead of actionable provenance.
  - Evidence: live `Artifact` outputs, for example `target/dogfood-live-tail-2026-05-04/Artifact_.graph_causal_.md` and `Artifact_.ls_root_.md`.
  - Observed: edges such as `crates/artifact-cli/ -> crates/artifact-core/` with evidence `resolved_import` or `package_manifest:artifact-core` show `Where` as `aggregate`.
  - Why it hurts trust: the edge may be true, but an agent cannot jump to the exact import or manifest row that proves it. `aggregate` is weaker than the product contract for non-trivial edges.
  - Boundary: aggregate package edges are useful summaries; they still need at least one representative source path/line or manifest path.
  - Follow-up: attach representative evidence locations to package-level aggregate edges, and collapse additional locations behind hidden/expand.

- Issue: soft script/token proof sensors can still render as `Proof Pattern` and `role_script_target`.
  - Evidence: reviewer pass over live dogfood outputs `target/dogfood-live-wide-2026-05-04/PABG_.siblings_scope_.md`, `main_cluster_.siblings_scope_.md`, and `PABG_.proof_changed_.md`.
  - Observed: `siblings apps/web/.storybook` prints `cargo test`, `make test`, and `make test-agent-runtime` under `Proof Pattern` using `[role_script_target; medium]`; `siblings agents` prints broad Makefile/package scripts as proof patterns for `agents/secrets/*`; `proof changed` can show only soft evidence while omitting a real `## Unknown` section.
  - Why it hurts trust: weak path/name/script overlap looks like actionable proof. It also leaks old role vocabulary through `role_script_target`.
  - Boundary: the script rows are source-backed and may be useful soft hints, but they are not deterministic proof for the anchor/scope.
  - Follow-up: render these rows only under `Soft Evidence` with the disclaimer, add/surface `Unknown` when no direct deterministic proof exists, and expand dogfood trust checks to catch `role_script_target` plus false `unknown_lines` counts.

- Issue: `cone` still renders soft proof edges under the generic `Proof` section.
  - Evidence: live PATH `codemap 0.2.12`, repo `/Users/amir/Documents/projects/Sillentway-VPN`, command `codemap --root /Users/amir/Documents/projects/Sillentway-VPN cone src/masque-core/src/client/routing.rs --depth 1`.
  - Observed: `cone` prints `test_surface_tokens`, `test_surface_tokens_via_direct_consumer`, and `test_import_via_direct_consumer` under `## Proof`, while `codemap proof src/masque-core/src/client/routing.rs` correctly renders the same set as `## Soft Evidence` and keeps `direct_test_import_not_found`.
  - Why it hurts trust: agents read `cone` and `proof` together. If `proof` says the evidence is soft but `cone` presents the same rows as proof, `cone` can still look stronger than the evidence supports.
  - Boundary: the edges are useful and source-backed; the issue is section semantics and hard/soft separation in the human cone output.
  - Status: fixed in the same `0.2.12` slice; repeated live PATH probe renders these rows under `## Soft Evidence`.

- Issue: `changed` proof sensor counts can contradict Unknown for mediated/soft proof.
  - Evidence: live PATH `codemap 0.2.12`, repo `/Users/amir/Documents/projects/Sillentway-VPN`, output `target/dogfood-live-0.2.12-20260504T041504/Sillentway-VPN_.changed.md`.
  - Observed: `changed` prints `Sensor Counts - direct: 8; indirect: 9; missing_direct: 0` while the same report has no runnable proof command and `Unknown direct_test_import_not_found: 6`.
  - Why it hurts trust: a count named `direct` sounds like direct deterministic proof. When soft token/mediated sensors feed that count, the summary weakens the fail-open Unknown contract.
  - Boundary: the raw sensors may be useful, but the count vocabulary must not imply proof coverage.
  - Status: fixed in `0.2.13`; repeated live PATH probe renders `runnable_direct: 0`, `soft: 17`, and `missing_direct_unknown: 6` for the same changed set.

- Issue: symbol-anchor mediated proof could suppress broad fallback without any fail-open Unknown.
  - Evidence: reviewer repro during the `0.2.14` slice using `codemap proof 'src/local-flow.tsx#chooseFocus' --format json` on a focused fixture.
  - Observed: proof contained `test_imported_symbol_reference_via_local_symbol_consumer` with `strength: medium`, `fallback: []`, and `unknowns: []`.
  - Why it hurts trust: `_via_local_symbol_consumer` is useful mediated evidence, but without Unknown it can look like enough direct symbol proof.
  - Boundary: suppressing broad fallback is acceptable when an exact mediated runnable command exists; the defect was absence of a missing-direct proof Unknown.
  - Status: fixed in `0.2.14`; symbol anchors now emit `direct_test_import_not_found` when no direct/specific proof exists.

- Issue: `proof-map --raw-sensors` still uses section names that sound stronger than the sensors.
  - Evidence: follow-up live review on proof-heavy repos after `0.2.14`.
  - Observed: commands such as `make doctor`, `make next`, and `make validate-receipts` can appear under `Direct` even when the evidence is token/path/name overlap rather than direct runnable validation.
  - Why it hurts trust: `codemap proof` calls this `Soft Evidence`, while `proof-map` can make the same class look direct/load-bearing.
  - Boundary: source-backed script/name matches may be useful map facts; the defect is confidence vocabulary.
  - Follow-up: give `proof-map` the same proof classes as `proof`: hard/direct validation, mediated evidence, soft token evidence, setup/support, evidence-only, unknown.
  - Status: fixed in `0.2.15`; markdown proof-map now renders Hard Proof, Direct Evidence, Mediated Evidence, Soft Token Evidence, and Setup / Support Surfaces, and runnable commands are filtered to runnable validation proof only.

- Issue: `changed --section unknown` can report false `dynamic_import` for static multiline Python imports.
  - Evidence: follow-up live review on `tools/run_carrier_bound_growth_episode_v1_beta.py`.
  - Observed: `changed --section unknown` reported `dynamic_import` around a normal multiline import `from tools.run_gpt_oss_low_level_active_carrier_probe_v0_041 import (...)`, while `cone` resolved the same import as `resolved_import; high`.
  - Why it hurts trust: two lenses disagree on the same syntax; Unknown becomes a false alarm instead of a map blind spot.
  - Boundary: dynamic import Unknown is still needed for actual runtime imports; Python `from ... import (...)` must stay static.
  - Follow-up: make dynamic import detectors language-aware and token-aware enough to reject Python static import statements, including multiline headers.
  - Status: fixed in `0.2.15`; diff-map gates dynamic import Unknowns to JS-like files and keeps Python static module targets as structural-line edges.

- Issue: `--all` and hidden expand semantics are inconsistent.
  - Evidence: follow-up live review of `codemap ls . --all` and `codemap proof ... --all`.
  - Observed: `ls . --all` can still print hidden material with a self-referential expand back to `codemap ls . --all`; `proof --all` is rejected even though `ls`, `cone`, and `changed` accept it.
  - Why it hurts trust: an agent expects `--all` to mean the current lens has expanded its collapsed material, and stable flags should not vary casually across core map commands.
  - Boundary: line budgets may still require `--limit`; the defect is self-referential expansion and uneven flag support.
  - Follow-up: avoid self-expand when `--all` is already active, and accept `--all` on proof as an alias for a larger/deeper proof report or explicitly documented no-op.
  - Status: fixed in `0.2.15` for `ls` and `proof`; `ls --all` raises the default limit and `proof --all` is accepted as an expanded proof report.

- Issue: Python private helpers are rendered as `Exports`.
  - Evidence: follow-up live review of Python proof-runner files.
  - Observed: private symbols such as `_candidate`, `_digest`, and `_tick` appear under `Exports` even when symbol rows say `exported=false`.
  - Why it hurts trust: for Python, underscore helpers are not public API surfaces; calling them exports can send an agent toward the wrong boundary.
  - Boundary: listing the symbols is useful; naming them public exports is the defect.
  - Follow-up: render Python private helpers under `Symbols`, and only show `Public Exports` when language/manifest evidence supports public export semantics.
  - Status: fixed in `0.2.15`; Python def/class symbols remain visible with `exported=false`, but are no longer copied into file exports.

- Issue: proof-heavy runner cones miss natural proof-slice neighbors.
  - Evidence: follow-up live review around proof runners and receipts.
  - Observed: `cone` shows imports for a runner, but not the natural deterministic proof bundle around it: owner doc, receipt, review, Makefile target, doctor/next rails.
  - Why it hurts trust: agents use cone for owner neighborhoods; proof-slice repos need source-backed runner/receipt/review/rail links without turning into recommendations.
  - Boundary: this is not a correctness verdict; the missing surface is a deterministic neighbor map gap.
  - Follow-up: add a bounded named-neighbor map for runner -> receipt -> review -> experiment doc -> Makefile target -> doctor check, with provenance and no advice language.

- Issue: `diff-map` can produce noisy `added_structural_line -> unknown_target` rows.
  - Evidence: follow-up live review of `diff-map` added structural lines.
  - Observed: repeated `unknown_target` rows do not include enough line content or target evidence to help decide anything.
  - Why it hurts trust: it looks structural without carrying actionable structure.
  - Boundary: line-level change events can be useful; rows with no useful target/content need stricter collapse or a clearer Unknown.
  - Follow-up: collapse repeated `unknown_target` structural-line events and include short provenance/content only when it adds map value.
  - Status: fixed in `0.2.15` for import/export structural lines without a deterministic target; diff-map skips targetless rows instead of emitting `unknown_target`.

- Issue: primary map outputs lack a visible snapshot/fingerprint.
  - Evidence: follow-up live review where changed count moved from 5 to 7 between calls while the repo was changing.
  - Observed: `status` exposes fingerprint/cache state, but ordinary `ls`, `cone`, `changed`, `proof`, and lenses do not show a snapshot id in the header.
  - Why it hurts trust: an agent can compare maps from different repo states and mistake a race for a fact.
  - Boundary: exact cache diagnostics can stay in `status`; primary maps need a compact snapshot line.
  - Follow-up: add a compact map snapshot line: root, git head/dirty fingerprint, and cache state or scan fingerprint.
  - Status: fixed in `0.2.15`; markdown map outputs now include root and scan fingerprint prefix, including cached lens fast paths. Cache state/strategy stay in `status/doctor` so identical maps do not drift only because they were served from a lens artifact.

- Issue: zero-footprint wording is too broad.
  - Evidence: follow-up live review of `status`.
  - Observed: codemap does not write into the inspected repo by default, but it does write external cache under `~/Library/Caches/codemap/...`.
  - Why it hurts trust: `zero-footprint` can be read as no writes anywhere.
  - Boundary: external cache writes are acceptable; the claim should be precise.
  - Follow-up: use `zero repo footprint` in user-facing output/docs.
  - Status: fixed in `0.2.15`; status/doctor markdown now says `Zero repo footprint default`.

- Issue: Rust module aggregator edges can overstate cone blast radius.
  - Evidence: follow-up live review of `cone src/map/proof_entry.rs` through `src/map.rs`.
  - Observed: the cone can show many sibling module imports because the module aggregator imports them, although the anchor file itself does not directly import all siblings.
  - Why it hurts trust: formal module edges are real, but without mediator wording an agent may overestimate direct coupling.
  - Boundary: keep the edges; mark the mediator explicitly.
  - Follow-up: label these as mediated by module aggregator/public index rather than direct anchor imports.

- Issue: `ls . --all` still uses `hidden` wording inside bounded surface examples.
  - Evidence: live PATH dogfood after installing `0.2.15` on the codemap repo.
  - Observed: the top-level Hidden section no longer self-expands, but directory surface rows still say `hidden: N examples` because each aggregate only prints a sample of examples.
  - Why it hurts trust: after `--all`, the word `hidden` can still read as undisclosed map material rather than a line-budgeted example sample.
  - Boundary: line budgets are still required; `--all` should include collapsed structural classes, not dump every file path in large aggregate surfaces.
  - Follow-up: rename per-surface `hidden` to `additional examples` or attach an explicit `--limit` expansion so it does not look like `--all` ignored hidden map material.

- Issue: `changed --section unknown` can become a wall of repeated missing-direct rows on broad dirty sets.
  - Evidence: live PATH dogfood after installing `0.2.15` on the codemap repo with 28 changed files.
  - Observed: the section correctly stayed fail-open, but repeated `direct_test_import_not_found` and `nearest_proof_scope` entries dominated the output.
  - Why it hurts trust: a useful Unknown section should reveal what detector evidence is missing, not bury distinct blind spots under repeated identical prose.
  - Boundary: do not suppress Unknown; group repeated Unknowns by kind/effect and keep representative paths plus exact expand commands.
  - Follow-up: add compact grouping for repeated Unknowns in section output while keeping `--all` or `--limit` paths to the full list.

- Issue: schema anchors still receive source-centric `direct_test_import_not_found` wording.
  - Evidence: live PATH dogfood after installing `0.2.15` on `/Users/amir/Documents/projects/Levelly-1` with `codemap proof apps/api/prisma/schema.prisma --all`.
  - Observed: Prisma proof correctly showed `db:migrate:status` as hard proof and migration/codegen/seed as setup/support, but Unknown also said no direct test import/symbol/e2e route was found.
  - Why it hurts trust: for schema owners, missing source test imports may be less relevant than missing schema check, migration, generated-client consumer, env link, CI deploy/status reference, or contract check.
  - Boundary: keep fail-open Unknown; specialize missing-direct wording by anchor role so schema/config/manifest anchors do not inherit source-only proof language.
  - Follow-up: emit schema-specific missing evidence rows for schema anchors and reserve `direct_test_import_not_found` for source/symbol anchors.

- Issue: cold root fast-path maps can print `fingerprint=unknown`.
  - Evidence: live PATH dogfood after installing `0.2.15` on `/Users/amir/Documents/projects/PABG` with `codemap ls .`.
  - Observed: the bounded root inventory fast path avoided a full scan and printed `Map Snapshot: ... fingerprint=unknown`.
  - Why it hurts trust: snapshot headers should help agents compare maps; `unknown` is honest but weak when the fast path already has a deterministic inventory file list.
  - Boundary: do not force a full repo scan just to compute the snapshot; use a bounded inventory fingerprint for cold root fast paths.
  - Follow-up: compute a stable inventory snapshot fingerprint from the fast-path structural file list and root path.
  - Status: fixed in `0.2.15`; cold root inventory maps now use a deterministic bounded inventory fingerprint instead of `unknown`.

- Issue: clean fast-path maps can still print `fingerprint=unknown`.
  - Evidence: live PATH dogfood after installing `0.2.15` on `/Users/amir/Documents/projects/Levelly-1` with `codemap changed --section unknown` in a clean tree.
  - Observed: clean changed output had no changed anchors, but snapshot said `fingerprint=unknown`.
  - Why it hurts trust: clean/no-op maps are still maps; agents may compare them with later dirty maps and need a stable boundary.
  - Boundary: keep clean fast paths cheap; prefer cached status fingerprint, falling back to bounded inventory fingerprint.
  - Follow-up: share the fast-path snapshot helper across cached and clean fast paths.
  - Status: fixed in `0.2.15`; cached and clean fast paths now share a snapshot helper with cached-status then bounded-inventory fallback.
