use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::model::{
    HiddenGroup, PlaceReport, ProofSurface, SiblingsReport, StructuralEdge, Surface, Unknown,
};

use super::{
    LensArtifact, current_status_fingerprint, format_version, read_lens_artifact,
    write_lens_artifact,
};

pub struct SiblingsLensKey<'a> {
    pub cache_dir: &'a Path,
    pub version: &'a str,
    pub root: &'a Path,
    pub scope: &'a str,
    pub include_hidden: bool,
    pub limit: usize,
}

pub struct PlaceLensKey<'a> {
    pub cache_dir: &'a Path,
    pub version: &'a str,
    pub root: &'a Path,
    pub scope: &'a str,
    pub kind: &'a str,
    pub include_hidden: bool,
    pub limit: usize,
}

pub fn read_siblings_report(key: SiblingsLensKey<'_>) -> Option<SiblingsReport> {
    let cached: CachedSiblingsLens = read_lens_artifact(
        key.cache_dir,
        "siblings-current.json",
        key.version,
        key.root,
    )?;
    if cached.scope != key.scope
        || cached.include_hidden != key.include_hidden
        || cached.limit != key.limit
    {
        return None;
    }
    Some(cached.report.into_report())
}

pub fn write_siblings_report(key: SiblingsLensKey<'_>, report: &SiblingsReport) -> Result<()> {
    let cached = CachedSiblingsLens {
        format_version: format_version(),
        version: key.version.to_string(),
        root: key.root.to_string_lossy().to_string(),
        fingerprint: current_status_fingerprint(key.cache_dir).unwrap_or_default(),
        scope: key.scope.to_string(),
        include_hidden: key.include_hidden,
        limit: key.limit,
        report: CachedSiblingsReport::from_report(report),
    };
    write_lens_artifact(key.cache_dir, "siblings-current.json", &cached)
}

pub fn read_place_report(key: PlaceLensKey<'_>) -> Option<PlaceReport> {
    let cached: CachedPlaceLens =
        read_lens_artifact(key.cache_dir, "place-current.json", key.version, key.root)?;
    if cached.scope != key.scope
        || cached.kind != key.kind
        || cached.include_hidden != key.include_hidden
        || cached.limit != key.limit
    {
        return None;
    }
    Some(cached.report.into_report())
}

pub fn write_place_report(key: PlaceLensKey<'_>, report: &PlaceReport) -> Result<()> {
    let cached = CachedPlaceLens {
        format_version: format_version(),
        version: key.version.to_string(),
        root: key.root.to_string_lossy().to_string(),
        fingerprint: current_status_fingerprint(key.cache_dir).unwrap_or_default(),
        scope: key.scope.to_string(),
        kind: key.kind.to_string(),
        include_hidden: key.include_hidden,
        limit: key.limit,
        report: CachedPlaceReport::from_report(report),
    };
    write_lens_artifact(key.cache_dir, "place-current.json", &cached)
}

#[derive(Deserialize, Serialize)]
struct CachedSiblingsLens {
    format_version: u64,
    version: String,
    root: String,
    fingerprint: String,
    scope: String,
    include_hidden: bool,
    limit: usize,
    report: CachedSiblingsReport,
}

impl LensArtifact for CachedSiblingsLens {
    fn format_version(&self) -> u64 {
        self.format_version
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn root(&self) -> &str {
        &self.root
    }

    fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

#[derive(Deserialize, Serialize)]
struct CachedPlaceLens {
    format_version: u64,
    version: String,
    root: String,
    fingerprint: String,
    scope: String,
    kind: String,
    include_hidden: bool,
    limit: usize,
    report: CachedPlaceReport,
}

impl LensArtifact for CachedPlaceLens {
    fn format_version(&self) -> u64 {
        self.format_version
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn root(&self) -> &str {
        &self.root
    }

    fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

#[derive(Deserialize, Serialize)]
struct CachedSiblingsReport {
    kind: String,
    schema_version: String,
    scope: String,
    same_kind: Vec<Surface>,
    route_service_test_triplets: Vec<Surface>,
    shared_helpers: Vec<StructuralEdge>,
    shared_contracts: Vec<StructuralEdge>,
    proof_pattern: Vec<ProofSurface>,
    unknowns: Vec<Unknown>,
    hidden: Vec<HiddenGroup>,
    expand: Vec<String>,
}

impl CachedSiblingsReport {
    fn from_report(report: &SiblingsReport) -> Self {
        Self {
            kind: report.kind.to_string(),
            schema_version: report.schema_version.to_string(),
            scope: report.scope.clone(),
            same_kind: report.same_kind.clone(),
            route_service_test_triplets: report.route_service_test_triplets.clone(),
            shared_helpers: report.shared_helpers.clone(),
            shared_contracts: report.shared_contracts.clone(),
            proof_pattern: report.proof_pattern.clone(),
            unknowns: report.unknowns.clone(),
            hidden: report.hidden.clone(),
            expand: report.expand.clone(),
        }
    }

    fn into_report(self) -> SiblingsReport {
        SiblingsReport {
            kind: "siblings_report",
            schema_version: "2",
            scope: self.scope,
            same_kind: self.same_kind,
            route_service_test_triplets: self.route_service_test_triplets,
            shared_helpers: self.shared_helpers,
            shared_contracts: self.shared_contracts,
            proof_pattern: self.proof_pattern,
            unknowns: self.unknowns,
            hidden: self.hidden,
            expand: self.expand,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct CachedPlaceReport {
    kind: String,
    schema_version: String,
    scope: String,
    requested_kind: String,
    existing_surfaces: Vec<Surface>,
    local_conventions: Vec<String>,
    paired_proof_pattern: Vec<ProofSurface>,
    shared_contracts: Vec<StructuralEdge>,
    unknowns: Vec<Unknown>,
    hidden: Vec<HiddenGroup>,
    expand: Vec<String>,
}

impl CachedPlaceReport {
    fn from_report(report: &PlaceReport) -> Self {
        Self {
            kind: report.kind.to_string(),
            schema_version: report.schema_version.to_string(),
            scope: report.scope.clone(),
            requested_kind: report.requested_kind.clone(),
            existing_surfaces: report.existing_surfaces.clone(),
            local_conventions: report.local_conventions.clone(),
            paired_proof_pattern: report.paired_proof_pattern.clone(),
            shared_contracts: report.shared_contracts.clone(),
            unknowns: report.unknowns.clone(),
            hidden: report.hidden.clone(),
            expand: report.expand.clone(),
        }
    }

    fn into_report(self) -> PlaceReport {
        PlaceReport {
            kind: "place_report",
            schema_version: "2",
            scope: self.scope,
            requested_kind: self.requested_kind,
            existing_surfaces: self.existing_surfaces,
            local_conventions: self.local_conventions,
            paired_proof_pattern: self.paired_proof_pattern,
            shared_contracts: self.shared_contracts,
            unknowns: self.unknowns,
            hidden: self.hidden,
            expand: self.expand,
        }
    }
}
