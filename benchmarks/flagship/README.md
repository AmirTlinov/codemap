# Flagship behavioral corpus

This directory owns the real S15 corpus definition and its external verification
surface. `corpus-blueprint.json` pins 30 tasks across six unrelated repositories and
four ecosystem families: 12 analysis tasks, 12 multi-file implementation tasks, and
six exact/local negative controls, split evenly into calibration and holdout.

The blueprint is not an accepted result. The materializer creates history-free task
repositories at exact source commits, seeds the negative controls, extracts hidden
consumer tests outside those repositories, and emits a runnable draft plus a hash
receipt:

```bash
python3 benchmarks/flagship/materialize.py \
  benchmarks/flagship/corpus-blueprint.json \
  --out-dir target/flagship-s15-corpus-v1 \
  --remote-only
```

Before model execution, prove that every task has a real baseline gap and retains its
frozen provenance. A successful command means no task already satisfies all external
criteria; inspect the per-task receipts to require `required=false` and
`provenance=true`:

```bash
python3 scripts/benchmark-codemap-ab.py \
  target/flagship-s15-corpus-v1/tasks.jsonl \
  --codex-bin codex --codemap-bin target/release/codemap \
  --out-dir target/flagship-s15-preflight-v1 --preflight-only
```

Freeze only from a clean commit and the exact release binary that will enter the
receipt. Calibration may tune implementation and rubric before a new freeze. Holdout
must not influence extractors, prompts, budgets, weights, or acceptance:

```bash
python3 scripts/benchmark-codemap-flagship.py freeze \
  target/flagship-s15-corpus-v1/corpus-draft.json \
  --out-dir target/flagship-s15-frozen-v1 \
  --codex-bin codex --codemap-bin target/release/codemap

python3 scripts/benchmark-codemap-flagship.py run \
  target/flagship-s15-frozen-v1/manifest.json calibration \
  --out-dir target/flagship-s15-calibration-v1
```

`verify.py` is the only task-verifier dispatcher. Hidden overlays are copied after the
candidate patch is captured, so oracle bytes and verifier side effects cannot be
attributed to either arm. `browser_focused_clipboard_test.js` is an external consumer
owned by the corresponding browser task. All of these transitive artifacts are hashed
by the frozen manifest and checked again on load.
