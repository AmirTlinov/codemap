#!/usr/bin/env python3
"""External behavior contract for the PostgreSQL backup GitOps slice."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts/ops/ci"))
from yaml_loader import load_all_yaml  # noqa: E402


def truth(value: Any) -> bool:
    return str(value).lower() == "true"


def documents() -> list[tuple[Path, dict[str, Any]]]:
    out = []
    for path in sorted((ROOT / "deploy/k8s/base").rglob("*.yaml")):
        for document in load_all_yaml(path):
            if isinstance(document, dict) and document.get("kind"):
                out.append((path, document))
    return out


def pod_script(cronjob: dict[str, Any]) -> tuple[dict[str, Any], list[dict[str, Any]], str]:
    pod = cronjob["spec"]["jobTemplate"]["spec"]["template"]["spec"]
    containers = [*pod.get("initContainers", []), *pod.get("containers", [])]
    script = "\n".join(
        " ".join(str(part) for part in [*item.get("command", []), *item.get("args", [])])
        for item in containers
    )
    return pod, containers, script


def selector_matches(selector: dict[str, Any], labels: dict[str, Any]) -> bool:
    expected = selector.get("matchLabels", selector)
    return bool(expected) and all(labels.get(key) == value for key, value in expected.items())


def rule_ports(rule: dict[str, Any]) -> set[int]:
    out = set()
    for row in rule.get("ports", []):
        try:
            out.add(int(row.get("port")))
        except (TypeError, ValueError):
            pass
    return out


def owned_resources(root: Path) -> set[Path]:
    pending = [root / "deploy/k8s/base/kustomization.yaml"]
    seen: set[Path] = set()
    while pending:
        manifest = pending.pop().resolve()
        if manifest in seen or not manifest.is_file():
            continue
        seen.add(manifest)
        data = load_all_yaml(manifest)[0]
        for entry in data.get("resources", []):
            target = (manifest.parent / entry).resolve()
            if target.is_dir():
                target = target / "kustomization.yaml"
            if target.name == "kustomization.yaml":
                pending.append(target)
            elif target.is_file():
                seen.add(target)
    return seen


class PostgresBackupTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.docs = documents()
        candidates = []
        for path, resource in cls.docs:
            if resource["kind"] != "CronJob":
                continue
            try:
                pod, containers, script = pod_script(resource)
            except (KeyError, TypeError):
                continue
            if "pg_dump" in script:
                candidates.append((path, resource, pod, containers, script))
        if len(candidates) != 1:
            raise AssertionError(f"expected one PostgreSQL backup CronJob, found {len(candidates)}")
        cls.path, cls.cronjob, cls.pod, cls.containers, cls.script = candidates[0]
        cls.labels = cls.cronjob["spec"]["jobTemplate"]["spec"]["template"]["metadata"][
            "labels"
        ]

    def test_remote_readback_precedes_success(self) -> None:
        script = self.script.lower()
        self.assertIn("pg_dump", script)
        self.assertTrue("mc cp" in script or "aws s3 cp" in script or "rclone copy" in script)
        remote_copies = script.count("mc cp") + script.count("aws s3 cp")
        readback = remote_copies >= 2 or any(
            command in script for command in ("mc cat", "rclone cat", "s3api get-object")
        )
        self.assertTrue(readback, "backup must upload and independently read back")
        self.assertGreaterEqual(script.count("sha256sum"), 2)
        self.assertTrue(any(word in script for word in ("test ", "cmp ", "diff ", "if [")))
        self.assertTrue(all("@sha256:" in item.get("image", "") for item in self.containers))
        self.assertTrue(truth(self.cronjob["spec"].get("suspend", "false")) is False)

    def test_runtime_is_least_privilege(self) -> None:
        self.assertEqual(str(self.pod.get("automountServiceAccountToken")).lower(), "false")
        pod_security = self.pod.get("securityContext", {})
        self.assertTrue(truth(pod_security.get("runAsNonRoot")))
        for container in self.containers:
            security = container.get("securityContext", {})
            self.assertEqual(str(security.get("allowPrivilegeEscalation")).lower(), "false")
            self.assertTrue(truth(security.get("readOnlyRootFilesystem")))
            self.assertIn("ALL", security.get("capabilities", {}).get("drop", []))

    def test_network_is_only_dns_postgres_and_minio(self) -> None:
        policies = [resource for _, resource in self.docs if resource["kind"] == "NetworkPolicy"]
        selected = [
            policy
            for policy in policies
            if selector_matches(policy["spec"].get("podSelector", {}), self.labels)
        ]
        self.assertTrue(selected, "backup pod needs an owning NetworkPolicy")
        egress = [rule for policy in selected for rule in policy["spec"].get("egress", [])]
        ports = set().union(*(rule_ports(rule) for rule in egress))
        self.assertEqual(ports, {53, 5432, 9000})
        self.assertFalse(any(not rule.get("to") for rule in egress), "unbounded egress is forbidden")

    def test_gitops_alert_and_runbook_own_the_slice(self) -> None:
        self.assertIn(self.path.resolve(), owned_resources(ROOT))
        alerts = [
            rule
            for _, resource in self.docs
            if resource["kind"] == "PrometheusRule"
            for group in resource.get("spec", {}).get("groups", [])
            for rule in group.get("rules", [])
        ]
        stale = [
            rule
            for rule in alerts
            if "backup" in str(rule.get("alert", "")).lower()
            and "kube_cronjob_status_last_successful_time" in str(rule.get("expr", ""))
        ]
        self.assertTrue(stale, "a stale-backup alert must be rendered by GitOps")

        runbooks = list((ROOT / "docs").rglob("*.md"))
        bodies = [(path, path.read_text(encoding="utf-8", errors="replace").lower()) for path in runbooks]
        matches = [
            (path, body)
            for path, body in bodies
            if "postgres" in body and "backup" in body and "restore" in body
        ]
        self.assertTrue(matches, "operators need a PostgreSQL restore runbook")
        self.assertTrue(any("sha256" in body or "checksum" in body for _, body in matches))


if __name__ == "__main__":
    unittest.main()
