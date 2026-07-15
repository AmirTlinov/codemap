// Responsibility: root-inventory-observation-provenance
use crate::model::{
    CoverageCertificate, CoverageClosure, CoverageReason, ExtractorCapability, ObservationLedger,
    UnsupportedObservation,
};

const ROOT_SCOPE: &str = ".";

/// Visibility already decided by a root `ls .` owner for one inventory group.
pub(crate) struct RootInventoryGroupVisibility {
    pub observed: usize,
    pub shown: usize,
    pub expand: Option<String>,
}

/// Exact machine basis owned by one root inventory projection owner.
///
/// The full-index owner and the bounded cold inventory owner each declare
/// their own extractor truth; this module only converts that truth into
/// certificate-backed horizons and never rediscovers scope or recomputes
/// group facts.
pub(crate) struct RootInventoryObservationInput {
    pub snapshot: String,
    /// Current-level entries the surface classifier actually visited.
    pub classified_entries: u64,
    /// Direct entries forming the finite candidate universe for level-local
    /// role groups such as test surfaces.
    pub current_level_entries: u64,
    /// Canonical package manifests forming the finite candidate universe.
    pub package_manifest_candidates: Vec<String>,
    /// Candidates whose supported package grammar was actually inspected.
    pub package_manifests_visited: Vec<String>,
    /// Read/parse gaps reported by the package-discovery owner.
    pub package_manifest_unsupported: Vec<UnsupportedObservation>,
    /// Script manifests the root-only catalog actually consulted.
    pub script_manifests_visited: Vec<String>,
    /// Script manifests outside the root-only catalog traversal.
    pub script_manifests_excluded: Vec<String>,
    /// Whether role classification ran over the full index or only over the
    /// bounded cold inventory grammar.
    pub full_index: bool,
    /// Whether the complete current-level path-atlas grammar classified the
    /// finite visible inventory without requiring a source index.
    pub complete_current_level_atlas: bool,
    pub directory_surfaces: RootInventoryGroupVisibility,
    pub packages: RootInventoryGroupVisibility,
    pub scripts: RootInventoryGroupVisibility,
    pub tests: RootInventoryGroupVisibility,
    /// Test-looking current-level entries whose role body was unavailable to
    /// the classifier and could therefore add a test surface.
    pub test_surface_unsupported: Vec<UnsupportedObservation>,
}

pub(crate) fn record_root_inventory_observations(
    input: RootInventoryObservationInput,
    ledger: &mut ObservationLedger,
) {
    record_group(
        ledger,
        &input.directory_surfaces,
        "directory_surfaces",
        directory_surfaces_certificate(&input),
    );
    record_group(
        ledger,
        &input.packages,
        "packages",
        packages_certificate(&input),
    );
    record_group(
        ledger,
        &input.scripts,
        "scripts",
        scripts_certificate(&input),
    );
    record_group(
        ledger,
        &input.tests,
        "test_surfaces",
        test_surfaces_certificate(&input),
    );
}

/// Splits root-scope script-manifest candidates into the manifests the
/// root-only catalog consults and the nested manifests it never reads.
pub(crate) fn root_script_manifest_partition<'a>(
    candidates: impl IntoIterator<Item = &'a str>,
) -> (Vec<String>, Vec<String>) {
    let mut visited = Vec::new();
    let mut excluded = Vec::new();
    for rel in candidates {
        if !root_script_manifest_name(rel) {
            continue;
        }
        if rel.contains('/') {
            excluded.push(rel.to_string());
        } else {
            visited.push(rel.to_string());
        }
    }
    (visited, excluded)
}

fn root_script_manifest_name(rel: &str) -> bool {
    matches!(
        std::path::Path::new(rel)
            .file_name()
            .and_then(|name| name.to_str()),
        Some(
            "package.json"
                | "Cargo.toml"
                | "go.mod"
                | "pyproject.toml"
                | "requirements.txt"
                | "Package.swift"
                | "GNUmakefile"
                | "Makefile"
                | "makefile"
                | "Justfile"
                | "justfile"
        )
    )
}

fn record_group(
    ledger: &mut ObservationLedger,
    visibility: &RootInventoryGroupVisibility,
    group: &str,
    certificate: CoverageCertificate,
) {
    ledger.record(
        group,
        ROOT_SCOPE,
        visibility.observed as u64,
        visibility.shown as u64,
        certificate,
        visibility.expand.clone(),
    );
}

fn base_certificate(
    input: &RootInventoryObservationInput,
    query_kind: &str,
    eligible: u64,
    visited: u64,
    reasons: Vec<CoverageReason>,
) -> CoverageCertificate {
    CoverageCertificate::new(
        query_kind,
        ROOT_SCOPE,
        input.snapshot.clone(),
        eligible,
        visited,
        if reasons.is_empty() {
            CoverageClosure::Closed
        } else {
            CoverageClosure::Open
        },
        reasons,
    )
}

/// The current-level surface catalog closes when either the full source index
/// or the complete path-atlas grammar classified the finite visible inventory.
fn directory_surfaces_certificate(input: &RootInventoryObservationInput) -> CoverageCertificate {
    let complete = input.full_index || input.complete_current_level_atlas;
    let mut certificate = base_certificate(
        input,
        "root_level_directory_surfaces",
        input.classified_entries,
        input.classified_entries,
        if complete {
            Vec::new()
        } else {
            vec![CoverageReason::UnsupportedConstruct]
        },
    );
    certificate.extractor_capabilities = vec![if input.full_index {
        root_capability(
            "codemap.root-ls-surfaces",
            "multi",
            &[
                "directory_inventory",
                "domain_boundary",
                "file_role_or_extension",
                "manifest_file",
                "package_manifest",
                "script_catalog",
            ],
        )
    } else if input.complete_current_level_atlas {
        root_capability(
            "codemap.root-atlas-surfaces",
            "path",
            &[
                "contract_data_runtime_containers",
                "deployment_ci_containers",
                "directory_inventory",
                "domain_package_containers",
                "verification_containers",
            ],
        )
    } else {
        root_capability(
            "codemap.root-inventory-surfaces",
            "path",
            &[
                "ci_path_convention",
                "directory_inventory",
                "file_extension_kind",
                "manifest_name",
            ],
        )
    }];
    certificate.unsupported = input.package_manifest_unsupported.clone();
    certificate
        .unsupported
        .extend(input.test_surface_unsupported.clone());
    certificate
}

/// Package facts are exact manifest-name projections. Manifests an owner
/// skipped without reading are exact disjoint exclusions, so a bounded owner
/// keeps a typed open count instead of a silently smaller "complete" one.
fn packages_certificate(input: &RootInventoryObservationInput) -> CoverageCertificate {
    let unsupported_paths = input
        .package_manifest_unsupported
        .iter()
        .map(|gap| gap.file.clone())
        .collect::<Vec<_>>();
    let mut certificate = base_certificate(
        input,
        "package_manifest_inventory",
        input.package_manifest_candidates.len() as u64,
        input.package_manifests_visited.len() as u64,
        (!(input.full_index || input.complete_current_level_atlas))
            .then_some(CoverageReason::UnsupportedConstruct)
            .into_iter()
            .collect(),
    );
    if !unsupported_paths.is_empty() {
        certificate
            .excluded_files_by_reason
            .insert(CoverageReason::UnsupportedConstruct, unsupported_paths);
    }
    certificate.unsupported = input.package_manifest_unsupported.clone();
    certificate.extractor_capabilities = vec![root_capability(
        "codemap.package-manifests",
        "manifest",
        &[
            "cargo_package_manifest",
            "go_module_manifest",
            "javascript_package_manifest",
            "python_project_manifest",
            "swift_package_manifest",
        ],
    )];
    certificate
}

/// The root script catalog consults repository-root manifests only and keeps
/// a verification-oriented grammar, so nested manifests are exact exclusions
/// and the count never claims to close over the nested scope.
fn scripts_certificate(input: &RootInventoryObservationInput) -> CoverageCertificate {
    let visited = input.script_manifests_visited.len() as u64;
    let excluded = input.script_manifests_excluded.clone();
    let mut certificate = base_certificate(
        input,
        "root_script_catalog",
        visited + excluded.len() as u64,
        visited,
        vec![CoverageReason::UnsupportedConstruct],
    );
    if !excluded.is_empty() {
        certificate
            .excluded_files_by_reason
            .insert(CoverageReason::IncompleteTraversal, excluded);
    }
    certificate.extractor_capabilities = vec![root_capability(
        "codemap.root-script-catalog",
        "manifest",
        &[
            "make_like_targets",
            "package_json_verification_scripts",
            "root_script_catalog",
        ],
    )];
    certificate
}

/// Test surfaces are level-local role projections over the finite direct
/// entry universe. They close only when a declared classifier covers the
/// complete current-level grammar.
fn test_surfaces_certificate(input: &RootInventoryObservationInput) -> CoverageCertificate {
    let complete = input.full_index || input.complete_current_level_atlas;
    let mut certificate = base_certificate(
        input,
        "root_level_test_surfaces",
        input.current_level_entries,
        input.current_level_entries,
        if complete {
            Vec::new()
        } else {
            vec![CoverageReason::UnsupportedConstruct]
        },
    );
    certificate.extractor_capabilities = vec![if input.full_index {
        root_capability(
            "codemap.test-surface-roles",
            "path",
            &["test_path_convention", "test_role_classifier"],
        )
    } else if input.complete_current_level_atlas {
        root_capability(
            "codemap.root-atlas-surfaces",
            "path",
            &[
                "directory_inventory",
                "test_file_convention",
                "test_path_convention",
            ],
        )
    } else {
        root_capability(
            "codemap.root-inventory-surfaces",
            "path",
            &["directory_inventory", "file_extension_kind"],
        )
    }];
    certificate.unsupported = input.test_surface_unsupported.clone();
    certificate
}

fn root_capability(id: &str, language: &str, constructs: &[&str]) -> ExtractorCapability {
    ExtractorCapability {
        extractor_id: id.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        language: language.to_string(),
        constructs: constructs.iter().map(|value| value.to_string()).collect(),
    }
}
