#!/usr/bin/env bash
set -euo pipefail
test -f contracts/events.openapi.yaml
test -f packages/events-api/src/events.ts
