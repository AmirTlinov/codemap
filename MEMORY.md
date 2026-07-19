# Codemap repository memory

- `src/repo/resolution/` owns deterministic import-to-indexed-file resolution; `resolve_imports` records resolved targets and bindings on `FileInfo`.
- Python resolution lives in `src/repo/resolution/languages.rs::resolve_python`: a bare spec checks the importing file's directory before repository layout and detected-package fallbacks; relative specs use Python dot levels.
- The observable contract is a `cone` import edge with `evidence=resolved_import`; `tests/structural_map/unresolved_import_unknowns.rs` covers both unresolved local imports and package-local bare imports.
- Repository verification is defined by root `AGENTS.md`; after edits inspect `codemap changed` and `codemap proof changed` before running the full gate.
