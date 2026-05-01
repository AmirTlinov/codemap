# Slice 08: Fast Scanner, Ignore Rules, Generated And Vendor Detection

## Intent

Make cold scans fast and trustworthy by avoiding directories and files that
should not be treated as source ownership.

## Scope

Likely files:

```txt
src/repo/*
src/cache/*
src/model/*
tests/fixtures/*
tests/structural_map/*
```

## Implementation Steps

1. Centralize repository walking policy.
2. Respect:
   - `.gitignore`;
   - common generated/vendor dirs;
   - dependency dirs like `node_modules`, `.venv`, `target`, `dist`, `build`;
   - lockfile-specific package manager artifacts;
   - `.ctx.yml` ignore overrides if already supported.
3. Add generated-file detection by hard evidence:
   - generated header comment;
   - known generated path;
   - manifest/tool output directory;
   - source map pairing.
4. Store generated/vendor status on `FileInfo` or equivalent.
5. Ensure generated files may appear as surfaces but are not treated as owners
   when deterministic source exists.
6. Add scanner timing counters:
   - files visited;
   - files skipped;
   - bytes scanned;
   - ignored groups;
   - generated groups.

## Acceptance

- Cold scans skip obvious heavy directories.
- Generated/vendor files do not dominate root maps.
- Ignored/generated counts are visible in doctor or hidden groups.
- The scanner never silently skips source directories without an explainable
  rule.
- Performance improves without losing source facts.

## Load-Bearing Tests

Fixtures must include:

- ignored dependency directory;
- generated client file;
- generated file with source owner;
- vendor directory with source-like files;
- root map where generated files are hidden as a group.

Tests fail if generated/vendor files become primary owner surfaces.

## Live Dogfood

Run cold and warm:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio doctor
codemap --root /Users/amir/Documents/projects/Sillentway-VPN doctor
codemap --root <third-project> ls .
```

Record skipped counts and whether any real source disappeared.

## Reviewer Checklist

Reviewer checks:

```txt
no hidden source loss
ignore rules are explainable
generated ownership is fail-closed
scanner counters are honest
performance gain is real
```

## Done When

Scanning is faster and less noisy without pretending generated artifacts are
the source of truth.

