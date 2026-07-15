// Responsibility: structural-map-module-root

mod graph_lens;
pub use graph_lens::graph_lens;

mod boundary;
mod boundary_facts;
mod command_inference;
mod command_inference_roles;
mod cone;
mod count_provenance;
mod edges;
mod entry;
mod facts;
mod impact;
mod lenses;
mod listing;
mod observation_provenance;
mod package_consumers;
mod proof;
mod resolve;
mod scope_repair;
mod status;
mod symbols;
mod teach;
mod test_edges;
mod test_surface;
#[cfg(test)]
mod tests;
mod unknowns;

pub(crate) use boundary::*;
pub(crate) use boundary_facts::*;
pub(crate) use command_inference::*;
pub(crate) use command_inference_roles::*;
pub(crate) use cone::*;
pub(crate) use count_provenance::*;
pub(crate) use edges::*;
pub(crate) use entry::*;
pub(crate) use facts::*;
pub(crate) use impact::*;
pub(crate) use lenses::*;
pub(crate) use listing::*;
pub(crate) use observation_provenance::*;
pub(crate) use package_consumers::*;
pub(crate) use proof::*;
pub(crate) use resolve::*;
pub(crate) use scope_repair::*;
pub(crate) use status::*;
pub(crate) use symbols::*;
pub(crate) use teach::*;
pub(crate) use test_edges::*;
pub(crate) use test_surface::*;
pub(crate) use unknowns::*;
