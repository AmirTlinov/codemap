# Feature TODO: Structural Map Completion Ledger

This is the execution ledger. Do not check a box because a plan exists. A slice
is done only after implementation, gates, reviewer PASS, live dogfood, and an
honest agent-satisfaction verdict.

Legend:

```txt
Implemented: code/docs/tests for the slice are complete.
Gates: fmt, tests, clippy, doctor, diff-check passed.
Reviewer: reviewer subagent returned PASS.
Live: spritestudio + Sillentway-VPN + third repo were exercised where relevant.
Satisfied: the implementing agent would voluntarily use the result again.
```

## Slice Ledger

### Slice 01: Product lock, baseline inventory, invariant guards

- [ ] Slice 01 implemented
- [ ] Slice 01 gates passed
- [ ] Slice 01 reviewer PASS
- [ ] Slice 01 live dogfood complete
- [ ] Slice 01 agent satisfied

### Slice 02: Daily command surface and alias cleanup

- [ ] Slice 02 implemented
- [ ] Slice 02 gates passed
- [ ] Slice 02 reviewer PASS
- [ ] Slice 02 live dogfood complete
- [ ] Slice 02 agent satisfied

### Slice 03: Unified fact model and constructors

- [ ] Slice 03 implemented
- [ ] Slice 03 gates passed
- [ ] Slice 03 reviewer PASS
- [ ] Slice 03 live dogfood complete
- [ ] Slice 03 agent satisfied

### Slice 04: Evidence locations and typed unknowns

- [ ] Slice 04 implemented
- [ ] Slice 04 gates passed
- [ ] Slice 04 reviewer PASS
- [ ] Slice 04 live dogfood complete
- [ ] Slice 04 agent satisfied

### Slice 05: Scope model, root maps, expand protocol

- [ ] Slice 05 implemented
- [ ] Slice 05 gates passed
- [ ] Slice 05 reviewer PASS
- [ ] Slice 05 live dogfood complete
- [ ] Slice 05 agent satisfied

### Slice 06: Compact markdown grammar and renderer

- [x] Slice 06 implemented within closure boundary
- [x] Slice 06 gates passed
- [x] Slice 06 reviewer PASS
- [x] Slice 06 live dogfood complete
- [x] Slice 06 agent satisfied

Boundary:

```txt
closed: daily/focused compact map grammar for ls, cone, changed, impact,
proof, proof-map, runtime, hidden/unknown, surfaces, and contract exports.
excluded: doctor/status diagnostics, graph/boundary diagnostics, tiny metadata
headers that are not repeated path spam.
next: stop renderer micro-polish and return to facts/schemas/cache/lens depth.
```

### Slice 07: Schema rail and golden validation

- [x] Slice 07 implemented within closure boundary
- [x] Slice 07 gates passed
- [x] Slice 07 reviewer PASS
- [x] Slice 07 live dogfood complete
- [x] Slice 07 agent satisfied

Boundary:

```txt
closed: manifest/schema-command parity for every listed schema, public JSON
report validation against manifest-selected schemas, json_kind/schema_version
parity for structural outputs, doctor schema alias, and legacy/router schema
absence guards.
interpreted: "golden JSON" is covered by real fixture command outputs validated
against schemas, not committed snapshot files.
excluded: a separate golden snapshot framework and exhaustive live lens behavior
dogfood; those would add process without stronger schema-contract proof.
next: move to scanner/cache/facts depth, not more schema ceremony unless a new
public JSON command or report shape appears.
```

### Slice 08: Fast scanner, ignore/generated/vendor detection

- [x] Slice 08 implemented within closure boundary
- [x] Slice 08 gates passed
- [x] Slice 08 reviewer PASS
- [x] Slice 08 live dogfood complete
- [x] Slice 08 agent satisfied

Boundary:

```txt
closed: shared scanner policy returns scan stats, common ignored dirs are
excluded from inventory and config discovery, ignored stats count unique
ignored roots, generated headers/path conventions mark files as generated,
doctor/status expose scanner groups via status_report v3.
excluded: full generated source-owner resolution, source-map pairing, codegen
config tracing, and partial-rescan cache. Those belong to later generated/cache
slices, not scanner-policy closure.
next: warm cache/index freshness.
```

### Slice 09: Warm cache, indexes, incremental freshness

- [ ] Slice 09 implemented
- [ ] Slice 09 gates passed
- [ ] Slice 09 reviewer PASS
- [ ] Slice 09 live dogfood complete
- [ ] Slice 09 agent satisfied

### Slice 10: Git structural events, changed, diff-map

- [ ] Slice 10 implemented
- [ ] Slice 10 gates passed
- [ ] Slice 10 reviewer PASS
- [ ] Slice 10 live dogfood complete
- [ ] Slice 10 agent satisfied

### Slice 11: Symbol/import/export extraction matrix

- [ ] Slice 11 implemented
- [ ] Slice 11 gates passed
- [ ] Slice 11 reviewer PASS
- [ ] Slice 11 live dogfood complete
- [ ] Slice 11 agent satisfied

### Slice 12: Package/workspace graph and boundaries

- [ ] Slice 12 implemented
- [ ] Slice 12 gates passed
- [ ] Slice 12 reviewer PASS
- [ ] Slice 12 live dogfood complete
- [ ] Slice 12 agent satisfied

### Slice 13: ls, graph causal, root map quality

- [ ] Slice 13 implemented
- [ ] Slice 13 gates passed
- [ ] Slice 13 reviewer PASS
- [ ] Slice 13 live dogfood complete
- [ ] Slice 13 agent satisfied

### Slice 14: cone exact anchors and directory aggregation

- [ ] Slice 14 implemented
- [ ] Slice 14 gates passed
- [ ] Slice 14 reviewer PASS
- [ ] Slice 14 live dogfood complete
- [ ] Slice 14 agent satisfied

### Slice 15: Runtime lens

- [ ] Slice 15 implemented
- [ ] Slice 15 gates passed
- [ ] Slice 15 reviewer PASS
- [ ] Slice 15 live dogfood complete
- [ ] Slice 15 agent satisfied

### Slice 16: Contract lens

- [ ] Slice 16 implemented
- [ ] Slice 16 gates passed
- [ ] Slice 16 reviewer PASS
- [ ] Slice 16 live dogfood complete
- [ ] Slice 16 agent satisfied

### Slice 17: Proof-map and proof safety

- [ ] Slice 17 implemented
- [ ] Slice 17 gates passed
- [ ] Slice 17 reviewer PASS
- [ ] Slice 17 live dogfood complete
- [ ] Slice 17 agent satisfied

### Slice 18: Impact lens

- [ ] Slice 18 implemented
- [ ] Slice 18 gates passed
- [ ] Slice 18 reviewer PASS
- [ ] Slice 18 live dogfood complete
- [ ] Slice 18 agent satisfied

### Slice 19: Delete lens

- [ ] Slice 19 implemented
- [ ] Slice 19 gates passed
- [ ] Slice 19 reviewer PASS
- [ ] Slice 19 live dogfood complete
- [ ] Slice 19 agent satisfied

### Slice 20: Boundary-map lens

- [ ] Slice 20 implemented
- [ ] Slice 20 gates passed
- [ ] Slice 20 reviewer PASS
- [ ] Slice 20 live dogfood complete
- [ ] Slice 20 agent satisfied

### Slice 21: Flow lens

- [ ] Slice 21 implemented
- [ ] Slice 21 gates passed
- [ ] Slice 21 reviewer PASS
- [ ] Slice 21 live dogfood complete
- [ ] Slice 21 agent satisfied

### Slice 22: Siblings and place lenses

- [ ] Slice 22 implemented
- [ ] Slice 22 gates passed
- [ ] Slice 22 reviewer PASS
- [ ] Slice 22 live dogfood complete
- [ ] Slice 22 agent satisfied

### Slice 23: Non-code, assets, data, events, generated ownership

- [ ] Slice 23 implemented
- [ ] Slice 23 gates passed
- [ ] Slice 23 reviewer PASS
- [ ] Slice 23 live dogfood complete
- [ ] Slice 23 agent satisfied

### Slice 24: Unknown taxonomy, scope repair, fail-closed traversal

- [ ] Slice 24 implemented
- [ ] Slice 24 gates passed
- [ ] Slice 24 reviewer PASS
- [ ] Slice 24 live dogfood complete
- [ ] Slice 24 agent satisfied

### Slice 25: Performance, path stability, and cognitive regression gates

- [ ] Slice 25 implemented
- [ ] Slice 25 gates passed
- [ ] Slice 25 reviewer PASS
- [ ] Slice 25 live dogfood complete
- [ ] Slice 25 agent satisfied

### Slice 26: Fixture matrix across stacks

- [ ] Slice 26 implemented
- [ ] Slice 26 gates passed
- [ ] Slice 26 reviewer PASS
- [ ] Slice 26 live dogfood complete
- [ ] Slice 26 agent satisfied

### Slice 27: Live adoption harness and local PATH ergonomics

- [ ] Slice 27 implemented
- [ ] Slice 27 gates passed
- [ ] Slice 27 reviewer PASS
- [ ] Slice 27 live dogfood complete
- [ ] Slice 27 agent satisfied

### Slice 28: Final audit, cleanup, TODO closure

- [ ] Slice 28 implemented
- [ ] Slice 28 gates passed
- [ ] Slice 28 reviewer PASS
- [ ] Slice 28 live dogfood complete
- [ ] Slice 28 agent satisfied

## Recurring Gate Commands

Run after every meaningful slice:

```bash
cargo fmt --check
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo run --quiet --bin codemap -- doctor
git diff --check
```

If a slice only edits planning docs, run at least:

```bash
git diff --check -- docs/plans/feature
```

## Required Live Probes

At minimum, run on:

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

Add slice-specific probes from each `SLICE-*.md` file.
Every slice inherits the three-repo live requirement from `PLAN.md`; if a repo
lacks a relevant anchor, record that explicitly instead of skipping it silently.
Use `scripts/dogfood-codemap.sh` as the default read-only harness unless a slice
explicitly proves it needs a separate script.

## Live Probe Notes Template

Use this template inside the slice proof notes or final PR/commit summary. Do
not mark `Live Dogfood` checked until the answers are concrete.

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
