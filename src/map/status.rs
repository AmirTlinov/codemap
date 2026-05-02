#[derive(Debug, Serialize)]
pub struct StatusReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub root: String,
    pub cwd: String,
    pub vcs: Option<String>,
    pub config: Option<String>,
    pub config_errors: Vec<String>,
    pub nearest_agents: Option<String>,
    pub cache_dir: String,
    pub cache_state: String,
    pub cache_strategy: String,
    pub cache_artifacts: Vec<crate::model::CacheArtifactStatus>,
    pub scanner: crate::model::ScanStats,
    pub timings: crate::model::ProjectTimings,
    pub zero_footprint_default: bool,
    pub package_manager: String,
    pub languages: Vec<String>,
    pub files_scanned: usize,
    pub files_reused: usize,
    pub domains: Vec<DomainStatus>,
    pub scripts: Vec<String>,
    pub fingerprint: String,
    pub boundary_findings: usize,
    pub unclassified_source_files: Vec<String>,
    pub unclassified_count: usize,
}

#[derive(Debug, Serialize)]
pub struct DomainStatus {
    pub id: String,
    pub path: String,
    pub config: Option<String>,
}

pub fn status_report(project: &Project) -> StatusReport {
    let unclassified: Vec<String> = project
        .files
        .values()
        .filter(|file| repo::is_source_ext(&file.ext) && file.roles.is_empty())
        .map(|file| file.rel.clone())
        .collect();
    StatusReport {
        kind: "status_report",
        schema_version: "4",
        root: project.root.to_string_lossy().to_string(),
        cwd: project.cwd.to_string_lossy().to_string(),
        vcs: project.vcs.clone(),
        config: project.config_path.clone(),
        config_errors: project
            .config_errors
            .iter()
            .map(|error| format!("{}: {}", error.path, error.error))
            .collect(),
        nearest_agents: project.nearest_agents.clone(),
        cache_dir: project.cache_dir.to_string_lossy().to_string(),
        cache_state: project.cache_state.clone(),
        cache_strategy: project.cache_strategy.clone(),
        cache_artifacts: project.cache_artifacts.clone(),
        scanner: project.scan_stats.clone(),
        timings: project.timings.clone(),
        zero_footprint_default: true,
        package_manager: project.package_manager.clone(),
        languages: project.languages.iter().cloned().collect(),
        files_scanned: project.files.len(),
        files_reused: project.files_reused,
        domains: project
            .domains
            .iter()
            .map(|d| DomainStatus {
                id: d.id.clone(),
                path: d.path.clone(),
                config: d.config_path.clone(),
            })
            .collect(),
        scripts: project.scripts.iter().map(|s| s.command.clone()).collect(),
        fingerprint: cache::fingerprint(project, None),
        boundary_findings: boundary_findings(project, None).len(),
        unclassified_count: unclassified.len(),
        unclassified_source_files: unclassified.into_iter().take(30).collect(),
    }
}
