# Slice 10: Git Structural Events, `changed`, And `diff-map`

## Intent

After edits, show how the map changed, not a textual diff dump.

`changed` is the daily entrypoint. `diff-map` is the detailed structural delta.

## Report Requirements

Capture:

```txt
changed files
added/removed imports
added/removed exports
changed symbols
added/removed runtime routes
added/removed env dependencies
added/removed contract surfaces
proof sensor changes
new/resolved unknowns
deleted/renamed/untracked structural events
typechanged/conflicted structural events
lockfile/manifest/config structural concern
generated-file ownership concern
expand targets into impact/proof/cone
```

## Scope

Likely files:

```txt
src/repo/changed.rs
src/map/lenses/changed.rs
src/map/lenses/diff_map.rs
src/render/changed.rs
schemas/changed.schema.json
schemas/diff-map.schema.json
tests/structural_map/changed_lens.rs
```

## Implementation Steps

1. Normalize git state:
   - unstaged;
   - staged;
   - untracked;
   - deleted;
   - renamed;
   - typechanged;
   - conflict;
   - lockfile/manifest/config changes;
   - ignored-but-imported files where deterministic.
   - compare base for `--since`.
2. Build old/new `FileInfo` for changed files using the same extractors.
3. Compare structural facts, not raw text.
4. Treat comments-only changes as no structural delta unless comments affect
   soft surfaces.
5. Render `changed` as compact grouped summary.
6. Render `diff-map` with full structural events and locations.
7. Add expand commands:
   - `codemap impact --changed`;
   - `codemap proof-map --changed`;
   - `codemap cone <changed-anchor>`.

## Acceptance

- Adding an import creates an added edge.
- Removing an export creates a removed contract/export surface.
- Body-only changes mark changed symbol body, not fake new edges.
- Comments-only changes do not create false structural events.
- Deleted files create delete events and affected reverse edges.
- Renamed files show `old -> new` and whether consumers still point at old path.
- Untracked tests/proof sensors are visible.
- Conflicts and typechanged files are explicit git-state concerns, not generic
  missing-file noise.
- Lockfile/manifest/config changes raise package/workspace/config concerns.
- Generated-file edits point at source ownership when deterministic, or unknown
  when ownership is not known.

## Load-Bearing Tests

Fixture tests must cover:

- added import;
- removed import;
- added export;
- removed export;
- route addition;
- env lookup addition;
- proof file addition;
- comment-only change;
- deleted file;
- renamed file.
- typechanged file;
- conflicted file;
- lockfile/manifest change;
- generated file change with known and unknown source owner.

Tests fail if `changed` falls back to raw textual diff.

## Live Dogfood

Run on dirty repos if available:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio changed
codemap --root /Users/amir/Documents/projects/spritestudio diff-map --changed
codemap --root /Users/amir/Documents/projects/Sillentway-VPN changed
```

Record whether it replaces the first manual `git diff --stat` / `rg` pass.

## Reviewer Checklist

Reviewer checks:

```txt
structural delta, not text diff
dirty/staged/untracked handled
comments-only is not overclaimed
locations included
expand goes to useful next lenses
```

## Done When

After-edit orientation starts with `codemap changed`.
