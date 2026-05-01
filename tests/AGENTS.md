# Test Map

This directory verifies codemap behavior.

Areas:

- `structural_map.rs` is the include-root for one integration crate.
- `structural_map/support_*` owns shared fixtures and helpers.
- `structural_map/*` topic files own map/proof/impact/boundary scenarios.
- `line_budget.rs` guards AI-friendly file and AGENTS map size.

Keep tests topic-local. Do not split topic files into separate integration crates unless runtime cost is intentional.
