// Responsibility: map-listing-directory-edge-endpoints
use crate::model::Project;
use crate::repo;

pub(crate) fn directory_edge_endpoint_at_depth(
    project: &Project,
    scope: &str,
    path: &str,
    depth: usize,
) -> String {
    let scope = repo::normalize_rel_path(scope);
    let path = repo::normalize_rel_path(path);
    let depth = depth.max(1);
    if scope == "." {
        if let Some(endpoint) = package_endpoint_at_depth(project, &path, depth) {
            return endpoint;
        }
        return top_level_endpoint_at_depth(&path, depth);
    }
    if let Some(rest) = path.strip_prefix(&format!("{}/", scope.trim_end_matches('/'))) {
        let mut parts = rest.split('/').collect::<Vec<_>>();
        if parts.len() <= 1 {
            return format!("{}/", scope.trim_end_matches('/'));
        }
        parts.pop();
        let dirs = parts.into_iter().take(depth).collect::<Vec<_>>();
        if dirs.is_empty() {
            format!("{}/", scope.trim_end_matches('/'))
        } else {
            format!("{}/{}/", scope.trim_end_matches('/'), dirs.join("/"))
        }
    } else {
        if let Some(endpoint) = package_endpoint_at_depth(project, &path, depth) {
            return endpoint;
        }
        top_level_endpoint_at_depth(&path, depth)
    }
}

fn package_endpoint_at_depth(project: &Project, path: &str, depth: usize) -> Option<String> {
    let path = repo::normalize_rel_path(path);
    // The outermost package is the map envelope; depth expands inside that envelope.
    let package = project
        .packages
        .iter()
        .filter(|package| package.path != "." && path_under_scope(&path, &package.path))
        .min_by_key(|package| package.path.len())?;
    let rest = path_relative_to_map(&path, &package.path).unwrap_or_else(|| ".".to_string());
    if depth <= 1 || rest == "." {
        return Some(format!("{}/", package.path.trim_end_matches('/')));
    }
    let mut parts = rest.split('/').collect::<Vec<_>>();
    if parts.len() <= 1 {
        return Some(format!("{}/", package.path.trim_end_matches('/')));
    }
    parts.pop();
    let dirs = parts
        .into_iter()
        .take(depth.saturating_sub(1))
        .collect::<Vec<_>>();
    if dirs.is_empty() {
        Some(format!("{}/", package.path.trim_end_matches('/')))
    } else {
        Some(format!(
            "{}/{}/",
            package.path.trim_end_matches('/'),
            dirs.join("/")
        ))
    }
}

fn path_relative_to_map(path: &str, base: &str) -> Option<String> {
    let path = repo::normalize_rel_path(path);
    let base = repo::normalize_rel_path(base);
    if base == "." {
        return Some(path);
    }
    if path == base {
        return Some(".".to_string());
    }
    path.strip_prefix(&format!("{}/", base.trim_end_matches('/')))
        .map(str::to_string)
}

fn top_level_endpoint_at_depth(path: &str, depth: usize) -> String {
    let mut parts = path.split('/');
    let depth = depth.max(1);
    let segments = path.split('/').collect::<Vec<_>>();
    if segments.len() <= 1 {
        return path.to_string();
    }
    if let (Some(first), Some(_second)) = (parts.next(), parts.next())
        && matches!(
            first,
            "apps" | "packages" | "services" | "domains" | "crates" | "modules"
        )
    {
        let take = (2 + depth.saturating_sub(1)).min(segments.len().saturating_sub(1));
        return format!("{}/", segments[..take].join("/"));
    }
    let take = depth.min(segments.len().saturating_sub(1));
    format!("{}/", segments[..take].join("/"))
}

pub(crate) fn path_under_scope(path: &str, scope: &str) -> bool {
    let path = repo::normalize_rel_path(path);
    let scope = repo::normalize_rel_path(scope);
    scope == "." || path == scope || path.starts_with(&format!("{}/", scope.trim_end_matches('/')))
}

pub(crate) fn workspace_edge_directory_target(
    project: &Project,
    scope: &str,
    target: &str,
    endpoint_depth: usize,
) -> String {
    if project.files.contains_key(target)
        || project
            .packages
            .iter()
            .any(|package| package.path == target || package.manifest == target)
    {
        directory_edge_endpoint_at_depth(project, scope, target, endpoint_depth)
    } else {
        target.to_string()
    }
}
