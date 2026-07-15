// Responsibility: diff-map-lens-assembly
mod file_texts;
mod git;
mod line_surfaces;
mod report;
mod runtime;
mod surface_dedupe;
mod symbols;
mod unknowns;

pub(crate) use file_texts::*;
pub(crate) use git::*;
pub(crate) use line_surfaces::*;
pub(crate) use report::*;
pub(crate) use runtime::*;
pub(crate) use surface_dedupe::*;
pub(crate) use symbols::*;
pub(crate) use unknowns::*;

#[derive(Clone)]
pub struct SnapshotDiffBase {
    pub token: String,
    pub texts: std::collections::BTreeMap<String, String>,
    pub content_complete: bool,
}

pub enum DiffMapMode {
    WorkingTree,
    Staged,
    Since(String),
    Snapshot(SnapshotDiffBase),
}

pub struct ChangedDiffContext {
    pub mode: DiffMapMode,
    pub selection: crate::model::ChangeSelection,
}
