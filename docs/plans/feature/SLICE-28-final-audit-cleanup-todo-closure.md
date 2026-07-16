# Slice 28: Final Audit, Cleanup, And TODO Closure

## Intent

Close the feature only when the whole system is coherent, reviewed, and useful
end to end.

## Scope

Likely files:

```txt
README.md
docs/PRODUCT.md
docs/IMPLEMENTATION.md
docs/plans/feature/TODO.md
schemas/*
tests/*
scripts/*
src/*
```

## Implementation Steps

1. Run full gates:
   - `cargo fmt --check`;
   - `cargo test --quiet`;
   - `cargo clippy --all-targets -- -D warnings`;
   - `cargo run --quiet --bin codemap -- doctor`;
   - `git diff --check`.
2. Run all fixture/golden/schema/performance gates.
3. Run live adoption harness.
4. Audit public docs for:
   - no router language;
   - no ranking/semantic/LLM promises;
   - no release/publishing scope;
   - daily command flow first;
   - focused lenses as expand paths.
5. Audit code for:
   - legacy v1 leakage into structural lenses;
   - duplicate private parsers;
   - direct edge/surface construction outside helpers;
   - false hard evidence;
   - unbounded output paths.
6. Update `TODO.md` only for slices actually completed.
7. Spawn final reviewer with:
   - changed file summary;
   - gate outputs;
   - live probe summary;
   - known remaining gaps;
   - explicit product invariants.

## Acceptance

- Every public lens works.
- Every public JSON output has schema.
- Root views are bounded.
- Exact anchors are useful.
- Unknowns are typed.
- Performance is acceptable.
- Markdown is compact.
- JSON is complete.
- Live dogfood is acceptable.
- No known false structural claim remains.
- Final reviewer returns PASS.

## Load-Bearing Tests

Final closure needs all tests plus live proof. A green unit suite alone is not
enough.

Required proof:

```txt
cargo fmt --check
cargo test --quiet
cargo clippy --all-targets -- -D warnings
cargo run --quiet --bin codemap -- doctor
git diff --check
scripts/dogfood-codemap.py /Users/amir/Documents/projects/spritestudio
scripts/dogfood-codemap.py /Users/amir/Documents/projects/Sillentway-VPN
scripts/dogfood-codemap.py <third-project>
```

## Live Dogfood

Repeat the full adoption harness from Slice 27, then perform one fresh real
navigation task in each required repo:

```txt
spritestudio: start from root map, find a real domain, inspect cone/proof
Sillentway-VPN: start from runtime/config map, inspect contract/proof
third repo: start from root map, inspect one runtime or package boundary
```

For each repo, final notes must answer:

```txt
Did codemap reduce manual ls/rg/git diff usage?
Was any output too noisy or duplicated?
Was any claim questionable or false?
Was warm speed acceptable?
Would the agent voluntarily start with codemap next time?
```

If any answer is unacceptable, do not close the plan. Convert the issue into a
blocker or a new explicit slice.

## Reviewer Checklist

Final reviewer checks:

```txt
all product invariants
all public lenses
all schemas
fixture coverage
live dogfood
speed/cognitive gates
legacy leakage
fake claims
TODO honesty
```

Reviewer must return one of:

```txt
PASS
CHANGES
BLOCK
```

## Done When

`TODO.md` is honestly checked, final reviewer returns PASS, and the final status
can truthfully say: `codemap` is faster, clearer, and more trustworthy than
manual shell-first navigation in the tested repos.
