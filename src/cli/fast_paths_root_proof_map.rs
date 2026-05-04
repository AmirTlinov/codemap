const COLD_ROOT_PROOF_MAP_FILE_THRESHOLD: usize = 800;

fn try_cold_root_proof_map_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::ProofMap(args) = command else {
        return Ok(None);
    };
    ensure_single_proof_map_selector(args)?;
    if args.raw_sensors
        || args.changed
        || args.staged
        || args.since.is_some()
        || proof_map_has_explicit_files(args)
    {
        return Ok(None);
    }
    let Some(target) = args.target.as_deref() else {
        return Ok(None);
    };

    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let target = root_relative_arg(&root, target)?;
    if target != "." {
        return Ok(None);
    }

    let files = repo::structural_inventory_candidate_files(&root);
    if root_inventory_has_ctx_config(&root, &files) {
        return Ok(None);
    }
    if files.len() < COLD_ROOT_PROOF_MAP_FILE_THRESHOLD {
        return Ok(None);
    }

    let fingerprint = crate::cache::inventory_fingerprint(&root, &files);
    let report = map::root_inventory_proof_map_report(&root, &files, args.limit);
    set_inventory_map_snapshot_with_fingerprint(&root, &fingerprint);
    output(args.format, &report, || render::proof_map(&report))?;
    Ok(Some(()))
}

fn root_inventory_has_ctx_config(root: &Path, files: &[String]) -> bool {
    if [".ctx.yml", ".ctx.yaml", ".ctx.json"]
        .iter()
        .any(|name| root.join(name).exists())
    {
        return true;
    }
    files.iter().any(|rel| {
        Path::new(rel)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| matches!(name, ".ctx.yml" | ".ctx.yaml" | ".ctx.json"))
    })
}
