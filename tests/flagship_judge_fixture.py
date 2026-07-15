#!/usr/bin/env python3
"""Synthetic black-box proof for blind judge execution and manual-audit merging."""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "tests"))

from flagship_gate_fixture import (  # noqa: E402
    fixture_repos,
    freeze,
    run_receipt,
    task_rows,
    write,
)
from flagship_judge_runner import JUDGE_IDS, merge_audits, run_judging  # noqa: E402
from flagship_judging import prepare_assignments, read_jsonl  # noqa: E402


def tools(root: Path) -> tuple[Path, Path, Path]:
    codex = root / "fake-judge-codex.py"
    codemap = root / "fake-codemap.py"
    verifier = root / "verify.py"
    write(
        codex,
        """import json, pathlib, sys
if '--version' in sys.argv:
    print('codex-cli blind-fixture')
    raise SystemExit(0)
request = json.loads(sys.argv[-1].split('REQUEST_JSON:\\n', 1)[1])
ratings = []
for candidate in request['candidates']:
    ratings.append({
        'assignment_id': candidate['assignment_id'],
        'candidate_id': candidate['candidate_id'],
        'scores': {criterion: 3 for criterion in candidate['criteria']},
        'reasons': {criterion: 'checked frozen evidence' for criterion in candidate['criteria']},
    })
pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_text(json.dumps({'ratings': ratings}))
print(json.dumps({'type': 'turn.completed', 'usage': {'input_tokens': 1, 'output_tokens': 1}}))
""",
    )
    write(
        codemap,
        "import json, sys\nprint('codemap 1.0') if '--version' in sys.argv else print(json.dumps({}))\n",
    )
    write(verifier, "raise SystemExit(0)\n")
    return codex, codemap, verifier


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="codemap-blind-judge-") as temporary:
        root = Path(temporary)
        repos = fixture_repos(root)
        codex, codemap, verifier = tools(root)
        tasks = task_rows(root, repos, verifier)
        manifest_path, manifest = freeze(root, tasks, codex, codemap)
        calibration = run_receipt(root, "calibration", tasks, manifest)
        holdout = run_receipt(root, "holdout", tasks, manifest)
        assignments, _ = prepare_assignments(
            manifest_path, tasks, [calibration, holdout], root / "assignments"
        )
        ratings_path = run_judging(manifest_path, assignments, root / "judging")
        ratings = read_jsonl(ratings_path)
        assert {row["judge_id"] for row in ratings} == set(JUDGE_IDS)
        assert len(ratings) == 12 * 3 * 2 * 2
        audit_packet = read_jsonl(root / "judging/manual-audit-packet.jsonl")
        assert len(audit_packet) == 12
        decisions = root / "manual-decisions.jsonl"
        write(
            decisions,
            "\n".join(
                json.dumps(
                    {
                        "assignment_id": row["assignment_id"],
                        "candidate_id": row["candidate_id"],
                        "audit_passed": True,
                    }
                )
                for row in audit_packet
            )
            + "\n",
        )
        merged = merge_audits(assignments, ratings_path, decisions, root / "ratings-final.jsonl")
        final = read_jsonl(merged)
        assert len(final) == len(ratings) + len(audit_packet)
        assert sum(row["role"] == "auditor" for row in final) == 12
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
