# Behavioral A/B Benchmark

`scripts/benchmark-codemap-ab.py` runs the same implementation or repository-analysis
task twice with Codex:

- `control`: `codemap` is blocked and ordinary repository navigation remains available;
- `codemap`: the same agent can optionally use the frozen `codemap` binary.

Both arms use the same git commit, task text, model, reasoning effort, sandbox, and
external verifier. Each arm receives a fresh detached worktree. Arm order alternates
between repetitions to reduce a simple run-order bias.

Before spending model calls, the harness runs every verifier against the untouched
base commit in a third disposable worktree. If every verifier already passes, the
task is rejected because it has no measured behavioral gap.

`--treatment-preflight` then checks that the corpus, runtime, binary, and verifiers can
execute together. Its agent outcomes are diagnostic: a stochastic miss is reported but
does not fail preflight or trigger product tuning. Only infrastructure invalidity or a
baseline verifier leak makes this readiness check fail.

The primary result is **externally verified completeness**, not token minimization.
Each task should be split into independent criteria that expose what the agent actually
covered: direct behavior, public contracts, downstream consumers, regressions, and
other task-specific consequences. Token and elapsed-time deltas are reported separately
as the resource cost of that result. A treatment that discovers and correctly handles
more of the repository may legitimately use more tokens.

The flagship evaluator also preserves each complete A/B action history, patch, verifier
output, elapsed time, and token usage in one pair context. One comparative agent explains
how the two attention trajectories found owners, contracts, consumers, and proof surfaces.
It cites concrete event markers and may report noise or uncertainty, but it does not score,
vote, or affect completeness and acceptance thresholds. The verifier answers whether the
task was completed; the trajectory report explains how codemap changed the route to it.

The default requested model configuration is:

```txt
model = gpt-5.6-sol
model_reasoning_effort = high
```

The model and `high` reasoning effort are represented by two explicit Codex settings rather
than an invented compound model name.

## Task Manifest

Tasks are JSONL: one independent JSON object per line.

```json
{"id":"session-timeout","repo":"/path/to/repo","base_ref":"task/session-timeout","prompt":"Fix the session timeout regression without changing the public API.","verify":[{"name":"runtime-behavior","category":"behavior","weight":4,"required":true,"command":["python3","/absolute/path/to/verify_runtime.py","{worktree}"],"timeout_seconds":300},{"name":"public-api","category":"contract","weight":3,"required":true,"command":["python3","/absolute/path/to/verify_api.py","{worktree}"]},{"name":"secondary-consumer","category":"downstream","weight":2,"required":false,"command":["python3","/absolute/path/to/verify_consumer.py","{worktree}"]},{"name":"regression-corpus","category":"regression","weight":1,"required":false,"command":["python3","/absolute/path/to/verify_regressions.py","{worktree}"]}]}
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
| verifier `category` | `behavior` | Independent coverage dimension, such as `behavior`, `contract`, `downstream`, or `regression`. |
| verifier `weight` | `1.0` | Positive contribution of this criterion to completeness. Set before the run. |
| verifier `required` | `true` | Whether failure blocks the task-level required outcome. Optional criteria still affect completeness. |
| verifier `timeout_seconds` | CLI default | Per-verifier timeout. |

Verifier arguments may contain `{worktree}`, `{repo}`, `{last_message}`, `{events}`,
and `{patch}` placeholders. Prefer a
verifier stored outside the task worktree so the model cannot make the evaluator pass
by editing it. Repository-local hidden tests are overlaid only after the candidate patch
is captured, so ordinary regression tests added by the agent remain valid work.

## Run

First validate the experiment matrix without spending model calls:

```bash
scripts/benchmark-codemap-ab.py tasks.jsonl --repetitions 3 --dry-run
```

Then run the paired experiment:

```bash
scripts/benchmark-codemap-ab.py tasks.jsonl \
  --model gpt-5.6-sol \
  --reasoning-effort high \
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
`--codemap-bin` accepts one direct executable path. Internal orchestration preserves
wrappers as argv arrays in the frozen manifest; shell command strings are not accepted.

The treatment prompt only says that a read-only structural map is available and names
its main entries. It does not prescribe when to call it, which scope to choose, which
Expand to follow, or how to verify the task. Control has the same task and ordinary tools,
but agent-attributed codemap calls are blocked. An analytical repository change still
fails that task's external zero-write boundary. Exact/local controls use the same direct
edit prompt in both arms; optional map use remains part of the measured cost.

The harness does not parse task text or choose a command. Its shim uses native process ancestry
(`libproc` on macOS, `/proc` on Linux) to separate an agent navigation call from a codemap
consumer launched inside the project's own tests. Agent calls are blocked in control and routed
to the frozen benchmark binary in treatment; internal consumers keep using the project's own
built binary and do not count as navigation. Completed Codex command events are independently
matched against attributed shim calls, so invoking the frozen binary by an absolute path cannot
bypass arm attribution. The shim records argv and the actual exit status. The report keeps
raw invocation records plus neutral call facts: command, argument, scope kind, argv,
display text, status, and success. Ignored internal calls and the event-trace comparison
remain visible as diagnostics. A treatment agent may ignore, misuse, or recover from codemap;
all of those trajectories remain outcome and cost evidence. They are not converted into a map
defect unless the captured map itself omitted, distorted, or buried the required structural fact.
Arm validity requires only that control never gains access to codemap; treatment behavior is
not graded for following a command sequence.

Use `--resume` after interruption. Existing trials are reused only when the task,
base commit, composed arm prompt, activity parser and harness bytes, model, reasoning,
timeout, trial order, Codex version, arm, and verifier configuration produce the same
fingerprint. Codemap command artifacts and the full benchmark identity also participate,
so replacing a wrapper or binary without changing its version invalidates the old trial.
An agent crash, agent timeout, or verifier timeout is retried exactly once with that
fingerprint. The first raw attempt moves to `attempts/attempt-1`; the second attempt stays
at the trial root and links the preserved result. A normal verifier failure or any
treatment choice is product evidence and is never retried as infrastructure. Control access to
codemap is arm contamination and is not repaired by retrying the model.

For the frozen 72-run corpus, `scripts/benchmark-codemap-flagship.py evaluate` attempts
one comparative trajectory report per pair with the same frozen Codex identity.
Missing, incomplete, or failed analysis remains visible in `acceptance.json` but cannot
change external completeness, wins, costs, or acceptance. `--resume` reuses a report only
when its pair-context hash, analyzer prompt, model, reasoning effort, and Codex bytes match.

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
only. A crash, timeout, arm contamination, or broken exact-control boundary invalidates
the pair; navigation mistakes remain measured product losses.

Because the score is external, it measures the effects of understanding instead of
trusting an agent's self-description. A strong corpus contains tasks where the direct
edit is easy but important coupled surfaces are not obvious from the prompt.

Exact/local controls name the file, current bytes, and replacement bytes in the identical task
prompt. Both arms receive the same direct edit outcome. Treatment may use or ignore codemap;
the control measures whether availability alone preserves outcome without adding material cost.

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
completeness score according to its fixed weight. A Codex crash, timeout, or control-arm
codemap attempt makes the pair invalid. Treatment map use, non-use, failed calls, and
recovery remain part of the product result.

One task and one repetition are a smoke test, not evidence of general lift. A useful
product result needs multiple representative tasks from unfamiliar repositories,
several repetitions, external deterministic criteria across multiple categories, and
inspection of invalid pairs and patches alongside aggregate completeness. Report time
and tokens as secondary cost, preferably with cached input visible, not as a proxy for
quality.

## Frozen Flagship Gate

The exploratory harness remains the execution owner. The flagship wrapper adds only
`freeze`, `run`, and `evaluate`; it does not score prose or introduce another agent.

### Corpus and freeze

The corpus contains exactly six repositories across at least four ecosystems. Each
repository contributes one deterministic investigation, one multi-owner
implementation, and one exact/local control: 18 tasks total. Two counterbalanced
repetitions produce 36 pairs and 72 agent runs.

Each task declares executable deterministic verifiers. Investigation verifiers check
source-backed claims and concrete `path:line` citations inside local spans of the frozen
causal facts; an arbitrary line from the expected file or an opened-file inventory does
not score. They do not match prescribed report wording.
Implementation verifiers run independent hidden behavior groups or contract checks, so a partial
implementation produces deterministic partial completeness instead of one opaque pass bit. Their
prompts state the user-level outcome without publishing the internal owner path or full oracle
checklist. Exact controls check their local outcome. Response length, model
self-report, and another model's opinion are not evidence.

Each investigation prompt names one real runtime, file, or symbol anchor and asks a
behavioral question. It does not enumerate the expected owners, proof files, or chain
vocabulary; those remain hidden verifier claims rather than hints available to either arm.

```bash
python3 benchmarks/flagship/materialize.py \
  benchmarks/flagship/corpus-blueprint.json \
  --out-dir target/flagship-corpus-v1 --remote-only

cargo build --release --locked
python3 scripts/benchmark-codemap-flagship.py freeze \
  target/flagship-corpus-v1/corpus-draft.json \
  --out-dir target/flagship-frozen-v1 \
  --codemap-bin target/release/codemap
```

The manifest fixes task and prompt bytes, repository commits, verifier bytes, model
`gpt-5.6-sol`, reasoning `high`, the attributable codemap binary SHA-256, arm order,
timeouts, concurrency, one infrastructure retry, and the acceptance thresholds. Any
change requires a new frozen identity and a complete rerun.

### Run and evaluate

```bash
python3 scripts/benchmark-codemap-flagship.py run \
  target/flagship-frozen-v1/manifest.json \
  --out-dir target/flagship-run-v1

python3 scripts/benchmark-codemap-flagship.py evaluate \
  target/flagship-frozen-v1/manifest.json \
  --run-dir target/flagship-run-v1 \
  --out-dir target/flagship-acceptance-v1

python3 scripts/verify-flagship-acceptance.py \
  target/flagship-acceptance-v1/acceptance.json

python3 scripts/package-release.py build-evidence \
  --version 1.0.0 \
  --acceptance target/flagship-acceptance-failed/acceptance.json \
  --acceptance target/flagship-acceptance-v1/acceptance.json \
  --out-dir dist

python3 scripts/package-release.py verify-evidence \
  --archive dist/flagship-evidence-v1.0.0.tar.gz \
  --checksum dist/flagship-evidence-v1.0.0.tar.gz.sha256 \
  --version 1.0.0
```

Both arms use fresh detached worktrees and separate external caches. A verifier that
already passes at baseline rejects the task before model execution. An infrastructure
failure retries once with the identical manifest. A repeated failure, missing or extra
arm, arm contamination, provenance mismatch, or read-only task write remains in the
fixed denominator and makes the gate red.

Acceptance has three product conditions:

1. treatment wins at least 8 of 12 complex tasks by mean deterministic completeness
   across both repetitions and loses none;
2. treatment does not trail control on the two-repeat success count of any required criterion,
   and every exact control preserves the same two-repeat outcome count regardless of which
   stochastic repetition missed;
3. median complex overhead is at most 20% wall time and 15% input tokens, while median
   exact-control overhead is at most 10% for both metrics.

`acceptance.json` inventories the manifest, frozen tasks, raw results, per-trial
receipts, and verifier outputs by SHA-256. The independent verifier imports none of the
evaluator. The measured claim is restricted to this frozen six-repository corpus.
The release evidence archive requires exactly one accepted receipt and preserves every
supplied failed attempt, raw trajectory, diff, and external verifier output under its own
immutable prefix. Reproducible `codemap-cache` contents are deliberately omitted: they are
derived navigation state, not agent actions, outputs, diffs, or verifier evidence.
