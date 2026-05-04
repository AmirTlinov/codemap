# Codemap Live Dogfood Notes

## 2026-05-04

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
