#[test]
fn ci_owner_cone_keeps_step_kind_diversity_under_default_limit() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"large-ci-fixture","private":true,"scripts":{"release:prod":"node scripts/release.js"}}"#,
    );
    write(&repo.path().join("scripts/release.js"), "console.log('release')\n");
    let mut workflow =
        "name: ci\non: [push]\njobs:\n  ci:\n    runs-on: ubuntu-latest\n    steps:\n"
            .to_string();
    for index in 0..25 {
        workflow.push_str(&format!("      - run: cargo test -p crate{index}\n"));
    }
    workflow.push_str(
        "      - run: pnpm release:prod\n      - run: pnpm install --frozen-lockfile\n      - run: echo \"done\"\n",
    );
    write(&repo.path().join(".github/workflows/ci.yml"), &workflow);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "large ci cone"]);

    let cone = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", ".github/workflows/ci.yml", "--depth", "1"])
        .output()
        .expect("ci cone should run");
    assert!(
        cone.status.success(),
        "ci cone failed: {}",
        String::from_utf8_lossy(&cone.stderr)
    );
    let markdown = String::from_utf8(cone.stdout).expect("markdown utf8");
    for expected in [
        "ci_validation_step -> `cargo test -p crate0`",
        "ci_release_step -> `pnpm release:prod`",
        "ci_setup_step -> `pnpm install --frozen-lockfile`",
        "ci_control_step -> `echo \"done\"`",
        "outgoing edges hidden by limit",
    ] {
        assert!(
            markdown.contains(expected),
            "large CI cone should keep representative step kinds before hidden expansion: {expected}\n{markdown}"
        );
    }
}

#[test]
fn ci_owner_cone_uses_logical_shell_commands_for_block_continuations() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  ci:\n    runs-on: ubuntu-latest\n    steps:\n      - run: |\n          CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \\\n            cargo build --release \\\n            -p masque-core\n          python3 - <<'PY'\n          print('not a shell command')\n          PY\n          case \"${target}\" in\n            linux)\n              echo linux\n              ;;\n            *)\n              echo other\n              ;;\n          esac\n          ./scripts/check-doc-links\n          pnpm release:prod \\\n            --output release_artifact_manifest_v1.json\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ci continuations"]);

    let cone = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", ".github/workflows/ci.yml", "--depth", "1"])
        .output()
        .expect("ci cone should run");
    assert!(
        cone.status.success(),
        "ci cone failed: {}",
        String::from_utf8_lossy(&cone.stderr)
    );
    let markdown = String::from_utf8(cone.stdout).expect("markdown utf8");
    assert!(
        markdown.contains(
            "ci_validation_step -> `CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc cargo build --release -p masque-core`"
        ) && markdown.contains(
            "ci_validation_step -> `./scripts/check-doc-links`"
        ) && markdown.contains(
            "ci_release_step -> `pnpm release:prod --output release_artifact_manifest_v1.json`"
        ),
        "CI cone should join shell continuation lines into logical commands: {markdown}"
    );
    assert!(
        !markdown.contains("ci_release_step -> `--output release_artifact_manifest_v1.json`")
            && !markdown.contains("ci_validation_step -> `-p masque-core`")
            && !markdown.contains("print('not a shell command')")
            && !markdown.contains("ci_control_step -> `linux)`")
            && !markdown.contains("ci_control_step -> `*)`")
            && !markdown.contains("ci_control_step -> `;;`")
            && !markdown.contains("ci_control_step -> `esac`"),
        "CI cone must not classify shell continuation, heredoc, or syntax-only fragments as standalone commands: {markdown}"
    );
}
