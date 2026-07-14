// Responsibility: repo-roles-schema-contract
use crate::repo::is_source_ext;
use std::path::Path;

pub(crate) fn is_schema_contract_surface(rel: &str, name: &str, ext: &str) -> bool {
    if !is_contract_surface_ext(ext) {
        return false;
    }
    let path = Path::new(rel);
    let stem = contract_surface_stem(path, name);
    if matches!(
        stem.as_str(),
        "schema"
            | "schemas"
            | "dto"
            | "dtos"
            | "types"
            | "interface"
            | "interfaces"
            | "migration"
            | "migrations"
    ) {
        return true;
    }
    if [
        ".schema.",
        ".dto.",
        ".types.",
        ".interface.",
        ".migration.",
        ".contract.",
    ]
    .iter()
    .any(|marker| name.contains(marker))
    {
        return true;
    }
    path.parent()
        .into_iter()
        .flat_map(|parent| parent.components())
        .filter_map(|component| component.as_os_str().to_str())
        .map(str::to_ascii_lowercase)
        .any(|part| {
            matches!(
                part.as_str(),
                "schema"
                    | "schemas"
                    | "dto"
                    | "dtos"
                    | "types"
                    | "interfaces"
                    | "contract"
                    | "contracts"
                    | "migration"
                    | "migrations"
            )
        })
}

fn is_contract_surface_ext(ext: &str) -> bool {
    is_source_ext(ext)
        || matches!(
            ext,
            "json" | "yaml" | "yml" | "sql" | "prisma" | "graphql" | "gql" | "proto" | "avsc"
        )
}

fn contract_surface_stem(path: &Path, name: &str) -> String {
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.ends_with(".d.ts") {
        stem.trim_end_matches(".d").to_string()
    } else {
        stem
    }
}
