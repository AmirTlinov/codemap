# Renderer Map

This directory only prints already-built reports.

Files:

- `status.rs` renders doctor/status and JSON printer.
- `ls_cone.rs` renders structural `ls` and `cone`.
- `impact_proof.rs` renders `impact` and `proof`.
- `boundaries_graph_init.rs` renders boundaries, graph, init/bootstrap text.
- `helpers.rs` owns Markdown tables, bullets, code blocks, Mermaid escaping.

Do not compute repo facts here. If output needs new data, add it to `model` and build it in `map`.
