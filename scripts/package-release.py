#!/usr/bin/env python3
"""Build deterministic release archives and verify their binary identity."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run(argv: list[str], cwd: Path | None = None) -> str:
    result = subprocess.run(argv, cwd=cwd, capture_output=True, text=True, check=False)
    if result.returncode:
        raise ValueError(f"command failed ({result.returncode}): {' '.join(argv)}\n{result.stderr}")
    return result.stdout.strip()


def package_version() -> str:
    for line in (ROOT / "Cargo.toml").read_text(encoding="utf-8").splitlines():
        if line.startswith("version = "):
            return line.split('"', 2)[1]
    raise ValueError("Cargo.toml package version is missing")


def doctor(binary: Path) -> dict[str, Any]:
    return json.loads(run([str(binary), "doctor", "--format", "json"]))


def verify_binary(binary: Path, version: str, source_commit: str | None = None) -> dict[str, Any]:
    observed = run([str(binary), "--version"])
    if observed != f"codemap {version}":
        raise ValueError(f"binary version mismatch: {observed!r}")
    report = doctor(binary)
    identity = report["build_identity"]
    if identity["semver"] != version or identity["dirty_build"] is not False:
        raise ValueError("release binary identity is not clean or version-aligned")
    if source_commit and identity["source_commit"] != source_commit:
        raise ValueError("release binary source commit differs from the tag commit")
    if identity["binary_sha256"] != sha256(binary):
        raise ValueError("doctor binary hash differs from archive input")
    return identity


def tar_bytes(files: list[tuple[Path, str, int]]) -> bytes:
    stream = io.BytesIO()
    with tarfile.open(fileobj=stream, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for source, name, mode in files:
            body = source.read_bytes()
            info = tarfile.TarInfo(name)
            info.size = len(body)
            info.mode = mode
            info.mtime = 0
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            archive.addfile(info, io.BytesIO(body))
    return gzip.compress(stream.getvalue(), compresslevel=9, mtime=0)


def build_archive(binary: Path, target: str, version: str, out_dir: Path) -> Path:
    commit = os.environ.get("CODEMAP_SOURCE_COMMIT") or run(["git", "rev-parse", "HEAD"], ROOT)
    identity = verify_binary(binary, version, commit)
    stem = f"codemap-v{version}-{target}"
    executable = "codemap.exe" if binary.suffix == ".exe" else "codemap"
    archive = out_dir / f"{stem}.tar.gz"
    out_dir.mkdir(parents=True, exist_ok=True)
    archive.write_bytes(
        tar_bytes(
            [
                (binary, f"{stem}/{executable}", 0o755),
                (ROOT / "LICENSE", f"{stem}/LICENSE", 0o644),
                (ROOT / "README.md", f"{stem}/README.md", 0o644),
            ]
        )
    )
    sidecar = archive.with_suffix(archive.suffix + ".sha256")
    sidecar.write_text(f"{sha256(archive)}  {archive.name}\n", encoding="utf-8")
    receipt = {
        "kind": "codemap_release_artifact",
        "version": version,
        "target": target,
        "source_commit": commit,
        "binary": {"name": executable, "sha256": sha256(binary), "identity": identity},
        "archive": {"name": archive.name, "sha256": sha256(archive)},
    }
    (out_dir / f"{stem}.receipt.json").write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return archive


def safe_extract(archive: Path, destination: Path) -> Path:
    with tarfile.open(archive, "r:gz") as bundle:
        members = bundle.getmembers()
        for member in members:
            path = Path(member.name)
            if path.is_absolute() or ".." in path.parts or not member.isfile():
                raise ValueError(f"unsafe release archive member: {member.name}")
        bundle.extractall(destination, filter="data")
    binaries = [path for path in destination.rglob("codemap*") if path.name in {"codemap", "codemap.exe"}]
    if len(binaries) != 1:
        raise ValueError("release archive must contain exactly one codemap binary")
    binaries[0].chmod(0o755)
    return binaries[0]


def verify_archive(archive: Path, checksum: Path, version: str) -> None:
    expected, name = checksum.read_text(encoding="utf-8").split()
    if name != archive.name or expected != sha256(archive):
        raise ValueError("release archive checksum mismatch")
    with tempfile.TemporaryDirectory(prefix="codemap-release-smoke-") as temporary:
        binary = safe_extract(archive, Path(temporary))
        verify_binary(binary, version)


def verify_source(tag: str) -> None:
    version = package_version()
    if tag != f"v{version}":
        raise ValueError(f"tag {tag!r} does not match Cargo version v{version}")
    head = run(["git", "rev-parse", "HEAD"], ROOT)
    tagged = run(["git", "rev-list", "-n", "1", tag], ROOT)
    if head != tagged:
        raise ValueError("checked-out commit differs from the release tag")


def write_index(directory: Path) -> Path:
    output = directory / "SHA256SUMS"
    files = sorted(path for path in directory.iterdir() if path.is_file() and path != output)
    output.write_text("".join(f"{sha256(path)}  {path.name}\n" for path in files), encoding="utf-8")
    return output


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    commands = root.add_subparsers(dest="command", required=True)
    build = commands.add_parser("build")
    build.add_argument("--binary", required=True)
    build.add_argument("--target", required=True)
    build.add_argument("--version", required=True)
    build.add_argument("--out-dir", required=True)
    verify = commands.add_parser("verify-archive")
    verify.add_argument("--archive", required=True)
    verify.add_argument("--checksum", required=True)
    verify.add_argument("--version", required=True)
    source = commands.add_parser("verify-source")
    source.add_argument("--tag", required=True)
    index = commands.add_parser("index")
    index.add_argument("--dir", required=True)
    return root


def main() -> int:
    args = parser().parse_args()
    try:
        if args.command == "build":
            print(build_archive(Path(args.binary), args.target, args.version, Path(args.out_dir)))
        elif args.command == "verify-archive":
            verify_archive(Path(args.archive), Path(args.checksum), args.version)
        elif args.command == "verify-source":
            verify_source(args.tag)
        else:
            print(write_index(Path(args.dir)))
        return 0
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"codemap release package: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
