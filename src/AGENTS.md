# Source Map

This directory is the codemap runtime, organized as a real module tree.

Main areas:

- `cli/` parses commands, root hints, config validation, run-mode safety.
- `repo/` builds project truth from files, manifests, imports, symbols, anchors, git.
- `map/` turns repo truth into structural reports: `ls`, `cone`, `impact`, `proof`, `boundaries`.
- `render/` prints Markdown, JSON-facing text, Mermaid, bootloader snippets.
- `model.rs` + `model/` own serializable report/schema structs and shared enums.
- `cache.rs` + `cache/` own external cache paths, fingerprints, snapshots, artifacts.

Read path:

1. command surface in `cli/`;
2. repo extraction in `repo/`;
3. report construction in `map/`;
4. output shape in `render/` and `schemas/`.

Module rules:

- every file is a real `mod` with explicit imports; no `include!` in `src/`;
- every file declares one `// Responsibility: kebab-case-id` and stays ≤400 lines;
- layer direction is compile-time: `repo` imports no `map`/`render`/`cli`;
- parent modules re-export children (`pub(crate) use x::*;`) to keep flat
  `crate::map::*` / `crate::repo::*` paths stable; never shadow your own glob
  re-export with an explicit `use` of the same name.
