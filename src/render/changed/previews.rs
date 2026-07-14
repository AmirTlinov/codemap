// Responsibility: changed-preview-formatting

pub(crate) fn changed_preview_paths(paths: &[String], limit: usize) -> String {
    let shown = paths
        .iter()
        .take(limit)
        .map(|path| format!("`{path}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let hidden = paths.len().saturating_sub(limit);
    if hidden == 0 {
        shown
    } else {
        format!("{shown} +{hidden} hidden")
    }
}

pub(crate) fn changed_preview_list(values: &[String], limit: usize) -> String {
    let shown = values
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let hidden = values.len().saturating_sub(limit);
    if hidden == 0 {
        shown
    } else {
        format!("{shown} +{hidden} hidden")
    }
}

pub(crate) fn changed_selector_suffix(selector: &str) -> String {
    if selector == "--changed" {
        String::new()
    } else {
        format!(" {selector}")
    }
}

pub(crate) fn changed_common_dir_prefix(paths: &[&str]) -> Option<String> {
    if paths.len() < 2 {
        return None;
    }
    let mut common = paths.first()?.split('/').collect::<Vec<_>>();
    common.pop();
    for path in paths.iter().skip(1) {
        let mut segments = path.split('/').collect::<Vec<_>>();
        segments.pop();
        let len = common
            .iter()
            .zip(segments.iter())
            .take_while(|(left, right)| left == right)
            .count();
        common.truncate(len);
        if common.is_empty() {
            return None;
        }
    }
    Some(format!("{}/", common.join("/")))
}

pub(crate) fn changed_relative_path(path: &str, prefix: Option<&str>) -> String {
    prefix
        .and_then(|prefix| path.strip_prefix(prefix))
        .unwrap_or(path)
        .to_string()
}
