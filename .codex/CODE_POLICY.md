# Code file policy

The repository has one small mechanical guardrail: a new code file must contain
at most 400 physical lines. `tests/line_budget.rs` owns the enforced check and
runs it through normal `cargo test` and CI.

"One responsibility" remains an architectural review question: a file should
have one coherent reason to change and should be split when independently
changing concerns appear. No comment, marker, or regex can prove that property,
so the hook does not try to police it.

Generated files with an explicit `@generated`, `Code generated`, or `DO NOT
EDIT` header are excluded. Build, dependency, coverage, and VCS directories are
excluded.

## Why this is not a blocking Stop hook

Codex 0.144.4 has an open continuation bug: after a blocking `Stop` hook, it can
send the locally generated continuation UUID to the Responses API as a message
ID. The API correctly rejects that ID because message IDs must begin with
`msg_`, leaving the session unable to continue. See
[`openai/codex#20783`](https://github.com/openai/codex/issues/20783).

The project therefore does not install a blocking lifecycle hook until that bug
is fixed in the shipped Codex version. A deterministic failing test is a harder
and safer enforcement boundary than a hook that can corrupt the agent loop.

## Existing debt

Files that were already over 400 lines when this policy was introduced are
listed in `.codex/legacy-oversize.tsv`. Each recorded size is a debt ceiling:

- an unlisted file may not cross 400 lines;
- a listed file may shrink gradually but may not exceed its ceiling;
- when a listed file reaches 400 lines, its exemption must be removed;
- lowering a ceiling after a meaningful split is encouraged, but ordinary small
  edits do not require baseline bookkeeping.

## Manual checks

```bash
python3 .codex/hooks/code_policy.py self-test
python3 .codex/hooks/code_policy.py check-all
cargo test --test line_budget
```
