use codemap_fixture_rust_cli::config_version;

#[test]
fn config_has_version() {
    assert_eq!(config_version(), 1);
}
