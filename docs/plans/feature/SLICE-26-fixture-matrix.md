# Slice 26: Fixture Matrix Across Stacks

## Intent

Prove `codemap` is project-agnostic, not overfit to its own repo or one app.

## Required Fixtures

Minimum:

```txt
fixtures/ts-monorepo
fixtures/next-app
fixtures/node-backend
fixtures/python-fastapi
fixtures/go-http-service
fixtures/rust-cli
fixtures/mixed-monorepo
```

Each fixture must cover:

```txt
root ls
graph causal
cone file
cone directory
runtime
contract
proof-map
diff-map
impact
delete
boundary-map
flow
siblings
place
unknown dynamic case
```

## Scope

Likely files:

```txt
fixtures/*
tests/fixtures/*
tests/structural_map/*
tests/golden*
schemas/*
```

## Implementation Steps

1. Audit existing fixtures and map coverage gaps.
2. Add or extend fixtures with minimal code per stack.
3. Add dynamic blind spot case per fixture where natural.
4. Add golden JSON validation for each public lens.
5. Add markdown cognitive golden for root and exact anchors.
6. Add changed/diff fixtures using old/new file text or git fixture harness.
7. Keep fixtures small and purposeful.

## Acceptance

- Every public lens has fixture coverage.
- Every supported stack has root/exact/runtime/proof coverage.
- Dynamic cases fail closed.
- Fixture tests are load-bearing, not file-existence checks.
- Fixture output stays compact.

## Load-Bearing Tests

Tests fail if:

- a public lens has no fixture coverage;
- a fixture lacks unknown dynamic case;
- JSON golden violates schema;
- markdown root output exceeds budget;
- a dynamic route/env/import becomes hard evidence.

## Live Dogfood

This slice uses fixtures plus live repos. Run:

```bash
cargo test --quiet fixture
codemap --root /Users/amir/Documents/projects/spritestudio ls .
codemap --root /Users/amir/Documents/projects/Sillentway-VPN runtime .
codemap --root <third-project> ls .
```

Record which live gaps are not covered by fixtures yet.

## Reviewer Checklist

Reviewer checks:

```txt
fixture breadth
no overfitting
dynamic fail-closed cases
schema validation
load-bearing assertions
```

## Done When

The test matrix catches regressions across stacks before live dogfood does.
