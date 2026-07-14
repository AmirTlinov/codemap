# Repo Extraction Map

This directory builds deterministic repo truth.

Flow:

1. `root.rs`, `config.rs`, `scan.rs` resolve scope and files.
2. `roles/`, `file_extract.rs`, `symbol_ranges.rs` classify files and symbols.
3. language groups extract imports, exports, surfaces, tests, packages.
4. `resolution/` (imports, paths, ts_aliases, languages) resolves edges.
5. `project.rs`, `packages/metadata.rs`, `changed.rs` assemble project state.

Group owners:

- `js/` owns JS/TS scanning: scanner, imports, params, call parse, JSX refs and accessibility, code strip.
- `roles/` owns file role classification: source, build/CI, custom, structural surfaces.
- `packages/` owns package detection, dependency edges, scripts, workspace members.
- `resolution/` owns import/path/language resolution.
- `surfaces/` owns UI/test surface phrase and token extraction.
- `components/` owns component contract analysis; `playwright/` owns e2e test surfaces.
- `tests/` owns the repo unit tests.

Facts here must come from files, manifests, git, or explicit anchors.
This layer imports no `map`, `render`, or `cli` names — keep it that way.
