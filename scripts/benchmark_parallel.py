"""Deterministic bounded concurrency for independent benchmark pairs."""

from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Callable, TypeVar


Job = TypeVar("Job")
Result = TypeVar("Result")


def run_ordered(jobs: list[Job], worker: Callable[[Job], Result], workers: int) -> list[Result]:
    if workers == 1:
        return [worker(job) for job in jobs]
    completed: dict[int, Result] = {}
    with ThreadPoolExecutor(max_workers=workers, thread_name_prefix="codemap-benchmark") as pool:
        futures = {pool.submit(worker, job): index for index, job in enumerate(jobs)}
        for future in as_completed(futures):
            completed[futures[future]] = future.result()
    return [completed[index] for index in range(len(jobs))]
