#!/usr/bin/env bash
set -euo pipefail

tap_dir=""
formula_file=""
formula_path="Formula/ctx.rb"
tag=""
repo="${CTX_GITHUB_REPO:-AmirTlinov/ctx}"
commit=0

usage() {
  cat >&2 <<'USAGE'
Usage:
  scripts/update-homebrew-tap.sh --tap-dir DIR --tag vX.Y.Z [--repo owner/name] [--formula-path Formula/ctx.rb] [--commit]
  scripts/update-homebrew-tap.sh --tap-dir DIR --formula-file FILE [--formula-path Formula/ctx.rb] [--commit]

Updates a local Homebrew tap checkout from a generated ctx.rb formula.
It never pushes. Use --commit to create a local tap commit.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tap-dir)
      tap_dir="${2:?missing value for --tap-dir}"
      shift 2
      ;;
    --formula-file)
      formula_file="${2:?missing value for --formula-file}"
      shift 2
      ;;
    --formula-path)
      formula_path="${2:?missing value for --formula-path}"
      shift 2
      ;;
    --tag)
      tag="${2:?missing value for --tag}"
      shift 2
      ;;
    --repo)
      repo="${2:?missing value for --repo}"
      shift 2
      ;;
    --commit)
      commit=1
      shift
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

if [[ -z "$tap_dir" ]]; then
  usage
  exit 2
fi
if [[ -n "$formula_file" && -n "$tag" ]]; then
  echo "use either --formula-file or --tag, not both" >&2
  exit 2
fi
if [[ -z "$formula_file" && -z "$tag" ]]; then
  usage
  exit 2
fi
if [[ "$formula_path" = /* || "$formula_path" = *".."* ]]; then
  echo "--formula-path must be a tap-relative path without '..': $formula_path" >&2
  exit 2
fi

tap_dir="$(cd "$tap_dir" && pwd)"
if [[ -n "$formula_file" ]]; then
  formula_file="$(cd "$(dirname "$formula_file")" && pwd)/$(basename "$formula_file")"
else
  if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-.][A-Za-z0-9.]+)?$ ]]; then
    echo "tag must look like vX.Y.Z: $tag" >&2
    exit 1
  fi
  if ! command -v gh >/dev/null 2>&1; then
    echo "gh is required when --tag is used" >&2
    exit 1
  fi
  tmp="$(mktemp -d)"
  cleanup() {
    rm -rf "$tmp"
  }
  trap cleanup EXIT
  gh release download "$tag" --repo "$repo" --pattern ctx.rb --dir "$tmp" >/dev/null
  formula_file="$tmp/ctx.rb"
fi

if [[ ! -f "$formula_file" ]]; then
  echo "formula file not found: $formula_file" >&2
  exit 1
fi
ruby -c "$formula_file" >/dev/null
grep -Fq "class Ctx < Formula" "$formula_file"
grep -Fq 'bin.install "ctx"' "$formula_file"

dest="$tap_dir/$formula_path"
mkdir -p "$(dirname "$dest")"
cp "$formula_file" "$dest"
ruby -c "$dest" >/dev/null

if [[ "$commit" = 1 ]]; then
  if [[ ! -d "$tap_dir/.git" ]]; then
    echo "--commit requires a git checkout tap dir: $tap_dir" >&2
    exit 1
  fi
  git -C "$tap_dir" add "$formula_path"
  if git -C "$tap_dir" diff --cached --quiet -- "$formula_path"; then
    echo "Homebrew formula unchanged: $formula_path"
  else
    version_label="${tag:-release formula}"
    git -C "$tap_dir" commit -m "Update ctx Homebrew formula to ${version_label}"
  fi
fi

echo "Updated $dest"
