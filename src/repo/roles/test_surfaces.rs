// Responsibility: repo-roles-test-surfaces
use crate::model::FileInfo;
use crate::repo::code_without_comments_or_strings;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

pub(crate) fn is_test_path(rel: &str, ext: &str) -> bool {
    rel.contains("/tests/")
        || rel.contains("/test/")
        || rel.starts_with("tests/")
        || rel.starts_with("test/")
        || rel.contains("/__tests__/")
        || rel.contains(".test.")
        || rel.contains(".spec.")
        || rel.ends_with("_test.rs")
        || rel.ends_with("_test.go")
        || (ext == "py"
            && rel
                .rsplit('/')
                .next()
                .map(|name| name.starts_with("test_"))
                .unwrap_or(false))
}

pub(crate) fn is_e2e_test_path(rel: &str) -> bool {
    let rel = rel.to_ascii_lowercase();
    rel.contains("/e2e/")
        || rel.contains("/e2e-")
        || rel.contains(".e2e.")
        || rel.contains("/playwright/")
        || rel.contains("/cypress/")
}

pub(crate) fn is_test_support_path(rel: &str) -> bool {
    let rel = rel.to_ascii_lowercase();
    let name = Path::new(&rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    rel.contains("/support/")
        || rel.contains("/helpers/")
        || rel.contains("/fixtures/")
        || rel.contains("/mocks/")
        || rel.contains("/setup")
        || rel.contains(".setup.")
        || name.starts_with("support_")
        || name.starts_with("support-")
        || name.starts_with("helper_")
        || name.starts_with("helper-")
}

pub(crate) fn source_has_test_declaration(root: &Path, info: &FileInfo) -> bool {
    if info.content_hash.is_none() {
        return false;
    }
    let Ok(text) = fs::read_to_string(root.join(&info.rel)) else {
        return false;
    };
    match info.ext.as_str() {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => {
            js_test_declaration_re().is_match(&code_without_comments_or_strings(&text, &info.ext))
        }
        "rs" => text.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("#[test")
                || (trimmed.starts_with("#[") && trimmed.contains("::test"))
        }),
        "py" => py_test_declaration_re().is_match(&text),
        "go" => go_test_declaration_re().is_match(&text),
        "swift" => swift_test_declaration_re().is_match(&text),
        _ => false,
    }
}

fn js_test_declaration_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)(^|[^A-Za-z0-9_$])(test|it|describe)(\s*\.\s*describe)?\s*\("#)
            .expect("valid js test declaration regex")
    })
}

fn py_test_declaration_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(async\s+def|def)\s+test_[A-Za-z0-9_]*\s*\("#)
            .expect("valid python test declaration regex")
    })
}

fn go_test_declaration_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*func\s+Test[A-Za-z0-9_]*\s*\("#)
            .expect("valid go test declaration regex")
    })
}

fn swift_test_declaration_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*func\s+test[A-Za-z0-9_]*\s*\("#)
            .expect("valid swift test declaration regex")
    })
}
