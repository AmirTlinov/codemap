// Responsibility: incremental-project-fact-and-index-reconstruction
use crate::cache;
use crate::model::{
    CodemapConfig, ConfigLoadError, Domain, FileInfo, PackageDependency, PackageInfo, Project,
    ProjectTimings, ScanGroup, ScanStats, ScriptInfo,
};
use crate::repo::{
    VERSION, apply_codemap_config_roles, detect_languages, detect_package_edges,
    detect_package_manager, detect_packages, detect_scripts, detect_ts_path_aliases,
    discover_domains, enrich_accessible_surfaces_from_component_contracts, resolve_imports,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Instant;

pub(crate) struct ProjectBuildInput {
    pub root: PathBuf,
    pub cwd: PathBuf,
    pub vcs: Option<String>,
    pub cache_dir: PathBuf,
    pub config_path: Option<String>,
    pub config_errors: Vec<ConfigLoadError>,
    pub nearest_agents: Option<String>,
    pub anchors: CodemapConfig,
    pub files: BTreeMap<String, FileInfo>,
    pub scan_stats: ScanStats,
    pub cache_strategy: String,
    pub files_reused: usize,
    pub files_rebuilt: usize,
    pub old_cached_files: Option<(String, BTreeMap<String, FileInfo>)>,
}

pub(crate) fn build_project_from_files(input: ProjectBuildInput) -> Project {
    let facts_started = Instant::now();
    let derived = derive_project_facts(
        &input.root,
        input.files,
        &input.anchors,
        input.config_path.as_deref(),
    );
    let facts_ms = facts_started.elapsed().as_millis();
    let reverse_started = Instant::now();
    let reverse_update = if let Some((fingerprint, old_files)) = &input.old_cached_files {
        cache::incremental_reverse_imports(
            &input.cache_dir,
            VERSION,
            &input.root,
            fingerprint,
            old_files,
            &derived.files,
        )
    } else {
        cache::full_reverse_imports(&derived.files)
    };
    let reverse_index_ms = reverse_started.elapsed().as_millis();
    Project {
        root: input.root,
        cwd: input.cwd,
        vcs: input.vcs,
        cache_dir: input.cache_dir,
        config_path: input.config_path,
        config_errors: input.config_errors,
        nearest_agents: input.nearest_agents,
        files: derived.files,
        reverse_imports: reverse_update.index,
        packages: derived.packages,
        package_edges: derived.package_edges,
        domains: derived.domains,
        package_manager: derived.package_manager,
        scripts: derived.scripts,
        languages: derived.languages,
        anchors: input.anchors,
        cache_state: String::new(),
        cache_artifacts: Vec::new(),
        cache_strategy: input.cache_strategy,
        cache_work: crate::model::CacheWork {
            per_file_facts_reused: input.files_reused,
            per_file_facts_rebuilt: input.files_rebuilt,
            reverse_import_strategy: reverse_update.strategy.to_string(),
            reverse_import_targets_rebuilt: reverse_update.affected_targets,
        },
        files_reused: input.files_reused,
        scan_stats: input.scan_stats,
        timings: ProjectTimings {
            facts_ms,
            reverse_index_ms,
            ..ProjectTimings::default()
        },
        structural_fingerprint: std::sync::OnceLock::new(),
    }
}

pub(crate) fn rebuild_project_facts(project: &mut Project) {
    let derived = derive_project_facts(
        &project.root,
        std::mem::take(&mut project.files),
        &project.anchors,
        project.config_path.as_deref(),
    );
    project.files = derived.files;
    project.packages = derived.packages;
    project.package_edges = derived.package_edges;
    project.domains = derived.domains;
    project.package_manager = derived.package_manager;
    project.scripts = derived.scripts;
    project.languages = derived.languages;
    project.reverse_imports = cache::full_reverse_imports(&project.files).index;
    project.structural_fingerprint = std::sync::OnceLock::new();
}

struct DerivedProjectFacts {
    files: BTreeMap<String, FileInfo>,
    packages: Vec<PackageInfo>,
    package_edges: Vec<PackageDependency>,
    domains: Vec<Domain>,
    package_manager: String,
    scripts: Vec<ScriptInfo>,
    languages: BTreeSet<String>,
}

fn derive_project_facts(
    root: &std::path::Path,
    mut files: BTreeMap<String, FileInfo>,
    anchors: &CodemapConfig,
    config_path: Option<&str>,
) -> DerivedProjectFacts {
    apply_codemap_config_roles(&mut files, anchors);
    let packages = detect_packages(root, &files);
    let ts_path_aliases = detect_ts_path_aliases(root, &files);
    resolve_imports(root, &mut files, &packages, &ts_path_aliases);
    enrich_accessible_surfaces_from_component_contracts(root, &mut files);
    let package_edges = detect_package_edges(root, &files, &packages);
    let scripts = detect_scripts(root, &files);
    let package_manager = detect_package_manager(&files);
    let languages = detect_languages(&files);
    let domains = discover_domains(root, &files, anchors, config_path);
    DerivedProjectFacts {
        files,
        packages,
        package_edges,
        domains,
        package_manager,
        scripts,
        languages,
    }
}

pub(crate) fn cached_scan_stats(
    cached: &ScanStats,
    rescanned: ScanStats,
    files: &BTreeMap<String, FileInfo>,
) -> ScanStats {
    ScanStats {
        files_visited: rescanned.files_visited,
        files_scanned: rescanned.files_scanned,
        files_skipped: rescanned.files_skipped,
        bytes_scanned: rescanned.bytes_scanned,
        ignored: cached.ignored.clone(),
        generated: generated_scan_groups(files),
        inventory_boundaries: rescanned.inventory_boundaries,
    }
}

fn generated_scan_groups(files: &BTreeMap<String, FileInfo>) -> Vec<ScanGroup> {
    let generated = files
        .values()
        .filter(|file| file.has_role("generated"))
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    if generated.is_empty() {
        Vec::new()
    } else {
        vec![ScanGroup {
            reason: "generated_path_or_header".to_string(),
            count: generated.len(),
            examples: generated.into_iter().take(5).collect(),
        }]
    }
}
