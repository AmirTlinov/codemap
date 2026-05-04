# CLI Map

This directory is command orchestration, not code-map logic.

Files:

- `args.rs` defines Clap command and flag shapes.
- `run.rs` dispatches commands to repo/map/render surfaces.
- `schema_and_roots.rs` resolves schema text and root hints.
- `init.rs` owns explicit project writes only.
- `proof_run.rs` owns `proof --run` safety.
- `diff_args.rs` parses changed/staged/since/files selectors.
- `files.rs` owns `codemap files`.
- `anchors_*` owns `.codemap.yml` validation reporting.

Do not add structural inference here. CLI should call `repo` or `map`.
