# Agent Bootstrap

This repository builds `codemap`: a structural code-map CLI for AI coding agents.

Keep the product invariant clear:

- global binary, project-agnostic;
- zero repository writes by default;
- external cache by default;
- root `codemap ls .` returns a bounded domain/package map, not the whole project;
- root `codemap graph --lens causal` is the current-level map lens, not a recursive file dump;
- exact scopes/files use `codemap ls <anchor>` and `codemap cone <anchor>`;
- after edits use `codemap diff-map --changed`, `codemap impact --changed`, `codemap proof-map --changed`, and `codemap proof --changed`;
- optional `.ctx.yml` semantic anchors only when hard architecture truth cannot be inferred;
- no task router, no ranking engine, no embeddings, no LLM in the hard path;
- `proof` prints a plan by default and runs commands only with `--run`.

Start code work with:

```bash
cargo test
```

Before finishing:

```bash
cargo fmt --check
cargo test
cargo run --bin codemap -- doctor
```
