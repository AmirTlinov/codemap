#[test]
fn cone_resolves_rust_crate_super_and_symbol_path_imports_inside_packages() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/core\"]\n",
    );
    write(
        &repo.path().join("crates/core/Cargo.toml"),
        "[package]\nname = \"core-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(
        &repo.path().join("crates/core/src/lib.rs"),
        "pub mod client;\npub mod network_guard;\npub mod tun;\npub mod vpn_config;\n",
    );
    write(
        &repo.path().join("crates/core/src/client/mod.rs"),
        "pub mod args;\npub mod routing;\n",
    );
    write(
        &repo.path().join("crates/core/src/client/args.rs"),
        "pub struct Args;\n",
    );
    write(
        &repo.path().join("crates/core/src/network_guard.rs"),
        "pub struct DnsRestoreMethod;\npub struct NetworkStateGuard;\n",
    );
    write(
        &repo.path().join("crates/core/src/tun.rs"),
        "pub fn setup_routing() {}\n",
    );
    write(
        &repo.path().join("crates/core/src/vpn_config.rs"),
        "pub struct VpnConfig;\n",
    );
    write(
        &repo.path().join("crates/core/src/client/routing.rs"),
        "use crate::network_guard::DnsRestoreMethod;\nuse crate::network_guard::NetworkStateGuard;\nuse crate::tun::setup_routing;\nuse crate::vpn_config::VpnConfig;\nuse super::args::Args;\n\npub fn setup_client_routing(_args: Args) {\n    setup_routing();\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "rust crate path fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "crates/core/src/client/routing.rs",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    for target in [
        "crates/core/src/network_guard.rs",
        "crates/core/src/tun.rs",
        "crates/core/src/vpn_config.rs",
        "crates/core/src/client/args.rs",
    ] {
        assert!(
            cone["outgoing"]
                .as_array()
                .expect("outgoing")
                .iter()
                .any(|edge| {
                    edge["to"] == target
                        && edge["type"] == "imports"
                        && edge["evidence"] == "resolved_import"
                }),
            "Rust crate/super import should resolve to {target}: {cone:#}"
        );
    }
    assert!(
        cone["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .all(|unknown| unknown["kind"] != "unresolved_import"),
        "resolved Rust crate/super imports should not remain unresolved unknowns: {cone:#}"
    );
}
