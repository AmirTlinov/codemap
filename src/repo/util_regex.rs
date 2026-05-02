pub fn normalize_rel_path(path: &str) -> String {
    let mut out = path.replace('\\', "/");
    while out.contains("//") {
        out = out.replace("//", "/");
    }
    let mut parts = Vec::new();
    for part in out.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

pub fn tokenize(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map(str::to_ascii_lowercase)
        .filter(|s| s.len() >= 2)
        .collect()
}

pub fn path_tokens(rel: &str) -> BTreeSet<String> {
    tokenize(&rel.replace(['/', '-', '_'], " "))
}

fn unique_strings(items: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

fn unique_pairs(items: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

fn unique_triples(items: Vec<(String, String, String)>) -> Vec<(String, String, String)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

pub fn is_source_ext(ext: &str) -> bool {
    SOURCE_EXTS.iter().any(|x| x == &ext)
}

pub fn is_asset_ext(ext: &str) -> bool {
    ASSET_EXTS.iter().any(|x| x == &ext)
}

fn is_snapshot_ext(ext: &str) -> bool {
    matches!(ext, "snap" | "snapshot")
}

fn identifier_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"[A-Za-z_$][A-Za-z0-9_$]*"#).expect("valid identifier regex"))
}

fn jsx_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"<\s*/?\s*([A-Z][A-Za-z0-9_$]*)\b"#).expect("valid jsx tag regex")
    })
}

fn js_function_params_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)\bfunction(?:\s+[A-Za-z_$][A-Za-z0-9_$]*)?\s*\((?P<params>[^)]*)\)"#)
            .expect("valid js function params regex")
    })
}

fn js_arrow_params_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)\((?P<params>[^)]*)\)\s*(?::\s*[^=]+?)?=>"#)
            .expect("valid js arrow params regex")
    })
}

fn js_labelledby_local_binding_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)\b(?:const|let|var)\s+(?:labelledBy\b|\{[^}]*\blabelledBy\b)|\bfunction\s+labelledBy\b|\bclass\s+labelledBy\b"#)
            .expect("valid js labelledBy local binding regex")
    })
}

fn js_method_params_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?s)\b(?P<name>[A-Za-z_$][A-Za-z0-9_$]*)\s*\((?P<params>[^)]*)\)\s*(?::\s*[^={]+?)?\{"#,
        )
        .expect("valid js method params regex")
    })
}

fn js_single_arrow_param_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)(?:^|[=(:,]\s*)(?P<param>[A-Za-z_$][A-Za-z0-9_$]*)\s*=>"#)
            .expect("valid js single arrow param regex")
    })
}

fn js_for_binding_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)\bfor\s*(?:await\s*)?\(\s*(?:const|let|var)\s+(?P<binding>[^;)]*?)\s+(?:of|in)\b"#)
            .expect("valid js for binding regex")
    })
}

fn js_catch_param_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)\bcatch\s*\(\s*(?P<param>[^)]*?)\s*\)"#)
            .expect("valid js catch param regex")
    })
}

fn js_static_import_statement_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?s)^\s*import\s+(?P<type>type\s+)?(?:(?P<clause>.+?)\s+from\s*)?['"](?P<spec>[^'"]+)['"]"#,
        )
        .expect("valid js static import statement regex")
    })
}

fn js_reexport_statement_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)^\s*export\s+(?P<type>type\s+)?(?:(?P<star>\*)|\{(?P<named>.*?)\})\s+from\s*['"](?P<spec>[^'"]+)['"]"#)
            .expect("valid js re-export statement regex")
    })
}

fn js_export_from_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)^\s*export\s+.+?\s+from\s*['"](?P<spec>[^'"]+)['"]"#)
            .expect("valid js export-from regex")
    })
}

fn js_export_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"\bexport\s+(?:default\s+)?(?:class|function|const|let|var|interface|type|enum)\s+([A-Za-z0-9_]+)"#)
            .expect("valid js export regex")
    })
}

fn css_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*@import\s+(?:url\(\s*)?['"](?P<spec>[^'"]+)['"]"#)
            .expect("valid css import regex")
    })
}

fn js_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*(?P<export>export\s+)?(?:default\s+)?(?:async\s+)?(?P<kind>function|class|const|let|var|interface|type|enum)\s+(?P<name>[A-Za-z_$][A-Za-z0-9_$]*)"#)
            .expect("valid js symbol regex")
    })
}

fn js_default_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*export\s+default\s+(?:async\s+)?(?P<kind>function|class)\b(?:\s+(?P<name>[A-Za-z_$][A-Za-z0-9_$]*))?"#)
            .expect("valid js default symbol regex")
    })
}

fn py_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(?:from\s+([A-Za-z0-9_\.]+)\s+import|import\s+([A-Za-z0-9_\.]+))"#)
            .expect("valid python import regex")
    })
}

fn swift_package_name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"name:\s*"([^"]+)""#).expect("valid swift package name regex"))
}

fn swift_package_path_dependency_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"\.package\s*\(\s*path:\s*"([^"]+)""#)
            .expect("valid swift package path dependency regex")
    })
}

fn swift_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(?:@\w+(?:\([^)]*\))?\s+)?import\s+(?:(?:class|struct|enum|protocol|func|var|typealias)\s+)?([A-Za-z_][A-Za-z0-9_]*)"#)
            .expect("valid swift import regex")
    })
}

fn swift_type_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*(?:@\w+(?:\([^)]*\))?\s+)*(?P<mods>(?:(?:public|open|package|internal|fileprivate|private|final|static|class|indirect)\s+)*)?(?P<kind>class|struct|enum|protocol|actor)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"#)
            .expect("valid swift type symbol regex")
    })
}

fn swift_func_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*(?:@\w+(?:\([^)]*\))?\s+)*(?P<mods>(?:(?:public|open|package|internal|fileprivate|private|static|class|mutating|nonmutating|override|final)\s+)*)?func\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\("#)
            .expect("valid swift function symbol regex")
    })
}

fn swift_property_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*(?:@\w+(?:\([^)]*\))?\s+)*(?P<mods>(?:(?:public|open|package|internal|fileprivate|private|static|class|weak|unowned|lazy|override|final)\s+)*)?(?P<kind>let|var)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\b"#)
            .expect("valid swift property symbol regex")
    })
}

fn py_def_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^(?:class|def)\s+([A-Za-z0-9_]+)"#).expect("valid py def regex")
    })
}

fn rust_use_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(?:use|pub\s+use)\s+([A-Za-z0-9_:]+)"#).expect("valid rust use regex")
    })
}

fn rust_mod_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(?:pub\s+)?mod\s+([A-Za-z0-9_]+)\s*;"#).expect("valid rust mod regex")
    })
}

fn rust_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*(?P<pub>pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?P<kind>fn|struct|enum|trait|mod)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"#)
            .expect("valid rust symbol regex")
    })
}

fn rust_impl_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*impl(?:<[^>]+>)?\s+(?P<name>[A-Za-z_][A-Za-z0-9_:<>]*(?:\s+for\s+[A-Za-z_][A-Za-z0-9_:<>]*)?)"#)
            .expect("valid rust impl regex")
    })
}

fn python_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*(?:async\s+)?(?P<kind>def|class)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"#)
            .expect("valid python symbol regex")
    })
}

fn go_func_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*func\s+(?P<receiver>\([^)]*\)\s*)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\("#)
            .expect("valid go function symbol regex")
    })
}

fn go_type_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*type\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s+(?P<kind>struct|interface|[A-Za-z_][A-Za-z0-9_]*)"#)
            .expect("valid go type symbol regex")
    })
}
