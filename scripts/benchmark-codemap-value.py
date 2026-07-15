#!/usr/bin/env python3
"""Measure codemap's deterministic context-compression value.

This is a read-only benchmark for the target repositories. It does not claim
that an LLM became smarter. It shows how many approximate tokens codemap's
daily map uses compared with a visible text context baseline, and how much
agent-navigation signal that map carries.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from codemap_identity import CodemapIdentityError, benchmark_binary_identity, resolve_codemap_command


CLAIM_BOUNDARY = (
    "This benchmark proves deterministic context compression and navigation "
    "signal density. It does not prove behavioral model lift; use a paired "
    "model A/B task benchmark for that."
)
TEXT_EXTS = {
    ".c",
    ".cc",
    ".cfg",
    ".cpp",
    ".cs",
    ".css",
    ".go",
    ".graphql",
    ".h",
    ".hpp",
    ".html",
    ".java",
    ".js",
    ".json",
    ".jsx",
    ".kt",
    ".lua",
    ".mjs",
    ".md",
    ".php",
    ".prisma",
    ".proto",
    ".py",
    ".rb",
    ".rs",
    ".scss",
    ".sh",
    ".sql",
    ".svelte",
    ".swift",
    ".toml",
    ".ts",
    ".tsx",
    ".txt",
    ".vue",
    ".yaml",
    ".yml",
}
TEXT_NAMES = {
    ".env.example",
    ".env.sample",
    ".gitignore",
    "AGENTS.md",
    "Cargo.lock",
    "Cargo.toml",
    "Dockerfile",
    "Gemfile",
    "Gemfile.lock",
    "Makefile",
    "Package.resolved",
    "Package.swift",
    "README.md",
    "bun.lockb",
    "go.mod",
    "go.sum",
    "justfile",
    "package-lock.json",
    "package.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "poetry.lock",
    "pyproject.toml",
    "uv.lock",
    "yarn.lock",
}
SOURCE_EXTS = {
    ".c",
    ".cc",
    ".cpp",
    ".cs",
    ".go",
    ".h",
    ".hpp",
    ".java",
    ".js",
    ".jsx",
    ".kt",
    ".lua",
    ".mjs",
    ".php",
    ".py",
    ".rb",
    ".rs",
    ".svelte",
    ".swift",
    ".ts",
    ".tsx",
    ".vue",
}

IGNORE_DIRS = {
    ".cache",
    ".git",
    ".next",
    ".turbo",
    ".venv",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "vendor",
}

PATH_RE = re.compile(
    r"(?<![A-Za-z0-9_@./-])"
    r"([A-Za-z0-9_@./-]+"
    r"\.(?:c|cc|cfg|cpp|cs|css|go|graphql|h|hpp|html|java|js|json|jsx|kt|lua|md|mjs|php|prisma|proto|py|rb|rs|scss|sh|sql|svelte|swift|toml|ts|tsx|txt|vue|ya?ml))"
)


@dataclass
class CommandResult:
    label: str
    args: list[str]
    status: int
    elapsed_ms: int
    stdout: str
    stderr: str

    @property
    def bytes(self) -> int:
        return len(self.stdout.encode("utf-8"))


def repo_script_root() -> Path:
    return Path(__file__).resolve().parents[1]


def canonical(path: Path) -> Path:
    return path.expanduser().resolve()


def safe_label(text: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", text).strip("_") or "repo"


def approx_tokens(byte_count: int, chars_per_token: float) -> int:
    if byte_count <= 0:
        return 0
    return max(1, round(byte_count / chars_per_token))


def run(args: list[str], cwd: Path, timeout_s: int = 30) -> CommandResult:
    started = time.monotonic_ns()
    try:
        output = subprocess.run(
            args,
            cwd=cwd,
            text=True,
            encoding="utf-8",
            errors="replace",
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout_s,
            check=False,
        )
        status = output.returncode
        stdout = output.stdout
        stderr = output.stderr
    except subprocess.TimeoutExpired as exc:
        status = 124
        stdout = exc.stdout or ""
        stderr = (exc.stderr or "") + f"\ntimeout after {timeout_s}s"
    elapsed_ms = (time.monotonic_ns() - started) // 1_000_000
    return CommandResult(
        label=" ".join(args),
        args=args,
        status=status,
        elapsed_ms=int(elapsed_ms),
        stdout=stdout,
        stderr=stderr,
    )


def git_files(root: Path) -> list[str] | None:
    output = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-c", "-o", "--exclude-standard", "-z"],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if output.returncode != 0:
        return None
    return [
        item.decode("utf-8", errors="replace").replace("\\", "/")
        for item in output.stdout.split(b"\0")
        if item
    ]


def walked_files(root: Path) -> list[str]:
    files: list[str] = []
    for base, dirs, names in os.walk(root):
        dirs[:] = sorted(name for name in dirs if name not in IGNORE_DIRS)
        for name in sorted(names):
            path = Path(base) / name
            rel = path.relative_to(root).as_posix()
            files.append(rel)
    return files


def visible_files(root: Path) -> list[str]:
    files = git_files(root)
    if files is None:
        files = walked_files(root)
    return sorted(
        rel
        for rel in files
        if rel
        and not any(part in IGNORE_DIRS for part in rel.split("/"))
        and (root / rel).is_file()
    )


def is_text_baseline_file(path: Path) -> bool:
    return path.name in TEXT_NAMES or path.suffix.lower() in TEXT_EXTS


def is_source_file(path: Path) -> bool:
    return path.suffix.lower() in SOURCE_EXTS


def baseline_context(root: Path, chars_per_token: float, max_file_bytes: int) -> dict:
    files = visible_files(root)
    included: list[dict] = []
    skipped_large = 0
    skipped_binary_or_unknown = 0
    total_bytes = 0
    for rel in files:
        path = root / rel
        if not is_text_baseline_file(path):
            skipped_binary_or_unknown += 1
            continue
        try:
            size = path.stat().st_size
        except OSError:
            continue
        if size > max_file_bytes:
            skipped_large += 1
            continue
        total_bytes += size
        included.append({"path": rel, "bytes": size})
    return {
        "kind": "visible_text_context",
        "file_count": len(included),
        "bytes": total_bytes,
        "approx_tokens": approx_tokens(total_bytes, chars_per_token),
        "skipped_large_files": skipped_large,
        "skipped_binary_or_unknown_files": skipped_binary_or_unknown,
    }


def first_source_anchor(root: Path) -> str | None:
    for rel in visible_files(root):
        if is_source_file(root / rel):
            return rel
    return None


def command_spec(root: Path) -> list[tuple[str, list[str]]]:
    commands = [
        ("ls_root", ["ls", "."]),
        ("changed", ["changed"]),
        ("proof_changed", ["proof", "changed"]),
    ]
    anchor = first_source_anchor(root)
    if anchor:
        commands.append(("cone_anchor", ["cone", anchor, "--depth", "1"]))
    return commands


def run_codemap(
    root: Path,
    codemap_cmd: list[str],
    out_repo_dir: Path,
    chars_per_token: float,
) -> dict:
    command_rows = []
    combined = []
    for label, args in command_spec(root):
        result = run([*codemap_cmd, "--root", str(root), *args], cwd=root, timeout_s=45)
        output_path = out_repo_dir / f"{label}.md"
        output_path.write_text(result.stdout, encoding="utf-8")
        if result.stderr.strip():
            (out_repo_dir / f"{label}.stderr.log").write_text(result.stderr, encoding="utf-8")
        command_rows.append(
            {
                "label": label,
                "args": args,
                "status": result.status,
                "elapsed_ms": result.elapsed_ms,
                "bytes": result.bytes,
                "approx_tokens": approx_tokens(result.bytes, chars_per_token),
                "artifact": str(output_path),
            }
        )
        combined.append(result.stdout)
    text = "\n".join(combined)
    tokens = approx_tokens(len(text.encode("utf-8")), chars_per_token)
    paths = sorted(set(PATH_RE.findall(text)))
    expand_count = len(re.findall(r"(?m)^\s*(?:expand:\s*)?codemap\s+", text))
    unknown_count = len(re.findall(r"\bUnknown\b|\bunknown\b", text))
    proof_count = len(re.findall(r"\bProof\b|\bproof\b", text))
    elapsed_ms = sum(row["elapsed_ms"] for row in command_rows)
    return {
        "commands": command_rows,
        "bytes": len(text.encode("utf-8")),
        "approx_tokens": tokens,
        "elapsed_ms": elapsed_ms,
        "unique_path_mentions": len(paths),
        "expand_commands": expand_count,
        "unknown_mentions": unknown_count,
        "proof_mentions": proof_count,
        "path_signal_per_1k_tokens": round((len(paths) * 1000) / max(tokens, 1), 2),
        "expand_per_1k_tokens": round((expand_count * 1000) / max(tokens, 1), 2),
        "all_commands_succeeded": all(row["status"] == 0 for row in command_rows),
    }


def ratio(numerator: int, denominator: int) -> float | None:
    if denominator <= 0:
        return None
    return round(numerator / denominator, 2)


def savings_percent(baseline_tokens: int, codemap_tokens: int) -> float | None:
    if baseline_tokens <= 0:
        return None
    return round(max(0.0, 1.0 - (codemap_tokens / baseline_tokens)) * 100, 1)


def repo_row(
    root: Path,
    codemap_cmd: list[str],
    codemap_identity: dict,
    out_dir: Path,
    chars_per_token: float,
    max_file_bytes: int,
) -> dict:
    label = safe_label(root.name)
    out_repo_dir = out_dir / label
    out_repo_dir.mkdir(parents=True, exist_ok=True)
    baseline = baseline_context(root, chars_per_token, max_file_bytes)
    codemap = run_codemap(root, codemap_cmd, out_repo_dir, chars_per_token)
    succeeded = codemap["all_commands_succeeded"]
    compression = (
        ratio(baseline["approx_tokens"], codemap["approx_tokens"]) if succeeded else None
    )
    token_savings = (
        savings_percent(baseline["approx_tokens"], codemap["approx_tokens"])
        if succeeded
        else None
    )
    total_navigation_signals = (
        codemap["unique_path_mentions"]
        + codemap["expand_commands"]
        + codemap["unknown_mentions"]
        + codemap["proof_mentions"]
    )
    return {
        "repo": str(root),
        "label": label,
        "report_prelude": {"codemap": codemap_identity},
        "claim_boundary": CLAIM_BOUNDARY,
        "baseline": baseline,
        "codemap_daily_map": codemap,
        "result": {
            "status": "ok" if succeeded else "failed",
            "compression_ratio_vs_visible_text": compression,
            "token_savings_percent_vs_visible_text": token_savings,
            "navigation_signal_density": {
                "unique_path_mentions": codemap["unique_path_mentions"],
                "path_signal_per_1k_tokens": codemap["path_signal_per_1k_tokens"],
                "expand_commands": codemap["expand_commands"],
                "expand_per_1k_tokens": codemap["expand_per_1k_tokens"],
                "unknown_mentions": codemap["unknown_mentions"],
                "proof_mentions": codemap["proof_mentions"],
                "total_navigation_signals": total_navigation_signals,
                "navigation_signals_per_1k_tokens": round(
                    (total_navigation_signals * 1000)
                    / max(codemap["approx_tokens"], 1),
                    2,
                ),
            },
        },
    }


def format_int(value: int | None) -> str:
    if value is None:
        return "-"
    return f"{value:,}"


def format_float(value: float | None, suffix: str = "") -> str:
    if value is None:
        return "-"
    return f"{value}{suffix}"


def write_summary(out_dir: Path, rows: list[dict]) -> None:
    jsonl = out_dir / "summary.jsonl"
    jsonl.write_text(
        "\n".join(json.dumps(row, sort_keys=True) for row in rows) + "\n",
        encoding="utf-8",
    )
    lines = [
        "# codemap value benchmark",
        "",
        CLAIM_BOUNDARY,
        "",
        f"codemap identity: `{rows[0]['report_prelude']['codemap']['build_identity']['semver']}` "
        f"at `{rows[0]['report_prelude']['codemap']['build_identity']['executable_path']}` "
        f"sha256 `{rows[0]['report_prelude']['codemap']['build_identity']['binary_sha256']}`",
        "",
        "| Repo | Status | Baseline tokens | codemap tokens | Saved | Compression | Paths | Expands | Unknown | Proof | Time |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for row in rows:
        baseline_tokens = row["baseline"]["approx_tokens"]
        codemap_tokens = row["codemap_daily_map"]["approx_tokens"]
        result = row["result"]
        signal = result["navigation_signal_density"]
        lines.append(
            "| {label} | {status} | {baseline} | {codemap} | {saved} | {compression} | {paths} | {expands} | {unknown} | {proof} | {elapsed}ms |".format(
                label=row["label"],
                status=result["status"],
                baseline=format_int(baseline_tokens),
                codemap=format_int(codemap_tokens),
                saved=format_float(result["token_savings_percent_vs_visible_text"], "%"),
                compression=format_float(result["compression_ratio_vs_visible_text"], "x"),
                paths=format_int(signal["unique_path_mentions"]),
                expands=format_int(signal["expand_commands"]),
                unknown=format_int(signal["unknown_mentions"]),
                proof=format_int(signal["proof_mentions"]),
                elapsed=format_int(row["codemap_daily_map"]["elapsed_ms"]),
            )
        )
    lines.extend(
        [
            "",
            "| Repo | Navigation signals | Signals / 1k map tokens |",
            "| --- | ---: | ---: |",
        ]
    )
    for row in rows:
        signal = row["result"]["navigation_signal_density"]
        lines.append(
            "| {label} | {signals} | {density} |".format(
                label=row["label"],
                signals=format_int(signal["total_navigation_signals"]),
                density=format_float(signal["navigation_signals_per_1k_tokens"]),
            )
        )
    lines.extend(
        [
            "",
            "## How to read this",
            "",
            "- Baseline tokens are the approximate tokens in visible tracked/unignored text files.",
            "- codemap tokens are `ls .`, `changed`, `proof changed`, and one `cone <source> --depth 1` map.",
            "- Saved/compression shows context compression, not model intelligence.",
            "- Paths, Expands, Unknown, and Proof are navigation signals in the compact map.",
            "",
            "Artifacts: per-command markdown outputs are stored beside this summary.",
        ]
    )
    (out_dir / "summary.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Benchmark codemap deterministic context compression."
    )
    parser.add_argument("repos", nargs="+", help="Repository roots to benchmark.")
    parser.add_argument(
        "--codemap-bin",
        help="Explicit executable or quoted Python/POSIX-shell wrapper (then CODEMAP_BIN, local target, PATH).",
    )
    parser.add_argument(
        "--out-dir",
        default=str(repo_script_root() / "target" / "codemap-value-benchmark"),
        help="Output directory for summaries and captured codemap maps.",
    )
    parser.add_argument(
        "--chars-per-token",
        type=float,
        default=4.0,
        help="Approximate bytes/chars per token estimate.",
    )
    parser.add_argument(
        "--max-file-bytes",
        type=int,
        default=900_000,
        help="Skip larger files in the visible text baseline.",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        codemap_cmd, resolution = resolve_codemap_command(args.codemap_bin, repo_script_root())
        identity_root = canonical(Path(args.repos[0]))
        codemap_identity = benchmark_binary_identity(codemap_cmd, resolution, identity_root)
    except (OSError, CodemapIdentityError) as exc:
        print(f"codemap benchmark: {exc}", file=sys.stderr)
        return 2
    out_dir = canonical(Path(args.out_dir))
    out_dir.mkdir(parents=True, exist_ok=True)
    rows = []
    for repo_arg in args.repos:
        root = canonical(Path(repo_arg))
        if not root.is_dir():
            print(f"missing repo: {root}", file=sys.stderr)
            return 2
        print(f"[benchmark] repo={root}", file=sys.stderr)
        rows.append(
            repo_row(
                root=root,
                codemap_cmd=codemap_cmd,
                codemap_identity=codemap_identity,
                out_dir=out_dir,
                chars_per_token=args.chars_per_token,
                max_file_bytes=args.max_file_bytes,
            )
        )
    write_summary(out_dir, rows)
    print(f"benchmark summary: {out_dir / 'summary.md'}")
    print(f"benchmark jsonl: {out_dir / 'summary.jsonl'}")
    failed = [
        row["label"]
        for row in rows
        if not row["codemap_daily_map"]["all_commands_succeeded"]
    ]
    if failed:
        print(
            "benchmark failed: codemap probes failed for " + ", ".join(failed),
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
