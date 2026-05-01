fn ensure_graph_lens(lens: &str) -> Result<()> {
    match lens.to_ascii_lowercase().as_str() {
        "causal" | "impact" | "proof" | "boundary" | "boundaries" => Ok(()),
        _ => {
            bail!("unknown graph lens `{lens}`; expected one of: causal, impact, proof, boundaries")
        }
    }
}

fn schema_text(kind: SchemaKind) -> &'static str {
    match kind {
        SchemaKind::Manifest => include_str!("../../schemas/manifest.json"),
        SchemaKind::Status => include_str!("../../schemas/status.schema.json"),
        SchemaKind::Files => include_str!("../../schemas/files.schema.json"),
        SchemaKind::Ls => include_str!("../../schemas/ls.schema.json"),
        SchemaKind::Cone => include_str!("../../schemas/cone.schema.json"),
        SchemaKind::Impact => include_str!("../../schemas/impact.schema.json"),
        SchemaKind::DiffMap => include_str!("../../schemas/diff-map.schema.json"),
        SchemaKind::Contract => include_str!("../../schemas/contract.schema.json"),
        SchemaKind::Runtime => include_str!("../../schemas/runtime.schema.json"),
        SchemaKind::Proof => include_str!("../../schemas/proof.schema.json"),
        SchemaKind::ProofMap => include_str!("../../schemas/proof-map.schema.json"),
        SchemaKind::Delete => include_str!("../../schemas/delete.schema.json"),
        SchemaKind::BoundaryMap => include_str!("../../schemas/boundary-map.schema.json"),
        SchemaKind::Flow => include_str!("../../schemas/flow.schema.json"),
        SchemaKind::Siblings => include_str!("../../schemas/siblings.schema.json"),
        SchemaKind::Place => include_str!("../../schemas/place.schema.json"),
        SchemaKind::Anchors => include_str!("../../schemas/anchors.schema.json"),
        SchemaKind::AnchorValidation => include_str!("../../schemas/anchor-validation.schema.json"),
        SchemaKind::Graph => include_str!("../../schemas/graph.schema.json"),
        SchemaKind::Boundaries => include_str!("../../schemas/boundaries.schema.json"),
    }
}

fn command_root_hint(command: &CommandKind, ambient_root: Option<&Path>) -> Option<PathBuf> {
    match command {
        CommandKind::Ls(args) => absolute_path_hint(Some(&args.path)),
        CommandKind::Cone(args) => absolute_path_hint(Some(&args.path)),
        CommandKind::Files(args) => absolute_path_hint(args.path.as_deref()),
        CommandKind::Init(args) => init_root_hint(args.path.as_deref(), ambient_root),
        CommandKind::Impact(args) => {
            absolute_files_hint(args.files.as_deref(), &args.positional_files)
        }
        CommandKind::DiffMap(args) => {
            absolute_files_hint(args.files.as_deref(), &args.positional_files)
        }
        CommandKind::Contract(args) => absolute_path_hint(Some(&args.path)),
        CommandKind::Runtime(args) => absolute_path_hint(Some(&args.scope)),
        CommandKind::Proof(args) => proof_root_hint(args),
        CommandKind::ProofMap(args) => proof_map_root_hint(args),
        CommandKind::Delete(args) => absolute_path_hint(Some(&args.path)),
        CommandKind::BoundaryMap(args) => absolute_path_hint(Some(&args.scope)),
        CommandKind::Flow(args) => absolute_path_hint(Some(&args.path)),
        CommandKind::Siblings(args) => absolute_path_hint(Some(&args.scope)),
        CommandKind::Place(args) => absolute_path_hint(Some(&args.scope)),
        CommandKind::Graph(args) => absolute_path_hint(args.path.as_deref()),
        _ => None,
    }
}

fn init_root_hint(path: Option<&str>, ambient_root: Option<&Path>) -> Option<PathBuf> {
    let hint = absolute_path_hint(path)?;
    if ambient_root.is_some() {
        None
    } else {
        Some(hint)
    }
}

fn proof_root_hint(args: &ProofArgs) -> Option<PathBuf> {
    absolute_path_hint(args.target.as_deref())
        .or_else(|| absolute_files_hint(args.files.as_deref(), &[]))
}

fn proof_map_root_hint(args: &ProofMapArgs) -> Option<PathBuf> {
    absolute_path_hint(args.target.as_deref())
        .or_else(|| absolute_files_hint(args.files.as_deref(), &[]))
}

fn absolute_path_hint(path: Option<&str>) -> Option<PathBuf> {
    path.map(PathBuf::from).filter(|path| path.is_absolute())
}

fn absolute_files_hint(files: Option<&str>, positional: &[String]) -> Option<PathBuf> {
    files
        .into_iter()
        .flat_map(|files| files.split(','))
        .chain(positional.iter().map(String::as_str))
        .filter_map(absolute_file_root_hint)
        .next()
}

fn absolute_file_root_hint(value: &str) -> Option<PathBuf> {
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return None;
    }
    let absolute = path.canonicalize().unwrap_or(path);
    if absolute.is_file() {
        absolute.parent().map(Path::to_path_buf)
    } else {
        Some(absolute)
    }
}
