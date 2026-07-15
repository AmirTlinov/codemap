# Behavioral A/B Benchmark

`scripts/benchmark-codemap-ab.py` runs the same implementation or repository-analysis
task twice with Codex:

- `control`: `codemap` is blocked and ordinary repository navigation remains available;
- `codemap`: the agent must follow the mode-specific proportional-entry protocol.

Both arms use the same git commit, task text, model, reasoning effort, sandbox, and
external verifier. Each arm receives a fresh detached worktree. Arm order alternates
between repetitions to reduce a simple run-order bias.

Before spending model calls, the harness runs every verifier against the untouched
base commit in a third disposable worktree. If every verifier already passes, the
task is rejected because it has no measured behavioral gap.

The primary result is **externally verified completeness**, not token minimization.
Each task should be split into independent criteria that expose what the agent actually
covered: direct behavior, public contracts, downstream consumers, regressions, and
other task-specific consequences. Token and elapsed-time deltas are reported separately
as the resource cost of that result. A treatment that discovers and correctly handles
more of the repository may legitimately use more tokens.

The default requested model configuration is:

```txt
model = gpt-5.6-sol
model_reasoning_effort = xhigh
```

`gpt-5.6-sol-xhigh` is therefore represented by two explicit Codex settings rather
than an invented compound model name.

## Task Manifest

Tasks are JSONL: one independent JSON object per line.

```json
{"id":"session-timeout","repo":"/path/to/repo","base_ref":"task/session-timeout","prompt":"Fix the session timeout regression without changing the public API.","verify":[{"name":"runtime-behavior","category":"behavior","weight":4,"required":true,"command":["python3","/absolute/path/to/verify_runtime.py","{worktree}"],"timeout_seconds":300},{"name":"public-api","category":"contract","weight":3,"required":true,"command":["python3","/absolute/path/to/verify_api.py","{worktree}"]},{"name":"secondary-consumer","category":"downstream","weight":2,"required":false,"command":["python3","/absolute/path/to/verify_consumer.py","{worktree}"]},{"name":"regression-corpus","category":"regression","weight":1,"required":false,"command":["python3","/absolute/path/to/verify_regressions.py","{worktree}"]}],"protected_paths":["tests/hidden"]}
```

Required fields:

| Field | Meaning |
| --- | --- |
| `id` | Stable task identity, unique in the file. |
| `repo` | Git repository. Relative paths resolve beside the task manifest. |
| `prompt` | Task text passed identically to both arms. |
| `verify` | One or more deterministic verifier objects. Commands are argv arrays, never shell strings. |

Optional fields:

| Field | Default | Meaning |
| --- | --- | --- |
| `base_ref` | `HEAD` | Exact task starting commit or ref. |
| `mode` | `implementation` | `implementation` changes code; `analysis` produces an evidence-backed report without repository edits. |
| `protected_paths` | `[]` | Paths whose modification forces the task outcome to fail. |
| verifier `category` | `behavior` | Independent coverage dimension, such as `behavior`, `contract`, `downstream`, or `regression`. |
| verifier `weight` | `1.0` | Positive contribution of this criterion to completeness. Set before the run. |
| verifier `required` | `true` | Whether failure blocks the task-level required outcome. Optional criteria still affect completeness. |
| verifier `timeout_seconds` | CLI default | Per-verifier timeout. |

Verifier arguments may contain `{worktree}`, `{repo}`, `{last_message}`, `{events}`,
and `{patch}` placeholders. Prefer a
verifier stored outside the task worktree so the model cannot make the evaluator pass
by editing it. If repository tests are part of the evaluator, put immutable or hidden
tests in `protected_paths` or invoke them from outside the worktree.

## Run

First validate the experiment matrix without spending model calls:

```bash
scripts/benchmark-codemap-ab.py tasks.jsonl --repetitions 3 --dry-run
```

Then run the paired experiment:

```bash
scripts/benchmark-codemap-ab.py tasks.jsonl \
  --model gpt-5.6-sol \
  --reasoning-effort xhigh \
  --repetitions 3 \
  --codemap-bin target/debug/codemap
```

The harness invokes `codex exec` with ephemeral sessions, ignored user config and
exec rules, disabled fanout/multi-agent behavior, `workspace-write`, and explicit
model/reasoning settings. The treatment receives an external codemap cache; the
target worktree stays free of codemap artifacts.

Binary resolution is deterministic: explicit `--codemap-bin`, then `CODEMAP_BIN`,
then a local debug/release target, then `PATH`. Summary and per-trial JSON preserve
one shared `report_prelude.codemap` with exact argv, version, executable artifacts,
and SHA-256. These fields participate in the resume fingerprint, so replacing a
binary without changing its version invalidates the old trial.
`--codemap-bin` accepts a direct executable or a quoted Python/POSIX-shell
wrapper; use a direct executable for other runtime dispatchers.

Treatment starts from the narrowest usable anchor named by the task: `codemap ls <scope>`
for an exact file or directory, `codemap cone <file#symbol>` for an anchored symbol, or
`codemap where <symbol>` when only the exact symbol name is known. It uses `codemap ls .`
only when the scope is unknown. In `implementation` mode, either entry is followed by
`codemap changed` and `codemap proof changed`. In `analysis` mode, an exact entry is
sufficient; root orientation must be followed by another focused map. Any analytical
repository change fails the outcome. Control has ordinary tools and blocks agent-attributed
codemap calls.

The harness does not parse task text or choose a command. Its shim uses native process ancestry
(`libproc` on macOS, `/proc` on Linux) to separate an agent navigation call from a codemap
consumer launched inside the project's own tests. Agent calls are blocked in control and routed
to the frozen benchmark binary in treatment; internal consumers keep using the project's own
built binary and do not count as navigation. Completed Codex command events are independently
matched against attributed shim calls, so invoking the frozen binary by an absolute path cannot
bypass the arm protocol. The shim records argv and the actual exit status; only successful calls
can satisfy the protocol. The report keeps
the raw `invocation_results`, `first_entry`, `entry_is_first_invocation`, `entry_kind` (`none`,
`exact`, or `root`), `root_entry`, `exact_entry`, `mixed`, `ordered_daily`, and whether a
focused call followed root orientation. Ignored internal calls and the event-trace comparison
remain visible as separate receipts. This keeps task understanding in the agent while making the
published protocol machine-checkable without reimplementing clap in the harness.

Use `--resume` after interruption. Existing trials are reused only when the task,
base commit, composed arm prompt, protocol/parser and harness bytes, model, reasoning,
timeout, trial order, Codex version, arm, and verifier configuration produce the same
fingerprint. Codemap command artifacts and the full benchmark identity also participate,
so replacing a wrapper or binary without changing its version invalidates the old trial.

## Designing a Completeness Benchmark

Do not make one broad test suite the only criterion. It collapses “barely works” and
“understood the whole change surface” into the same green result. Instead, declare
separate hidden or immutable checks for the consequences that a well-oriented agent
should notice.

| Category | What it establishes |
| --- | --- |
| `behavior` | The requested runtime behavior actually changed. |
| `contract` | Public API, schema, CLI, compatibility, or other promises remain correct. |
| `downstream` | Consumers and coupled surfaces were found and updated where needed. |
| `regression` | Existing behavior outside the narrow happy path still works. |

Weights express task value, not verifier difficulty, and must be fixed before either
arm runs. Use `required: true` only for non-negotiable acceptance criteria or safety
guardrails. Keep other valid consequences optional-but-scored so that two nominally
passing patches can still differ in completeness.

The score for a trial is:

```txt
completeness = sum(weights of passed criteria) / sum(all criterion weights)
```

Pair comparison uses the required task outcome first; when both arms have the same
required outcome, weighted completeness decides which one covered more. Token use
never decides the winner. This deliberately permits the useful result “codemap was
more complete and cost more context” rather than misclassifying extra observation as
inefficiency.

Aggregate completeness, pass rates, time, and token deltas use complete valid pairs
only. If one arm crashes or violates its protocol, neither half of that pair leaks into
the effect estimate.

Because the score is external, it measures the effects of understanding instead of
trusting an agent's self-description. A strong corpus contains tasks where the direct
edit is easy but important coupled surfaces are not obvious from the prompt.

Exact/local negative controls should name their usable anchor directly in the identical task
prompt and pre-register the allowed exact first argv/anchor set together with
`entry_is_first_invocation=true`, `entry_kind=exact`, `root_entry=false`, and `mixed=false` as
treatment receipt criteria. Implementation trials also require `ordered_daily=true`. They test
that codemap preserves the control outcome without charging for root orientation; time and token
deltas remain visible resource costs rather than a substitute for that check.

## Artifacts and Scoring

Each trial preserves:

```txt
trials/<task>-r<repetition>-<arm>/
  prompt.txt
  events.jsonl
  last-message.md
  codex.stderr.log
  patch.diff
  git-status.txt
  verify-*.stdout.log
  verify-*.stderr.log
  result.json
```

The run root also contains `results.jsonl`, `summary.json`, and `summary.md`.

A failed required verifier is a failed task outcome. Any failed verifier lowers the
completeness score according to its fixed weight. A Codex crash, timeout, control-arm
codemap attempt, or treatment arm that skips the required codemap workflow makes the
pair invalid rather than silently counting it as a product loss.

One task and one repetition are a smoke test, not evidence of general lift. A useful
product result needs multiple representative tasks from unfamiliar repositories,
several repetitions, external deterministic criteria across multiple categories, and
inspection of invalid pairs and patches alongside aggregate completeness. Report time
and tokens as secondary cost, preferably with cached input visible, not as a proxy for
quality.

## Frozen Flagship Gate

The exploratory harness above remains the execution owner. The S15 release/nightly gate wraps
it with `scripts/benchmark-codemap-flagship.py`; it does not introduce another runner or change
an arm prompt after evidence exists.

### Freeze before model calls

A draft corpus names exactly 12 analysis, 12 multi-file implementation, and 6 exact/local
negative-control tasks. Every class is split equally between `calibration` and `holdout`.
Each split must retain at least six repositories and four ecosystem families. The draft also
fixes model, reasoning effort, timeouts, three-or-more repetitions, bootstrap seed/iterations,
pair-order algorithm, allowed invalidation reasons, acceptance boundaries, blind-assignment
seed, and manual-audit sample.

Each task adds executable `benchmark` metadata to the ordinary task manifest:

```json
{
  "benchmark": {
    "repo_id": "billing-service",
    "ecosystem": "typescript",
    "task_class": "implementation",
    "split": "holdout",
    "ordinal_criteria": [],
    "exception_criteria": ["secondary-consumer"]
  }
}
```

Every deterministic verifier declares `scoring: "deterministic"` and an external
`evidence_surface`. Analysis tasks additionally declare ordinal criteria with fixed `id`,
category, weight, maximum score, evidence surface, and blind-judge protocol. An implementation
rubric must expose behavior, contract, downstream, and regression separately. All tasks include
required and provenance criteria. Negative controls pre-register `expected_same_outcome: true`
and the exact first codemap argv strings allowed by their prompt.

Freeze resolves every repository ref to one commit per repository, hashes task bytes, external
verifier files, harness/protocol bytes, Codex executable/version, and the attributable codemap
binary. It materializes immutable split manifests and the counterbalanced arm schedule:

```bash
cargo build --release --locked
scripts/benchmark-codemap-flagship.py freeze corpus-draft.json \
  --out-dir target/codemap-ab/flagship-frozen \
  --codemap-bin target/release/codemap
```

Any later prompt, weight, verifier, repository SHA, model, binary, timeout, schedule, harness, or
protocol change requires a new frozen corpus. A holdout result is never carried across that
boundary.

### Execute isolated splits

```bash
scripts/benchmark-codemap-flagship.py run \
  target/codemap-ab/flagship-frozen/corpus-manifest.json calibration \
  --out-dir target/codemap-ab/flagship-calibration

scripts/benchmark-codemap-flagship.py run \
  target/codemap-ab/flagship-frozen/corpus-manifest.json holdout \
  --out-dir target/codemap-ab/flagship-holdout
```

The wrapper accepts no model, rubric, repetition, timeout, Codex, or codemap overrides. Both
arms still run through `benchmark-codemap-ab.py` with fresh detached worktrees and separate
external caches. A task whose verifier already passes at the frozen baseline is rejected before
model calls. Rejected, missing, duplicated, crashed, timed-out, protocol-invalid, or provenance-
ambiguous pairs stay visible in the expected denominator; they cannot silently disappear from
acceptance. Calibration is development evidence only and is always reported separately.

### Blind analysis judgment

```bash
scripts/benchmark-codemap-flagship.py prepare-judging MANIFEST \
  --calibration-dir CALIBRATION --holdout-dir HOLDOUT --out-dir JUDGING
```

The frozen seed counterbalances anonymous candidates `A/B`; public assignments contain only the
candidate report, artifact hash, and rubric ids. The sealed key maps candidates back to arms only
for final aggregation. Ratings are JSONL with `assignment_id`, `candidate_id`, independent
`judge_id`, `role: "judge"`, and integer criterion scores. Exactly two judges score each
candidate. A disagreement requires one `role: "adjudicator"` row while identity remains blind.
The frozen manual-audit sample additionally requires one `role: "auditor"` row with
`audit_passed: true`. Krippendorff ordinal alpha is published per rubric; alpha below `0.67`
invalidates acceptance rather than being repaired post hoc.

### Aggregate and verify

```bash
scripts/benchmark-codemap-flagship.py evaluate MANIFEST \
  --calibration-dir CALIBRATION --holdout-dir HOLDOUT \
  --assignments JUDGING/assignments.jsonl \
  --assignment-key JUDGING/assignment-key.jsonl \
  --ratings ratings.jsonl --out-dir ACCEPTANCE

scripts/verify-flagship-acceptance.py ACCEPTANCE/acceptance.json
```

The evaluator combines deterministic and ordinal criteria with their frozen weights, aggregates
repetitions inside each task, tasks inside each repository, and only then macro-averages
repositories. Holdout acceptance requires the one-sided 95% paired-bootstrap lower bound above
zero, no deterministic required regression, positive median task delta, at least 60% complex-task
wins, non-inferior downstream/contract/regression categories, bounded complex and negative-control
resource overhead, zero analysis writes, complete provenance, and valid blind agreement.
Time and input are costs, never the primary winner. A resource exception is possible only for
pre-registered criteria, after the primary endpoint and negative controls pass, and when at least
60% of over-budget complex tasks gain those criteria.

`acceptance.json` hashes the frozen manifest, every raw trial/verifier artifact, blind assignment,
key, and rating. The independent verifier imports none of the scoring implementation; it checks
artifact immutability, denominator truth, split separation, agreement, primary-bound state, and
that the final verdict equals every normative check. The synthetic black-box test runs on normal
CI; the expensive frozen matrix is a release/nightly gate, not a pull-request model-call ritual.
