#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
canonical_path() {
  python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$1"
}

raw_out_dir="${CODEMAP_DOGFOOD_OUT:-$repo_root/target/dogfood-codemap}"
out_dir="$(canonical_path "$raw_out_dir")"
cache_dir="${CODEMAP_DOGFOOD_CACHE:-$out_dir/cache}"
repo_target="$(canonical_path "$repo_root/target")"
resolved_tmp="$(canonical_path "${TMPDIR:-/tmp}")"

case "$out_dir" in
  "$repo_target"/*|"$resolved_tmp"/*) ;;
  *)
    echo "refusing to clean CODEMAP_DOGFOOD_OUT outside repo target or temp: $raw_out_dir" >&2
    exit 2
    ;;
esac
mkdir -p "$out_dir" "$cache_dir"
find "$out_dir" -maxdepth 1 -type f \( -name '*.md' -o -name '*.log' -o -name '*.summary.jsonl' -o -name 'summary.jsonl' \) -delete

if command -v codemap >/dev/null 2>&1; then
  codemap_bin=(codemap)
else
  codemap_bin=(cargo run --quiet --manifest-path "$repo_root/Cargo.toml" --bin codemap --)
fi

targets=(
  "/Users/amir/Documents/projects/spritestudio"
  "/Users/amir/Documents/projects/Sillentway-VPN"
)

if [[ $# -gt 0 ]]; then
  targets=("$@")
else
  while IFS= read -r candidate; do
    case "$candidate" in
      */spritestudio|*/Sillentway-VPN) ;;
      */.*) ;;
      */tools) ;;
      *)
        targets+=("$candidate")
        break
        ;;
    esac
  done < <(find /Users/amir/Documents/projects -mindepth 1 -maxdepth 1 -type d ! -name '.*' | sort)
fi

json_escape() {
  python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))'
}

run_probe() {
  local target="$1"
  local name
  name="$(basename "$target" | tr -c 'A-Za-z0-9_.-' '_')"
  local log="$out_dir/$name.log"
  local summary="$out_dir/$name.summary.jsonl"
  : >"$log"
  : >"$summary"
  if [[ ! -d "$target" ]]; then
    printf '{"repo":%s,"status":"missing"}\n' "$(printf '%s' "$target" | json_escape)" >>"$summary"
    return 0
  fi
  local commands=(
    "doctor"
    "ls ."
    "graph --lens causal"
    "runtime ."
    "proof-map ."
    "changed"
    "proof --changed"
  )
  for command_text in "${commands[@]}"; do
    local start end status line_count
    start="$(python3 -c 'import time; print(time.time_ns())')"
    set +e
    CODEMAP_CACHE_DIR="$cache_dir" "${codemap_bin[@]}" --root "$target" $command_text >"$out_dir/$name.$(tr ' /' '__' <<<"$command_text").md" 2>>"$log"
    status=$?
    set -e
    end="$(python3 -c 'import time; print(time.time_ns())')"
    line_count="$(wc -l <"$out_dir/$name.$(tr ' /' '__' <<<"$command_text").md" | tr -d ' ')"
    printf '{"repo":%s,"command":%s,"status":%s,"elapsed_ms":%s,"lines":%s}\n' \
      "$(printf '%s' "$target" | json_escape)" \
      "$(printf '%s' "$command_text" | json_escape)" \
      "$status" \
      "$(((end - start) / 1000000))" \
      "$line_count" >>"$summary"
  done
}

for target in "${targets[@]}"; do
  run_probe "$target"
done

: >"$out_dir/summary.jsonl"
while IFS= read -r summary_file; do
  cat "$summary_file" >>"$out_dir/summary.jsonl"
done < <(find "$out_dir" -maxdepth 1 -type f -name '*.summary.jsonl' ! -name '.*' ! -name 'summary.jsonl' | sort)
echo "dogfood summary: $out_dir/summary.jsonl"
