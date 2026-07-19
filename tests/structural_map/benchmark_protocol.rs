#[test]
fn ab_activity_records_navigation_without_judging_it() {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/benchmark-codemap-ab.py");
    let probe = r#"import json, pathlib, runpy, sys
sys.path.insert(0, str(pathlib.Path(sys.argv[1]).parent))
activity = runpy.run_path(sys.argv[1])["codemap_activity"]
root = sys.argv[2]
selected = str(pathlib.Path(root) / "src")
print(json.dumps([
    activity(["ls --format json", "cone src/pricing.py", "proof changed"], root),
    activity([{"argv":["cone","missing.py"],"status":2},{"argv":["cone","src/pricing.py"],"status":0}], root),
    activity([{"argv":["--root",selected,"ls",selected],"status":0}], root),
]))
"#;
    let output = python()
        .args([
            "-c",
            probe,
            script.to_str().unwrap(),
            env!("CARGO_MANIFEST_DIR"),
        ])
        .output()
        .expect("protocol probe");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows: Value = serde_json::from_slice(&output.stdout).expect("protocol json");
    assert_eq!(rows[0]["calls"][0]["command"], "ls");
    assert_eq!(rows[0]["calls"][0]["scope_kind"], "current_level");
    assert_eq!(rows[0]["calls"][1]["command"], "cone");
    assert_eq!(rows[0]["calls"][1]["scope_kind"], "scoped");
    assert_eq!(rows[0]["calls"][2]["command"], "proof");
    assert_eq!(rows[0]["calls"][2]["argument"], "changed");
    assert_eq!(rows[1]["failed_invocation_count"], 1);
    assert_eq!(rows[1]["calls"][0]["succeeded"], false);
    assert_eq!(rows[1]["calls"][1]["succeeded"], true);
    assert_eq!(rows[2]["calls"][0]["scope_kind"], "current_level");
    for forbidden in [
        "ordered_daily",
        "entry_is_first_successful_invocation",
        "focused_after_root",
        "compliant",
    ] {
        assert!(rows.as_array().unwrap().iter().all(|row| row.get(forbidden).is_none()));
    }
}

#[test]
fn ab_arm_shim_ignores_project_internal_codemap_consumers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = python()
        .arg(root.join("tests/protocol_shim_fixture.py"))
        .output()
        .expect("protocol shim fixture should run");
    assert!(
        output.status.success(),
        "protocol shim failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn ab_treatment_prompt_offers_the_map_without_prescribing_agent_behavior() {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/benchmark-codemap-ab.py");
    let probe = r#"import json, pathlib, runpy, sys
sys.path.insert(0, str(pathlib.Path(sys.argv[1]).parent))
m = runpy.run_path(sys.argv[1])
from flagship_stats import task_aggregate
task = m["Task"]("exact", "implementation", pathlib.Path("."), "HEAD", "abc", "edit", [], "exact_control")
def activity(invocations):
    return {"invocation_count": invocations}
def pair(control, codemap):
    return {"task_id":"stochastic", "repo_id":"fixture", "task_class":"exact_control",
            "control_score":control, "codemap_score":codemap,
            "control_outcome":bool(control), "codemap_outcome":bool(codemap),
            "required_criteria":{"core":{"control":control, "codemap":codemap}},
            "control_elapsed":10, "codemap_elapsed":10, "control_input":10, "codemap_input":10}
print(json.dumps({
    "version": m["ARM_PROMPT_VERSION"],
    "implementation": m["ARM_PROMPTS"]["codemap"],
    "analysis": m["ANALYSIS_ARM_PROMPTS"]["codemap"],
    "exact": m["task_prompt"](task, "codemap"),
    "exact_control": m["task_prompt"](task, "control"),
    "assignment_valid": [
        m["arm_assignment_valid"]("codemap", activity(0)),
        m["arm_assignment_valid"]("codemap", activity(1)),
        m["arm_assignment_valid"]("control", activity(0)),
        m["arm_assignment_valid"]("control", activity(1)),
    ],
    "preflight": m["treatment_preflight_summary"](
        [task],
        [{"task_id":"exact", "run_valid":True, "outcome_passed":False}],
        [{"task_id":"exact", "baseline_passed":False}],
    ),
    "stochastic": task_aggregate([pair(1.0, 0.0), pair(0.0, 1.0)]),
}))
"#;
    let output = python()
        .args(["-c", probe, script.to_str().unwrap()])
        .output()
        .expect("treatment prompt probe");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let prompt: Value = serde_json::from_slice(&output.stdout).expect("prompt json");
    assert_eq!(prompt["version"], 18);
    for arm in ["implementation", "analysis", "exact"] {
        let body = prompt[arm].as_str().unwrap();
        assert!(body.contains("optional read-only"));
        assert!(body.contains("Use codemap when it is useful"));
        for forbidden in [
            "before ordinary inspection",
            "must",
            "never replace",
            "After editing",
            "Do not run broad",
            "requires no map call",
        ] {
            assert!(!body.contains(forbidden), "{arm} prescribes `{forbidden}`: {body}");
        }
    }
    assert!(prompt["exact"]
        .as_str()
        .unwrap()
        .contains("no repository-navigation uncertainty"));
    assert!(prompt["exact_control"]
        .as_str()
        .unwrap()
        .contains("no repository-navigation uncertainty"));
    assert!(!prompt["exact"]
        .as_str()
        .unwrap()
        .contains("requires no map call"));
    assert!(!prompt["exact"]
        .as_str()
        .unwrap()
        .contains("one command-execution shell call"));
    assert_eq!(
        prompt["assignment_valid"],
        serde_json::json!([true, true, true, false])
    );
    assert_eq!(prompt["preflight"]["infrastructure_ready"], true);
    assert_eq!(
        prompt["preflight"]["outcome_misses"],
        serde_json::json!(["exact"])
    );
    assert_eq!(
        prompt["stochastic"]["required_criterion_losses"],
        serde_json::json!([])
    );
    assert_eq!(
        prompt["stochastic"]["control_outcomes"],
        serde_json::json!([true, false])
    );
    assert_eq!(
        prompt["stochastic"]["codemap_outcomes"],
        serde_json::json!([false, true])
    );
}

#[test]
fn ab_fingerprint_tracks_composed_prompt_timeout_and_order() {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/benchmark-codemap-ab.py");
    let probe = r#"import argparse, json, pathlib, runpy, sys
sys.path.insert(0, str(pathlib.Path(sys.argv[1]).parent))
m = runpy.run_path(sys.argv[1])
task = m["Task"]("t", "implementation", pathlib.Path("."), "HEAD", "abc", "edit", [])
args = argparse.Namespace(model="m", reasoning_effort="high", timeout_seconds=10)
def fingerprint(order=0):
    return m["trial_fingerprint"](task, "abc", "codemap", order, args, "codex", "codemap", [], {})
values = [fingerprint()]
m["ARM_PROMPTS"]["codemap"] += "\nchanged prompt bytes\n"
values.append(fingerprint())
args.timeout_seconds = 11
values.append(fingerprint())
values.append(fingerprint(1))
print(json.dumps(values))
"#;
    let output = python()
        .args(["-c", probe, script.to_str().unwrap()])
        .output()
        .expect("fingerprint probe");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let values: Value = serde_json::from_slice(&output.stdout).expect("fingerprint json");
    let unique = values
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        unique.len(),
        4,
        "every runtime contract change must invalidate resume"
    );
}
