"""Serialized disposable Git worktree mutations for parallel benchmark runs."""

from __future__ import annotations

from pathlib import Path
import threading

from benchmark_parallel import ProcessResult, run_process


_LOCKS: dict[Path, threading.Lock] = {}
_LOCKS_GUARD = threading.Lock()


def _repository_lock(repo: Path) -> threading.Lock:
    key = repo.resolve()
    with _LOCKS_GUARD:
        return _LOCKS.setdefault(key, threading.Lock())


def add_worktree(repo: Path, worktree: Path, commit: str) -> ProcessResult:
    with _repository_lock(repo):
        return run_process(
            ["git", "-C", str(repo), "worktree", "add", "--detach", str(worktree), commit],
            repo,
            120,
        )


def remove_worktree(repo: Path, worktree: Path) -> ProcessResult:
    with _repository_lock(repo):
        return run_process(
            ["git", "-C", str(repo), "worktree", "remove", "--force", str(worktree)],
            repo,
            120,
        )
