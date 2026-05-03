#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'codemap version guard: %s\n' "$*" >&2
  exit 1
}

note() {
  printf 'codemap version guard: %s\n' "$*" >&2
}

package_version_from_text() {
  sed -n 's/^version = "\([^"]*\)"/\1/p' | head -n 1
}

worktree_version() {
  [ -f Cargo.toml ] || fail "Cargo.toml not found; run from the repository root"
  package_version_from_text < Cargo.toml
}

ref_version() {
  git show "$1:Cargo.toml" 2>/dev/null | package_version_from_text
}

resolve_base() {
  local requested="${1:-${CODEMAP_VERSION_BASE:-}}"
  if [ -n "$requested" ]; then
    git rev-parse --verify "$requested^{commit}" 2>/dev/null && return 0
    fail "base ref '$requested' is not a commit"
  fi

  if [ -n "${GITHUB_BASE_REF:-}" ] && git rev-parse --verify "origin/${GITHUB_BASE_REF}^{commit}" >/dev/null 2>&1; then
    git merge-base HEAD "origin/${GITHUB_BASE_REF}"
    return 0
  fi

  if [ -n "${GITHUB_EVENT_PATH:-}" ] && [ -f "$GITHUB_EVENT_PATH" ]; then
    local before
    before="$(sed -n 's/^[[:space:]]*"before":[[:space:]]*"\([0-9a-f]\{7,\}\)".*/\1/p' "$GITHUB_EVENT_PATH" | head -n 1)"
    if [ -n "$before" ] && ! printf '%s' "$before" | grep -Eq '^0+$' && git cat-file -e "$before^{commit}" 2>/dev/null; then
      printf '%s\n' "$before"
      return 0
    fi
  fi

  if git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' >/dev/null 2>&1; then
    local upstream
    upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}')"
    git merge-base HEAD "$upstream"
    return 0
  fi

  if git rev-parse --verify 'HEAD^' >/dev/null 2>&1; then
    git rev-parse --verify 'HEAD^'
    return 0
  fi

  return 1
}

semver_triplet() {
  local version="$1"
  if [[ ! "$version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)([-+].*)?$ ]]; then
    return 1
  fi
  printf '%s %s %s\n' "${BASH_REMATCH[1]}" "${BASH_REMATCH[2]}" "${BASH_REMATCH[3]}"
}

semver_gt() {
  local current="$1"
  local base="$2"
  local c_major c_minor c_patch b_major b_minor b_patch
  read -r c_major c_minor c_patch < <(semver_triplet "$current") || fail "current version '$current' is not semver x.y.z"
  read -r b_major b_minor b_patch < <(semver_triplet "$base") || fail "base version '$base' is not semver x.y.z"

  if (( c_major > b_major )); then return 0; fi
  if (( c_major < b_major )); then return 1; fi
  if (( c_minor > b_minor )); then return 0; fi
  if (( c_minor < b_minor )); then return 1; fi
  (( c_patch > b_patch ))
}

base_commit="$(resolve_base "${1:-}")" || {
  note "no base commit found; skipping"
  exit 0
}

current_version="$(worktree_version)"
[ -n "$current_version" ] || fail "Cargo.toml package version is missing"

base_version="$(ref_version "$base_commit")"
[ -n "$base_version" ] || fail "base Cargo.toml package version is missing at $base_commit"

changed_files="$(
  {
    git diff --name-only "$base_commit"...HEAD
    git diff --cached --name-only
    git diff --name-only
    git ls-files --others --exclude-standard
  } | sort -u
)"

if [ -z "$changed_files" ]; then
  note "no changed files since $base_commit"
  exit 0
fi

if ! semver_gt "$current_version" "$base_version"; then
  printf '%s\n' "$changed_files" >&2
  fail "changed files require Cargo.toml package version bump: $base_version -> $current_version"
fi

note "version bump ok: $base_version -> $current_version"
