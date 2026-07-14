// Responsibility: map-listing-directory-owner-support
use crate::map::{is_support_artifact_path, manifest_dir_for_rel};
use crate::model::{FileInfo, PackageInfo, Project};
use crate::repo;
use std::path::Path;

pub(crate) fn lockfiles_for_package<'a>(
    project: &'a Project,
    package: &PackageInfo,
) -> Vec<&'a FileInfo> {
    let package_dir = package.path.trim_end_matches('/');
    project
        .files
        .values()
        .filter(|file| file.has_role("lockfile"))
        .filter(|file| match package.ecosystem.as_str() {
            "rust" => {
                if package.path == "." {
                    file.rel == "Cargo.lock"
                } else {
                    file.rel == format!("{package_dir}/Cargo.lock")
                }
            }
            "javascript" => {
                matches!(
                    Path::new(&file.rel)
                        .file_name()
                        .and_then(|name| name.to_str()),
                    Some(
                        "pnpm-lock.yaml"
                            | "pnpm-lock.yml"
                            | "package-lock.json"
                            | "yarn.lock"
                            | "bun.lock"
                            | "bun.lockb"
                    )
                ) && ((package.path == "." && !file.rel.contains('/'))
                    || file.rel == format!("{package_dir}/pnpm-lock.yaml")
                    || file.rel == format!("{package_dir}/package-lock.json")
                    || file.rel == format!("{package_dir}/yarn.lock"))
            }
            "python" => matches!(
                Path::new(&file.rel)
                    .file_name()
                    .and_then(|name| name.to_str()),
                Some("poetry.lock" | "pdm.lock" | "uv.lock")
            ),
            _ => false,
        })
        .collect()
}

pub(crate) fn should_hide_owner_edge_path(path: &str, scope_is_support: bool) -> bool {
    !scope_is_support && is_support_artifact_path(path)
}

pub(crate) fn schema_owner_directory(rel: &str) -> String {
    let dir = manifest_dir_for_rel(rel);
    if dir.ends_with("/prisma") || dir.ends_with("/db") || dir.ends_with("/schema") {
        dir
    } else if rel.contains("/migrations/") {
        rel.split("/migrations/")
            .next()
            .map(repo::normalize_rel_path)
            .unwrap_or(dir)
    } else {
        dir
    }
}
