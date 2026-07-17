# Flagship behavioral corpus

This directory owns the 18-task flagship corpus: six pinned repositories across
four ecosystem families, with one deterministic investigation, one multi-owner
implementation, and one exact/local control per repository.

`materialize.py` creates clean repository snapshots, isolated exact-control
mutations, hidden verifier overlays, a verification spec, and a runnable draft.
Investigation reports are checked as source-backed claims with concrete `path:line`
citations against frozen source bytes; an opened-file inventory does not score.
The verifier checks frozen causal anchors and real citations, not prescribed prose.
Required claims define the task's causal core; additional claims measure completeness.
Implementation tasks use hidden tests and contract checks; exact controls verify the local byte change.
An investigation prompt exposes one real runtime/file/symbol anchor, not the expected
owner and proof vocabulary that its external verifier checks.

```bash
python3 benchmarks/flagship/materialize.py \
  benchmarks/flagship/corpus-blueprint.json \
  --out-dir target/flagship-corpus-v1 --remote-only

cargo build --release --locked
python3 scripts/benchmark-codemap-flagship.py freeze \
  target/flagship-corpus-v1/corpus-draft.json \
  --out-dir target/flagship-frozen-v1 \
  --codemap-bin target/release/codemap

python3 scripts/benchmark-codemap-flagship.py run \
  target/flagship-frozen-v1/manifest.json \
  --out-dir target/flagship-run-v1

python3 scripts/benchmark-codemap-flagship.py evaluate \
  target/flagship-frozen-v1/manifest.json \
  --run-dir target/flagship-run-v1 \
  --out-dir target/flagship-acceptance-v1
```

Freeze is the experiment boundary. Any binary, task, criterion, threshold, repo
commit, model, or limit change requires a new identity and a complete rerun.
Infrastructure failures retry once with the same manifest; a second failure stays
in the 72-run denominator and makes acceptance red.

`verify.py` is the only task-verifier dispatcher. Hidden overlays are copied only
after the candidate patch is captured. Every transitive verifier byte is hashed by
the manifest and rechecked before execution and evaluation.
