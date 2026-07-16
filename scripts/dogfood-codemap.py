#!/usr/bin/env python3
"""Run the cross-platform, read-only codemap daily and focused dogfood probes."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
LINE_BUDGETS = {
    "doctor": 180,
    "ls_root": 150,
    "ls_links": 120,
    "changed": 120,
    "proof_changed": 120,
}
LATENCY_BUDGETS = {
    "ls_root": 5_000,
    "changed": 3_000,
    "proof_changed": 2_000,
    "doctor": 3_000,
    "proof_map_root": 2_000,
    "graph_causal": 3_000,
    "runtime_root": 3_000,
}
TRUST_PATTERNS = [
    re.compile(r"##\s+Mutation Roles"),
    re.compile(r"\[role="),
    re.compile(r"\broles="),
    re.compile(r"^\s+roles:\s"),
    re.compile(r"\bRole patterns\b"),
    re.compile(r"##\s+Unclassified Source Files"),
]
SOURCE_EXTENSIONS = (".ts", ".tsx", ".js", ".jsx", ".rs", ".go", ".py", ".swift")


def canonical(path: Path) -> Path:
    return path.expanduser().resolve()


def is_within(path: Path, parent: Path) -> bool:
    try:
        path.relative_to(parent)
        return path != parent
    except ValueError:
        return False


def output_dir() -> Path:
    raw = Path(os.environ.get("CODEMAP_DOGFOOD_OUT", ROOT / "target/dogfood-codemap"))
    resolved = canonical(raw)
    allowed = (canonical(ROOT / "target"), canonical(Path(tempfile.gettempdir())))
    if not any(is_within(resolved, parent) for parent in allowed):
        raise ValueError(
            f"refusing to clean CODEMAP_DOGFOOD_OUT outside repo target or temp: {raw}"
        )
    return resolved


def codemap_command() -> list[str]:
    explicit = os.environ.get("CODEMAP_BIN")
    if explicit:
        return [str(canonical(Path(explicit)))]
    installed = shutil.which("codemap")
    if installed:
        return [installed]
    return [
        "cargo",
        "run",
        "--quiet",
        "--manifest-path",
        str(ROOT / "Cargo.toml"),
        "--bin",
        "codemap",
        "--",
    ]


def progress(message: str) -> None:
    print(f"[dogfood] {message}", file=sys.stderr, flush=True)


def safe_label(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", value).strip("_") or "repo"


def git_paths(target: Path) -> list[str]:
    result = subprocess.run(
        ["git", "-C", str(target), "ls-files", "-c", "-o", "--exclude-standard"],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode == 0:
        return sorted(line for line in result.stdout.splitlines() if line)
    rows = []
    ignored = {".git", "node_modules", "target", "dist", "build"}
    for base, dirs, files in os.walk(target):
        dirs[:] = [name for name in dirs if name not in ignored]
        for name in files:
            rows.append(Path(base, name).relative_to(target).as_posix())
    return sorted(rows)


def first_existing(target: Path, candidates: list[str], paths: list[str]) -> str | None:
    for candidate in candidates:
        if (target / candidate).is_file():
            return candidate
    candidate_set = set(candidates)
    return next((path for path in paths if path in candidate_set), None)


def anchors(target: Path) -> dict[str, str | None]:
    paths = git_paths(target)
    source = next((path for path in paths if path.endswith(SOURCE_EXTENSIONS)), None)
    manifests = ["pnpm-workspace.yaml", "pnpm-workspace.yml", "Cargo.toml", "package.json", "pyproject.toml", "go.mod", "Package.swift"]
    schemas = ["apps/api/prisma/schema.prisma", "prisma/schema.prisma", "schema.prisma"]
    envs = [".env.example", ".env.sample", ".env.production.example", ".env.development.example"]
    workflows = [".github/workflows/ci.yml", ".github/workflows/ci.yaml", ".github/workflows/test.yml", ".github/workflows/test.yaml"]
    manifest = first_existing(target, manifests, paths)
    schema = first_existing(target, schemas, paths) or next(
        (path for path in paths if path.endswith("schema.prisma") or "/migrations/" in path), None
    )
    environment = first_existing(target, envs, paths) or next(
        (path for path in paths if Path(path).name.startswith(".env.") and path.endswith(".example")), None
    )
    workflow = first_existing(target, workflows, paths) or next(
        (path for path in paths if ".github/workflows/" in path and path.endswith((".yml", ".yaml"))), None
    )
    owner = next((value for value in (manifest, schema, environment, workflow) if value), None)
    contract = first_existing(target, ["package.json", "Cargo.toml", "pyproject.toml", "go.mod"], paths)
    return {"source": source, "contract": contract, "owner": owner, "manifest": manifest, "schema": schema, "env": environment, "ci": workflow}


def probe_specs(target: Path) -> list[tuple[str, list[str]]]:
    specs = [
        ("ls_root", ["ls", "."]),
        ("changed", ["changed"]),
        ("proof_changed", ["proof", "changed"]),
        ("doctor", ["doctor"]),
        ("ls_links", ["ls", ".", "--section", "links"]),
        ("graph_causal", ["graph", "--lens", "causal"]),
        ("runtime_root", ["runtime", "."]),
        ("proof_map_root", ["proof-map", "."]),
    ]
    found = anchors(target)
    source = found["source"]
    if source:
        specs.extend((label, [command, source]) for label, command in (("cone_anchor", "cone"), ("flow_anchor", "flow"), ("delete_anchor", "delete")))
        scope = str(Path(source).parent).replace("\\", "/")
        if scope != ".":
            specs.extend((("siblings_scope", ["siblings", scope]), ("place_test_scope", ["place", scope, "--kind", "test"])))
    if found["contract"]:
        specs.append(("contract_anchor", ["contract", found["contract"]]))
    if found["owner"]:
        specs.extend((("cone_owner", ["cone", found["owner"]]), ("proof_owner", ["proof", found["owner"]])))
    for kind in ("manifest", "schema", "env", "ci"):
        if found[kind]:
            specs.extend(((f"cone_owner_{kind}", ["cone", found[kind]]), (f"proof_owner_{kind}", ["proof", found[kind]])))
    return specs


def count_matches(text: str, pattern: str) -> int:
    expression = re.compile(pattern, re.IGNORECASE)
    return sum(bool(expression.search(line)) for line in text.splitlines())


def run_probe(target: Path, label: str, args: list[str], out: Path, cache: Path, log: Path) -> dict[str, object]:
    name = safe_label(target.name)
    output_path = out / f"{name}.{safe_label(label)}.md"
    argv = [*codemap_command(), "--root", str(target), *args]
    progress(f"run repo={name} label={label} command={' '.join(args)}")
    started = time.perf_counter_ns()
    result = subprocess.run(argv, capture_output=True, text=True, env={**os.environ, "CODEMAP_CACHE_DIR": str(cache)}, check=False)
    elapsed = (time.perf_counter_ns() - started) // 1_000_000
    output_path.write_text(result.stdout, encoding="utf-8")
    with log.open("a", encoding="utf-8") as stream:
        stream.write(result.stderr)
    lines = len(result.stdout.splitlines())
    line_budget = LINE_BUDGETS.get(label, 160 if label.startswith("cone_owner") else 140 if label.startswith("proof_owner") else 180)
    latency_budget = LATENCY_BUDGETS.get(label, 2_000 if label in {"flow_anchor", "delete_anchor", "siblings_scope", "cone_owner_env", "proof_owner_env"} else 3_000)
    trust = sum(sum(bool(pattern.search(line)) for pattern in TRUST_PATTERNS) for line in result.stdout.splitlines())
    row = {"repo": str(target), "label": label, "command": " ".join(args), "status": result.returncode, "elapsed_ms": elapsed, "latency_budget_ms": latency_budget, "latency_status": "ok" if elapsed <= latency_budget else "slow", "lines": lines, "line_budget": line_budget, "hidden_lines": count_matches(result.stdout, "hidden"), "unknown_lines": count_matches(result.stdout, "unknown|No deterministic proof sensor"), "map_quality_lines": count_matches(result.stdout, "Map Quality|map_quality|without static readers|without deterministic proof|stale_lens_artifact"), "trust_violations": trust, "budget_status": "ok" if lines <= line_budget else "over"}
    progress(f"done repo={name} label={label} status={result.returncode} elapsed_ms={elapsed}/{latency_budget} latency={row['latency_status']} lines={lines}/{line_budget} budget={row['budget_status']} trust_violations={trust} output={output_path.name}")
    return row


def main(argv: list[str]) -> int:
    try:
        out = output_dir()
    except ValueError as error:
        print(error, file=sys.stderr)
        return 2
    cache = canonical(Path(os.environ.get("CODEMAP_DOGFOOD_CACHE", out / "cache")))
    out.mkdir(parents=True, exist_ok=True)
    cache.mkdir(parents=True, exist_ok=True)
    for path in out.iterdir():
        if path.is_file() and (path.suffix in {".md", ".log"} or path.name.endswith(".summary.jsonl") or path.name == "summary.jsonl"):
            path.unlink()
    targets = [canonical(Path(value)) for value in argv] if argv else [Path("/Users/amir/Documents/projects/spritestudio"), Path("/Users/amir/Documents/projects/Sillentway-VPN")]
    progress(f"start targets={len(targets)} out={out} cache={cache}")
    all_rows = []
    for index, target in enumerate(targets, 1):
        name = safe_label(target.name)
        log = out / f"{name}.log"
        progress(f"repo-start index={index}/{len(targets)} name={name} path={target}")
        if not target.is_dir():
            rows = [{"repo": str(target), "status": "missing"}]
            progress(f"repo-missing index={index}/{len(targets)} name={name} path={target}")
        else:
            rows = [run_probe(target, label, args, out, cache, log) for label, args in probe_specs(target)]
            progress(f"repo-done index={index}/{len(targets)} name={name} summary={name}.summary.jsonl")
        (out / f"{name}.summary.jsonl").write_text("".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows), encoding="utf-8")
        all_rows.extend(rows)
    summary = out / "summary.jsonl"
    summary.write_text("".join(json.dumps(row, separators=(",", ":")) + "\n" for row in all_rows), encoding="utf-8")
    failures = sum(row.get("status", 0) != 0 for row in all_rows)
    over = sum(row.get("budget_status") == "over" for row in all_rows)
    trust = sum(int(row.get("trust_violations", 0) or 0) for row in all_rows)
    slow = sum(row.get("latency_status") == "slow" for row in all_rows)
    primary = {"ls_root", "changed", "proof_changed", "proof_map_root", "cone_owner", "proof_owner"}
    primary_slow = sum(row.get("label") in primary and row.get("latency_status") == "slow" for row in all_rows)
    counts = f"probes={len(all_rows)} failures={failures} over_budget={over} slow={slow} primary_slow={primary_slow} trust_violations={trust}"
    strict = os.environ.get("CODEMAP_DOGFOOD_STRICT", "0").lower() not in {"", "0", "false", "no"}
    max_slow = int(os.environ.get("CODEMAP_DOGFOOD_MAX_SLOW", "0"))
    problems = ([f"failures={failures}"] if failures else []) + ([f"over_budget={over}"] if over else []) + ([f"trust_violations={trust}"] if trust else []) + ([f"primary_slow={primary_slow}"] if primary_slow else []) + ([f"slow={slow} > max_slow={max_slow}"] if slow > max_slow else [])
    if strict and problems:
        print("strict_fail " + " ".join(problems), file=sys.stderr)
    progress(f"summary {counts} path={summary}")
    print(f"dogfood summary: {summary}")
    return 1 if strict and problems else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
