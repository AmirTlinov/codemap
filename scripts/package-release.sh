#!/usr/bin/env bash
set -euo pipefail

out_dir="dist"
target=""

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{ print $1 }'
    return
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{ print $1 }'
    return
  fi
  echo "missing sha256sum or shasum" >&2
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      out_dir="${2:?missing value for --out-dir}"
      shift 2
      ;;
    --target)
      target="${2:?missing value for --target}"
      shift 2
      ;;
    -h|--help)
      echo "Usage: scripts/package-release.sh [--out-dir DIR] [--target RUST_TARGET]"
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

pkgid="$(cargo pkgid --quiet)"
version="${pkgid##*@}"
if [[ -z "$target" ]]; then
  target="$(rustc -vV | awk '/^host:/ { print $2 }')"
fi
if [[ -z "$version" || -z "$target" ]]; then
  echo "failed to resolve package version or Rust target" >&2
  exit 1
fi

build_args=(build --release --bin ctx)
if [[ -n "$target" ]]; then
  build_args+=(--target "$target")
fi
cargo "${build_args[@]}"

binary="target/${target}/release/ctx"
if [[ ! -x "$binary" ]]; then
  echo "release binary not found: $binary" >&2
  exit 1
fi

archive_base="ctx-v${version}-${target}"
stage="$(mktemp -d)"
cleanup() {
  rm -rf "$stage"
}
trap cleanup EXIT

mkdir -p "$stage/$archive_base" "$out_dir"
cp "$binary" "$stage/$archive_base/ctx"
cp README.md LICENSE "$stage/$archive_base/"

archive="$out_dir/${archive_base}.tar.gz"
tar -C "$stage" -czf "$archive" "$archive_base"
checksum="$(sha256_file "$archive")"
printf '%s  %s\n' "$checksum" "$(basename "$archive")" > "${archive}.sha256"

echo "$archive"
echo "${archive}.sha256"
