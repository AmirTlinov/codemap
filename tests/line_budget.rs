use std::fs;
use std::path::{Path, PathBuf};

const MAX_RUST_LINES: usize = 500;
const MAX_AGENTS_LINES: usize = 60;

#[test]
fn rust_source_and_test_files_stay_ai_sized() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut oversize = Vec::new();
    for scope in ["src", "tests"] {
        visit_files(&root.join(scope), &mut |path| {
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                return;
            }
            let lines = line_count(path);
            if lines > MAX_RUST_LINES {
                oversize.push(format!(
                    "{} has {lines} lines; max is {MAX_RUST_LINES}",
                    rel(&root, path)
                ));
            }
        });
    }
    assert!(
        oversize.is_empty(),
        "AI-friendly Rust file budget exceeded:\n{}",
        oversize.join("\n")
    );
}

#[test]
fn agents_maps_stay_short() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut oversize = Vec::new();
    visit_files(&root, &mut |path| {
        if path.file_name().and_then(|name| name.to_str()) != Some("AGENTS.md") {
            return;
        }
        let lines = line_count(path);
        if lines > MAX_AGENTS_LINES {
            oversize.push(format!(
                "{} has {lines} lines; max is {MAX_AGENTS_LINES}",
                rel(&root, path)
            ));
        }
    });
    assert!(
        oversize.is_empty(),
        "AGENTS.md map budget exceeded:\n{}",
        oversize.join("\n")
    );
}

fn visit_files(dir: &Path, f: &mut impl FnMut(&Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if matches!(name, ".git" | "target") {
            continue;
        }
        if path.is_dir() {
            visit_files(&path, f);
        } else {
            f(&path);
        }
    }
}

fn line_count(path: &Path) -> usize {
    fs::read_to_string(path)
        .map(|text| text.lines().count())
        .unwrap_or(0)
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
