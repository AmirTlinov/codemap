#!/usr/bin/env python3
"""Black-box contract for source-backed investigation outcomes."""

from __future__ import annotations

import json
import importlib.util
import subprocess
import shutil
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VERIFY = ROOT / "benchmarks/flagship/verify.py"


def source_claim(
    root: Path,
    message: str,
    *,
    source_body: str = "fn owner() { contract(); }\n",
    evidence_path: str = "src/owner.rs",
) -> subprocess.CompletedProcess[str]:
    source = root / "src/owner.rs"
    source.parent.mkdir(exist_ok=True)
    source.write_text(source_body, encoding="utf-8")
    spec = {
        "tasks": {
            "task": {
                "claim": {
                    "kind": "source_claim",
                    "evidence": [{"path": evidence_path, "contains": ["contract()"]}],
                    "citations": [evidence_path],
                }
            }
        }
    }
    spec_path = root / "source-spec.json"
    message_path = root / "source-message.md"
    events_path = root / "source-events.jsonl"
    spec_path.write_text(json.dumps(spec), encoding="utf-8")
    message_path.write_text(message, encoding="utf-8")
    events_path.write_text("", encoding="utf-8")
    return subprocess.run(
        [
            sys.executable,
            str(VERIFY),
            str(spec_path),
            "task",
            "claim",
            str(root),
            str(message_path),
            str(events_path),
        ],
        capture_output=True,
        text=True,
    )


def main() -> int:
    blueprint = json.loads(
        (ROOT / "benchmarks/flagship/corpus-blueprint.json").read_text(encoding="utf-8")
    )
    investigations = [
        task for task in blueprint["tasks"] if task["task_class"] == "investigation"
    ]
    assert len(investigations) == 6
    assert all(
        action["kind"] == "source_claim"
        for task in investigations
        for action in task["criteria"].values()
    )
    assert all(
        "answer_contains" not in action
        for task in investigations
        for action in task["criteria"].values()
    )
    assert all(len(task["criteria"]) >= 4 for task in investigations)
    assert all(
        any(action.get("required") is True for action in task["criteria"].values())
        and any(action.get("required") is False for action in task["criteria"].values())
        for task in investigations
    )
    assert all("machine check" not in task["prompt"] for task in investigations)
    assert all(
        set(action["citations"]) <= {row["path"] for row in action["evidence"]}
        for task in investigations
        for action in task["criteria"].values()
    )
    ratio = next(
        task for task in investigations if task["id"] == "ratio-deterministic-investigation"
    )
    ratio_paths = {
        evidence["path"]
        for action in ratio["criteria"].values()
        for evidence in action["evidence"]
    }
    assert "crates/ratiotissue-cli/src/continuation/feedback.rs" not in ratio_paths
    assert "what world operation actually occurs" in ratio["prompt"]
    pabg_investigation = next(
        task for task in investigations if task["id"] == "pabg-deterministic-investigation"
    )
    assert pabg_investigation["criteria"]["assembled-package-boundary"]["citations"] == [
        "apps/web/src/lib/replay/package-dir.ts"
    ]
    assert pabg_investigation["criteria"]["live-integrity-and-materialization"][
        "citations"
    ] == [
        "apps/web/src/lib/replay/loader.ts",
        "apps/web/src/lib/replay/package-dir.ts",
    ]
    assert pabg_investigation["criteria"]["policy-boundary"]["citations"] == [
        "apps/web/src/lib/replay/loader.ts"
    ]

    implementations = {
        task["id"]: task
        for task in blueprint["tasks"]
        if task["task_class"] == "implementation"
    }
    assert all(len(task["criteria"]) >= 4 for task in implementations.values())
    assert all(
        all(action["kind"] == "commands" for action in task["criteria"].values())
        for task in implementations.values()
    )
    browser = implementations["browser-focused-clipboard"]
    assert "both `clipboard.write` and `clipboard.writeSvg`" in browser["prompt"]
    assert "ordered structured `tab` and `offscreen` attempts" in browser["prompt"]
    assert "kill switch" in browser["prompt"]
    assert {"tab-provenance", "offscreen-fallback", "combined-failure"} <= set(
        browser["criteria"]
    )
    browser_oracle = (ROOT / "benchmarks/flagship/browser_focused_clipboard_test.js").read_text(
        encoding="utf-8"
    )
    assert "result?.attempts" in browser_oracle
    assert "value.selectedTabId" in browser_oracle
    assert "value.selectedTabSource" in browser_oracle
    assert "contextKind(row) === \"tab\"" in browser_oracle
    assert "tab.ok !== false" in browser_oracle
    assert "value?.carrierDetails" in browser_oracle
    assert "value?.carrier?.world" in browser_oracle
    assert "value?.tabCarrier?.world" in browser_oracle
    assert "carrier?.kind || carrier?.type" in browser_oracle
    assert "combined?.data?.attempts" in browser_oracle
    with tempfile.TemporaryDirectory(prefix="codemap-browser-receipt-") as raw:
        browser_root = Path(raw)
        worker = browser_root / "vendor/browser_extension/service_worker.js"
        worker.parent.mkdir(parents=True)
        worker.write_text(
            """
const state = { enabled: true, focusedTabId: null };
async function dispatchRpc(method, params = {}) {
  const requested = Object.prototype.hasOwnProperty.call(params, "tabId");
  const tabId = Number(requested ? params.tabId : state.focusedTabId);
  const source = requested ? "request" : "focused";
  const rows = await chrome.scripting.executeScript({
    target: { tabId, frameIds: [0] }, world: "ISOLATED", func() {}, args: [],
  });
  return {
    selectedTab: { tabId: String(tabId), source },
    selectedTabId: String(tabId),
    carrier: "isolated_top_frame",
    attempts: [{
      kind: "tab", carrier: "isolated_top_frame", tabId: String(tabId), source,
      world: "ISOLATED", frameId: 0, ok: true, result: rows[0].result,
    }],
  };
}
""",
            encoding="utf-8",
        )
        for criterion in ("tab-provenance", "svg-carrier"):
            semantic_receipt = subprocess.run(
                [
                    "node",
                    str(ROOT / "benchmarks/flagship/browser_focused_clipboard_test.js"),
                    str(browser_root),
                    criterion,
                ],
                capture_output=True,
                text=True,
            )
            assert semantic_receipt.returncode == 0, semantic_receipt.stderr
    openslop = implementations["openslop-active-plan"]
    assert "public `active-plan`" in openslop["prompt"]
    assert "every ROADMAP row in order as `slices`" in openslop["prompt"]
    assert "including zero values" in openslop["prompt"]
    assert "status/review/visual proof artifacts" in openslop["prompt"]
    assert "OPEN_SLOP_REPO_ROOT" in openslop["prompt"]
    assert {"workspace-projection", "stdio-contract", "consumer-probes"} <= set(
        openslop["criteria"]
    )
    active_plan_spec = importlib.util.spec_from_file_location(
        "active_plan_oracle",
        ROOT / "benchmarks/flagship/oracles/openslop/active_plan_test.py",
    )
    assert active_plan_spec is not None and active_plan_spec.loader is not None
    active_plan_oracle = importlib.util.module_from_spec(active_plan_spec)
    active_plan_spec.loader.exec_module(active_plan_oracle)
    assert active_plan_oracle.slice_id({"id": "S01"}) == "S01"
    assert active_plan_oracle.slice_id({"slice": "S01"}) == "S01"
    visual = {"kind": "visual_proof", "state": "missing", "available": False}
    assert active_plan_oracle.proof_artifacts({"artifacts": [visual]}) == {
        "visual": visual
    }
    grouped = {"visual_proof": {"state": "missing", "available": False}}
    assert active_plan_oracle.proof_artifacts({"artifacts": grouped}) == {
        "visual": grouped["visual_proof"]
    }
    original_root, original_run = active_plan_oracle.ROOT, active_plan_oracle.run
    try:
        with tempfile.TemporaryDirectory(prefix="codemap-openslop-probe-") as raw:
            probe_root = Path(raw)
            package = probe_root / "apps/macos-app/Package.swift"
            source = probe_root / "apps/macos-app/Sources/OpenSlopActivePlanProbe/main.swift"
            source.parent.mkdir(parents=True)
            package.write_text("// package\n", encoding="utf-8")
            source.write_text("// probe\n", encoding="utf-8")
            calls = []

            def record(argv, **kwargs):
                calls.append((argv, kwargs))
                return subprocess.CompletedProcess(argv, 0, "PASS", "")

            active_plan_oracle.ROOT = probe_root
            active_plan_oracle.run = record
            active_plan_oracle.run_consumer_probe()
            assert calls[0][0][:3] == ["swift", "run", "--disable-sandbox"]
            assert calls[0][0][-1] == "OpenSlopActivePlanProbe"
            source.unlink()
            active_plan_oracle.run_consumer_probe()
            assert calls[1][0] == ["make", "probe-active-plan"]
    finally:
        active_plan_oracle.ROOT, active_plan_oracle.run = original_root, original_run
    codemap = implementations["codemap-response-projection"]
    assert "src/map/lenses/runtime/paths.rs#runtime_route_path_analysis" in codemap["prompt"]
    assert "balanced return expressions" in codemap["prompt"]
    assert "passive telemetry call" in codemap["prompt"]
    assert "awaited security or session dependency" in codemap["prompt"]
    assert "rejection prevents the protected handler" in codemap["prompt"]
    assert "not awaited stays off the guard chain" in codemap["prompt"]
    assert "retains that field's provenance" in codemap["prompt"]
    assert "only when a field is absent" in codemap["prompt"]
    assert "recordCsrfFailure" not in codemap["prompt"]
    backup = implementations["main-postgres-backup"]
    assert "existing parent `deploy/k8s/base`" in backup["prompt"]
    assert "kube_cronjob_status_last_successful_time" in backup["prompt"]
    assert "without YAML anchors or aliases" in backup["prompt"]
    pabg = implementations["pabg-global-text-chat"]
    assert "TownHub domain boundary" in pabg["prompt"]
    assert "dedicated `text_chat` and `text_chat_broadcast`" in pabg["prompt"]
    assert "independent of the existing emoji allowance" in pabg["prompt"]
    hook_commands = pabg["criteria"]["web-consumer"]["commands"]
    assert any(
        "src/hooks/flagship_text_chat_hook.test.ts" in command["argv"]
        for command in hook_commands
    )
    chat_oracle = (
        ROOT / "benchmarks/flagship/oracles/pabg/flagship_text_chat.test.ts"
    ).read_text(encoding="utf-8")
    assert "embeddedHistory ??" in chat_oracle
    assert 'frame.type === "text_chat_broadcast"' in chat_oracle
    hook_oracle = (
        ROOT / "benchmarks/flagship/oracles/pabg/flagship_text_chat_hook.test.ts"
    ).read_text(encoding="utf-8")
    assert 'text: "First"' in hook_oracle
    assert "chat_history:" not in hook_oracle
    ratio_implementation = implementations["ratio-codex-live-episodes"]
    assert "continuation.rs#cmd_pulse_world_loop" in ratio_implementation["prompt"]
    assert "ratiotissue live-codex-sessions" in ratio_implementation["prompt"]
    assert "actionwave/world-return contact facts" in ratio_implementation["prompt"]
    assert "paired with `function_call_output` by the same `call_id`" in ratio_implementation[
        "prompt"
    ]
    assert "email addresses" in ratio_implementation["prompt"]
    assert "no_action" in ratio_implementation["prompt"]
    ratio_oracle = (
        ROOT / "benchmarks/flagship/oracles/ratio/live_codex_sessions_test.py"
    ).read_text(encoding="utf-8")
    assert '"type": "function_call"' in ratio_oracle
    assert '"type": "function_call_output"' in ratio_oracle
    assert '"call_id": CALL_ID' in ratio_oracle
    assert '"contact=observed"' in ratio_oracle
    assert "assert observed_contact(paired.stdout)" in ratio_oracle
    assert "assert explicit_no_contact(unpaired)" in ratio_oracle
    ratio_spec = importlib.util.spec_from_file_location(
        "ratio_live_codex_oracle",
        ROOT / "benchmarks/flagship/oracles/ratio/live_codex_sessions_test.py",
    )
    assert ratio_spec and ratio_spec.loader
    ratio_module = importlib.util.module_from_spec(ratio_spec)
    ratio_spec.loader.exec_module(ratio_module)
    assert ratio_module.observed_contact(
        "live_codex_episode contact=true actionwave=observed_external "
        "world_return=observed_external"
    )
    assert ratio_module.observed_contact(
        "live_codex_episode contact=paired actionwave=observed_external_action "
        "world_return=observed_external_return calls=1"
    )
    assert ratio_module.observed_contact(
        "live_codex_episode actionwave=observed world_return=observed contacts=1 "
        "contact_ids=contact-fixture"
    )
    assert not ratio_module.observed_contact(
        "live_codex_episode contact=true actionwave=no_action world_return=no_contact"
    )
    assert not ratio_module.observed_contact(
        "live_codex_episode contact=true actionwave=synthetic world_return=synthetic"
    )

    with tempfile.TemporaryDirectory(prefix="codemap-postgres-oracle-") as raw:
        oracle_root = Path(raw)
        oracle_path = oracle_root / "deploy/k8s/base/backup/flagship_postgres_backup_test.py"
        oracle_path.parent.mkdir(parents=True)
        shutil.copy2(
            ROOT / "benchmarks/flagship/oracles/main/postgres_backup_test.py",
            oracle_path,
        )
        yaml_loader = oracle_root / "scripts/ops/ci/yaml_loader.py"
        yaml_loader.parent.mkdir(parents=True)
        yaml_loader.write_text("def load_all_yaml(path): return []\n", encoding="utf-8")
        oracle_spec = importlib.util.spec_from_file_location(
            "postgres_backup_oracle", oracle_path
        )
        assert oracle_spec is not None and oracle_spec.loader is not None
        oracle = importlib.util.module_from_spec(oracle_spec)
        oracle_spec.loader.exec_module(oracle)
        assert oracle.has_checksum_comparison("expected | sha256sum -c -")
        assert oracle.has_checksum_comparison('test "$expected" = "$actual"')
        assert not oracle.has_checksum_comparison("sha256sum backup.sql.gz")
        assert oracle.remote_copy_count("restic backup /work") == 1
        assert (
            oracle.remote_copy_count(
                'mc --config-dir /work/mc cp "$remote" /work/readback/backup.dump'
            )
            == 1
        )
        assert oracle.has_remote_readback(
            "restic backup /work; restic restore latest --target /verify"
        )
        assert not oracle.has_remote_readback("restic backup /work")

    with tempfile.TemporaryDirectory(prefix="codemap-source-claim-verifier-") as raw:
        root = Path(raw)
        passed = source_claim(root, "owner is evidenced at src/owner.rs:1\n")
        assert passed.returncode == 0, passed.stdout
        receipt = json.loads(passed.stdout)
        assert receipt["evidence_source"] == "frozen_source_and_cited_report"
        assert receipt["cited_lines"] == {"src/owner.rs": [1]}

        differently_worded = source_claim(
            root,
            "The call is delegated by the implementation (src/owner.rs:1).\n",
        )
        assert differently_worded.returncode == 0, differently_worded.stdout

        assert source_claim(root, "owner without a citation\n").returncode == 1
        assert source_claim(root, "owner at src/owner.rs:999\n").returncode == 1
        assert source_claim(root, "owner at ../src/owner.rs:1\n").returncode == 1
        assert source_claim(
            root,
            "owner is claimed at src/owner.rs:1\n",
            source_body="fn unrelated() {}\n",
        ).returncode == 1
        assert source_claim(
            root,
            "owner is claimed at ../owner.rs:1\n",
            evidence_path="../owner.rs",
        ).returncode == 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
