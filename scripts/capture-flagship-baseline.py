#!/usr/bin/env python3
"""Freeze the S00 B01-B12 acceptance baseline without writing target repos."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
CONTRACT_REVISION = "flagship-v1-draft"


def sha256_bytes(body: bytes) -> str:
    return hashlib.sha256(body).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def git(root: Path, *args: str, text: bool = True) -> str | bytes:
    return subprocess.check_output(["git", "-C", str(root), *args], text=text)


def git_state(root: Path) -> dict[str, Any]:
    status = git(root, "status", "--porcelain=v2", "-z", "--untracked-files=all", text=False)
    assert isinstance(status, bytes)
    return {
        "path": str(root.resolve()),
        "commit": str(git(root, "rev-parse", "HEAD")).strip(),
        "branch": str(git(root, "rev-parse", "--abbrev-ref", "HEAD")).strip(),
        "dirty": bool(status),
        "dirty_fingerprint_sha256": sha256_bytes(status),
        "status_records": status.count(b"\0"),
    }


def run_probe(
    label: str,
    argv: list[str],
    cwd: Path,
    env: dict[str, str],
    expected_statuses: tuple[int, ...] = (0,),
    timeout: int = 180,
) -> dict[str, Any]:
    started = time.monotonic()
    try:
        proc = subprocess.run(
            argv,
            cwd=cwd,
            env=env,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
        status = proc.returncode
        stdout = proc.stdout
        stderr = proc.stderr
    except subprocess.TimeoutExpired as exc:
        status = 124
        stdout = exc.stdout or b""
        stderr = (exc.stderr or b"") + b"\nprobe timed out\n"
    return {
        "label": label,
        "argv": argv,
        "cwd": str(cwd.resolve()),
        "status": status,
        "expected_statuses": list(expected_statuses),
        "valid_status": status in expected_statuses,
        "elapsed_ms": round((time.monotonic() - started) * 1000),
        "stdout": stdout,
        "stderr": stderr,
    }


def assertion(assertion_id: str, passed: bool, expected: Any, actual: Any) -> dict[str, Any]:
    return {"id": assertion_id, "passed": passed, "expected": expected, "actual": actual}


def text(result: dict[str, Any]) -> str:
    return result["stdout"].decode("utf-8", errors="replace")


def freeze_observation(
    baseline_root: Path,
    observed_at: str,
    observation_id: str,
    title: str,
    repository: dict[str, Any],
    binary: dict[str, Any],
    results: list[dict[str, Any]],
    assertions: list[dict[str, Any]],
    source_artifacts: list[Path] | None = None,
    notes: list[str] | None = None,
) -> dict[str, Any]:
    receipt_id = f"{observation_id}-{observed_at.replace(':', '').replace('-', '')}-{binary['sha256'][:12]}"
    receipt_dir = baseline_root / receipt_id
    receipt_dir.mkdir(parents=True)
    artifacts = []
    commands = []
    for result in results:
        commands.append({key: result[key] for key in ("label", "argv", "cwd", "status", "elapsed_ms")})
        for stream in ("stdout", "stderr"):
            path = receipt_dir / f"{result['label']}.{stream}.log"
            path.write_bytes(result[stream])
            artifacts.append(
                {"path": str(path), "sha256": sha256_file(path), "bytes": path.stat().st_size}
            )
    for source in source_artifacts or []:
        artifacts.append(
            {"path": str(source.resolve()), "sha256": sha256_file(source), "bytes": source.stat().st_size}
        )
    valid = all(result["valid_status"] for result in results) and bool(assertions)
    status = "invalid" if not valid else "reproduced" if all(item["passed"] for item in assertions) else "changed"
    manifest = {
        "kind": "codemap_flagship_baseline_observation",
        "contract_revision": CONTRACT_REVISION,
        "baseline_id": receipt_id,
        "observation_id": observation_id,
        "title": title,
        "observed_at": observed_at,
        "repository": repository,
        "command_argv": commands,
        "binary_identity": binary,
        "environment_summary": {
            "platform": platform.platform(),
            "python": platform.python_version(),
            "cache_dir": os.environ.get("CODEMAP_CACHE_DIR"),
            "network_used": False,
        },
        "stdout_stderr_artifact_hashes": artifacts,
        "machine_checkable_assertion": assertions,
        "status": status,
        "notes": notes or [],
    }
    manifest_path = receipt_dir / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    return {
        "observation_id": observation_id,
        "baseline_id": receipt_id,
        "status": status,
        "manifest_path": str(manifest_path),
        "manifest_sha256": sha256_file(manifest_path),
    }


def parse_version(body: str) -> str | None:
    match = re.search(r"\bcodemap ([0-9]+\.[0-9]+\.[0-9]+)\b", body)
    return match.group(1) if match else None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--codemap-bin", type=Path, required=True)
    parser.add_argument("--codemap-root", type=Path, required=True)
    parser.add_argument("--main-cluster-root", type=Path, required=True)
    parser.add_argument(
        "--output", type=Path, default=REPO_ROOT / "target" / "flagship-baseline"
    )
    parser.add_argument(
        "--simple-ab", type=Path, default=REPO_ROOT / "target/codemap-ab/final-smoke-20260713T214010Z/summary.json"
    )
    parser.add_argument(
        "--main-cluster-ab",
        type=Path,
        default=REPO_ROOT / "target/codemap-ab/main-cluster-analysis-20260714/evaluation.json",
    )
    args = parser.parse_args()
    codemap_bin = args.codemap_bin.resolve()
    codemap_root = args.codemap_root.resolve()
    main_root = args.main_cluster_root.resolve()
    output = args.output.resolve()
    allowed_roots = [(REPO_ROOT / "target").resolve(), Path(tempfile.gettempdir()).resolve()]
    if not any(output == root or output.is_relative_to(root) for root in allowed_roots):
        parser.error("--output must stay under repository target/ or the system temp directory")
    for path in (codemap_bin, args.simple_ab, args.main_cluster_ab):
        if not path.exists():
            parser.error(f"required artifact is missing: {path}")
    if git_state(codemap_root)["dirty"] or git_state(main_root)["dirty"]:
        parser.error("both attributed input worktrees must be clean")

    observed_at = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    cache_root = output / f"cache-{observed_at.replace(':', '').replace('-', '')}"
    cache_root.mkdir(parents=True, exist_ok=True)
    os.environ["CODEMAP_CACHE_DIR"] = str(cache_root)
    env = dict(os.environ, CODEMAP_CACHE_DIR=str(cache_root))
    candidate_version = run_probe("candidate-version", [str(codemap_bin), "--version"], codemap_root, env)
    binary = {
        "path": str(codemap_bin),
        "version": text(candidate_version).strip(),
        "sha256": sha256_file(codemap_bin),
        "source_commit": git_state(codemap_root)["commit"],
        "source_dirty": False,
    }
    before = {"codemap": git_state(codemap_root), "main_cluster": git_state(main_root)}
    receipts: list[dict[str, Any]] = []

    installed = shutil.which("codemap")
    b01_results = [candidate_version]
    if installed:
        b01_results.append(run_probe("installed-version", [installed, "--version"], codemap_root, env))
    doctor = run_probe("doctor", [str(codemap_bin), "--root", str(codemap_root), "doctor"], codemap_root, env)
    b01_results.append(doctor)
    candidate_semver = parse_version(text(candidate_version))
    installed_semver = parse_version(text(b01_results[1])) if installed else None
    cargo_semver = re.search(r'^version = "([^"]+)"', (codemap_root / "Cargo.toml").read_text(), re.M).group(1)
    receipts.append(freeze_observation(
        output, observed_at, "B01", "binary identity", before["codemap"], binary, b01_results,
        [assertion("candidate_matches_manifest", candidate_semver == cargo_semver, cargo_semver, candidate_semver),
         assertion("installed_candidate_mismatch_visible_to_fixture", installed_semver != candidate_semver, "different versions", {"installed": installed_semver, "candidate": candidate_semver}),
         assertion("doctor_omits_executable_identity", "Executable" not in text(doctor) and "Binary SHA" not in text(doctor), True, "identity fields absent")],
        notes=[f"installed executable: {installed or 'not found'}"]
    ))

    where = run_probe("where-cone-report", [str(codemap_bin), "--root", str(codemap_root), "where", "ConeReport"], codemap_root, env)
    refs = run_probe("source-references", ["git", "grep", "-l", "-w", "ConeReport", "--", "src"], codemap_root, env)
    ref_count = len(text(refs).splitlines())
    receipts.append(freeze_observation(
        output, observed_at, "B02", "consumer zero", before["codemap"], binary, [where, refs],
        [assertion("reported_consumer_zero", "consumers: `0`" in text(where), True, "consumers: `0`" in text(where)),
         assertion("source_has_multiple_references", ref_count >= 8, ">= 8 source files", ref_count)]
    ))

    help_result = run_probe("top-level-help", [str(codemap_bin), "--help"], codemap_root, env)
    readme = run_probe("readme", ["cat", "README.md"], codemap_root, env)
    receipts.append(freeze_observation(
        output, observed_at, "B03", "daily discoverability", before["codemap"], binary, [help_result, readme],
        [assertion("where_hidden_from_top_help", not re.search(r"(?m)^\s+where\s", text(help_result)), True, "where absent"),
         assertion("where_absent_from_readme", "codemap where" not in text(readme), True, "codemap where absent")]
    ))

    def codemap_probe(label: str, *argv: str, timeout: int = 180) -> dict[str, Any]:
        return run_probe(label, [str(codemap_bin), "--root", str(main_root), *argv], main_root, env, timeout=timeout)

    graph = codemap_probe("root-causal-graph", "graph", "--lens", "causal")
    contains_edges = len(re.findall(r"\| `\.` \| contains \|", text(graph)))
    receipts.append(freeze_observation(
        output, observed_at, "B04", "root relation map", before["main_cluster"], binary, [graph],
        [assertion("visible_edges_are_containment", contains_edges >= 10, ">= 10 containment edges", contains_edges),
         assertion("large_hidden_horizon", "graph nodes hidden by limit | 692" in text(graph), True, "692 hidden nodes")]
    ))

    workflow = codemap_probe("workflow-cone", "cone", ".github/workflows/release-prod.yml", "--depth", "2")
    workflow_body = text(workflow)
    receipts.append(freeze_observation(
        output, observed_at, "B05", "workflow execution", before["main_cluster"], binary, [workflow],
        [assertion("workflow_length_observed", "lines: `1494`" in workflow_body, 1494, "1494" if "1494" in workflow_body else None),
         assertion("execution_chain_absent", not any(edge in workflow_body for edge in ("script_invocation", "deployment_mutation", "runtime_smoke", "receipt_write")), [], [edge for edge in ("script_invocation", "deployment_mutation", "runtime_smoke", "receipt_write") if edge in workflow_body])]
    ))

    runtime = codemap_probe("control-center-runtime", "runtime", "apps/control-center")
    receipts.append(freeze_observation(
        output, observed_at, "B06", "runtime transformations", before["main_cluster"], binary, [runtime],
        [assertion("panel_routes_visible", text(runtime).count("/api/agent/panels") >= 4, ">= 4 panel route entries", text(runtime).count("/api/agent/panels")),
         assertion("bounded_runtime_horizon", "runtime routes hidden by limit: 215" in text(runtime) and "environment surfaces hidden by limit: 202" in text(runtime), {"routes": 215, "env": 202}, "limits present")]
    ))

    migration = "apps/control-center/db/migrations/2025-10-14-create-control-center-outbox.sql"
    contract = codemap_probe("outbox-contract", "contract", migration)
    contract_body = text(contract)
    receipts.append(freeze_observation(
        output, observed_at, "B07", "contract and data lineage", before["main_cluster"], binary, [contract],
        [assertion("soft_test_mass_visible", contract_body.count("test_surface_tokens") >= 10, ">= 10 visible soft tests", contract_body.count("test_surface_tokens")),
         assertion("main_lineage_absent", not re.search(r"\b(?:adapter|dispatcher)\s+->", contract_body) and "Deployment" not in contract_body and "Secret" not in contract_body, [], re.findall(r"\b(?:adapter|dispatcher)\s+->|Deployment|Secret", contract_body)),
         assertion("hidden_verification_mass", "contract verification edges hidden by limit: 120" in contract_body, 120, 120 if "limit: 120" in contract_body else None)]
    ))

    with tempfile.TemporaryDirectory(prefix="codemap-s00-dirty-") as temp:
        dirty_root = Path(temp) / "repo"
        subprocess.run(["git", "-C", str(codemap_root), "worktree", "add", "--detach", str(dirty_root), before["codemap"]["commit"]], check=True, capture_output=True)
        try:
            fixture_dir = dirty_root / "s00-dirty-fixture"
            fixture_dir.mkdir()
            for index in range(1, 113):
                (fixture_dir / f"file_{index:03}.rs").write_text(f"pub fn dirty_{index:03}() -> usize {{ {index} }}\n")
            dirty_before = git_state(dirty_root)
            changed = run_probe("changed-112-paths", [str(codemap_bin), "--root", str(dirty_root), "changed"], dirty_root, env)
            dirty_after = git_state(dirty_root)
            receipts.append(freeze_observation(
                output, observed_at, "B08", "dirty changed path", {"before": dirty_before, "after": dirty_after, "fixture": "112 generated untracked Rust files"}, binary, [changed],
                [assertion("all_dirty_paths_observed", "112` total files" in text(changed) and "selected files: `112`" in text(changed), 112, 112 if "selected files: `112`" in text(changed) else None),
                 assertion("dirty_fixture_unchanged_by_codemap", dirty_before["dirty_fingerprint_sha256"] == dirty_after["dirty_fingerprint_sha256"], dirty_before["dirty_fingerprint_sha256"], dirty_after["dirty_fingerprint_sha256"]),
                 assertion("bounded_display_hides_work", "Changed: `30` shown / `112` total files" in text(changed), True, "30 shown / 112 total")],
                notes=["elapsed_ms is the frozen cold command cost for this synthetic broad-dirty acceptance fixture"]
            ))
        finally:
            subprocess.run(["git", "-C", str(codemap_root), "worktree", "remove", "--force", str(dirty_root)], check=True, capture_output=True)

    cone = run_probe("lens-reports-cone", [str(codemap_bin), "--root", str(codemap_root), "cone", "src/model/lens_reports.rs", "--depth", "1"], codemap_root, env)
    repeated_tests = text(cone).count("tests -> `src/model/lens_reports.rs`")
    receipts.append(freeze_observation(
        output, observed_at, "B09", "repeated fact disclosure", before["codemap"], binary, [cone],
        [assertion("verification_facts_repeat_across_sections", repeated_tests >= 5, ">= 5 repeated test edges", repeated_tests),
         assertion("hidden_counts_are_split", "9 more Soft verification sensors" in text(cone) and "hidden: 8 soft surface matches edges" in text(cone), True, "two hidden counters")]
    ))

    composition = run_probe("changed-composition", ["git", "grep", "-n", "-E", r"diff_map_report\(|impact_report\(|proof_map_report\(", "--", "src/map/lenses/changed.rs"], codemap_root, env)
    evidence_reads = run_probe("evidence-source-reads", ["git", "grep", "-n", "read_to_string", "--", "src/evidence.rs", "src/map/edges.rs"], codemap_root, env)
    receipts.append(freeze_observation(
        output, observed_at, "B10", "repeated cold work", before["codemap"], binary, [composition, evidence_reads],
        [assertion("changed_composes_three_reports", all(name in text(composition) for name in ("diff_map_report", "impact_report", "proof_map_report")), ["diff_map", "impact", "proof_map"], "three report builders"),
         assertion("evidence_paths_reread_source", text(evidence_reads).count("read_to_string") >= 3, ">= 3 read sites", text(evidence_reads).count("read_to_string"))]
    ))

    swallowed = run_probe("cache-soft-failures", ["git", "grep", "-n", "-E", r"\.ok\(\)|let _ =", "--", "src/cache"], codemap_root, env)
    retention = run_probe("cache-lifecycle-contract", ["git", "grep", "-n", "-E", r"retention|privacy|garbage|\bgc\b|evict|expiry|expire|ttl", "--", "src/cache"], codemap_root, env, expected_statuses=(0, 1))
    receipts.append(freeze_observation(
        output, observed_at, "B11", "cache truth", before["codemap"], binary, [swallowed, retention],
        [assertion("cache_failures_softened", text(swallowed).count(".ok()") + text(swallowed).count("let _ =") >= 10, ">= 10 softened operations", text(swallowed).count(".ok()") + text(swallowed).count("let _ =")),
         assertion("cache_lifecycle_contract_absent", not text(retention).strip(), True, "no lifecycle terms")]
    ))

    simple = json.loads(args.simple_ab.read_text())
    complex_ab = json.loads(args.main_cluster_ab.read_text())
    receipts.append(freeze_observation(
        output, observed_at, "B12", "behavioral evidence", before["codemap"], binary, [],
        [assertion("simple_task_quality_tie", simple["effect"]["pass_rate_delta_percentage_points"] == 0.0, 0.0, simple["effect"]["pass_rate_delta_percentage_points"]),
         assertion("simple_task_cost_increased", simple["effect"]["median_elapsed_delta_ms"] > 0 and simple["effect"]["median_input_token_delta"] > 0, "positive time and input deltas", simple["effect"]),
         assertion("main_cluster_score_lift", complex_ab["mean_scores"] == {"control": 95.0, "codemap": 97.5}, {"control": 95.0, "codemap": 97.5}, complex_ab["mean_scores"]),
         assertion("main_cluster_citation_lift", complex_ab["evidence_shape"]["control"]["valid_citations"] == 54 and complex_ab["evidence_shape"]["codemap"]["valid_citations"] == 61, {"control": 54, "codemap": 61}, {arm: complex_ab["evidence_shape"][arm]["valid_citations"] for arm in ("control", "codemap")})],
        source_artifacts=[args.simple_ab, args.main_cluster_ab],
        notes=["Historical A/B artifacts retain their own binary hashes; the S00 capture binary validates their frozen claims but does not rerun model trials."]
    ))

    after = {"codemap": git_state(codemap_root), "main_cluster": git_state(main_root)}
    footprint_unchanged = before == after
    index = {
        "kind": "codemap_flagship_baseline_index",
        "contract_revision": CONTRACT_REVISION,
        "observed_at": observed_at,
        "binary_identity": binary,
        "repositories_before": before,
        "repositories_after": after,
        "zero_repo_footprint": footprint_unchanged,
        "observations": receipts,
        "summary": {status: sum(item["status"] == status for item in receipts) for status in ("reproduced", "changed", "invalid")},
    }
    index_id = f"S00-baseline-{observed_at.replace(':', '').replace('-', '')}-{binary['sha256'][:12]}"
    index_dir = output / index_id
    index_dir.mkdir(parents=True)
    index_path = index_dir / "manifest.json"
    index_path.write_text(json.dumps(index, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({"baseline_id": index_id, "manifest_path": str(index_path), "manifest_sha256": sha256_file(index_path), "summary": index["summary"], "zero_repo_footprint": footprint_unchanged}, indent=2))
    return 0 if footprint_unchanged and index["summary"]["invalid"] == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
