use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};

mod changed_proof;
mod navigation;
mod proof_map;
mod siblings_place;

const LENS_ARTIFACT_FORMAT_VERSION: u64 = 31;
const LENS_ARTIFACTS: &[&str] = &[
    "ls-current.json",
    "cone-current.json",
    "changed-current.json",
    "proof-changed.json",
    "proof-map-current.json",
    "siblings-current.json",
    "place-current.json",
];

pub use changed_proof::{
    read_changed_report, read_proof_changed_report, write_changed_report,
    write_proof_changed_report,
};
pub use navigation::{
    ConeLensKey, LsLensKey, read_cone_report, read_inventory_ls_report, read_ls_report,
    write_cone_report, write_inventory_ls_report, write_ls_report,
};
pub use proof_map::{read_proof_map_report, write_proof_map_report};
pub use siblings_place::{
    PlaceLensKey, SiblingsLensKey, read_place_report, read_siblings_report, write_place_report,
    write_siblings_report,
};

pub fn format_version() -> u64 {
    LENS_ARTIFACT_FORMAT_VERSION
}

pub fn artifact_names() -> &'static [&'static str] {
    LENS_ARTIFACTS
}

fn read_lens_artifact<T>(cache_dir: &Path, name: &str, version: &str, root: &Path) -> Option<T>
where
    T: LensArtifact + DeserializeOwned,
{
    let fingerprint = current_status_fingerprint(cache_dir)?;
    read_lens_artifact_with_fingerprint(cache_dir, name, version, root, &fingerprint)
}

fn read_lens_artifact_with_fingerprint<T>(
    cache_dir: &Path,
    name: &str,
    version: &str,
    root: &Path,
    fingerprint: &str,
) -> Option<T>
where
    T: LensArtifact + DeserializeOwned,
{
    let text = fs::read_to_string(cache_dir.join(name)).ok()?;
    let cached: T = serde_json::from_str(&text).ok()?;
    if cached.format_version() != LENS_ARTIFACT_FORMAT_VERSION {
        return None;
    }
    if cached.version() != version {
        return None;
    }
    if cached.root() != root.to_string_lossy() {
        return None;
    }
    if cached.fingerprint() != fingerprint {
        return None;
    }
    Some(cached)
}

fn write_lens_artifact<T>(cache_dir: &Path, name: &str, cached: &T) -> Result<()>
where
    T: Serialize,
{
    fs::create_dir_all(cache_dir)?;
    let body = serde_json::to_string_pretty(cached)?;
    fs::write(cache_dir.join(name), format!("{body}\n"))?;
    Ok(())
}

fn current_status_fingerprint(cache_dir: &Path) -> Option<String> {
    super::cached_status_fingerprint(cache_dir)
}

trait LensArtifact {
    fn format_version(&self) -> u64;
    fn version(&self) -> &str;
    fn root(&self) -> &str;
    fn fingerprint(&self) -> &str;
}
