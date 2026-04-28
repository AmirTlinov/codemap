#!/usr/bin/env bash
set -euo pipefail

cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo run --bin ctx -- doctor

for schema in capsule impact verify anchors locate explain widen graph boundaries; do
  cargo run --bin ctx -- schema "$schema" >/dev/null
done

cargo package --allow-dirty --no-verify
package_list="$(cargo package --allow-dirty --list)"
for required in \
  "schemas/capsule.schema.json" \
  "schemas/impact.schema.json" \
  "schemas/verify.schema.json" \
  "schemas/anchors.schema.json" \
  "schemas/locate.schema.json" \
  "schemas/explain.schema.json" \
  "schemas/widen.schema.json" \
  "schemas/graph.schema.json" \
  "schemas/boundaries.schema.json" \
  "tests/e2e_workflow.rs" \
  "fixtures/mixed-monorepo/package.json"
do
  grep -qx "$required" <<<"$package_list"
done
