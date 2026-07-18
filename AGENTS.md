# Agent Bootstrap

This repository builds `codemap`: a structural code-map CLI for AI coding agents.

Keep the product invariant clear:

- global binary, project-agnostic;
- zero repository writes by default;
- external cache by default;
- preserve the narrowest task-named entry: `codemap where <symbol>` when only a symbol is known,
  `codemap cone <file-or-file#symbol>` for an exact file, or `codemap ls <directory>` for a directory;
- never widen a task-named file to its parent directory;
- use root orientation `codemap ls .` only when the relevant scope is unknown; it returns a
  bounded domain/package map, not the whole project;
- root `codemap graph --lens causal` is the current-level map lens, not a recursive file dump;
- after edits use `codemap changed` first, then `codemap proof changed`;
- use focused lenses such as `diff-map`, `impact`, and `proof-map` through
  exact `expand` commands when the changed/proof map asks for more detail;
- optional `.codemap.yml` semantic anchors only when hard architecture truth cannot be inferred;
- no task router, no ranking engine, no embeddings, no LLM in the hard path;
- `proof` prints a plan by default and runs commands only with `--run`;
- `контракт-спецификация.md` owns the one outcome-based flagship criterion;
  strengthen existing map owners from external task evidence instead of adding
  governance layers, subjective scoring, or a task router;
- every new code file stays at or below 400 physical lines; legacy oversize files
  may shrink gradually but may not exceed the ceilings recorded in
  `.codex/legacy-oversize.tsv`;
- each code file has one coherent reason to change; split independently changing
  concerns through their existing owners. This is an architectural review
  invariant, not a required marker or regex check.

Start code work with:

```bash
cargo test
```

Before finishing:

```bash
python3 .codex/hooks/code_policy.py check-all
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
python3 scripts/check-version-bump.py
cargo run --bin codemap -- doctor
```
