#!/usr/bin/env python3
"""Black-box proof for one immutable flagship bundle with failed attempts."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / "scripts/package-release.py"


def write(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body, encoding="utf-8")


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def acceptance(root: Path, label: str, accepted: bool) -> Path:
    attempt = root / label
    manifest = attempt / "frozen/manifest.json"
    raw = attempt / "run/results.jsonl"
    verifier = attempt / "run/verifier.stdout.log"
    cache = attempt / "run/trials/task/codemap-cache/inventory.json"
    write(
        manifest,
        json.dumps(
            {
                "codemap_identity": {
                    "build_identity": {
                        "source_commit": ("a" if accepted else "b") * 40,
                        "binary_sha256": ("c" if accepted else "d") * 64,
                    }
                }
            }
        ),
    )
    write(raw, '{"arm":"control"}\n{"arm":"codemap"}\n')
    write(verifier, "external verifier\n")
    write(cache, "derived cache\n")
    resources = {
        "complex_median_time_overhead": 0.1,
        "complex_median_input_overhead": 0.1,
        "exact_median_time_overhead": 0.05,
        "exact_median_input_overhead": 0.05,
    }
    receipt = attempt / "acceptance/acceptance.json"
    report = {
        "kind": "codemap_flagship_acceptance",
        "version": 1,
        "manifest": str(manifest),
        "manifest_sha256": digest(manifest),
        "evidence": [
            {"path": str(path), "sha256": digest(path)}
            for path in (manifest, raw, verifier, cache)
        ],
        "acceptance": {
            "accepted": accepted,
            "complex": {"wins": 8 if accepted else 7, "losing_tasks": []},
            "resources": resources,
        },
    }
    write(receipt, json.dumps(report, indent=2, sort_keys=True) + "\n")
    write(receipt.with_name("acceptance.md"), "PASSED\n" if accepted else "FAILED\n")
    return receipt


def run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run([sys.executable, str(PACKAGE), *args], capture_output=True, text=True)


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="codemap-flagship-evidence-") as raw:
        root = Path(raw)
        failed = acceptance(root, "failed", False)
        accepted = acceptance(root, "accepted", True)
        out = root / "dist"
        built = run(
            "build-evidence",
            "--version",
            "1.0.0",
            "--acceptance",
            str(failed),
            "--acceptance",
            str(accepted),
            "--out-dir",
            str(out),
        )
        assert built.returncode == 0, built.stderr
        archive = out / "flagship-evidence-v1.0.0.tar.gz"
        checksum = out / "flagship-evidence-v1.0.0.tar.gz.sha256"
        verified = run(
            "verify-evidence",
            "--archive",
            str(archive),
            "--checksum",
            str(checksum),
            "--version",
            "1.0.0",
        )
        assert verified.returncode == 0, verified.stderr
        with tarfile.open(archive, "r:gz") as bundle:
            body = bundle.extractfile("flagship-evidence-v1.0.0/bundle.json")
            assert body is not None
            manifest = json.load(body)
            assert [row["accepted"] for row in manifest["attempts"]] == [False, True]
            assert [row["omitted_derived_cache_files"] for row in manifest["attempts"]] == [1, 1]
            assert any(row["path"].endswith("results.jsonl") for row in manifest["files"])
            assert any(row["path"].endswith("verifier.stdout.log") for row in manifest["files"])
            assert not any("codemap-cache" in row["path"] for row in manifest["files"])
        checksum.write_text(f"{'0' * 64}  {archive.name}\n", encoding="utf-8")
        assert run(
            "verify-evidence",
            "--archive",
            str(archive),
            "--checksum",
            str(checksum),
            "--version",
            "1.0.0",
        ).returncode != 0
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
