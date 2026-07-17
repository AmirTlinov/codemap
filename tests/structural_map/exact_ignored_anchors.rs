// Responsibility: exact navigation through bounded common-ignore directories
#[test]
fn exact_ignored_anchors_are_hydrated_without_expanding_root_inventory() {
    let repo = TempDir::new().expect("ignored exact repo");
    let cache = TempDir::new().expect("ignored exact cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("README.md"), "# Exact ignored anchor\n");
    write(
        &repo
            .path()
            .join("vendor/browser_extension/service_worker.js"),
        "export async function dispatchRpc(request) {\n  return request.method;\n}\n",
    );
    git(repo.path(), &["add", "README.md"]);
    git(
        repo.path(),
        &["add", "-f", "vendor/browser_extension/service_worker.js"],
    );
    git(repo.path(), &["commit", "-qm", "tracked vendor anchor"]);

    let root_before = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert!(!json_text(&root_before).contains("dispatchRpc"));

    let ls_args = [
        "ls",
        "vendor/browser_extension/service_worker.js",
        "--format",
        "json",
    ];
    let cold_ls = run_json(repo.path(), cache.path(), &ls_args);
    let warm_ls = run_json(repo.path(), cache.path(), &ls_args);
    assert_eq!(cold_ls, warm_ls, "exact ignored ls cache drift");
    assert!(json_text(&cold_ls).contains("dispatchRpc"), "{cold_ls:#}");

    let cone_args = [
        "cone",
        "vendor/browser_extension/service_worker.js#dispatchRpc",
        "--format",
        "json",
    ];
    let cold_cone = run_json(repo.path(), cache.path(), &cone_args);
    let warm_cone = run_json(repo.path(), cache.path(), &cone_args);
    assert_eq!(cold_cone, warm_cone, "exact ignored cone cache drift");
    assert_eq!(
        cold_cone["anchor"]["path"],
        "vendor/browser_extension/service_worker.js#dispatchRpc",
        "{cold_cone:#}"
    );

    let flow = run_json(
        repo.path(),
        cache.path(),
        &[
            "flow",
            "vendor/browser_extension/service_worker.js#dispatchRpc",
            "--format",
            "json",
        ],
    );
    assert!(
        flow["steps"]
            .as_array()
            .expect("flow steps")
            .iter()
            .any(|step| step["kind"] == "symbol_anchor"
                && step["anchor"]
                    == "vendor/browser_extension/service_worker.js#dispatchRpc"),
        "flow must hydrate an exact tracked ignored symbol: {flow:#}"
    );
    assert!(
        flow["unknown_breaks"]
            .as_array()
            .expect("flow unknowns")
            .iter()
            .all(|unknown| unknown["kind"] != "missing_symbol_anchor"),
        "hydrated symbol must not be reported missing: {flow:#}"
    );

    let root_after = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_eq!(root_before, root_after, "exact hydration polluted root inventory");
}

#[test]
fn exact_ignored_oversized_file_keeps_its_unread_body_boundary() {
    let repo = TempDir::new().expect("ignored oversized repo");
    let cache = TempDir::new().expect("ignored oversized cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    let mut body = "export function mustRemainUnread() {}\n".to_string();
    body.push_str(&"x".repeat(910_000));
    write(&repo.path().join("vendor/oversized.js"), &body);
    git(repo.path(), &["add", "-f", "vendor/oversized.js"]);
    git(repo.path(), &["commit", "-qm", "tracked oversized vendor file"]);

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "vendor/oversized.js", "--format", "json"],
    );
    assert!(!json_text(&ls).contains("mustRemainUnread"), "{ls:#}");
    assert!(
        ls["anchor"]["symbols"].as_array().is_some_and(Vec::is_empty),
        "{ls:#}"
    );
}

#[cfg(unix)]
#[test]
fn exact_ignored_symlink_never_hydrates_its_target() {
    use std::os::unix::fs::symlink;

    let repo = TempDir::new().expect("ignored symlink repo");
    let cache = TempDir::new().expect("ignored symlink cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("outside.js"),
        "export function symlinkTargetMustRemainUnread() {}\n",
    );
    fs::create_dir_all(repo.path().join("vendor")).expect("vendor dir");
    symlink("../outside.js", repo.path().join("vendor/link.js")).expect("tracked symlink");
    git(repo.path(), &["add", "outside.js"]);
    git(repo.path(), &["add", "-f", "vendor/link.js"]);
    git(repo.path(), &["commit", "-qm", "tracked vendor symlink"]);

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "vendor/link.js", "--format", "json"],
    );
    assert!(!json_text(&ls).contains("symlinkTargetMustRemainUnread"), "{ls:#}");
}

fn json_text(value: &Value) -> String {
    serde_json::to_string(value).expect("json text")
}
