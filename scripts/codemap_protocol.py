"""Validate the stable machine-readable codemap agent envelope."""

from __future__ import annotations

from typing import Any


AGENT_EXIT_RESULTS = {
    0: "success",
    10: "valid_empty_map",
    20: "invalid_anchor",
}


def validate_agent_report(report: dict[str, Any], status: int) -> dict[str, Any]:
    """Validate the stable transport contract without reading human output."""
    if not isinstance(report, dict):
        raise ValueError("codemap report must be a JSON object")
    agent = report.get("agent")
    if not isinstance(agent, dict) or agent.get("envelope_version") != "1":
        raise ValueError("codemap report needs stable agent envelope v1")
    if agent.get("report_kind") != report.get("kind"):
        raise ValueError("agent report_kind must match kind")
    if agent.get("report_version") != report.get("schema_version"):
        raise ValueError("agent report_version must match schema_version")
    expected = AGENT_EXIT_RESULTS.get(status)
    if expected is None or agent.get("result") != expected:
        raise ValueError(f"exit {status} disagrees with agent result")
    if not isinstance(report.get("build_identity"), dict):
        raise ValueError("codemap report needs build_identity")
    for field in ("scope", "snapshot", "horizon"):
        if not isinstance(agent.get(field), dict):
            raise ValueError(f"agent {field} must be an object")
    expands = agent.get("expands")
    if not isinstance(expands, list):
        raise ValueError("agent expands must be argv arrays")
    for argv in expands:
        if (
            not isinstance(argv, list)
            or not argv
            or argv[0] != "codemap"
            or not all(isinstance(word, str) for word in argv)
            or not ("--json" in argv or "--format" in argv)
        ):
            raise ValueError("agent expand must be a codemap JSON argv array")
    return agent


def next_expand_argv(report: dict[str, Any], index: int = 0) -> list[str]:
    agent = validate_agent_report(report, 0)
    try:
        return list(agent["expands"][index])
    except (IndexError, KeyError, TypeError) as error:
        raise ValueError("codemap report has no requested expand") from error
