// Responsibility: repo-rust-reexport-bindings
use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;

use crate::model::ImportBindingsBySpec;

fn rust_reexport_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?m)^\s*pub(?:\((?:crate|super|self|in [^)]*)\))?\s+use\s+([A-Za-z0-9_:]+?)(\*|\{([^}]*)\}|)\s*(?:as\s+([A-Za-z0-9_]+)\s*)?;"#,
        )
        .expect("valid rust re-export regex")
    })
}

/// Records `pub use path::...` re-exports so consumer counting can tell a
/// barrel re-export flow from a proven zero. Keys mirror the specs captured
/// by `rust_use_re` so import resolution re-keys both together.
pub(crate) fn extract_rust_reexport_bindings(text: &str) -> ImportBindingsBySpec {
    let mut out: ImportBindingsBySpec = BTreeMap::new();
    for cap in rust_reexport_re().captures_iter(text) {
        let spec_raw = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        let tail = cap.get(2).map(|m| m.as_str()).unwrap_or_default();
        let entry = out.entry(spec_key(spec_raw, tail)).or_default();
        if tail == "*" {
            entry.insert("export:*".to_string(), "*".to_string());
        } else if let Some(named) = cap.get(3) {
            for item in named.as_str().split(',') {
                let item = item.trim();
                if item.is_empty() {
                    continue;
                }
                let mut parts = item.split_whitespace();
                let source = parts.next().unwrap_or_default().trim_matches(':');
                let exported = item.split(" as ").nth(1).map(str::trim).unwrap_or(source);
                entry.insert(format!("export:{exported}"), source.to_string());
            }
        } else {
            let source = spec_raw.rsplit("::").next().unwrap_or(spec_raw);
            let exported = cap.get(4).map(|m| m.as_str()).unwrap_or(source);
            if !source.is_empty() {
                entry.insert(format!("export:{exported}"), source.to_string());
            }
        }
    }
    out.retain(|_, bindings| !bindings.is_empty());
    out
}

fn spec_key(spec_raw: &str, tail: &str) -> String {
    if tail.is_empty() {
        spec_raw.to_string()
    } else {
        // `rust_use_re` captures up to the `*` or `{`, keeping the trailing `::`.
        spec_raw.to_string()
    }
}
