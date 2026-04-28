#!/usr/bin/env bash
set -euo pipefail

sha256_check() {
  checksum_file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c "$checksum_file"
    return
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -c "$checksum_file"
    return
  fi
  echo "missing sha256sum or shasum" >&2
  exit 1
}

cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo run --bin ctx -- doctor
./scripts/check-npm-wrapper.sh
./scripts/check-homebrew-formula.sh

for schema in status files capsule impact verify anchors locate explain widen graph boundaries; do
  cargo run --bin ctx -- schema "$schema" >/dev/null
done

cargo package --allow-dirty --no-verify
package_list="$(cargo package --allow-dirty --list)"
for forbidden in \
  "npm/agent-context-cli/vendor/" \
  "npm/agent-context-cli/node_modules/" \
  "npm/agent-context-cli/agent-context-cli-" \
  ".tgz"
do
  if grep -F "$forbidden" <<<"$package_list"; then
    echo "cargo package contains forbidden generated npm artifact matching: $forbidden" >&2
    exit 1
  fi
done
for required in \
  "schemas/capsule.schema.json" \
  "schemas/status.schema.json" \
  "schemas/files.schema.json" \
  "schemas/impact.schema.json" \
  "schemas/verify.schema.json" \
  "schemas/anchors.schema.json" \
  "schemas/locate.schema.json" \
  "schemas/explain.schema.json" \
  "schemas/widen.schema.json" \
  "schemas/graph.schema.json" \
  "schemas/boundaries.schema.json" \
  "schemas/manifest.json" \
  "docs/SCHEMA_POLICY.md" \
  "scripts/package-release.sh" \
  "scripts/check-npm-wrapper.sh" \
  "scripts/package-npm-wrapper.sh" \
  "scripts/generate-homebrew-formula.sh" \
  "scripts/check-homebrew-formula.sh" \
  "scripts/update-homebrew-tap.sh" \
  "npm/agent-context-cli/package.json" \
  "npm/agent-context-cli/bin/ctx" \
  "npm/agent-context-cli/scripts/install.js" \
  "npm/agent-context-cli/README.md" \
  "npm/agent-context-cli/LICENSE" \
  "tests/e2e_workflow.rs" \
  "tests/schema_policy.rs" \
  "fixtures/mixed-monorepo/package.json" \
  "fixtures/mixed-monorepo/tsconfig.json" \
  "fixtures/go-workspace/go.work.fixture" \
  "fixtures/go-workspace/services/replay/go.mod.fixture" \
  "fixtures/go-workspace/services/renderer/go.mod.fixture" \
  "fixtures/go-workspace/apps/api/go.mod.fixture" \
  "fixtures/python-workspace/pyproject.toml" \
  "fixtures/python-workspace/services/replay/pyproject.toml" \
  "fixtures/python-workspace/services/renderer/pyproject.toml" \
  "fixtures/python-workspace/apps/api/pyproject.toml" \
  "fixtures/rust-workspace/Cargo.toml.fixture"
do
  grep -qx "$required" <<<"$package_list"
done

release_dir="$(mktemp -d)"
cleanup() {
  rm -rf "$release_dir"
}
trap cleanup EXIT

release_output="$release_dir/package-output.txt"
./scripts/package-release.sh --out-dir "$release_dir" > "$release_output"
archive="$(sed -n '1p' "$release_output")"
checksum="$(sed -n '2p' "$release_output")"
test -f "$archive"
test -f "$checksum"
(cd "$(dirname "$archive")" && sha256_check "$(basename "$checksum")")
archive_list="$(tar -tf "$archive")"
archive_base="$(basename "$archive" .tar.gz)"
expected_list="$(printf '%s\n' \
  "$archive_base/" \
  "$archive_base/LICENSE" \
  "$archive_base/README.md" \
  "$archive_base/ctx")"
diff -u \
  <(printf '%s\n' "$expected_list" | LC_ALL=C sort) \
  <(printf '%s\n' "$archive_list" | LC_ALL=C sort)
