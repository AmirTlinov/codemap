fn unknown_from_added_line(
    rel: &str,
    line: usize,
    text: &str,
    unsupported_route_framework_context: &BTreeSet<String>,
) -> Option<Unknown> {
    let ext = std::path::Path::new(rel)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    let js_like = matches!(ext, "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs");
    let route_like = js_like || matches!(ext, "py" | "go");
    if js_like && dynamic_import_line(text) {
        return Some(unknown(
            "dynamic_import",
            Some(rel),
            Some(line),
            "import target is not a static string literal",
            "runtime dependency target is not resolved structurally",
            Some(format!("codemap ls {}", shell_quote(rel))),
        ));
    }
    if js_like && dynamic_require_line(text) {
        return Some(unknown(
            "js_require_dynamic",
            Some(rel),
            Some(line),
            "require target is not a static string literal",
            "runtime dependency target is not resolved structurally",
            Some(format!("codemap ls {}", shell_quote(rel))),
        ));
    }
    if dynamic_env_lookup_line(text) {
        return Some(unknown(
            "env_dynamic_lookup",
            Some(rel),
            Some(line),
            "environment variable key is dynamic",
            "runtime config dependency cannot be named structurally",
            Some(format!("codemap runtime {}", shell_quote(rel))),
        ));
    }
    if route_like && route_string_concat_line(text) {
        return Some(unknown(
            "route_string_concat",
            Some(rel),
            Some(line),
            "route path is composed instead of a static literal",
            "runtime route cannot be mapped to an exact path structurally",
            Some(format!("codemap runtime {}", shell_quote(rel))),
        ));
    }
    if route_like && route_dynamic_path_line(text) {
        return Some(unknown(
            "route_dynamic_path",
            Some(rel),
            Some(line),
            "route path is not a static literal",
            "runtime route cannot be mapped to an exact path structurally",
            Some(format!("codemap runtime {}", shell_quote(rel))),
        ));
    }
    if route_like && route_dynamic_method_line(text) {
        return Some(unknown(
            "route_dynamic_method",
            Some(rel),
            Some(line),
            "route method is computed instead of a static framework method",
            "runtime route is not added to the exact method/path map",
            Some(format!("codemap runtime {}", shell_quote(rel))),
        ));
    }
    if js_like && unsupported_framework_route_line(text, unsupported_route_framework_context) {
        return Some(unknown(
            "unsupported_framework_route",
            Some(rel),
            Some(line),
            "framework route decorator is recognized but not resolved by a deterministic adapter",
            "runtime route is not added to the exact method/path map",
            Some(format!("codemap runtime {}", shell_quote(rel))),
        ));
    }
    if raw_sql_literal_line(text) {
        return Some(unknown(
            "raw_sql_literal",
            Some(rel),
            Some(line),
            "raw SQL appears in code",
            "database table/column dependency is not resolved structurally",
            Some(format!("codemap cone {}", shell_quote(rel))),
        ));
    }
    None
}
