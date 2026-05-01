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
    let Some((_, tail)) = line.split_once("import") else {
        return false;
    };
    let tail = tail.trim_start();
    (tail.starts_with('(') || tail.starts_with(" (")) && quoted_literal_at(tail.trim_start_matches('(')).is_none()
}

fn dynamic_require_line(line: &str) -> bool {
    let Some((_, tail)) = line.split_once("require") else {
        return false;
    };
    let tail = tail.trim_start();
    tail.starts_with('(') && quoted_literal_at(tail.trim_start_matches('(')).is_none()
}

fn dynamic_env_lookup_line(line: &str) -> bool {
    line.contains("process.env[")
        || line.contains("import.meta.env[")
        || dynamic_call_arg(line, "Deno.env.get(")
        || dynamic_call_arg(line, "std::env::var(")
        || dynamic_call_arg(line, "env::var(")
        || dynamic_call_arg(line, "os.getenv(")
        || line.contains("os.environ[") && !line.contains("os.environ[\"") && !line.contains("os.environ['")
}

fn dynamic_call_arg(line: &str, call: &str) -> bool {
    let Some((_, tail)) = line.split_once(call) else {
        return false;
    };
    quoted_literal_at(tail).is_none()
}

fn route_string_concat_line(line: &str) -> bool {
    static_route_methods().iter().any(|method| {
        let call = format!(".{method}(");
        line.find(&call).is_some_and(|start| {
            if !route_like_receiver(&line[..start]) {
                return false;
            }
            let arg = line[start + call.len()..].trim_start();
            quoted_literal_at(arg).is_none() && (arg.contains('+') || arg.contains("${"))
        })
    })
}

fn raw_sql_literal_line(line: &str) -> bool {
    let upper = line.to_ascii_uppercase();
    ["SELECT ", "INSERT INTO ", "UPDATE ", "DELETE FROM "]
        .iter()
        .any(|needle| upper.contains(needle))
        && (line.contains('"') || line.contains('\'') || line.contains('`'))
}

fn side_effect_kind(line: &str) -> Option<(&'static str, &'static str)> {
    if line.contains("fetch(") || line.contains("axios.") {
        Some(("network_call", "static_network_call"))
    } else if line.contains("localStorage.setItem")
        || line.contains("sessionStorage.setItem")
        || line.contains("fs.writeFile")
        || line.contains("std::fs::write")
        || line.contains("os.WriteFile")
    {
        Some(("storage_write", "static_storage_write"))
    } else if raw_sql_literal_line(line)
        && (line.to_ascii_uppercase().contains("INSERT INTO ")
            || line.to_ascii_uppercase().contains("UPDATE ")
            || line.to_ascii_uppercase().contains("DELETE FROM "))
    {
        Some(("database_write", "raw_sql_mutation"))
    } else {
        None
    }
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
