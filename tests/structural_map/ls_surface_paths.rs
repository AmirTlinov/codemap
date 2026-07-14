// Responsibility: ls directory-surface `path` stays a real repo path or null

#[test]
fn ls_surface_paths_are_real_paths_and_script_path_is_the_defining_manifest() {
    let (repo, cache) = fixture();

    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let surfaces = json["directory"].as_array().expect("directory surfaces");
    for surface in surfaces {
        let Some(path) = surface["path"].as_str() else {
            continue;
        };
        assert!(
            repo.path().join(path).exists(),
            "directory surface `path` must be a real repo path or null, got `{path}`: {json:#}"
        );
    }
    let script_surface = surfaces
        .iter()
        .find(|surface| surface["kind"] == "script")
        .expect("script surface");
    assert_eq!(
        script_surface["path"], "package.json",
        "script surface path must be the defining manifest, not a `name: command` label: {json:#}"
    );
    assert!(
        script_surface["examples"]
            .as_array()
            .expect("script examples")
            .iter()
            .any(|example| {
                example
                    .as_str()
                    .is_some_and(|example| example.starts_with("test: "))
            }),
        "script `name: command` labels stay in examples: {json:#}"
    );
}
