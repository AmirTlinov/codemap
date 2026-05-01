# Slice 21: Flow Lens As Bounded Structural Path

## Intent

Show deterministic execution/request paths without pretending full dataflow.

`flow <anchor>` answers:

```txt
where does this path start?
which structural steps are known?
which contracts/proof sensors are connected?
where does traversal stop and why?
```

## Scope

Likely files:

```txt
src/map/lenses/flow.rs
src/render/flow.rs
schemas/flow.schema.json
tests/fixtures/*
tests/structural_map/*
```

## Sources

Flow may use only proven facts:

```txt
runtime route/file convention
static route registration
resolved import
symbol reference where deterministic
contract edge
env/config surface
proof edge
package boundary edge
```

Stop at:

```txt
dynamic import
DI token not resolved
raw SQL literal
reflection
macro expansion boundary
unsupported framework
unresolved import
ambiguous route owner
```

## Implementation Steps

1. Add `FlowStep` and `FlowReport`.
2. Support anchors:
   - route;
   - file;
   - symbol;
   - runtime surface.
3. Build bounded traversal over structural edges.
4. Add `unknown_breaks` with reason/effect/expand.
5. Include contracts and proof sensors connected to steps.
6. Add depth/step budget.
7. Ban complete-dataflow language.

## Acceptance

- Static route can lead to handler/file, imports, contract/proof where known.
- Symbol flow follows deterministic references only.
- Dynamic middle stops traversal with unknown_break.
- Flow has locations on steps.
- Output is bounded.
- No full dataflow claim exists.

## Load-Bearing Tests

Tests fail if:

- flow crosses dynamic import as if known;
- unsupported framework route becomes complete flow;
- raw SQL literal becomes fake table/dataflow;
- route-to-handler link lacks location;
- flow output claims completeness.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio flow <known-route-or-file>
codemap --root /Users/amir/Documents/projects/Sillentway-VPN flow <known-entrypoint>
codemap --root <third-project> flow <known-route-or-entrypoint>
```

Record whether flow clarifies what to read next without overclaiming.

## Reviewer Checklist

Reviewer checks:

```txt
bounded traversal
unknown breaks
no full dataflow claim
locations per step
contracts/proof attached only if evidenced
```

## Done When

Flow is a useful path lens, not a false program-analysis oracle.
