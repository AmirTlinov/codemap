# Source Map

This directory is the codemap runtime.

Main areas:

- `cli/` parses commands, root hints, config validation, run-mode safety.
- `repo/` builds project truth from files, manifests, imports, symbols, anchors, git.
- `map/` turns repo truth into structural reports: `ls`, `cone`, `impact`, `proof`, `boundaries`.
- `render/` prints Markdown, JSON-facing text, Mermaid, bootloader snippets.
- `model.rs` owns serializable report/schema structs and shared enums.
- `cache.rs` owns external cache paths and status artifacts.

Read path:

1. command surface in `cli/`;
2. repo extraction in `repo/`;
3. report construction in `map/`;
4. output shape in `render/` and `schemas/`.

Keep files small and named by the code surface they own.
