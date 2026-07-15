fn executable_sha256(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(fs::read(path).expect("read executable")))
}

#[cfg(unix)]
fn make_fake_codemap(dir: &Path, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("codemap");
    write(&path, body);
    let mut permissions = fs::metadata(&path).expect("fake metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake executable");
    path
}

#[cfg(unix)]
#[test]
fn doctor_attributes_candidate_and_different_path_binary() {
    let repo = TempDir::new().expect("identity repo");
    let cache = TempDir::new().expect("identity cache");
    let bin = TempDir::new().expect("identity bin");
    git(repo.path(), &["init", "-q"]);
    write(&repo.path().join("README.md"), "identity fixture\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["-c", "user.email=a@example.com", "-c", "user.name=a", "commit", "-qm", "fixture"]);
    let log = bin.path().join("probe.log");
    let fake = make_fake_codemap(
        bin.path(),
        &format!(
            "#!/bin/sh\ntouch path-probe-wrote\nprintf '%s\\n' \"$*\" >> '{}'\n[ \"$1\" = '--version' ] || exit 71\nprintf 'codemap 0.1.7\\n'\n",
            log.display()
        ),
    );
    let before = Command::new("git")
        .args(["status", "--porcelain=v1", "-z"])
        .current_dir(repo.path())
        .output()
        .expect("git status before")
        .stdout;
    let path = std::env::join_paths(
        std::iter::once(bin.path().to_path_buf()).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )),
    )
    .expect("fixture PATH");
    let output = codemap()
        .current_dir(repo.path())
        .env("PATH", &path)
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["doctor", "--format", "json"])
        .output()
        .expect("doctor identity fixture");
    assert!(
        output.status.success(),
        "doctor failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let doctor: Value = serde_json::from_slice(&output.stdout).expect("doctor json");
    assert_schema("schemas/status.schema.json", &doctor);
    let running = Path::new(doctor["build_identity"]["executable_path"].as_str().unwrap());
    let expected_running = Path::new(env!("CARGO_BIN_EXE_codemap"))
        .canonicalize()
        .expect("candidate executable");
    assert_eq!(running, expected_running);
    assert_eq!(doctor["build_identity"]["semver"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        doctor["build_identity"]["binary_sha256"],
        executable_sha256(&expected_running)
    );
    assert_eq!(doctor["path_identity"]["executable_path"], fake.canonicalize().unwrap().to_string_lossy().as_ref());
    assert_eq!(doctor["path_identity"]["semver"], "0.1.7");
    assert_eq!(doctor["path_identity"]["binary_sha256"], executable_sha256(&fake));
    assert_eq!(doctor["executable_mismatch"], true);
    let markdown = codemap()
        .current_dir(repo.path())
        .env("PATH", &path)
        .env("CODEMAP_CACHE_DIR", cache.path())
        .arg("doctor")
        .output()
        .expect("doctor identity markdown");
    assert!(markdown.status.success());
    let markdown = String::from_utf8_lossy(&markdown.stdout);
    assert!(markdown.contains(&format!(
        "| Running version | {} |",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(markdown.contains("| PATH version | 0.1.7 |"));
    assert!(markdown.contains("| Executable mismatch | true |"));
    assert_eq!(
        fs::read_to_string(&log).expect("version probe log"),
        "--version\n--version\n"
    );
    let after = Command::new("git")
        .args(["status", "--porcelain=v1", "-z"])
        .current_dir(repo.path())
        .output()
        .expect("git status after")
        .stdout;
    assert_eq!(before, after, "doctor must keep zero target-repo footprint");
    assert!(
        !repo.path().join("path-probe-wrote").exists(),
        "PATH version probes must run outside the target repository"
    );
}

#[cfg(unix)]
#[test]
fn doctor_canonicalizes_path_symlink_to_running_binary() {
    use std::os::unix::fs::symlink;
    let (repo, cache) = fixture();
    let bin = TempDir::new().expect("identity symlink bin");
    symlink(env!("CARGO_BIN_EXE_codemap"), bin.path().join("codemap"))
        .expect("symlink candidate");
    let path = std::env::join_paths(
        std::iter::once(bin.path().to_path_buf()).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )),
    )
    .expect("fixture PATH");
    let output = codemap()
        .current_dir(repo.path())
        .env("PATH", path)
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["doctor", "--format", "json"])
        .output()
        .expect("doctor symlink fixture");
    assert!(output.status.success());
    let doctor: Value = serde_json::from_slice(&output.stdout).expect("doctor json");
    assert_eq!(doctor["executable_mismatch"], false);
    assert_eq!(doctor["path_identity"]["version_probe"], "same_executable");
    assert_eq!(
        doctor["path_identity"]["executable_path"],
        doctor["build_identity"]["executable_path"]
    );
}

#[cfg(unix)]
#[test]
fn doctor_bounds_a_hanging_path_version_probe() {
    let (repo, cache) = fixture();
    let bin = TempDir::new().expect("hanging identity bin");
    make_fake_codemap(bin.path(), "#!/bin/sh\nexec /bin/sleep 10\n");
    let path = std::env::join_paths(
        std::iter::once(bin.path().to_path_buf()).chain(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        )),
    )
    .expect("fixture PATH");
    let started = std::time::Instant::now();
    let output = codemap()
        .current_dir(repo.path())
        .env("PATH", path)
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["doctor", "--format", "json"])
        .output()
        .expect("doctor timeout fixture");
    assert!(output.status.success());
    assert!(started.elapsed() < std::time::Duration::from_secs(6));
    let doctor: Value = serde_json::from_slice(&output.stdout).expect("doctor json");
    assert_eq!(doctor["path_identity"]["version_probe"], "unavailable");
    assert_eq!(doctor["path_identity"]["semver"], Value::Null);
    assert_eq!(doctor["executable_mismatch"], true);
}
