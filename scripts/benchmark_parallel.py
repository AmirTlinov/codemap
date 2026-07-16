"""Deterministic bounded concurrency for independent benchmark pairs."""

from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
import os
from pathlib import Path
import subprocess
import time
from typing import Any, Callable, TypeVar


Job = TypeVar("Job")
Result = TypeVar("Result")


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
    timed_out = False
    try:
        stdout, stderr = process.communicate(timeout=timeout_seconds)
    except subprocess.TimeoutExpired:
        timed_out = True
        stdout, stderr = terminate_process_tree(process)
    return ProcessResult(
        status=124 if timed_out else process.returncode,
        elapsed_ms=int((time.monotonic_ns() - started) // 1_000_000),
        stdout=stdout,
        stderr=stderr,
        timed_out=timed_out,
    )


def terminate_process_tree(process: subprocess.Popen[str]) -> tuple[str, str]:
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/PID", str(process.pid), "/T", "/F"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        return process.communicate()
    import signal

    try:
        os.killpg(process.pid, signal.SIGTERM)
        return process.communicate(timeout=5)
    except (ProcessLookupError, subprocess.TimeoutExpired):
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        return process.communicate()


def run_ordered(jobs: list[Job], worker: Callable[[Job], Result], workers: int) -> list[Result]:
    if workers == 1:
        return [worker(job) for job in jobs]
    completed: dict[int, Result] = {}
    with ThreadPoolExecutor(max_workers=workers, thread_name_prefix="codemap-benchmark") as pool:
        futures = {pool.submit(worker, job): index for index, job in enumerate(jobs)}
        for future in as_completed(futures):
            completed[futures[future]] = future.result()
    return [completed[index] for index in range(len(jobs))]
