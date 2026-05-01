# Slice 15: Runtime Lens

## Intent

Show deterministic execution entrypoints and runtime configuration without
guessing.

`runtime` answers:

```txt
how does this code get executed?
what scripts/routes/jobs/commands/env vars touch this scope?
where does runtime certainty stop?
```

## Scope

Likely files:

```txt
src/map/lenses/runtime.rs
src/repo/runtime*
src/model/*
src/render/*
schemas/runtime.schema.json
tests/fixtures/*
```

## Deterministic Extractors

Support hard/high evidence for:

```txt
package.json scripts/bin
Cargo bin/default bin/main.rs
pyproject scripts/entry-points
main.go
__main__.py
Next app/pages routes
Express/Fastify static route registrations
FastAPI/Flask decorators
Go net/http static handlers
Clap subcommand enums/derive where deterministic
CI/build files
workers/jobs by exact file convention
static env lookups
runtime realm markers:
  "use client"
  "use server"
  Next edge/node runtime exports
  middleware files
  server-only/browser-only imports where deterministic
  Rust cfg target_os / feature markers
config-as-code:
  tsconfig paths
  Playwright webServer/baseURL/testDir
  Next/Vite config
  Makefile targets
  Dockerfile commands
  GitHub Actions jobs
```

Dynamic cases become unknowns:

```txt
route_string_concat
route_dynamic_path
route_dynamic_method
env_dynamic_lookup
unsupported_framework_route
ambiguous_route_owner
```

## Implementation Steps

1. Add `RuntimeRoute` and runtime surface constructors if missing.
2. Extract package scripts and bins from manifests.
3. Extract framework routes only for exact static patterns.
4. Extract CLI entrypoints from language conventions.
5. Extract static env lookups with line locations.
6. Extract runtime realm facts and mark conditional surfaces as conditional,
   not always-on hard edges.
7. Extract config-as-code surfaces that affect runtime/proof/import resolution.
8. Link runtime routes/scripts/bins to handler files/symbols where deterministic.
9. Add runtime proof sensors from e2e route visits and command tests.
10. Render root runtime as containers, not every route.

## Acceptance

- `runtime .` shows scripts, CI, app/CLI containers, env groups.
- `runtime <scope>` shows relevant routes/commands/jobs/env refs.
- Static routes have method/path/location.
- Dynamic routes/env become typed unknowns.
- Realm facts are visible without becoming moral boundary claims.
- Config-as-code can feed runtime/proof/import maps where deterministic.
- Runtime does not claim unsupported frameworks as known.

## Load-Bearing Tests

Fixture matrix must include:

- Next app route;
- Express static route;
- FastAPI decorator;
- Go net/http route;
- Rust `main.rs` or Clap command;
- package script/bin;
- static env lookup;
- dynamic env lookup;
- dynamic route path.
- Next client/server/edge marker;
- Playwright config webServer/testDir;
- GitHub Actions job or Makefile target.

Tests fail if dynamic routes become hard routes.

## Live Dogfood

Run:

```bash
codemap --root /Users/amir/Documents/projects/spritestudio runtime .
codemap --root /Users/amir/Documents/projects/Sillentway-VPN runtime .
codemap --root <third-project> runtime .
```

Record whether runtime entries are clearer than manually reading manifests and
route folders.

## Reviewer Checklist

Reviewer checks:

```txt
only deterministic route claims
env dynamic unknowns
root runtime bounded
runtime links to proof where real
unsupported frameworks fail closed
```

## Done When

Runtime entrypoints are visible as map facts, not buried in manifests and
framework conventions.
