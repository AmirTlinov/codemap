# Behavioral A/B Benchmark

`scripts/benchmark-codemap-ab.py` runs the same implementation or repository-analysis
task twice with Codex:

- `control`: `codemap` is blocked and ordinary repository navigation remains available;
- `codemap`: the agent must follow the mode-specific codemap navigation protocol.

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

In `implementation` mode, treatment must run `codemap ls .` before editing and
`codemap changed` plus `codemap proof changed` after editing. In `analysis` mode,
treatment must run `codemap ls .` and at least one exact or focused map; any repository
change makes the analytical outcome fail. Control has ordinary repository tools but a
blocking codemap shim in both modes.

Use `--resume` after interruption. Existing trials are reused only when the task,
base commit, model, reasoning, Codex version, arm, and verifier configuration produce
the same fingerprint.

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
