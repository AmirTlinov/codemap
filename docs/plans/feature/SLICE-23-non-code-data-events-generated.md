# Slice 23: Non-Code, UI Assets, Data, Events, And Generated Ownership

## Intent

Map important relationships that are not plain imports, while keeping evidence
deterministic and fail-closed.

## Surface Families

Add or strengthen:

```txt
UI assets and styles
CSS/modules/tokens/themes
static assets referenced by code
SQL/schema/migration files
ORM schema/config
OpenAPI/GraphQL schemas
localStorage/sessionStorage keys
cookies
cache keys
Redis keys
S3/bucket path constants
message/event names where static
queue/topic/channel declarations
webhook identifiers
IPC/XPC channel identifiers
BroadcastChannel/postMessage/WebSocket message types
generated client/source ownership
test snapshots/fixtures
cross-language bridges from shell/config manifests
```

## Scope

Likely files:

```txt
src/repo/non_code*
src/repo/data*
src/repo/events*
src/model/*
src/map/lenses/*
schemas/*
tests/fixtures/*
```

## Implementation Steps

1. Add non-code surface kinds:
   - style;
   - asset;
   - theme_token;
   - migration;
   - database_schema;
   - api_schema;
   - event_topic;
   - generated_output;
   - fixture;
   - snapshot.
2. Extract static references:
   - import CSS/module;
   - asset path imports;
   - manifest-declared assets;
   - static SQL migration files;
   - static queue/topic names;
   - schema generator outputs.
   - static storage/config keys;
   - exact event emitter/webhook/queue/channel names;
   - Makefile/Dockerfile/GitHub Actions bridges to package commands.
3. Add unknowns for:
   - dynamic asset paths;
   - raw SQL literals;
   - dynamic topic names;
   - generated source owner unknown.
4. Link generated outputs back to source when deterministic.
5. Ensure docs/comments do not become hard facts.

## Acceptance

- UI/style/asset changes can appear in `changed`, `impact`, and `proof-map`.
- DB/schema/migration files appear as contract/data surfaces.
- Storage keys, cookies, cache keys, and exact message channels appear as data
  or event surfaces when statically known.
- Static event/topic edges appear where deterministic.
- Cross-language bridges are shown only from explicit manifests/config/scripts.
- Generated files are grouped and linked to source when possible.
- Dynamic data/event cases become unknowns.

## Load-Bearing Tests

Fixtures must include:

- CSS import;
- asset import;
- schema/migration file;
- localStorage or cookie key;
- static event/topic declaration;
- webhook or IPC/channel identifier;
- generated client with source marker;
- raw SQL literal unknown;
- dynamic asset path unknown;
- dynamic event/topic unknown.

Tests fail if raw SQL or dynamic event names become hard dataflow edges.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio ls .
codemap --root /Users/amir/Documents/projects/spritestudio impact --changed
codemap --root /Users/amir/Documents/projects/Sillentway-VPN ls .
```

Record whether non-code surfaces explain real change risk without flooding root.

## Reviewer Checklist

Reviewer checks:

```txt
non-code facts evidence-backed
generated ownership fail-closed
raw SQL not fake dataflow
event/string matching is exact API evidence only
assets/styles bounded
docs/comments soft only
```

## Done When

The map covers important non-code relationships without becoming a speculative
architecture document.
