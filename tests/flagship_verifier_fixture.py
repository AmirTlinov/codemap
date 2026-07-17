#!/usr/bin/env python3
"""Black-box contract for source-backed investigation outcomes."""

from __future__ import annotations

import json
import subprocess
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
