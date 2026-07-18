#!/usr/bin/env python3
"""Regression contract for PostgreSQL backup outcome discovery."""

from __future__ import annotations

import importlib.util
import shutil
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def load_oracle(root: Path):
    oracle_path = root / "deploy/k8s/base/backup/flagship_postgres_backup_test.py"
    oracle_path.parent.mkdir(parents=True)
    shutil.copy2(ROOT / "benchmarks/flagship/oracles/main/postgres_backup_test.py", oracle_path)
    yaml_loader = root / "scripts/ops/ci/yaml_loader.py"
    yaml_loader.parent.mkdir(parents=True)
    yaml_loader.write_text("def load_all_yaml(path): return []\n", encoding="utf-8")
    spec = importlib.util.spec_from_file_location("postgres_backup_oracle", oracle_path)
    assert spec is not None and spec.loader is not None
    oracle = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(oracle)
    return oracle


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="codemap-postgres-oracle-") as raw:
        oracle = load_oracle(Path(raw))
        assert oracle.has_checksum_comparison("expected | sha256sum -c -")
        assert oracle.has_checksum_comparison('test "$expected" = "$actual"')
        assert not oracle.has_checksum_comparison("sha256sum backup.sql.gz")
        assert oracle.remote_copy_count("restic backup /work") == 1
        assert oracle.has_remote_readback(
            "restic backup /work; restic restore latest --target /verify"
        )
        assert not oracle.has_remote_readback("restic backup /work")
        configmap = {
            "metadata": {"name": "postgres-backup"},
            "data": {
                "dump.sh": "pg_dump --file=/work/postgres.dump; restic backup /work",
                "verify.sh": "restic restore latest --target /verify; sha256sum -c /verify/postgres.dump.sha256",
                "unused.sh": "echo this mounted file is not invoked",
            },
        }
        pod = {
            "initContainers": [
                {
                    "command": ["/scripts/dump.sh"],
                    "volumeMounts": [{"name": "scripts", "mountPath": "/scripts"}],
                }
            ],
            "containers": [
                {
                    "command": ["/scripts/verify.sh"],
                    "volumeMounts": [{"name": "scripts", "mountPath": "/scripts"}],
                }
            ],
            "volumes": [
                {"name": "scripts", "configMap": {"name": "postgres-backup"}}
            ],
        }
        cronjob = {
            "spec": {"jobTemplate": {"spec": {"template": {"spec": pod}}}}
        }
        _, _, script = oracle.pod_script(cronjob, {"postgres-backup": configmap})
        assert "pg_dump" in script
        assert "restic restore" in script
        assert "sha256sum -c" in script
        assert "not invoked" not in script
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
