#[test]
fn ab_activity_records_navigation_without_judging_it() {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/benchmark-codemap-ab.py");
    let probe = r#"import json, pathlib, runpy, sys
sys.path.insert(0, str(pathlib.Path(sys.argv[1]).parent))
activity = runpy.run_path(sys.argv[1])["codemap_activity"]
root = sys.argv[2]
selected = str(pathlib.Path(root) / "src")
print(json.dumps([
    activity(["ls --format json", "cone src/pricing.py"], root),
    activity(["ls --format json src/pricing.py"], root),
    activity(["graph --lens causal", "ls src/pricing.py"], root),
    activity(["ls src/pricing.py", "proof changed", "changed"], root),
    activity(["ls ./", "cone src/pricing.py"], root),
    activity(["--root changed ls src/pricing.py"], root),
    activity(["ls --root changed src/pricing.py"], root),
    activity(["ls src/pricing.py", "changed", "proof changed"], root),
    activity(["ls .", "cone"], root),
    activity(["doctor ls src/pricing.py"], root),
    activity(["--format json ls src/pricing.py"], root),
    activity(["garbage ls src/pricing.py"], root),
    activity([{"argv":["ls",root],"status":0}], root),
    activity([{"argv":["ls","--bogus","src/pricing.py"],"status":2}], root),
    activity([{"argv":["--root",selected,"ls",selected],"status":0}], root),
    activity([{"argv":["ls","--root",selected,selected],"status":0}], root),
    activity(["cone ."], root),
    activity([{"argv":["cone",root],"status":0}], root),
    activity(["cone src/.."], root),
    activity(["ls src/pricing.py", "changed", "changed", "proof changed"], root),
    activity([{"argv":["cone","missing.py"],"status":2},{"argv":["cone","src/pricing.py"],"status":0}], root),
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
    assert_eq!(rows[0]["entry_kind"], "root");
    assert_eq!(rows[0]["focused_after_root"], true);
    assert_eq!(rows[1]["entry_kind"], "exact");
    assert_eq!(rows[2]["entry_is_first_successful_invocation"], false);
    assert_eq!(rows[3]["ordered_daily"], false);
    assert_eq!(rows[4]["entry_kind"], "root");
    assert_eq!(rows[4]["root_entry"], true);
    assert_eq!(rows[5]["entry_kind"], "exact");
    assert_eq!(rows[5]["first_entry"], "--root changed ls src/pricing.py");
    assert_eq!(rows[6]["entry_kind"], "exact");
    assert_eq!(rows[7]["ordered_daily"], true);
    assert_eq!(rows[8]["focused_after_root"], false);
    for index in 9..=11 {
        assert_eq!(rows[index]["entry_kind"], "none");
    }
    assert_eq!(rows[12]["entry_kind"], "root");
    assert_eq!(rows[12]["root_entry"], true);
    assert_eq!(rows[12]["exact_entry"], false);
    assert_eq!(rows[13]["failed_invocation_count"], 1);
    assert_eq!(rows[13]["entry_kind"], "none");
    for index in 14..=15 {
        assert_eq!(rows[index]["entry_kind"], "root");
        assert_eq!(rows[index]["root_entry"], true);
        assert_eq!(rows[index]["exact_entry"], false);
    }
    for index in 16..=18 {
        assert_eq!(rows[index]["entry_kind"], "root");
        assert_eq!(rows[index]["root_entry"], true);
    }
    assert_eq!(rows[19]["ordered_daily"], false);
    assert_eq!(rows[20]["failed_invocation_count"], 1);
    assert_eq!(rows[20]["first_entry"], "cone src/pricing.py");
    assert_eq!(rows[20]["entry_is_first_successful_invocation"], true);
}

#[test]
fn ab_protocol_ignores_project_internal_codemap_consumers() {
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
fn ab_treatment_prompt_keeps_navigation_proportionate() {
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
    "version": m["PROMPT_PROTOCOL_VERSION"],
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
    assert_eq!(prompt["version"], 17);
    assert!(prompt["implementation"]
        .as_str()
        .unwrap()
        .contains("codemap changed && codemap proof changed` once"));
    assert!(prompt["implementation"]
        .as_str()
        .unwrap()
        .contains("nearest existing parent"));
    assert!(prompt["implementation"]
        .as_str()
        .unwrap()
        .contains("shared contract you will edit"));
    assert!(prompt["analysis"]
        .as_str()
        .unwrap()
        .contains("Read the relevant linked source"));
    assert!(prompt["analysis"]
        .as_str()
        .unwrap()
        .contains("never replace an exact file with its parent directory"));
    assert!(prompt["analysis"]
        .as_str()
        .unwrap()
        .contains("printed exact Expand only when"));
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
m["ARM_PROMPTS"]["codemap"] += "\nchanged protocol bytes\n"
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
