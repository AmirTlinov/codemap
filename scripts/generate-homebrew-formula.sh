#!/usr/bin/env bash
set -euo pipefail

release_dir=""
tag=""
output=""
repo_url="${CTX_HOMEBREW_REPO_URL:-https://github.com/AmirTlinov/ctx}"

usage() {
  echo "Usage: scripts/generate-homebrew-formula.sh --release-dir DIR --tag vX.Y.Z [--output FILE]" >&2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release-dir)
      release_dir="${2:?missing value for --release-dir}"
      shift 2
      ;;
    --tag)
      tag="${2:?missing value for --tag}"
      shift 2
      ;;
    --output)
      output="${2:?missing value for --output}"
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

if [[ -z "$release_dir" || -z "$tag" ]]; then
  usage
  exit 2
fi
if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-.][A-Za-z0-9.]+)?$ ]]; then
  echo "tag must look like vX.Y.Z: $tag" >&2
  exit 1
fi

version="${tag#v}"
mac_target="aarch64-apple-darwin"
linux_target="x86_64-unknown-linux-gnu"

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

sha_from_sidecar() {
  local target="$1"
  local archive="ctx-${tag}-${target}.tar.gz"
  local archive_path="${release_dir}/${archive}"
  local sidecar="${release_dir}/${archive}.sha256"
  if [[ ! -f "$archive_path" ]]; then
    echo "missing release archive: $archive_path" >&2
    exit 1
  fi
  if [[ ! -f "$sidecar" ]]; then
    echo "missing checksum sidecar: $sidecar" >&2
    exit 1
  fi

  local checksum filename
  read -r checksum filename < "$sidecar"
  if [[ ! "$checksum" =~ ^[0-9a-f]{64}$ ]]; then
    echo "invalid sha256 in $sidecar" >&2
    exit 1
  fi
  if [[ "$filename" != "$archive" ]]; then
    echo "checksum sidecar $sidecar names $filename, expected $archive" >&2
    exit 1
  fi
  local actual
  actual="$(sha256_file "$archive_path")"
  if [[ "$checksum" != "$actual" ]]; then
    echo "checksum sidecar $sidecar does not match $archive" >&2
    exit 1
  fi
  printf '%s' "$checksum"
}

mac_sha="$(sha_from_sidecar "$mac_target")"
linux_sha="$(sha_from_sidecar "$linux_target")"

render_formula() {
  cat <<FORMULA
class Ctx < Formula
  desc "Deterministic task-specific context router for coding agents"
  homepage "${repo_url}"
  version "${version}"
  license "MIT"

  on_macos do
    on_arm do
      url "${repo_url}/releases/download/${tag}/ctx-${tag}-${mac_target}.tar.gz"
      sha256 "${mac_sha}"
    end
  end

  on_linux do
    on_intel do
      url "${repo_url}/releases/download/${tag}/ctx-${tag}-${linux_target}.tar.gz"
      sha256 "${linux_sha}"
    end
  end

  def install
    bin.install "ctx"
  end

  test do
    assert_match "ctx #{version}", shell_output("#{bin}/ctx --version")
  end
end
FORMULA
}

if [[ -n "$output" ]]; then
  mkdir -p "$(dirname "$output")"
  render_formula > "$output"
else
  render_formula
fi
