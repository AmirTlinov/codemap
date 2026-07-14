// Responsibility: package-detection-and-edges
mod detect;
mod edges_js_cargo;
mod edges_other;
mod metadata;
mod targets;

pub(crate) use detect::*;
pub(crate) use edges_js_cargo::*;
pub(crate) use edges_other::*;
pub(crate) use metadata::*;
pub(crate) use targets::*;
