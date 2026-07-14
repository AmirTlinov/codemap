# Renderer Map

This directory only prints already-built reports.

Group owners:

- `status.rs` renders doctor/status and the JSON printer; owns the `--brief` switch.
- `prelude.rs` renders the repo/worktree prelude block.
- `ls_cone.rs` and `cone_xray.rs` render structural `ls` and `cone`.
- `where_locator.rs` renders `where`.
- `changed/` renders the changed lens: sections, worktree, structural events, surface hints, proof, expand.
- `proof/` renders verification surfaces: impact, coverage, wiring, plan sections.
- `lenses.rs` renders the focused lenses (runtime, contract, flow, siblings, place, delete).
- `teach.rs` and `boundary_facts.rs` render teach output and boundary facts.
- `boundaries_graph_init.rs` renders boundaries, graph, init/bootstrap text.
- `helpers.rs` owns Markdown tables, bullets, code blocks, Mermaid escaping.

Do not compute repo facts here. If output needs new data, add it to `model` and build it in `map`.
