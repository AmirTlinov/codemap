#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package_dir="$repo_root/npm/agent-context-cli"

if ! command -v node >/dev/null 2>&1; then
  echo "node is required to check the npm wrapper" >&2
  exit 1
fi
if ! command -v npm >/dev/null 2>&1; then
  echo "npm is required to check the npm wrapper" >&2
  exit 1
fi

cargo_version="$(cd "$repo_root" && cargo pkgid --quiet)"
cargo_version="${cargo_version##*@}"
npm_version="$(node -e "console.log(require('$package_dir/package.json').version)")"
test "$npm_version" = "$cargo_version"
node --check "$package_dir/bin/ctx" >/dev/null
node --check "$package_dir/scripts/install.js" >/dev/null

tmp="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp"
}
trap cleanup EXIT

package_copy="$tmp/package"
cp -R "$package_dir" "$package_copy"

archive_output="$tmp/archive-output.txt"
"$repo_root/scripts/package-release.sh" --out-dir "$tmp/release" > "$archive_output"
archive="$(sed -n '1p' "$archive_output")"
test -f "$archive"
test -f "${archive}.sha256"

(
  cd "$package_copy"
  CTX_NPM_INSTALL_ARCHIVE="$archive" node scripts/install.js
  node bin/ctx --version
)

pack_json="$tmp/npm-pack.json"
(
  cd "$package_dir"
  npm pack --dry-run --json
) > "$pack_json"
node - "$pack_json" <<'NODE'
const fs = require("node:fs");
const pack = JSON.parse(fs.readFileSync(process.argv[2], "utf8"))[0];
const files = new Set(pack.files.map((file) => file.path));
for (const required of [
  "bin/ctx",
  "scripts/install.js",
  "package.json",
  "README.md",
  "LICENSE"
]) {
  if (!files.has(required)) {
    throw new Error(`npm package is missing ${required}`);
  }
}
for (const forbidden of ["vendor/ctx", "vendor/ctx.exe"]) {
  if (files.has(forbidden)) {
    throw new Error(`npm package must not include installed binary ${forbidden}`);
  }
}
NODE

npm_dist="$tmp/npm-dist"
npm_archive="$("$repo_root/scripts/package-npm-wrapper.sh" --out-dir "$npm_dist")"
test "$(basename "$npm_archive")" = "agent-context-cli-${npm_version}.tgz"
tar -tzf "$npm_archive" | LC_ALL=C sort > "$tmp/npm-archive-files.txt"
for required in \
  "package/bin/ctx" \
  "package/scripts/install.js" \
  "package/package.json" \
  "package/README.md" \
  "package/LICENSE"
do
  grep -qx "$required" "$tmp/npm-archive-files.txt"
done
if grep -Eq '^package/vendor/(ctx|ctx.exe)$' "$tmp/npm-archive-files.txt"; then
  echo "npm package archive includes an installed native binary" >&2
  exit 1
fi
