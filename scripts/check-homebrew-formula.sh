#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp="$(mktemp -d)"
cleanup() {
  rm -rf "$tmp"
}
trap cleanup EXIT

tag="v$(cd "$repo_root" && cargo pkgid --quiet | awk -F@ '{ print $NF }')"
mac_archive="ctx-${tag}-aarch64-apple-darwin.tar.gz"
linux_archive="ctx-${tag}-x86_64-unknown-linux-gnu.tar.gz"

sha256_file() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{ print $1 }'
    return
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{ print $1 }'
    return
  fi
  echo "missing sha256sum or shasum" >&2
  exit 1
}

touch "$tmp/$mac_archive" "$tmp/$linux_archive"
printf '%s  %s\n' "$(sha256_file "$tmp/$mac_archive")" "$mac_archive" > "$tmp/${mac_archive}.sha256"
printf '%s  %s\n' "$(sha256_file "$tmp/$linux_archive")" "$linux_archive" > "$tmp/${linux_archive}.sha256"

formula="$tmp/ctx.rb"
"$repo_root/scripts/generate-homebrew-formula.sh" \
  --release-dir "$tmp" \
  --tag "$tag" \
  --output "$formula"

ruby -c "$formula" >/dev/null
grep -Fq "class Ctx < Formula" "$formula"
grep -Fq "ctx-${tag}-aarch64-apple-darwin.tar.gz" "$formula"
grep -Fq "ctx-${tag}-x86_64-unknown-linux-gnu.tar.gz" "$formula"
grep -Fq 'bin.install "ctx"' "$formula"
grep -Fq 'ctx #{version}' "$formula"

printf '%064d  %s\n' 1 "$mac_archive" > "$tmp/${mac_archive}.sha256"
if "$repo_root/scripts/generate-homebrew-formula.sh" \
  --release-dir "$tmp" \
  --tag "$tag" >/dev/null 2>&1
then
  echo "mismatched Homebrew formula checksum unexpectedly passed" >&2
  exit 1
fi
printf '%s  %s\n' "$(sha256_file "$tmp/$mac_archive")" "$mac_archive" > "$tmp/${mac_archive}.sha256"

if "$repo_root/scripts/generate-homebrew-formula.sh" \
  --release-dir "$tmp" \
  --tag "not-a-version" >/dev/null 2>&1
then
  echo "invalid Homebrew formula tag unexpectedly passed" >&2
  exit 1
fi

tap="$tmp/tap"
mkdir -p "$tap/Formula"
git -C "$tap" init -q
git -C "$tap" config user.email "ctx@example.com"
git -C "$tap" config user.name "ctx"
"$repo_root/scripts/update-homebrew-tap.sh" \
  --tap-dir "$tap" \
  --formula-file "$formula" >/dev/null
cmp "$formula" "$tap/Formula/ctx.rb"
git -C "$tap" status --short | grep -Fq "?? Formula/"

git -C "$tap" add .
git -C "$tap" commit -qm "initial tap"
"$repo_root/scripts/update-homebrew-tap.sh" \
  --tap-dir "$tap" \
  --formula-file "$formula" \
  --commit >/dev/null
test -z "$(git -C "$tap" status --short)"

sed 's/ctx-v/ctx-v-test-/' "$formula" > "$tmp/ctx-updated.rb"
"$repo_root/scripts/update-homebrew-tap.sh" \
  --tap-dir "$tap" \
  --formula-file "$tmp/ctx-updated.rb" \
  --commit >/dev/null
git -C "$tap" log -1 --format=%s | grep -Fq "Update ctx Homebrew formula"
cmp "$tmp/ctx-updated.rb" "$tap/Formula/ctx.rb"
