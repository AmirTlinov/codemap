fn runtime_routes_for_file(project: &Project, file: &FileInfo) -> Vec<RuntimeRoute> {
    let mut routes = Vec::new();
    if let Some(route) = route_from_file_convention(project, file) {
        routes.push(route);
    }
    routes.extend(framework_routes_for_file(project, file));
    routes
}

fn framework_routes_for_file(project: &Project, file: &FileInfo) -> Vec<RuntimeRoute> {
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    let mut routes = Vec::new();
    for (line_number, line) in runtime_code_lines(&text) {
        if matches!(file.ext.as_str(), "js" | "jsx" | "ts" | "tsx") {
            routes.extend(javascript_route_registrations(
                &file.rel,
                &line,
                line_number,
            ));
        } else if file.ext == "py" {
            routes.extend(python_route_decorators(&file.rel, &line, line_number));
        } else if file.ext == "go" {
            routes.extend(go_route_registrations(&file.rel, &line, line_number));
        }
    }
    routes
}

fn unknowns_for_file(project: &Project, file: &FileInfo) -> Vec<Unknown> {
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let js_like = matches!(file.ext.as_str(), "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs");
    let route_like = js_like || matches!(file.ext.as_str(), "py" | "go");
    let env_like = js_like || matches!(file.ext.as_str(), "py" | "rs");
    let unsupported_route_framework_context = unsupported_framework_route_context(&text);
    for (line_number, line) in runtime_code_lines(&text) {
        if js_like && dynamic_import_line(&line) {
            out.push(unknown(
                "dynamic_import",
                Some(&file.rel),
                Some(line_number),
                "import target is not a static string literal",
                "runtime dependency target is not resolved structurally",
                Some(format!("codemap ls {}", shell_quote(&file.rel))),
            ));
        }
        if js_like && dynamic_require_line(&line) {
            out.push(unknown(
                "js_require_dynamic",
                Some(&file.rel),
                Some(line_number),
                "require target is not a static string literal",
                "runtime dependency target is not resolved structurally",
                Some(format!("codemap ls {}", shell_quote(&file.rel))),
            ));
        }
        if env_like && dynamic_env_lookup_line(&line) {
            out.push(unknown(
                "env_dynamic_lookup",
                Some(&file.rel),
                Some(line_number),
                "environment variable key is dynamic",
                "runtime config dependency cannot be named structurally",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if route_like && route_string_concat_line(&line) {
            out.push(unknown(
                "route_string_concat",
                Some(&file.rel),
                Some(line_number),
                "route path is composed instead of a static literal",
                "runtime route cannot be mapped to an exact path structurally",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        } else if route_like && route_dynamic_path_line(&line) {
            out.push(unknown(
                "route_dynamic_path",
                Some(&file.rel),
                Some(line_number),
                "route path is not a static literal",
                "runtime route cannot be mapped to an exact path structurally",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if route_like && route_dynamic_method_line(&line) {
            out.push(unknown(
                "route_dynamic_method",
                Some(&file.rel),
                Some(line_number),
                "route method is computed instead of a static framework method",
                "runtime route is not added to the exact method/path map",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if js_like
            && unsupported_framework_route_line(&line, &unsupported_route_framework_context)
        {
            out.push(unknown(
                "unsupported_framework_route",
                Some(&file.rel),
                Some(line_number),
                "framework route decorator is recognized but not resolved by a deterministic adapter",
                "runtime route is not added to the exact method/path map",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if route_like && route_object_dynamic_line(&line) {
            out.push(unknown(
                "route_object_dynamic",
                Some(&file.rel),
                Some(line_number),
                "route object does not expose static method and path fields",
                "runtime route cannot be mapped to an exact method/path structurally",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if route_like && let Some(kind) = route_mount_prefix_unknown_kind(&line) {
            let (reason, effect) = if kind == "route_mount_prefix" {
                (
                    "route prefix mounts middleware or a nested router",
                    "nested endpoints under this prefix are not expanded structurally",
                )
            } else {
                (
                    "route mount prefix is not a static literal",
                    "nested runtime routes cannot be mapped to exact paths structurally",
                )
            };
            out.push(unknown(
                kind,
                Some(&file.rel),
                Some(line_number),
                reason,
                effect,
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if raw_sql_literal_line(&line) {
            out.push(unknown(
                "raw_sql_literal",
                Some(&file.rel),
                Some(line_number),
                "raw SQL appears in code",
                "database table/column dependency is not resolved structurally",
                Some(format!("codemap cone {}", shell_quote(&file.rel))),
            ));
        }
    }
    out
}

fn side_effect_surfaces_for_file(project: &Project, file: &FileInfo) -> Vec<Surface> {
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (line_number, line) in runtime_code_lines(&text) {
        let Some((kind, evidence)) = side_effect_kind(&line) else {
            continue;
        };
        out.push(Surface {
            id: format!("surface:side_effect:{kind}:{}:{line_number}", file.rel),
            kind: kind.to_string(),
            path: Some(file.rel.clone()),
            role: Some("side_effect".to_string()),
            evidence: evidence.to_string(),
            strength: EvidenceStrength::Medium,
            count: Some(1),
            examples: vec![format!("{}:{line_number}", file.rel)],
            hidden_count: 0,
        });
    }
    out
}

fn dynamic_import_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    let Some((tail, code_tail)) = named_call_tail(line, &code, "import") else {
        return false;
    };
    code_tail.starts_with('(') && quoted_literal_at(tail.trim_start().trim_start_matches('(')).is_none()
}

fn dynamic_require_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    let Some((tail, code_tail)) = named_call_tail(line, &code, "require") else {
        return false;
    };
    code_tail.starts_with('(') && quoted_literal_at(tail.trim_start().trim_start_matches('(')).is_none()
}

fn named_call_tail<'a>(line: &'a str, code: &'a str, name: &str) -> Option<(&'a str, &'a str)> {
    let mut offset = 0;
    while let Some(found) = code[offset..].find(name) {
        let start = offset + found;
        let end = start + name.len();
        if name_has_call_boundary(code, start, end) {
            return Some((&line[end..], code[end..].trim_start()));
        }
        offset = end;
    }
    None
}

fn name_has_call_boundary(code: &str, start: usize, end: usize) -> bool {
    let before = code[..start].chars().next_back();
    let after = code[end..].chars().next();
    let valid_before =
        before.is_none_or(|ch| !is_identifier_char(ch) && ch != '.' && ch != '$');
    let valid_after = after.is_none_or(|ch| !is_identifier_char(ch) && ch != '$');
    valid_before && valid_after
}

fn dynamic_env_lookup_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    code.contains("process.env[")
        || code.contains("import.meta.env[")
        || dynamic_call_arg(line, &code, "Deno.env.get(")
        || dynamic_call_arg(line, &code, "std::env::var(")
        || dynamic_call_arg(line, &code, "env::var(")
        || dynamic_call_arg(line, &code, "os.getenv(")
        || dynamic_os_environ_lookup(line, &code)
}

fn dynamic_call_arg(line: &str, code: &str, call: &str) -> bool {
    let Some(start) = code.find(call) else {
        return false;
    };
    quoted_literal_at(&line[start + call.len()..]).is_none()
}

fn dynamic_os_environ_lookup(line: &str, code: &str) -> bool {
    let Some(start) = code.find("os.environ[") else {
        return false;
    };
    quoted_literal_at(&line[start + "os.environ[".len()..]).is_none()
}

fn route_string_concat_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    static_route_methods().iter().any(|method| {
        let call = format!(".{method}(");
        code.find(&call).is_some_and(|start| {
            if !route_like_receiver(&code[..start]) {
                return false;
            }
            let arg = line[start + call.len()..].trim_start();
            quoted_literal_at(arg).is_none() && (arg.contains('+') || arg.contains("${"))
        })
    })
}

fn route_dynamic_path_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    static_route_methods().iter().any(|method| {
        let call = format!(".{method}(");
        code.find(&call).is_some_and(|start| {
            if !route_like_receiver(&code[..start]) {
                return false;
            }
            let arg = line[start + call.len()..].trim_start();
            quoted_literal_at(arg).is_none()
        })
    })
}

fn route_dynamic_method_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    if go_dynamic_route_method_line(line, &code) {
        return true;
    }
    let mut offset = 0;
    while let Some(found) = code[offset..].find('[') {
        let start = offset + found;
        if route_like_receiver(&code[..start])
            && code[start..]
                .find("](")
                .is_some_and(|close| quoted_literal_at(&line[start + close + 2..]).is_some())
        {
            return true;
        }
        offset = start + 1;
    }
    false
}

fn go_dynamic_route_method_line(line: &str, code: &str) -> bool {
    let Some(start) = code
        .find("http.HandleFunc(")
        .or_else(|| code.find(".HandleFunc("))
    else {
        return false;
    };
    if quoted_literal_at(line[start..].split_once('(').map(|(_, tail)| tail).unwrap_or(""))
        .is_none()
    {
        return false;
    }
    let Some(open_paren) = code[start..].find('(').map(|found| start + found) else {
        return false;
    };
    let Some(close) = matching_close_paren(code, open_paren) else {
        return false;
    };
    go_route_has_methods_chain(code, close + 1)
        && go_route_method_in_chain(line, code, close + 1).is_none()
}

fn route_object_dynamic_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    let call = ".route(";
    let Some(start) = code.find(call) else {
        return false;
    };
    let arg_start = start + call.len();
    if !route_like_receiver(&code[..start]) {
        return false;
    }
    let Some((object_start, object_end)) = object_argument_range(&code, arg_start) else {
        return false;
    };
    let object_line = &line[object_start..object_end];
    let object_code = &code[object_start..object_end];
    object_field_literal(object_line, object_code, "method").is_none()
        || object_field_literal(object_line, object_code, "url")
            .or_else(|| object_field_literal(object_line, object_code, "path"))
            .is_none()
}

fn route_mount_prefix_unknown_kind(line: &str) -> Option<&'static str> {
    let code = code_shape_without_literal_content(line);
    let call = ".use(";
    let start = code.find(call)?;
    let arg_start = start + call.len();
    if !route_like_receiver(&code[..start]) {
        return None;
    }
    let arg = line[arg_start..].trim_start();
    if quoted_literal_at(arg).is_some_and(|path| path.starts_with('/')) {
        return Some("route_mount_prefix");
    }
    let first_arg = arg.split([',', ')']).next()?.trim();
    if !first_arg.is_empty()
        && arg.contains(',')
        && (first_arg.contains('+')
            || first_arg.contains("${")
            || first_arg.to_ascii_lowercase().contains("prefix")
            || first_arg.to_ascii_lowercase().contains("path")
            || first_arg.to_ascii_lowercase().contains("route"))
    {
        return Some("route_mount_dynamic_prefix");
    }
    None
}

fn raw_sql_literal_line(line: &str) -> bool {
    raw_sql_literal_kind(line).is_some()
}

fn side_effect_kind(line: &str) -> Option<(&'static str, &'static str)> {
    let code = code_shape_without_literal_content(line);
    if code.contains("fetch(") || code.contains("axios.") {
        Some(("network_call", "static_network_call"))
    } else if code.contains("localStorage.setItem")
        || code.contains("sessionStorage.setItem")
        || code.contains("fs.writeFile")
        || code.contains("std::fs::write")
        || code.contains("os.WriteFile")
    {
        Some(("storage_write", "static_storage_write"))
    } else if matches!(
        raw_sql_literal_kind(line),
        Some("INSERT INTO " | "UPDATE " | "DELETE FROM ")
    )
    {
        Some(("database_write", "raw_sql_mutation"))
    } else {
        None
    }
}

fn raw_sql_literal_kind(line: &str) -> Option<&'static str> {
    if !has_raw_sql_execution_context(line) {
        return None;
    }
    let literals = quoted_literal_contents(line)
        .into_iter()
        .map(|literal| literal.to_ascii_uppercase())
        .collect::<Vec<_>>();
    ["SELECT ", "INSERT INTO ", "UPDATE ", "DELETE FROM "]
        .into_iter()
        .find(|needle| literals.iter().any(|literal| literal.contains(needle)))
}

fn has_raw_sql_execution_context(line: &str) -> bool {
    let code = code_shape_without_literal_content(line).to_ascii_lowercase();
    [
        ".query(",
        "query(",
        ".execute(",
        "execute(",
        ".exec(",
        "exec(",
        "sqlx::query",
        "sql!",
        "$queryraw",
        "rawquery",
        "prepare(",
    ]
    .iter()
    .any(|needle| code.contains(needle))
}
