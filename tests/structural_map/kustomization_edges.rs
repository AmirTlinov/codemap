// Responsibility: Kustomization resource topology

#[test]
fn kustomization_resources_are_structural_parent_child_edges() {
    let repo = TempDir::new().expect("kustomization repo");
    let cache = TempDir::new().expect("kustomization cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("deploy/k8s/base/kustomization.yaml"),
        "resources:\n  - deployment.yaml\n  - backup\n",
    );
    write(
        &repo.path().join("deploy/k8s/base/deployment.yaml"),
        "apiVersion: apps/v1\nkind: Deployment\n",
    );
    write(
        &repo
            .path()
            .join("deploy/k8s/base/backup/kustomization.yaml"),
        "resources:\n  - cronjob.yaml\n",
    );
    write(
        &repo.path().join("deploy/k8s/base/backup/cronjob.yaml"),
        "apiVersion: batch/v1\nkind: CronJob\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "kustomization topology"]);

    let parent = run_json(
        repo.path(),
        cache.path(),
        &[
            "ls",
            "deploy/k8s/base/kustomization.yaml",
            "--format",
            "json",
        ],
    );
    let parent_imports = parent["edges"]
        .as_array()
        .expect("parent edges")
        .iter()
        .filter(|edge| edge["type"] == "imports")
        .map(|edge| edge["to"].as_str().expect("import target"))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        parent_imports,
        BTreeSet::from([
            "deploy/k8s/base/backup/kustomization.yaml",
            "deploy/k8s/base/deployment.yaml",
        ]),
        "parent Kustomization should resolve file and directory resources: {parent:#}"
    );

    let child = run_json(
        repo.path(),
        cache.path(),
        &[
            "ls",
            "deploy/k8s/base/backup/kustomization.yaml",
            "--format",
            "json",
        ],
    );
    assert!(
        child["edges"]
            .as_array()
            .expect("child edges")
            .iter()
            .any(|edge| edge["type"] == "imports"
                && edge["to"] == "deploy/k8s/base/backup/cronjob.yaml"),
        "child Kustomization should keep its resource edge: {child:#}"
    );
}
