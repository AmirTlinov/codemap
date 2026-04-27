# Implementation Plan

Build `ctx` as a fast deterministic CLI, then add adapters and routing depth without changing the user-facing contract.

## Phase 1: Repo Truth

- repo root discovery;
- git changed-file discovery;
- ignored/build/generated file filtering;
- manifest detection;
- external cache fingerprints.

Proof:

```bash
cargo test repo
ctx status --json
```

## Phase 2: Task Capsule

- scope scoring from task text, path, filenames, manifests, and nearby tests;
- bounded `read_first`;
- required `do_not_read_yet`;
- confidence and stop rules.

Proof:

```bash
ctx start --task "fix broken save" --format json
```

## Phase 3: Impact And Verification

- `ctx impact --changed`;
- package/test/public-boundary risk;
- `ctx verify --changed` print-only plan;
- `ctx verify --changed --run` explicit execution.

Proof:

```bash
ctx impact --changed --format json
ctx verify --changed
```

## Phase 4: Adapters

Start with robust lightweight adapters:

- generic;
- JavaScript/TypeScript;
- Rust;
- Python;
- Go;
- Swift.

Adapters should provide imports, exports, tests, commands, and package boundaries. They should not try to become full compilers.

## Phase 5: Optional Anchors

Add `.ctx.yml` support for semantic facts that code cannot reliably reveal:

- concepts;
- source-of-truth roles;
- derived state;
- forbidden boundaries;
- recovery paths;
- verification rules.

Hard architectural facts require explicit anchors. Heuristics can suggest, not enforce.

## Phase 6: Lenses

- `ctx explain`;
- `ctx widen`;
- `ctx graph --lens ownership|impact|boundary|verification`;
- Markdown, JSON, Mermaid renderers.

## Release Standard

No release is acceptable unless:

- `ctx` binary runs from PATH;
- default mode writes nothing to the target project;
- cache lives outside target repos;
- fixture tests cover at least Rust, TypeScript, Python, Go, Swift-like, and mixed monorepo shapes;
- low-confidence routing is explicit and actionable.
