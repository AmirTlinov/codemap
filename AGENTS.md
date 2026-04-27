# Agent Bootstrap

This repository builds the universal `ctx` CLI.

Keep the product invariant clear:

- global binary, project-agnostic;
- zero repository writes by default;
- external cache by default;
- optional `.ctx.yml` semantic anchors only when hard architecture truth cannot be inferred;
- no embeddings or LLM in the hard routing path;
- `verify` prints a plan by default and runs commands only with `--run`.

Start code work with:

```bash
cargo test
```

Before finishing:

```bash
cargo fmt --check
cargo test
cargo run --bin ctx -- doctor
```
