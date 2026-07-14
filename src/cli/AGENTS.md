# CLI Map

This directory is command orchestration, not code-map logic.

Owners:

- `args.rs` defines Clap command and flag shapes.
- `run.rs` dispatches commands to repo/map/render surfaces.
- `fast_paths/` serves warm-cache answers without a full project load.
- `schema_and_roots.rs` resolves schema text and root hints.
- `init.rs` owns explicit project writes only.
- `proof_run/` owns `proof --run` safety.
- `diff_args.rs` parses changed/staged/files selectors; `since_args.rs` resolves `--since` tokens.
- `section_args.rs` parses `--section` filters.
- `files.rs` owns `codemap files`.
- `anchors/` owns `.codemap.yml` validation reporting.

Do not add structural inference here. CLI should call `repo` or `map`.
