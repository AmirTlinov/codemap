# Structural Map Tests

This directory is one included integration test crate.

Support:

- `support_core.rs` has process helpers.
- `support_fixture.rs` builds the mixed monorepo fixture.
- `support_assertions.rs` has shared assertions.
- `support_*_fixture.rs` keeps large setup out of test bodies.

Topic files mirror product lenses:

- root and directory maps: `root_*`, `directory_*`;
- proof surfaces: `proof_*`, `support_import_*`, `ui_surface_*`;
- symbol xref: `symbol_*`;
- anchors, boundaries, schemas: `anchors_*`, `boundaries_graph_schema.rs`;
- legacy removal checks: `removed_legacy_surfaces.rs`.

Add new tests near the lens they protect, not at the root include file.
