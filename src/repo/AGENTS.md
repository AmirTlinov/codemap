# Repo Extraction Map

This directory builds deterministic repo truth.

Flow:

1. `root.rs`, `config.rs`, `scan.rs` resolve scope and files.
2. `roles.rs`, `file_extract.rs`, `symbol_ranges.rs` classify files and symbols.
3. language-specific chunks extract imports, exports, surfaces, tests, packages.
4. `import_resolution.rs`, `path_resolution.rs`, `ts_aliases.rs` resolve edges.
5. `project.rs`, `project_metadata.rs`, `changed.rs` assemble project state.

Important split points:

- JS/TS import and symbol handling live in `js_*`, `symbols_*`, `component_*`.
- UI/e2e surface extraction lives in `playwright_*`, `jsx_*`, `surface_literals.rs`.
- package dependency graph lives in `package_edges_*`.

Facts here must come from files, manifests, git, or explicit anchors.
