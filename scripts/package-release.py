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


def write_tar_gz(
    output: Path, files: list[tuple[Path, str]], generated: list[tuple[bytes, str]]
) -> None:
    with output.open("wb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as compressed:
        with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
            for source, name in files:
                info = tarfile.TarInfo(name)
                info.size = source.stat().st_size
                info.mode = 0o644
                info.mtime = 0
                info.uid = info.gid = 0
                info.uname = info.gname = ""
                with source.open("rb") as stream:
                    archive.addfile(info, stream)
            for body, name in generated:
                info = tarfile.TarInfo(name)
                info.size = len(body)
                info.mode = 0o644
                info.mtime = 0
                info.uid = info.gid = 0
                info.uname = info.gname = ""
                archive.addfile(info, io.BytesIO(body))


def load_acceptance(path: Path) -> tuple[dict[str, Any], list[Path]]:
    report = json.loads(path.read_text(encoding="utf-8"))
    if report.get("kind") != "codemap_flagship_acceptance" or report.get("version") != 1:
        raise ValueError(f"unsupported flagship acceptance receipt: {path}")
    files = [Path(row["path"]) for row in report.get("evidence", [])]
    files.extend(candidate for candidate in (path, path.with_name("acceptance.md")) if candidate.is_file())
    for source in files:
        if not source.is_file():
            raise ValueError(f"flagship evidence file is missing: {source}")
    expected = {row["path"]: row["sha256"] for row in report.get("evidence", [])}
    for source in files:
        if str(source) in expected and sha256(source) != expected[str(source)]:
            raise ValueError(f"flagship evidence hash mismatch: {source}")
    return report, sorted(set(files), key=lambda item: str(item))


def evidence_readme(version: str, attempts: list[dict[str, Any]]) -> bytes:
    rows = [f"# codemap flagship evidence v{version}", ""]
    for attempt in attempts:
        resources = attempt["resources"]
        state = "PASSED" if attempt["accepted"] else "FAILED"
        rows.extend(
            [
                f"## {attempt['label']}: {state}",
                "",
                f"- Complex wins: {attempt['complex_wins']}/12; losses: {attempt['complex_losses']}.",
                f"- Complex overhead: time {resources['complex_median_time_overhead']:.1%}; "
                f"input {resources['complex_median_input_overhead']:.1%}.",
                f"- Exact overhead: time {resources['exact_median_time_overhead']:.1%}; "
                f"input {resources['exact_median_input_overhead']:.1%}.",
                "",
            ]
        )
    rows.append("The claim is limited to the frozen six-repository corpus.")
    return ("\n".join(rows) + "\n").encode()


def build_evidence(version: str, receipts: list[Path], out_dir: Path) -> Path:
    stem = f"flagship-evidence-v{version}"
    files: list[tuple[Path, str]] = []
    inventory = []
    attempts = []
    for index, receipt in enumerate(receipts, 1):
        report, sources = load_acceptance(receipt.resolve())
        manifest = json.loads(Path(report["manifest"]).read_text(encoding="utf-8"))
        identity = manifest["codemap_identity"]["build_identity"]
        accepted = report["acceptance"]["accepted"] is True
        label = f"{index:02d}-{'accepted' if accepted else 'failed'}-{identity['source_commit'][:7]}"
        common = Path(os.path.commonpath([str(source.parent) for source in sources]))
        attempt = {
            "label": label,
            "accepted": accepted,
            "acceptance_sha256": sha256(receipt),
            "manifest_sha256": report["manifest_sha256"],
            "source_commit": identity["source_commit"],
            "binary_sha256": identity["binary_sha256"],
            "complex_wins": report["acceptance"]["complex"]["wins"],
            "complex_losses": len(report["acceptance"]["complex"]["losing_tasks"]),
            "resources": report["acceptance"]["resources"],
        }
        attempts.append(attempt)
        for source in sources:
            relative = source.relative_to(common).as_posix()
            member = f"attempts/{label}/{relative}"
            files.append((source, f"{stem}/{member}"))
            inventory.append({"path": member, "sha256": sha256(source)})
    if sum(attempt["accepted"] for attempt in attempts) != 1:
        raise ValueError("flagship bundle requires exactly one accepted attempt")
    manifest = {
        "kind": "codemap_flagship_evidence_bundle",
        "version": 1,
        "release_version": version,
        "attempts": attempts,
        "files": sorted(inventory, key=lambda row: row["path"]),
    }
    out_dir.mkdir(parents=True, exist_ok=True)
    output = out_dir / f"{stem}.tar.gz"
    generated = [
        ((json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode(), f"{stem}/bundle.json"),
        (evidence_readme(version, attempts), f"{stem}/README.md"),
    ]
    write_tar_gz(output, sorted(files, key=lambda row: row[1]), generated)
    output.with_suffix(output.suffix + ".sha256").write_text(
        f"{sha256(output)}  {output.name}\n", encoding="utf-8"
    )
    return output


def verify_evidence(archive: Path, checksum: Path, version: str) -> None:
    expected_hash, expected_name = checksum.read_text(encoding="utf-8").split()
    if expected_name != archive.name or expected_hash != sha256(archive):
        raise ValueError("flagship evidence checksum mismatch")
    stem = f"flagship-evidence-v{version}"
    with tarfile.open(archive, "r:gz") as bundle:
        members = bundle.getmembers()
        names = [member.name for member in members]
        if len(names) != len(set(names)) or any(
            Path(name).is_absolute() or ".." in Path(name).parts for name in names
        ):
            raise ValueError("unsafe or duplicate flagship evidence member")
        manifest_file = bundle.extractfile(f"{stem}/bundle.json")
        if manifest_file is None:
            raise ValueError("flagship evidence manifest is missing")
        manifest = json.load(manifest_file)
        if manifest.get("kind") != "codemap_flagship_evidence_bundle" or manifest.get("version") != 1:
            raise ValueError("unsupported flagship evidence manifest")
        if manifest.get("release_version") != version:
            raise ValueError("flagship evidence version mismatch")
        if sum(row.get("accepted") is True for row in manifest.get("attempts", [])) != 1:
            raise ValueError("flagship evidence must contain exactly one accepted attempt")
        for row in manifest.get("files", []):
            stream = bundle.extractfile(f"{stem}/{row['path']}")
            if stream is None or hashlib.sha256(stream.read()).hexdigest() != row["sha256"]:
                raise ValueError(f"flagship evidence member hash mismatch: {row['path']}")


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
    evidence = commands.add_parser("build-evidence")
    evidence.add_argument("--version", required=True)
    evidence.add_argument("--acceptance", action="append", required=True)
    evidence.add_argument("--out-dir", required=True)
    verify_evidence_parser = commands.add_parser("verify-evidence")
    verify_evidence_parser.add_argument("--archive", required=True)
    verify_evidence_parser.add_argument("--checksum", required=True)
    verify_evidence_parser.add_argument("--version", required=True)
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
        elif args.command == "index":
            print(write_index(Path(args.dir)))
        elif args.command == "build-evidence":
            print(build_evidence(args.version, [Path(path) for path in args.acceptance], Path(args.out_dir)))
        else:
            verify_evidence(Path(args.archive), Path(args.checksum), args.version)
        return 0
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"codemap release package: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
