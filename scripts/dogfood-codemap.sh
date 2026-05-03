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

progress() {
  printf '[dogfood] %s\n' "$*" >&2
}

line_budget_for() {
  case "$1" in
    doctor) echo 180 ;;
    ls_root) echo 150 ;;
    ls_links) echo 120 ;;
    changed) echo 120 ;;
    proof_changed) echo 120 ;;
    cone_owner*) echo 160 ;;
    proof_owner*) echo 140 ;;
    *) echo 180 ;;
  esac
}

latency_budget_for() {
  case "$1" in
    ls_root) echo 5000 ;;
    changed) echo 3000 ;;
    proof_changed) echo 2000 ;;
    doctor) echo 3000 ;;
    proof_map_root) echo 2000 ;;
    graph_causal|runtime_root) echo 3000 ;;
    flow_anchor|delete_anchor|siblings_scope|cone_owner_env|proof_owner_env) echo 2000 ;;
    *) echo 3000 ;;
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

first_owner_anchor() {
  local target="$1"
  for candidate in \
    pnpm-workspace.yaml \
    Cargo.toml \
    package.json \
    prisma/schema.prisma \
    apps/api/prisma/schema.prisma \
    .env.example \
    .env.production.example \
    .github/workflows/ci.yml \
    .github/workflows/test.yml; do
    if [[ -f "$target/$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  find "$target" \
    \( -path '*/node_modules' -o -path '*/target' -o -path '*/dist' -o -path '*/build' -o -path '*/.git' \) -prune \
    -o -type f \( \
      -name 'schema.prisma' -o \
      -name '.env.example' -o \
      -name 'package.json' -o \
      -name 'Cargo.toml' -o \
      -path '*/.github/workflows/*.yml' -o \
      -path '*/.github/workflows/*.yaml' \
    \) -print 2>/dev/null \
    | sed "s#^$target/##" \
    | sort \
    | head -n 1
}

first_manifest_owner_anchor() {
  local target="$1"
  for candidate in pnpm-workspace.yaml pnpm-workspace.yml Cargo.toml package.json pyproject.toml go.mod package.swift; do
    if [[ -f "$target/$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
}

first_schema_owner_anchor() {
  local target="$1"
  for candidate in apps/api/prisma/schema.prisma prisma/schema.prisma schema.prisma; do
    if [[ -f "$target/$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  find "$target" \
    \( -path '*/node_modules' -o -path '*/target' -o -path '*/dist' -o -path '*/build' -o -path '*/.git' \) -prune \
    -o -type f \( -name 'schema.prisma' -o -path '*/migrations/*.sql' \) -print 2>/dev/null \
    | sed "s#^$target/##" \
    | sort \
    | head -n 1
}

first_env_owner_anchor() {
  local target="$1"
  for candidate in .env.example .env.sample .env.production.example .env.development.example; do
    if [[ -f "$target/$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  find "$target" \
    \( -path '*/node_modules' -o -path '*/target' -o -path '*/dist' -o -path '*/build' -o -path '*/.git' \) -prune \
    -o -type f \( -name '.env.example' -o -name '.env.sample' -o -name '.env.*.example' \) -print 2>/dev/null \
    | sed "s#^$target/##" \
    | sort \
    | head -n 1
}

first_ci_owner_anchor() {
  local target="$1"
  for candidate in .github/workflows/ci.yml .github/workflows/ci.yaml .github/workflows/test.yml .github/workflows/test.yaml; do
    if [[ -f "$target/$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  find "$target" \
    \( -path '*/node_modules' -o -path '*/target' -o -path '*/dist' -o -path '*/build' -o -path '*/.git' \) -prune \
    -o -type f \( -path '*/.github/workflows/*.yml' -o -path '*/.github/workflows/*.yaml' \) -print 2>/dev/null \
    | sed "s#^$target/##" \
    | sort \
    | head -n 1
}

count_output_lines_matching() {
  local pattern="$1"
  local path="$2"
  python3 - "$pattern" "$path" <<'PY'
import re
import sys

pattern, path = sys.argv[1], sys.argv[2]
rx = re.compile(pattern)
try:
    with open(path, encoding="utf-8", errors="replace") as fh:
        print(sum(1 for line in fh if rx.search(line)))
except FileNotFoundError:
    print(0)
PY
}

count_trust_violations() {
  local path="$1"
  python3 - "$path" <<'PY'
import re
import sys

patterns = [
    re.compile(r"##\s+Mutation Roles"),
    re.compile(r"\[role="),
    re.compile(r"\broles="),
    re.compile(r"^\s+roles:\s"),
    re.compile(r"\bRole patterns\b"),
    re.compile(r"##\s+Unclassified Source Files"),
]
count = 0
try:
    with open(sys.argv[1], encoding="utf-8", errors="replace") as fh:
        for line in fh:
            count += sum(1 for pattern in patterns if pattern.search(line))
except FileNotFoundError:
    pass
print(count)
PY
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
  local start end elapsed_ms status line_count hidden_lines unknown_lines map_quality_lines trust_violations budget budget_status latency_budget latency_status
  progress "run repo=$name label=$label command=$command_text"
  start="$(python3 -c 'import time; print(time.time_ns())')"
  set +e
  CODEMAP_CACHE_DIR="$cache_dir" "${codemap_bin[@]}" --root "$target" "$@" >"$output_path" 2>>"$log"
  status=$?
  set -e
  end="$(python3 -c 'import time; print(time.time_ns())')"
  elapsed_ms="$(((end - start) / 1000000))"
  line_count="$(wc -l <"$output_path" | tr -d ' ')"
  hidden_lines="$(count_output_lines_matching 'hidden|Hidden' "$output_path")"
  unknown_lines="$(count_output_lines_matching 'unknown|Unknown|No deterministic proof sensor' "$output_path")"
  map_quality_lines="$(count_output_lines_matching 'Map Quality|map_quality|without static readers|without deterministic proof|stale_lens_artifact' "$output_path")"
  trust_violations="$(count_trust_violations "$output_path")"
  budget="$(line_budget_for "$label")"
  latency_budget="$(latency_budget_for "$label")"
  if (( line_count <= budget )); then
    budget_status="ok"
  else
    budget_status="over"
  fi
  if (( elapsed_ms <= latency_budget )); then
    latency_status="ok"
  else
    latency_status="slow"
  fi
  printf '{"repo":%s,"label":%s,"command":%s,"status":%s,"elapsed_ms":%s,"latency_budget_ms":%s,"latency_status":%s,"lines":%s,"line_budget":%s,"hidden_lines":%s,"unknown_lines":%s,"map_quality_lines":%s,"trust_violations":%s,"budget_status":%s}\n' \
    "$(printf '%s' "$target" | json_escape)" \
    "$(printf '%s' "$label" | json_escape)" \
    "$(printf '%s' "$command_text" | json_escape)" \
    "$status" \
    "$elapsed_ms" \
    "$latency_budget" \
    "$(printf '%s' "$latency_status" | json_escape)" \
    "$line_count" \
    "$budget" \
    "$hidden_lines" \
    "$unknown_lines" \
    "$map_quality_lines" \
    "$trust_violations" \
    "$(printf '%s' "$budget_status" | json_escape)" >>"$summary"
  progress "done repo=$name label=$label status=$status elapsed_ms=$elapsed_ms/$latency_budget latency=$latency_status lines=$line_count/$budget budget=$budget_status trust_violations=$trust_violations output=$(basename "$output_path")"
}

run_probe() {
  local target="$1"
  local index="${2:-?}"
  local total="${3:-?}"
  local name
  name="$(basename "$target" | tr -c 'A-Za-z0-9_.-' '_')"
  local log="$out_dir/$name.log"
  local summary="$out_dir/$name.summary.jsonl"
  : >"$log"
  : >"$summary"
  progress "repo-start index=$index/$total name=$name path=$target"
  if [[ ! -d "$target" ]]; then
    printf '{"repo":%s,"status":"missing"}\n' "$(printf '%s' "$target" | json_escape)" >>"$summary"
    progress "repo-missing index=$index/$total name=$name path=$target"
    return 0
  fi
  run_probe_command "$target" "$name" "$summary" "$log" ls_root ls .
  run_probe_command "$target" "$name" "$summary" "$log" changed changed
  run_probe_command "$target" "$name" "$summary" "$log" proof_changed proof changed
  run_probe_command "$target" "$name" "$summary" "$log" doctor doctor
  run_probe_command "$target" "$name" "$summary" "$log" ls_links ls . --section links
  run_probe_command "$target" "$name" "$summary" "$log" graph_causal graph --lens causal
  run_probe_command "$target" "$name" "$summary" "$log" runtime_root runtime .
  run_probe_command "$target" "$name" "$summary" "$log" proof_map_root proof-map .

  local source_anchor contract_anchor owner_anchor source_scope
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
  owner_anchor="$(first_owner_anchor "$target")"
  if [[ -n "$owner_anchor" ]]; then
    run_probe_command "$target" "$name" "$summary" "$log" cone_owner cone "$owner_anchor"
    run_probe_command "$target" "$name" "$summary" "$log" proof_owner proof "$owner_anchor"
  fi
  local manifest_owner schema_owner env_owner ci_owner
  manifest_owner="$(first_manifest_owner_anchor "$target")"
  schema_owner="$(first_schema_owner_anchor "$target")"
  env_owner="$(first_env_owner_anchor "$target")"
  ci_owner="$(first_ci_owner_anchor "$target")"
  if [[ -n "$manifest_owner" ]]; then
    run_probe_command "$target" "$name" "$summary" "$log" cone_owner_manifest cone "$manifest_owner"
    run_probe_command "$target" "$name" "$summary" "$log" proof_owner_manifest proof "$manifest_owner"
  fi
  if [[ -n "$schema_owner" ]]; then
    run_probe_command "$target" "$name" "$summary" "$log" cone_owner_schema cone "$schema_owner"
    run_probe_command "$target" "$name" "$summary" "$log" proof_owner_schema proof "$schema_owner"
  fi
  if [[ -n "$env_owner" ]]; then
    run_probe_command "$target" "$name" "$summary" "$log" cone_owner_env cone "$env_owner"
    run_probe_command "$target" "$name" "$summary" "$log" proof_owner_env proof "$env_owner"
  fi
  if [[ -n "$ci_owner" ]]; then
    run_probe_command "$target" "$name" "$summary" "$log" cone_owner_ci cone "$ci_owner"
    run_probe_command "$target" "$name" "$summary" "$log" proof_owner_ci proof "$ci_owner"
  fi
  progress "repo-done index=$index/$total name=$name summary=$(basename "$summary")"
}

progress "start targets=${#targets[@]} out=$out_dir cache=$cache_dir"
target_index=0
for target in "${targets[@]}"; do
  target_index=$((target_index + 1))
  run_probe "$target" "$target_index" "${#targets[@]}"
done

: >"$out_dir/summary.jsonl"
while IFS= read -r summary_file; do
  cat "$summary_file" >>"$out_dir/summary.jsonl"
done < <(find "$out_dir" -maxdepth 1 -type f -name '*.summary.jsonl' ! -name 'summary.jsonl' | sort)
summary_counts="$(python3 - "$out_dir/summary.jsonl" <<'PY'
import json
import sys

path = sys.argv[1]
rows = []
with open(path, encoding="utf-8") as fh:
    for line in fh:
        line = line.strip()
        if line:
            rows.append(json.loads(line))
failures = sum(1 for row in rows if row.get("status", 0) != 0)
over = sum(1 for row in rows if row.get("budget_status") == "over")
trust = sum(int(row.get("trust_violations", 0) or 0) for row in rows)
slow = sum(1 for row in rows if row.get("latency_status") == "slow")
print(f"probes={len(rows)} failures={failures} over_budget={over} slow={slow} trust_violations={trust}")
PY
)"
progress "summary $summary_counts path=$out_dir/summary.jsonl"
echo "dogfood summary: $out_dir/summary.jsonl"
