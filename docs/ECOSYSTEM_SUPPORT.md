# Ecosystem Support Contract

The versioned release declaration lives in `schemas/manifest.json` under
`ecosystem_support_version` and `ecosystem_support`. `codemap status --format json`
and `codemap doctor --format json` project that declaration onto the ecosystems
actually observed in the current repository. This keeps the release promise and
the live diagnostic on one owner.

Each row covers the same cells: inventory, symbols, imports, packages, runtime,
contracts, data, verification, and dynamic unknowns. Cell states mean:

- `verified`: a fixture and behavioral criterion exercise this cell;
- `structural`: deterministic facts exist, but the ecosystem tier makes a narrower promise;
- `inventory`: paths and Git state only;
- `unsupported`: the layer is outside the declared grammar and must remain open;
- `not_applicable`: the cell does not form part of that ecosystem surface.

## Release tiers

| Ecosystem | Tier | Public promise |
| --- | --- | --- |
| JavaScript/TypeScript | A | Behavioral structural map in the declared cells, including runtime and contract/data chains |
| Python | A | Behavioral structural map in the declared cells, including runtime and external-process verification boundaries |
| Rust | A | Behavioral structural map in the declared cells, including Cargo packages and supported runtime forms |
| Go | A | Behavioral structural map in the declared cells, including modules and supported HTTP runtime forms |
| Swift | B | Structural orientation across package, symbol, import, and verification cells |
| Shell, SQL, YAML/config, schema/protocol, generated clients | C | Inventory and typed boundaries; specialized observed cells do not raise the tier |
| Other source languages | C | Inventory coordinates and explicit unsupported classification only |

Tier A does not mean compiler-equivalent semantics. The machine declaration lists
limits for computed imports, macros, interface dispatch, dynamic module loading,
reflection, generated code, and framework forms. A row never borrows support from
another language merely because both occur in one repository.

## Mixed-language boundary rule

Cross-language relationships require exact carriers already owned by the map:
manifest dependencies, schema/codegen input and output paths, generated package
exports, static subprocess argv, protocol files, or migration/data identifiers.
A lexical resemblance cannot create a cross-language edge. If the exact carrier
is computed or external, the shared horizon stays open with
`external_runtime_boundary`, `unsupported_construct`, or `unsupported_language`.

Generated files stay visible and retain their source-language facts, while a
separate `generated clients` row records generated ownership. Exact source/output
paths are required before the contract lineage can claim provenance.

## Unsupported projects

Common source extensions without a semantic parser are still indexed as
`other source languages`. Root maps provide bounded directory and Git inventory;
exact file maps return `unsupported_language` rather than empty symbol or consumer
claims. Arbitrary binary and unknown data formats remain outside text indexing.
