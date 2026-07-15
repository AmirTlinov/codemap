"""Attribute benchmark codemap calls to agent commands or internal consumers."""

from __future__ import annotations

import ctypes
import json
import os
import platform
import shlex
import subprocess
import sys
from pathlib import Path


SHELL_EXECUTORS = {"bash", "dash", "env", "fish", "sh", "zsh"}
AGENT_EXECUTORS = {"codex", "codex-code-mode-host"}


class ProcBsdInfo(ctypes.Structure):
    _fields_ = [
        ("pbi_flags", ctypes.c_uint32),
        ("pbi_status", ctypes.c_uint32),
        ("pbi_xstatus", ctypes.c_uint32),
        ("pbi_pid", ctypes.c_uint32),
        ("pbi_ppid", ctypes.c_uint32),
        ("pbi_uid", ctypes.c_uint32),
        ("pbi_gid", ctypes.c_uint32),
        ("pbi_ruid", ctypes.c_uint32),
        ("pbi_rgid", ctypes.c_uint32),
        ("pbi_svuid", ctypes.c_uint32),
        ("pbi_svgid", ctypes.c_uint32),
        ("rfu_1", ctypes.c_uint32),
        ("pbi_comm", ctypes.c_char * 16),
        ("pbi_name", ctypes.c_char * 32),
        ("pbi_nfiles", ctypes.c_uint32),
        ("pbi_pgid", ctypes.c_uint32),
        ("pbi_pjobc", ctypes.c_uint32),
        ("e_tdev", ctypes.c_uint32),
        ("e_tpgid", ctypes.c_uint32),
        ("pbi_nice", ctypes.c_int32),
        ("pbi_start_tvsec", ctypes.c_uint64),
        ("pbi_start_tvusec", ctypes.c_uint64),
    ]


def _mac_process(pid: int) -> tuple[int, str] | None:
    libproc = ctypes.CDLL("/usr/lib/libproc.dylib")
    info = ProcBsdInfo()
    size = libproc.proc_pidinfo(
        pid, 3, 0, ctypes.byref(info), ctypes.sizeof(info)
    )
    if size != ctypes.sizeof(info):
        return None
    buffer = ctypes.create_string_buffer(4096)
    path_size = libproc.proc_pidpath(pid, buffer, len(buffer))
    path = buffer.value.decode(errors="replace") if path_size > 0 else ""
    name = bytes(info.pbi_name).split(b"\0", 1)[0].decode(errors="replace")
    return int(info.pbi_ppid), path or name


def _linux_process(pid: int) -> tuple[int, str] | None:
    stat = Path(f"/proc/{pid}/stat").read_text(encoding="utf-8")
    fields = stat.rsplit(") ", 1)[1].split()
    parent = int(fields[1])
    try:
        path = os.readlink(f"/proc/{pid}/exe")
    except OSError:
        path = stat.split("(", 1)[1].rsplit(")", 1)[0]
    return parent, path


def _process(pid: int) -> tuple[int, str] | None:
    try:
        if platform.system() == "Darwin":
            return _mac_process(pid)
        if platform.system() == "Linux":
            return _linux_process(pid)
    except (OSError, ValueError):
        return None
    return None


def ancestor_processes() -> list[str]:
    processes = []
    pid = os.getppid()
    for _ in range(24):
        row = _process(pid)
        if row is None:
            break
        parent, executable = row
        processes.append(executable)
        if parent <= 1 or parent == pid:
            break
        pid = parent
    return processes


def is_agent_direct(ancestors: list[str]) -> bool:
    for executable in ancestors:
        name = Path(executable).name
        if name in AGENT_EXECUTORS:
            return True
        if name not in SHELL_EXECUTORS:
            return False
    return False


def _candidate_from_process(executable: str) -> Path | None:
    for profile in ("debug", "release"):
        marker = f"/{profile}/deps/"
        if marker in executable:
            candidate = Path(executable.split(marker, 1)[0]) / profile / "codemap"
            if candidate.is_file():
                return candidate
    return None


def internal_codemap(ancestors: list[str], shim: Path) -> list[str] | None:
    candidates = [os.environ.get("CARGO_BIN_EXE_codemap")]
    target = os.environ.get("CARGO_TARGET_DIR")
    if target:
        candidates.extend(
            str(Path(target) / profile / "codemap") for profile in ("debug", "release")
        )
    worktree = os.environ.get("CODEMAP_AB_WORKTREE")
    if worktree:
        candidates.extend(
            str(Path(worktree) / "target" / profile / "codemap")
            for profile in ("debug", "release")
        )
    for raw in candidates:
        if raw and Path(raw).is_file() and Path(raw).resolve() != shim.resolve():
            return [str(Path(raw).resolve())]
    for executable in ancestors:
        candidate = _candidate_from_process(executable)
        if candidate is not None and candidate.resolve() != shim.resolve():
            return [str(candidate.resolve())]
    return None


def append_record(record: dict) -> None:
    with open(os.environ["CODEMAP_AB_INVOCATION_LOG"], "a", encoding="utf-8") as stream:
        stream.write(json.dumps(record, separators=(",", ":")) + "\n")


def main(arm: str, benchmark_command: list[str]) -> int:
    ancestors = ancestor_processes()
    direct = is_agent_direct(ancestors)
    source = "blocked"
    command = benchmark_command if direct and arm != "control" else None
    if not direct:
        command = internal_codemap(ancestors, Path(sys.argv[0]))
        source = "project" if command else "unavailable_internal"
    elif command:
        source = "benchmark"
    if command is None:
        message = (
            "codemap is unavailable in the control arm"
            if direct
            else "project codemap is unavailable"
        )
        print(message, file=sys.stderr)
        status = 127
    else:
        try:
            status = subprocess.run([*command, *sys.argv[1:]], check=False).returncode
        except OSError as exc:
            print(f"codemap launch failed: {exc}", file=sys.stderr)
            status = 127
    append_record(
        {
            "argv": sys.argv[1:],
            "status": status,
            "agent_direct": direct,
            "execution_source": source,
            "ancestor_executables": ancestors[:8],
        }
    )
    return status


def write_shim(shim_dir: Path, arm: str, benchmark_command: list[str]) -> Path:
    shim_dir.mkdir(parents=True, exist_ok=True)
    shim = shim_dir / "codemap"
    script_dir = str(Path(__file__).resolve().parent)
    body = f"""#!/usr/bin/env python3
import sys
sys.path.insert(0, {script_dir!r})
from codemap_protocol_shim import main
raise SystemExit(main({arm!r}, {benchmark_command!r}))
"""
    shim.write_text(body, encoding="utf-8")
    shim.chmod(0o755)
    return shim


def shell_profile_environment(shim_dir: Path) -> dict[str, str]:
    """Keep the benchmark shim first even when an agent starts a login shell."""
    profile_dir = shim_dir.parent / "shell-profile"
    profile_dir.mkdir(parents=True, exist_ok=True)
    export = f"export PATH={shlex.quote(str(shim_dir))}:\"$PATH\"\n"
    for name in (".zshenv", ".zprofile", "bash_env"):
        (profile_dir / name).write_text(export, encoding="utf-8")
    return {
        "ZDOTDIR": str(profile_dir),
        "BASH_ENV": str(profile_dir / "bash_env"),
        "ENV": str(profile_dir / "bash_env"),
    }
