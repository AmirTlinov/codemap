# Product Shape

`ctx` minimizes uncertainty for the next coding action.

It does not maximize context and does not try to make an agent understand the whole project before editing.

## Default UX

```bash
ctx start --task "fix auth refresh"
ctx impact --changed
ctx verify --changed
```

No project initialization is required.

## Optional Project Anchors

`.ctx.yml` is optional. It should contain semantic anchors that code cannot reliably reveal:

- source of truth;
- derived state;
- forbidden architectural edges;
- verification rules;
- recovery paths for forbidden moves.

Generated graphs, Mermaid views, impact reports, and caches are not committed by default.

## Output Invariant

A task capsule stays short:

- `read_first`: max 7 files;
- `do_not_read_yet`: max 8 entries;
- `verification`: max 3 commands;
- markdown capsule: max 150 lines.
