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

pub enum DiffMapMode {
    WorkingTree,
    Staged,
    Since(String),
}
