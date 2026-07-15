// Responsibility: stable-agent-json-envelope-projection
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU8, Ordering};

pub(crate) const AGENT_ENVELOPE_VERSION: &str = "1";

static REPORT_EXIT: AtomicU8 = AtomicU8::new(crate::cli::EXIT_SUCCESS);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ResultState {
    Success,
    ValidEmptyMap,
    InvalidAnchor,
}

pub(crate) fn record_report_result<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let value = serde_json::to_value(value)?;
    record_state(result_state(&value));
    Ok(())
}

pub(crate) fn take_report_exit() -> u8 {
    REPORT_EXIT.swap(crate::cli::EXIT_SUCCESS, Ordering::SeqCst)
}

pub(crate) fn decorate_agent_json(value: &mut Value) -> anyhow::Result<()> {
    let state = result_state(value);
    record_state(state);
    let Some(report) = value.as_object() else {
        anyhow::bail!("public JSON report must serialize as an object");
    };
    let envelope = json!({
        "envelope_version": AGENT_ENVELOPE_VERSION,
        "report_kind": string_field(report, "kind").unwrap_or("unknown"),
        "report_version": string_field(report, "schema_version").unwrap_or("unknown"),
        "result": state,
        "scope": agent_scope(report),
        "snapshot": agent_snapshot(report),
        "horizon": agent_horizon(report),
        "expands": collect_expands(value),
    });
    value
        .as_object_mut()
        .expect("checked JSON object")
        .insert("agent".to_string(), envelope);
    Ok(())
}

fn record_state(state: ResultState) {
    let code = match state {
        ResultState::Success => crate::cli::EXIT_SUCCESS,
        ResultState::ValidEmptyMap => crate::cli::EXIT_VALID_EMPTY,
        ResultState::InvalidAnchor => crate::cli::EXIT_INVALID_INPUT,
    };
    REPORT_EXIT.fetch_max(code, Ordering::SeqCst);
}

fn result_state(value: &Value) -> ResultState {
    let Some(report) = value.as_object() else {
        return ResultState::Success;
    };
    if report.get("mode").and_then(Value::as_str) == Some("missing")
        || report
            .get("anchor")
            .and_then(Value::as_object)
            .and_then(|anchor| anchor.get("kind"))
            .and_then(Value::as_str)
            .is_some_and(|kind| matches!(kind, "missing" | "missing_symbol"))
    {
        return ResultState::InvalidAnchor;
    }
    let valid_empty = match report.get("kind").and_then(Value::as_str) {
        Some("where_report") => report.get("total_matches").and_then(Value::as_u64) == Some(0),
        Some("ls_report") => {
            report.get("mode").and_then(Value::as_str) == Some("directory")
                && report
                    .get("directory")
                    .and_then(Value::as_array)
                    .is_some_and(Vec::is_empty)
        }
        _ => false,
    };
    if valid_empty {
        ResultState::ValidEmptyMap
    } else {
        ResultState::Success
    }
}

fn agent_scope(report: &Map<String, Value>) -> Value {
    let mut scope = Map::new();
    if let Some(root) = report
        .get("prelude")
        .and_then(Value::as_object)
        .and_then(|prelude| prelude.get("root"))
        .or_else(|| report.get("root"))
    {
        scope.insert("root".to_string(), root.clone());
    }
    for key in ["scope", "path", "anchor", "target", "selector", "query"] {
        if let Some(value) = report.get(key) {
            scope.insert(key.to_string(), value.clone());
        }
    }
    if scope.is_empty() {
        scope.insert(
            "state".to_string(),
            Value::String("not_applicable".to_string()),
        );
    }
    Value::Object(scope)
}

fn agent_snapshot(report: &Map<String, Value>) -> Value {
    let mut identities = BTreeSet::new();
    let report_value = Value::Object(report.clone());
    if let Some(token) = report_value
        .pointer("/session_snapshot/token")
        .and_then(Value::as_str)
    {
        identities.insert(token.to_string());
    }
    collect_named_strings(&report_value, "snapshot", &mut identities);
    if identities.is_empty()
        && let Some(head) = report_value
            .pointer("/prelude/head/oid")
            .and_then(Value::as_str)
    {
        identities.insert(head.to_string());
    }
    json!({
        "state": if identities.is_empty() { "unavailable" } else { "observed" },
        "identities": identities,
    })
}

fn agent_horizon(report: &Map<String, Value>) -> Value {
    let report_value = Value::Object(report.clone());
    let horizons = report_value
        .pointer("/observations/horizons")
        .and_then(Value::as_array);
    let mut reasons = BTreeSet::new();
    let mut statuses = BTreeSet::new();
    if let Some(horizons) = horizons {
        for horizon in horizons {
            if let Some(closure) = horizon.pointer("/count/closure").and_then(Value::as_str) {
                statuses.insert(closure.to_string());
            }
            if let Some(values) = horizon.pointer("/count/reasons").and_then(Value::as_array) {
                for reason in values.iter().filter_map(Value::as_str) {
                    reasons.insert(reason.to_string());
                }
            }
        }
    }
    let status = if statuses.contains("unavailable") {
        "unavailable"
    } else if statuses.contains("open") {
        "open"
    } else if statuses.contains("closed") {
        "closed"
    } else if report
        .get("unknowns")
        .and_then(Value::as_array)
        .is_some_and(|values| !values.is_empty())
    {
        "open"
    } else {
        "not_applicable"
    };
    json!({
        "status": status,
        "groups": horizons.map_or(0, Vec::len),
        "reasons": reasons,
        "certificate_count": report_value
            .pointer("/observations/certificates")
            .and_then(Value::as_object)
            .map_or(0, Map::len),
    })
}

fn collect_expands(value: &Value) -> Vec<Vec<String>> {
    let mut displays = BTreeSet::new();
    collect_expand_strings(value, None, &mut displays);
    displays
        .into_iter()
        .filter_map(|command| shell_words(&command))
        .filter(|argv| argv.first().is_some_and(|word| word == "codemap"))
        .map(machine_json_argv)
        .collect()
}

fn machine_json_argv(mut argv: Vec<String>) -> Vec<String> {
    if !argv
        .iter()
        .any(|word| matches!(word.as_str(), "--format" | "--json"))
    {
        argv.extend(["--format".to_string(), "json".to_string()]);
    }
    argv
}

fn collect_expand_strings(value: &Value, key: Option<&str>, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (child_key, child) in map {
                collect_expand_strings(child, Some(child_key), out);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_expand_strings(child, key, out);
            }
        }
        Value::String(command) if matches!(key, Some("expand" | "next")) => {
            out.insert(command.clone());
        }
        _ => {}
    }
}

fn shell_words(command: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in command.chars() {
        if escaped {
            word.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && quote != Some('\'') {
            escaped = true;
        } else if matches!(ch, '\'' | '"') {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            } else {
                word.push(ch);
            }
        } else if ch.is_whitespace() && quote.is_none() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(ch);
        }
    }
    if escaped || quote.is_some() {
        return None;
    }
    if !word.is_empty() {
        words.push(word);
    }
    Some(words)
}

fn collect_named_strings(value: &Value, name: &str, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == name {
                    if let Some(value) = child.as_str() {
                        out.insert(value.to_string());
                    }
                } else {
                    collect_named_strings(child, name, out);
                }
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_named_strings(child, name, out);
            }
        }
        _ => {}
    }
}

fn string_field<'a>(report: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    report.get(key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::{machine_json_argv, shell_words};

    #[test]
    fn expand_shell_words_preserve_quoted_paths_as_argv() {
        assert_eq!(
            shell_words("codemap --root '/tmp/repo with spaces' cone 'src/a b.ts#go'"),
            Some(vec![
                "codemap".to_string(),
                "--root".to_string(),
                "/tmp/repo with spaces".to_string(),
                "cone".to_string(),
                "src/a b.ts#go".to_string(),
            ])
        );
    }

    #[test]
    fn agent_expands_request_schema_backed_json() {
        assert_eq!(
            machine_json_argv(vec!["codemap".into(), "ls".into(), "src".into()]),
            ["codemap", "ls", "src", "--format", "json"]
        );
    }
}
