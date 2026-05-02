fn is_build_ci_surface(rel: &str, name: &str, ext: &str, tokens: &BTreeSet<String>) -> bool {
    is_explicit_ci_dir(rel)
        || (rel.starts_with(".github/actions/") && matches!(name, "action.yml" | "action.yaml"))
        || is_known_build_ci_name(name)
        || name.starts_with("dockerfile")
        || is_yaml_ci_name_or_path(rel, name, ext, tokens)
}

fn is_explicit_ci_dir(rel: &str) -> bool {
    rel.starts_with(".github/workflows/")
        || rel.starts_with(".circleci/")
        || rel.starts_with(".buildkite/")
        || rel.starts_with(".teamcity/")
}

fn is_known_build_ci_name(name: &str) -> bool {
    matches!(
        name,
        ".gitlab-ci.yml"
            | ".gitlab-ci.yaml"
            | ".travis.yml"
            | "azure-pipelines.yml"
            | "azure-pipelines.yaml"
            | "bitbucket-pipelines.yml"
            | "bitbucket-pipelines.yaml"
            | "cloudbuild.yml"
            | "cloudbuild.yaml"
            | "codemagic.yml"
            | "codemagic.yaml"
            | "bitrise.yml"
            | "bitrise.yaml"
            | ".drone.yml"
            | ".drone.yaml"
            | ".woodpecker.yml"
            | ".woodpecker.yaml"
            | "jenkinsfile"
            | "docker-compose.yml"
            | "docker-compose.yaml"
            | "compose.yml"
            | "compose.yaml"
            | "makefile"
            | "justfile"
            | "taskfile"
            | "taskfile.yml"
            | "taskfile.yaml"
            | "earthfile"
            | "build.gradle"
            | "build.gradle.kts"
    )
}

fn is_yaml_ci_name_or_path(
    rel: &str,
    name: &str,
    ext: &str,
    tokens: &BTreeSet<String>,
) -> bool {
    !is_source_ext(ext)
        && matches!(ext, "yml" | "yaml")
        && (matches!(
            name,
            "ci.yml"
                | "ci.yaml"
                | "workflow.yml"
                | "workflow.yaml"
                | "pipeline.yml"
                | "pipeline.yaml"
                | "release.yml"
                | "release.yaml"
        ) || rel.starts_with("ci/")
            || rel.starts_with(".ci/")
            || rel.contains("/ci/")
            || tokens.contains("workflow")
            || tokens.contains("workflows")
            || tokens.contains("pipeline")
            || tokens.contains("pipelines"))
}
