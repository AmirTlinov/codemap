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
    for (index, line) in text.lines().enumerate() {
        if line_is_comment(line) {
            continue;
        }
        let line_number = index + 1;
        if matches!(file.ext.as_str(), "js" | "jsx" | "ts" | "tsx") {
            routes.extend(javascript_route_registrations(
                &file.rel,
                line,
                line_number,
            ));
        } else if file.ext == "py" {
            routes.extend(python_route_decorators(&file.rel, line, line_number));
        } else if file.ext == "go" {
            routes.extend(go_route_registrations(&file.rel, line, line_number));
        }
    }
    routes
}

fn javascript_route_registrations(rel: &str, line: &str, line_number: usize) -> Vec<RuntimeRoute> {
    static_route_methods()
        .iter()
        .filter_map(|method| {
            let call = format!(".{method}(");
            let start = line.find(&call)?;
            if !route_like_receiver(&line[..start]) {
                return None;
            }
            let start = start + call.len();
            let path = quoted_literal_at(line[start..].trim_start())?;
            Some(RuntimeRoute {
                method: Some(method.to_ascii_uppercase()),
                path,
                file: rel.to_string(),
                evidence: "javascript_route_registration".to_string(),
                strength: EvidenceStrength::High,
                locations: vec![EvidenceLocation::line(
                    rel,
                    line_number,
                    "route_registration",
                )],
            })
        })
        .collect()
}

fn python_route_decorators(rel: &str, line: &str, line_number: usize) -> Vec<RuntimeRoute> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('@') {
        return Vec::new();
    }
    static_route_methods()
        .iter()
        .filter_map(|method| {
            let call = format!(".{method}(");
            let start = trimmed.find(&call)? + call.len();
            let path = quoted_literal_at(trimmed[start..].trim_start())?;
            Some(RuntimeRoute {
                method: Some(method.to_ascii_uppercase()),
                path,
                file: rel.to_string(),
                evidence: "python_route_decorator".to_string(),
                strength: EvidenceStrength::High,
                locations: vec![EvidenceLocation::line(
                    rel,
                    line_number,
                    "route_decorator",
                )],
            })
        })
        .collect()
}

fn go_route_registrations(rel: &str, line: &str, line_number: usize) -> Vec<RuntimeRoute> {
    let Some(start) = line
        .find("http.HandleFunc(")
        .or_else(|| line.find(".HandleFunc("))
    else {
        return Vec::new();
    };
    let Some(path) = quoted_literal_at(line[start..].split_once('(').map(|(_, tail)| tail).unwrap_or("")) else {
        return Vec::new();
    };
    vec![RuntimeRoute {
        method: Some("ANY".to_string()),
        path,
        file: rel.to_string(),
        evidence: "go_http_route_registration".to_string(),
        strength: EvidenceStrength::High,
        locations: vec![EvidenceLocation::line(rel, line_number, "route_registration")],
    }]
}

fn unknowns_for_file(project: &Project, file: &FileInfo) -> Vec<Unknown> {
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line_is_comment(line) {
            continue;
        }
        let line_number = index + 1;
        if dynamic_import_line(line) {
            out.push(unknown(
                "dynamic_import",
                Some(&file.rel),
                Some(line_number),
                "import target is not a static string literal",
                "runtime dependency target is not resolved structurally",
                Some(format!("codemap ls {}", shell_quote(&file.rel))),
            ));
        }
        if dynamic_require_line(line) {
            out.push(unknown(
                "js_require_dynamic",
                Some(&file.rel),
                Some(line_number),
                "require target is not a static string literal",
                "runtime dependency target is not resolved structurally",
                Some(format!("codemap ls {}", shell_quote(&file.rel))),
            ));
        }
        if dynamic_env_lookup_line(line) {
            out.push(unknown(
                "env_dynamic_lookup",
                Some(&file.rel),
                Some(line_number),
                "environment variable key is dynamic",
                "runtime config dependency cannot be named structurally",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if route_string_concat_line(line) {
            out.push(unknown(
                "route_string_concat",
                Some(&file.rel),
                Some(line_number),
                "route path is composed instead of a static literal",
                "runtime route cannot be mapped to an exact path structurally",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if raw_sql_literal_line(line) {
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
    for (index, line) in text.lines().enumerate() {
        if line_is_comment(line) {
            continue;
        }
        let line_number = index + 1;
        let Some((kind, evidence)) = side_effect_kind(line) else {
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

fn quoted_literal_at(value: &str) -> Option<String> {
    let value = value.trim_start();
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' && quote != '`' {
        return None;
    }
    if quote == '`' && value.contains("${") {
        return None;
    }
    let end = value[1..].find(quote)?;
    Some(value[1..1 + end].to_string())
}

fn static_route_methods() -> &'static [&'static str] {
    &["get", "post", "put", "patch", "delete", "all", "head", "options"]
}

fn dynamic_import_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    let Some(start) = code.find("import") else {
        return false;
    };
    let tail = &line[start + "import".len()..];
    let code_tail = code[start + "import".len()..].trim_start();
    code_tail.starts_with('(') && quoted_literal_at(tail.trim_start().trim_start_matches('(')).is_none()
}

fn dynamic_require_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    let Some(start) = code.find("require") else {
        return false;
    };
    let tail = &line[start + "require".len()..];
    let code_tail = code[start + "require".len()..].trim_start();
    code_tail.starts_with('(') && quoted_literal_at(tail.trim_start().trim_start_matches('(')).is_none()
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

fn quoted_literal_contents(line: &str) -> Vec<String> {
    let chars = line.chars().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let quote = chars[index];
        if !matches!(quote, '"' | '\'' | '`') {
            index += 1;
            continue;
        }
        let mut literal = String::new();
        let mut escaped = false;
        index += 1;
        while index < chars.len() {
            let ch = chars[index];
            if escaped {
                literal.push(ch);
                escaped = false;
            } else if ch == '\\' && quote != '`' {
                escaped = true;
            } else if ch == quote {
                out.push(literal);
                break;
            } else {
                literal.push(ch);
            }
            index += 1;
        }
        index += 1;
    }
    out
}

fn code_shape_without_literal_content(line: &str) -> String {
    let chars = line.chars().collect::<Vec<_>>();
    let mut out = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        let next = chars.get(index + 1).copied();
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                out.push(' ');
            } else if ch == '\\' && active_quote != '`' {
                escaped = true;
                out.push(' ');
            } else if ch == active_quote {
                quote = None;
                out.push(ch);
            } else {
                out.push(' ');
            }
            index += 1;
            continue;
        }
        if ch == '/' && next == Some('/') {
            out.extend(std::iter::repeat_n(' ', chars.len() - index));
            break;
        }
        if ch == '/' && next == Some('*') {
            out.push(' ');
            out.push(' ');
            index += 2;
            while index < chars.len() {
                if chars[index] == '*' && chars.get(index + 1) == Some(&'/') {
                    out.push(' ');
                    out.push(' ');
                    index += 2;
                    break;
                }
                out.push(' ');
                index += 1;
            }
            continue;
        }
        if ch == '#' {
            out.extend(std::iter::repeat_n(' ', chars.len() - index));
            break;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            escaped = false;
            out.push(ch);
            index += 1;
            continue;
        }
        out.push(ch);
        index += 1;
    }
    out
}

fn line_is_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
}

fn route_like_receiver(prefix: &str) -> bool {
    let receiver = prefix
        .trim_end()
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '$')
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    let receiver = receiver.to_ascii_lowercase();
    receiver == "app"
        || receiver == "api"
        || receiver == "router"
        || receiver == "server"
        || receiver == "fastify"
        || receiver.ends_with("router")
        || receiver.ends_with("server")
}
