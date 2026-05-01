fn runtime_worker_or_job_convention(rel: &str) -> bool {
    let path = std::path::Path::new(rel);
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file_name)
        .to_ascii_lowercase();
    if exact_runtime_worker_job_token(&stem)
        || split_runtime_tokens(&stem)
            .iter()
            .any(|token| exact_runtime_worker_job_token(token))
    {
        return true;
    }
    path.parent()
        .map(|parent| {
            parent
                .components()
                .filter_map(|component| component.as_os_str().to_str())
                .any(|segment| exact_runtime_worker_job_token(&segment.to_ascii_lowercase()))
        })
        .unwrap_or(false)
}

fn split_runtime_tokens(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !(ch.is_ascii_alphanumeric()))
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn exact_runtime_worker_job_token(value: &str) -> bool {
    matches!(
        value,
        "worker" | "workers" | "job" | "jobs" | "cron" | "crons"
    )
}
