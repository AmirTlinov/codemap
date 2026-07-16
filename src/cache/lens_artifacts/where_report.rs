// Responsibility: exact-where-lens-artifact
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{
    HiddenGroup, ObservationLedger, Unknown, WhereDefinition, WhereReport, WhereSuggestion,
};

use super::navigation::CachedConeReport;
use super::{
    LensArtifact, current_status_fingerprint, format_version, read_lens_artifact,
    write_lens_artifact,
};

pub struct WhereLensKey<'a> {
    pub cache_dir: &'a Path,
    pub version: &'a str,
    pub root: &'a Path,
    pub query: &'a str,
    pub kind_filter: Option<&'a str>,
    pub include_hidden: bool,
    pub limit: usize,
}

pub fn read_where_report(key: WhereLensKey<'_>) -> Option<WhereReport> {
    let cached: CachedWhereLens =
        read_lens_artifact(key.cache_dir, "where-current.json", key.version, key.root)?;
    if cached.query != key.query
        || cached.kind_filter.as_deref() != key.kind_filter
        || cached.include_hidden != key.include_hidden
        || cached.limit != key.limit
        || cached.report_sha256 != where_report_sha256(&cached.report)
    {
        return None;
    }
    let report = cached.report.into_report();
    report.validate_observations().ok()?;
    Some(report)
}

pub fn write_where_report(key: WhereLensKey<'_>, report: &WhereReport) -> Result<()> {
    let report = CachedWhereReport::from_report(report);
    let cached = CachedWhereLens {
        format_version: format_version(),
        version: key.version.to_string(),
        root: key.root.to_string_lossy().to_string(),
        fingerprint: current_status_fingerprint(key.cache_dir).unwrap_or_default(),
        query: key.query.to_string(),
        kind_filter: key.kind_filter.map(str::to_string),
        include_hidden: key.include_hidden,
        limit: key.limit,
        report_sha256: where_report_sha256(&report),
        report,
    };
    write_lens_artifact(key.cache_dir, "where-current.json", &cached)
}

#[derive(Deserialize, Serialize)]
struct CachedWhereLens {
    format_version: u64,
    version: String,
    root: String,
    fingerprint: String,
    query: String,
    kind_filter: Option<String>,
    include_hidden: bool,
    limit: usize,
    report_sha256: String,
    report: CachedWhereReport,
}

impl LensArtifact for CachedWhereLens {
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
struct CachedWhereReport {
    query: String,
    kind_filter: Option<String>,
    total_matches: usize,
    observations: ObservationLedger,
    definitions: Vec<WhereDefinition>,
    soft_suggestions: Vec<WhereSuggestion>,
    unknowns: Vec<Unknown>,
    hidden: Vec<HiddenGroup>,
    expand: Vec<String>,
    detail: Option<Box<CachedConeReport>>,
}

impl CachedWhereReport {
    fn from_report(report: &WhereReport) -> Self {
        Self {
            query: report.query.clone(),
            kind_filter: report.kind_filter.clone(),
            total_matches: report.total_matches,
            observations: report.observations.clone(),
            definitions: report.definitions.clone(),
            soft_suggestions: report.soft_suggestions.clone(),
            unknowns: report.unknowns.clone(),
            hidden: report.hidden.clone(),
            expand: report.expand.clone(),
            detail: report
                .detail
                .as_deref()
                .map(CachedConeReport::from_report)
                .map(Box::new),
        }
    }

    fn into_report(self) -> WhereReport {
        WhereReport {
            kind: "where_report",
            schema_version: WhereReport::SCHEMA_VERSION,
            query: self.query,
            kind_filter: self.kind_filter,
            total_matches: self.total_matches,
            observations: self.observations,
            definitions: self.definitions,
            soft_suggestions: self.soft_suggestions,
            unknowns: self.unknowns,
            hidden: self.hidden,
            expand: self.expand,
            detail: self.detail.map(|detail| Box::new(detail.into_report())),
        }
    }
}

fn where_report_sha256(report: &CachedWhereReport) -> String {
    let body = serde_json::to_vec(report).expect("cached where report should serialize");
    format!("{:x}", Sha256::digest(body))
}
