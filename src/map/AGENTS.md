# Structural Map

This directory turns `Project` truth into bounded maps for agents.

Group owners:

- `status.rs` and `entry.rs` build repo status and shared entrypoints.
- `listing/` builds bounded structural listing: `ls`, root inventory, directory edges, file metadata.
- `cone/` builds the local frame around one anchor: traversal, xray card, owner/env surfaces.
- `symbols/` builds symbol definition and reference edges, including `where`.
- `proof/` builds verification surface discovery: edges, owners (manifest/CI), wiring, coverage, commands.
- `test_edges.rs`, `test_surface.rs` link test surfaces to anchors.
- `impact.rs`, `command_inference*` build blast-radius and command plans.
- `boundary/`, `package_consumers.rs` check explicit boundaries and package consumers.
- `lenses/` builds focused lenses: changed, diff_map, proof_map, runtime, flow, contract, siblings/place, delete, boundary_map.
- `graph_lens.rs` renders small graph lenses only.

Invariant:

- show structural edges and hidden counts;
- do not rank by task text;
- do not guess source-of-truth;
- keep root/domain views bounded.
