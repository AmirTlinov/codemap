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

if [[ -n "${CODEMAP_BIN:-}" ]]; then
  codemap_bin=("$CODEMAP_BIN")
elif command -v codemap >/dev/null 2>&1; then
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

safe_label() {
  tr -c 'A-Za-z0-9_.-' '_' <<<"$1"
}

line_budget_for() {
  case "$1" in
    doctor) echo 180 ;;
    ls_root) echo 150 ;;
    changed) echo 120 ;;
    proof_changed) echo 120 ;;
    *) echo 180 ;;
  esac
}

first_source_anchor() {
  python3 - "$1" <<'PY'
import os
import subprocess
import sys

root = sys.argv[1]
exts = (".ts", ".tsx", ".js", ".jsx", ".rs", ".go", ".py", ".swift")
try:
    paths = subprocess.check_output(
        ["git", "-C", root, "ls-files", "-c", "-o", "--exclude-standard"],
        text=True,
        stderr=subprocess.DEVNULL,
    ).splitlines()
except Exception:
    paths = []
    for base, dirs, files in os.walk(root):
        dirs[:] = [d for d in dirs if d not in {".git", "node_modules", "target", "dist", "build"}]
        for name in files:
            rel = os.path.relpath(os.path.join(base, name), root).replace(os.sep, "/")
            paths.append(rel)

for rel in sorted(paths):
    if rel.endswith(exts):
        print(rel)
        break
PY
}

first_contract_anchor() {
  local target="$1"
  for candidate in package.json Cargo.toml pyproject.toml go.mod; do
    if [[ -f "$target/$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
}

run_probe_command() {
  local target="$1"
  local name="$2"
  local summary="$3"
  local log="$4"
  local label="$5"
  shift 5
  local command_text="$*"
  local output_path="$out_dir/$name.$(safe_label "$label").md"
  local start end status line_count budget budget_status
  start="$(python3 -c 'import time; print(time.time_ns())')"
  set +e
  CODEMAP_CACHE_DIR="$cache_dir" "${codemap_bin[@]}" --root "$target" "$@" >"$output_path" 2>>"$log"
  status=$?
  set -e
  end="$(python3 -c 'import time; print(time.time_ns())')"
  line_count="$(wc -l <"$output_path" | tr -d ' ')"
  budget="$(line_budget_for "$label")"
  if (( line_count <= budget )); then
    budget_status="ok"
  else
    budget_status="over"
  fi
  printf '{"repo":%s,"label":%s,"command":%s,"status":%s,"elapsed_ms":%s,"lines":%s,"line_budget":%s,"budget_status":%s}\n' \
    "$(printf '%s' "$target" | json_escape)" \
    "$(printf '%s' "$label" | json_escape)" \
    "$(printf '%s' "$command_text" | json_escape)" \
    "$status" \
    "$(((end - start) / 1000000))" \
    "$line_count" \
    "$budget" \
    "$(printf '%s' "$budget_status" | json_escape)" >>"$summary"
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
  run_probe_command "$target" "$name" "$summary" "$log" doctor doctor
  run_probe_command "$target" "$name" "$summary" "$log" ls_root ls .
  run_probe_command "$target" "$name" "$summary" "$log" graph_causal graph --lens causal
  run_probe_command "$target" "$name" "$summary" "$log" runtime_root runtime .
  run_probe_command "$target" "$name" "$summary" "$log" proof_map_root proof-map .
  run_probe_command "$target" "$name" "$summary" "$log" changed changed
  run_probe_command "$target" "$name" "$summary" "$log" proof_changed proof --changed

  local source_anchor contract_anchor source_scope
  source_anchor="$(first_source_anchor "$target")"
  if [[ -n "$source_anchor" ]]; then
    source_scope="$(dirname "$source_anchor")"
    run_probe_command "$target" "$name" "$summary" "$log" cone_anchor cone "$source_anchor"
    run_probe_command "$target" "$name" "$summary" "$log" flow_anchor flow "$source_anchor"
    run_probe_command "$target" "$name" "$summary" "$log" delete_anchor delete "$source_anchor"
    if [[ "$source_scope" != "." ]]; then
      run_probe_command "$target" "$name" "$summary" "$log" siblings_scope siblings "$source_scope"
      run_probe_command "$target" "$name" "$summary" "$log" place_test_scope place "$source_scope" --kind test
    fi
  fi
  contract_anchor="$(first_contract_anchor "$target")"
  if [[ -n "$contract_anchor" ]]; then
    run_probe_command "$target" "$name" "$summary" "$log" contract_anchor contract "$contract_anchor"
  fi
}

for target in "${targets[@]}"; do
  run_probe "$target"
done

: >"$out_dir/summary.jsonl"
while IFS= read -r summary_file; do
  cat "$summary_file" >>"$out_dir/summary.jsonl"
done < <(find "$out_dir" -maxdepth 1 -type f -name '*.summary.jsonl' ! -name 'summary.jsonl' | sort)
echo "dogfood summary: $out_dir/summary.jsonl"
