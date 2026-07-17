#[test]
fn benchmark_codex_runtime_exposes_only_auth_and_removes_trial_home() {
    let source = TempDir::new().expect("source Codex home");
    let support = TempDir::new().expect("runtime test support");
    write(&source.path().join("auth.json"), "fixture-auth\n");
    write(&source.path().join("config.toml"), "[plugins.browser]\nenabled = true\n");
    fs::create_dir(source.path().join("skills")).expect("source skills");
    fs::create_dir(source.path().join("plugins")).expect("source plugins");

    let probe = support.path().join("probe.py");
    write(
        &probe,
        r#"import json
import pathlib
import sys

sys.path.insert(0, sys.argv[1])
from benchmark_codex_runtime import codex_runtime_isolation_args, isolated_codex_runtime

source = pathlib.Path(sys.argv[2])
with isolated_codex_runtime({"CODEX_HOME": str(source), "SENTINEL": "kept"}) as runtime:
    home = pathlib.Path(runtime.env["CODEX_HOME"])
    assert home != source
    assert home.is_dir()
    assert runtime.env["SENTINEL"] == "kept"
    assert not (home / "config.toml").exists()
    assert not (home / "skills").exists()
    assert not (home / "plugins").exists()
    assert (home / "auth.json").is_symlink()
    assert (home / "auth.json").resolve() == (source / "auth.json").resolve()
    assert runtime.evidence() == {
        "codex_home": "isolated",
        "auth": "linked",
        "extensions": "disabled",
    }
    runtime_home = home
assert not runtime_home.exists()
args = codex_runtime_isolation_args()
disabled = [args[index + 1] for index in range(0, len(args), 2)]
assert all(args[index] == "--disable" for index in range(0, len(args), 2))
assert disabled == [
    "apps",
    "browser_use",
    "browser_use_external",
    "browser_use_full_cdp_access",
    "computer_use",
    "in_app_browser",
    "plugins",
    "remote_plugin",
]
"#,
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = python()
        .arg(&probe)
        .arg(repo_root.join("scripts"))
        .arg(source.path())
        .output()
        .expect("isolated Codex runtime probe");
    assert!(
        output.status.success(),
        "runtime probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
