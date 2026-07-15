#!/usr/bin/env python3
"""Black-box proof that release archives are deterministic and self-verifying."""

from __future__ import annotations

import hashlib
import os
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


with tempfile.TemporaryDirectory(prefix="codemap-release-package-") as temporary:
    root = Path(temporary)
    binary = root / "codemap"
    binary.write_text(
        """#!/usr/bin/env python3
import hashlib, json, pathlib, sys
path = pathlib.Path(sys.argv[0])
if '--version' in sys.argv:
    print('codemap 9.8.7')
elif 'doctor' in sys.argv:
    print(json.dumps({'build_identity': {
        'semver': '9.8.7', 'dirty_build': False, 'source_commit': 'fixture-commit',
        'binary_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
    }}))
else:
    raise SystemExit(2)
""",
        encoding="utf-8",
    )
    binary.chmod(0o755)
    env = {**os.environ, "CODEMAP_SOURCE_COMMIT": "fixture-commit"}
    archives = []
    for name in ("first", "second"):
        out = root / name
        result = subprocess.run(
            [
                sys.executable,
                str(ROOT / "scripts/package-release.py"),
                "build",
                "--binary",
                str(binary),
                "--target",
                "fixture-target",
                "--version",
                "9.8.7",
                "--out-dir",
                str(out),
            ],
            env=env,
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0, result.stderr
        archive = out / "codemap-v9.8.7-fixture-target.tar.gz"
        verify = subprocess.run(
            [
                sys.executable,
                str(ROOT / "scripts/package-release.py"),
                "verify-archive",
                "--archive",
                str(archive),
                "--checksum",
                str(archive) + ".sha256",
                "--version",
                "9.8.7",
            ],
            capture_output=True,
            text=True,
        )
        assert verify.returncode == 0, verify.stderr
        archives.append(hashlib.sha256(archive.read_bytes()).hexdigest())
    assert archives[0] == archives[1]
