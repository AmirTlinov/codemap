use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MAX_AGENTS_LINES: usize = 60;

#[test]
fn code_files_follow_400_line_ratchet() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = run_python(&root, "check-all");
    assert!(
        output.status.success(),
        "400-line code policy ratchet failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn code_policy_self_test_passes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = run_python(&root, "self-test");
    assert!(
        output.status.success(),
        "code-policy self-test failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
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

fn run_python(root: &Path, action: &str) -> Output {
    let script = root.join(".codex/hooks/code_policy.py");
    for executable in ["python3", "python"] {
        match Command::new(executable)
            .arg(&script)
            .arg(action)
            .current_dir(root)
            .output()
        {
            Ok(output) => return output,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => panic!("failed to run {executable}: {error}"),
        }
    }
    panic!("python3 or python is required for the repository code-policy hook")
}
