// Responsibility: repo-packages-metadata
use crate::model::FileInfo;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

mod domains;
mod scripts;
mod workspace_members;

pub(crate) use domains::*;
pub(crate) use scripts::*;
pub(crate) use workspace_members::*;

pub(crate) fn detect_languages(files: &BTreeMap<String, FileInfo>) -> BTreeSet<String> {
    files
        .values()
        .filter_map(|file| match file.language.as_str() {
            "unknown" | "config" | "markdown" => None,
            other => Some(other.to_string()),
        })
        .collect()
}
