"""Deterministic bounded concurrency for independent benchmark pairs."""

from __future__ import annotations

import atexit
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
import os
from pathlib import Path
import signal
import subprocess
import threading
import time
from typing import Any, Callable, TypeVar


Job = TypeVar("Job")
Result = TypeVar("Result")
_ACTIVE_PROCESSES: dict[int, subprocess.Popen[str]] = {}
_ACTIVE_LOCK = threading.Lock()


@dataclass
class ProcessResult:
    status: int
    elapsed_ms: int
    stdout: str
    stderr: str
    timed_out: bool


def run_process(
    args: list[str],
    cwd: Path,
    timeout_seconds: int,
    env: dict[str, str] | None = None,
) -> ProcessResult:
    started = time.monotonic_ns()
    options: dict[str, Any] = (
        {"creationflags": subprocess.CREATE_NEW_PROCESS_GROUP}
        if os.name == "nt"
        else {"start_new_session": True}
    )
    process = subprocess.Popen(
        args,
        cwd=cwd,
        env=env,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        **options,
    )
    with _ACTIVE_LOCK:
        _ACTIVE_PROCESSES[process.pid] = process
    timed_out = False
    try:
        try:
            stdout, stderr = process.communicate(timeout=timeout_seconds)
        except subprocess.TimeoutExpired:
            timed_out = True
            stdout, stderr = terminate_process_tree(process)
    finally:
        with _ACTIVE_LOCK:
            _ACTIVE_PROCESSES.pop(process.pid, None)
    return ProcessResult(
        status=124 if timed_out else process.returncode,
        elapsed_ms=int((time.monotonic_ns() - started) // 1_000_000),
        stdout=stdout,
        stderr=stderr,
        timed_out=timed_out,
    )


def terminate_process_tree(process: subprocess.Popen[str]) -> tuple[str, str]:
    stop_process_tree(process)
    return process.communicate()


def stop_process_tree(process: subprocess.Popen[str]) -> None:
    if process.poll() is not None:
        return
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/PID", str(process.pid), "/T", "/F"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        return

    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    deadline = time.monotonic() + 5
    while process.poll() is None and time.monotonic() < deadline:
        time.sleep(0.05)
    if process.poll() is None:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def terminate_active_processes() -> None:
    with _ACTIVE_LOCK:
        processes = list(_ACTIVE_PROCESSES.values())
    for process in processes:
        if process.poll() is None:
            try:
                stop_process_tree(process)
            except (OSError, subprocess.SubprocessError):
                pass


def _handle_termination(signum: int, _frame: Any) -> None:
    terminate_active_processes()
    raise SystemExit(128 + signum)


atexit.register(terminate_active_processes)
for _signal in (signal.SIGINT, signal.SIGTERM):
    signal.signal(_signal, _handle_termination)


def run_ordered(jobs: list[Job], worker: Callable[[Job], Result], workers: int) -> list[Result]:
    if workers == 1:
        return [worker(job) for job in jobs]
    completed: dict[int, Result] = {}
    with ThreadPoolExecutor(max_workers=workers, thread_name_prefix="codemap-benchmark") as pool:
        futures = {pool.submit(worker, job): index for index, job in enumerate(jobs)}
        for future in as_completed(futures):
            completed[futures[future]] = future.result()
    return [completed[index] for index in range(len(jobs))]
