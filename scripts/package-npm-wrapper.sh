#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package_dir="$repo_root/npm/agent-context-cli"
out_dir="$repo_root/dist"

usage() {
  echo "Usage: scripts/package-npm-wrapper.sh [--out-dir DIR]" >&2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      out_dir="${2:?missing value for --out-dir}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if ! command -v node >/dev/null 2>&1; then
  echo "node is required to package the npm wrapper" >&2
  exit 1
fi
if ! command -v npm >/dev/null 2>&1; then
  echo "npm is required to package the npm wrapper" >&2
  exit 1
fi

if [[ -n "${CTX_EXPECTED_VERSION:-}" ]]; then
  cargo_version="$CTX_EXPECTED_VERSION"
else
  cargo_version="$(cd "$repo_root" && cargo pkgid --quiet)"
  cargo_version="${cargo_version##*@}"
fi
npm_version="$(node -e "console.log(require('$package_dir/package.json').version)")"
if [[ "$npm_version" != "$cargo_version" ]]; then
  echo "npm package version $npm_version does not match Cargo version $cargo_version" >&2
  exit 1
fi

mkdir -p "$out_dir"
out_dir="$(cd "$out_dir" && pwd)"
pack_json="$(
  cd "$package_dir"
  npm pack --pack-destination "$out_dir" --json
)"

archive_name="$(node -e "const pack = JSON.parse(process.argv[1])[0]; console.log(pack.filename)" "$pack_json")"
archive_path="$out_dir/$archive_name"
expected_name="agent-context-cli-${npm_version}.tgz"

if [[ "$archive_name" != "$expected_name" ]]; then
  echo "npm pack wrote $archive_name, expected $expected_name" >&2
  exit 1
fi
if [[ ! -f "$archive_path" ]]; then
  echo "npm pack did not create $archive_path" >&2
  exit 1
fi

printf '%s\n' "$archive_path"
