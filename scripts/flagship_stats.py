"""Deterministic statistics for the frozen flagship behavioral gate."""

from __future__ import annotations

import math
import random
import statistics
from collections import defaultdict
from typing import Any


def median(values: list[float]) -> float | None:
    return statistics.median(values) if values else None


def relative_delta(treatment: float, control: float) -> float | None:
    if control <= 0:
        return 0.0 if treatment <= 0 else None
    return (treatment - control) / control


def quantile(values: list[float], probability: float) -> float:
    if not values:
        raise ValueError("quantile requires values")
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    fraction = position - lower
    return ordered[lower] * (1 - fraction) + ordered[upper] * fraction


def ordinal_alpha(units: list[list[int]], max_score: int) -> float | None:
    """Krippendorff alpha using marginal-frequency ordinal distance."""
    usable = [ratings for ratings in units if len(ratings) >= 2]
    population = [score for ratings in usable for score in ratings]
    if len(population) < 2:
        return None
    frequencies = [population.count(score) for score in range(max_score + 1)]

    def distance(left: int, right: int) -> float:
        if left == right:
            return 0.0
        low, high = sorted((left, right))
        mass = sum(frequencies[low : high + 1]) - (frequencies[low] + frequencies[high]) / 2
        return mass**2

    observed_sum = 0.0
    observed_pairs = 0
    for ratings in usable:
        for index, left in enumerate(ratings):
            for right in ratings[index + 1 :]:
                observed_sum += distance(left, right)
                observed_pairs += 1
    if observed_pairs == 0:
        return None
    expected_sum = 0.0
    expected_pairs = 0
    for index, left in enumerate(population):
        for right in population[index + 1 :]:
            expected_sum += distance(left, right)
            expected_pairs += 1
    expected = expected_sum / expected_pairs if expected_pairs else 0.0
    observed = observed_sum / observed_pairs
    if expected == 0:
        return 1.0 if observed == 0 else 0.0
    return 1.0 - observed / expected


def repo_macro(task_scores: list[dict[str, Any]]) -> float:
    by_repo: dict[str, list[float]] = defaultdict(list)
    for task in task_scores:
        by_repo[task["repo_id"]].append(float(task["delta"]))
    if not by_repo:
        raise ValueError("repo macro requires task scores")
    return statistics.mean(statistics.mean(values) for values in by_repo.values())


def paired_repo_bootstrap(
    task_scores: list[dict[str, Any]], iterations: int, seed: int, alpha: float
) -> dict[str, float | int]:
    """Resample paired tasks inside each repo, then macro-average repositories."""
    by_repo: dict[str, list[float]] = defaultdict(list)
    for task in task_scores:
        by_repo[task["repo_id"]].append(float(task["delta"]))
    if not by_repo:
        raise ValueError("bootstrap requires paired tasks")
    rng = random.Random(seed)
    samples: list[float] = []
    repos = sorted(by_repo)
    for _ in range(iterations):
        repo_means = []
        for repo in repos:
            values = by_repo[repo]
            resampled = [values[rng.randrange(len(values))] for _ in values]
            repo_means.append(statistics.mean(resampled))
        samples.append(statistics.mean(repo_means))
    return {
        "estimate": repo_macro(task_scores),
        "lower_bound": quantile(samples, alpha),
        "iterations": iterations,
        "seed": seed,
        "one_sided_alpha": alpha,
    }


def criterion_score(criteria: list[dict[str, Any]]) -> tuple[float, dict[str, float]]:
    total = sum(float(row["weight"]) for row in criteria)
    if total <= 0:
        raise ValueError("criterion weights must be positive")
    categories: dict[str, list[float]] = defaultdict(lambda: [0.0, 0.0])
    passed = 0.0
    for row in criteria:
        weight = float(row["weight"])
        value = float(row["value"])
        passed += weight * value
        categories[row["category"]][0] += weight * value
        categories[row["category"]][1] += weight
    return passed / total, {
        name: weighted / weight for name, (weighted, weight) in categories.items()
    }


def task_aggregate(pair_rows: list[dict[str, Any]]) -> dict[str, Any]:
    """Aggregate repetitions before a task contributes once to the gate."""
    if not pair_rows:
        raise ValueError("task aggregate requires pair rows")
    treatment = [float(row["codemap_score"]) for row in pair_rows]
    control = [float(row["control_score"]) for row in pair_rows]
    category_names = sorted(
        set().union(*(row["category_deltas"].keys() for row in pair_rows))
    )
    criterion_names = sorted(
        set().union(*(row["criterion_deltas"].keys() for row in pair_rows))
    )
    return {
        "task_id": pair_rows[0]["task_id"],
        "repo_id": pair_rows[0]["repo_id"],
        "task_class": pair_rows[0]["task_class"],
        "control_score": statistics.mean(control),
        "codemap_score": statistics.mean(treatment),
        "delta": statistics.mean(treatment) - statistics.mean(control),
        "category_deltas": {
            name: statistics.mean(row["category_deltas"].get(name, 0.0) for row in pair_rows)
            for name in category_names
        },
        "criterion_deltas": {
            name: statistics.mean(row["criterion_deltas"].get(name, 0.0) for row in pair_rows)
            for name in criterion_names
        },
        "control_outcomes": [row["control_outcome"] for row in pair_rows],
        "codemap_outcomes": [row["codemap_outcome"] for row in pair_rows],
        "time_overhead": median(
            [
                value
                for row in pair_rows
                if (value := relative_delta(row["codemap_elapsed"], row["control_elapsed"]))
                is not None
            ]
        ),
        "input_overhead": median(
            [
                value
                for row in pair_rows
                if (value := relative_delta(row["codemap_input"], row["control_input"]))
                is not None
            ]
        ),
    }
