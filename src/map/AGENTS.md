# Structural Map

This directory turns `Project` truth into bounded maps for agents.

Main report surfaces:

- `status.rs` and `entry.rs` build repo status and shared entrypoints.
- `ls.rs`, `directory_*`, `file_metadata.rs` build bounded structural listing.
- `cone_*`, `symbol_*`, `test_*` build local edge cones and verification edges.
- `impact.rs`, `proof_*`, `command_inference.rs` build blast-radius and verification surface plans.
- `boundary.rs`, `package_consumers.rs` check explicit boundaries and package consumers.
- `graph_lens.rs` renders small graph lenses only.

Invariant:

- show structural edges and hidden counts;
- do not rank by task text;
- do not guess source-of-truth;
- keep root/domain views bounded.
