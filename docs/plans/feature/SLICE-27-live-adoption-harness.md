# Slice 27: Live Adoption Harness Across Real Projects

## Intent

Prove usefulness in real work, not just fixtures.

This slice does not add publishing. This is a personal project adoption gate,
including local PATH ergonomics so daily use does not pay a `cargo run` tax.

## Required Repos

At minimum:

```txt
/Users/amir/Documents/projects/spritestudio
/Users/amir/Documents/projects/Sillentway-VPN
one additional repo under /Users/amir/Documents/projects
```

## Scope

Likely files:

```txt
scripts/dogfood-codemap.sh
scripts/live-adoption-notes.md or docs/plans/feature/live-probes.md
README.md
docs/PRODUCT.md
doctor/PATH install docs or helper
tests/*
```

## Implementation Steps

1. Extend the existing non-mutating `scripts/dogfood-codemap.sh` harness unless
   there is a concrete reason to split it. Do not create a parallel live-probe
   script just to rename the same responsibility.
2. Add or document a local install path:
   - `cargo install --path .`; or
   - a local symlink/helper that keeps target repos untouched.
3. Make `doctor` report whether the visible `codemap` binary in `PATH` matches
   the current build well enough for dogfood.
4. Probe daily commands:
   - `doctor`;
   - `ls .`;
   - `graph --lens causal`;
   - `changed`;
   - `proof --changed`.
5. Probe focused lenses where anchors exist:
   - `runtime .`;
   - `proof-map .`;
   - `cone <anchor>`;
   - `contract <anchor>`;
   - `flow <anchor>`;
   - `siblings <scope>`;
   - `place <scope> --kind test`;
   - `delete <anchor>` without deleting.
6. For each repo, perform one real navigation task without starting from
   manual `rg`.
7. Record:
   - commands;
   - timings;
   - manual fallback points;
   - noisy output;
   - questionable claims;
   - whether agent would use it again.
8. Convert unacceptable live failures into implementation issues before final
   closure.

## Acceptance

- Live probe script is non-mutating.
- At least three repos are exercised.
- Live results identify actual gaps, not marketing notes.
- Tool is useful enough to voluntarily use again in each repo, or gaps are
  turned into blockers.
- No repo writes occur by default.
- A local `codemap` binary is usable from outside this repo without invoking
  `cargo run --bin codemap --` manually.

## Load-Bearing Tests

This slice relies on:

- live script dry-run;
- command exit status checks;
- output budget checks;
- schema validation on JSON outputs;
- manual agent satisfaction notes.

## Live Dogfood

Run:

```bash
scripts/dogfood-codemap.sh /Users/amir/Documents/projects/spritestudio
scripts/dogfood-codemap.sh /Users/amir/Documents/projects/Sillentway-VPN
scripts/dogfood-codemap.sh <third-project>
```

Then run one real "find my way around this domain" task in each repo using
`codemap` first.

## Reviewer Checklist

Reviewer checks:

```txt
non-mutating harness
real commands and timings
failures captured as gaps
agent satisfaction credible
PATH install does not become publishing scope
no publication scope
```

## Done When

The tool has survived real repo use and the remaining gaps are either fixed or
explicit blockers.
