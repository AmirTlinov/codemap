// Responsibility: query-specific-definition-extractor-closure-audit
use crate::model::{CoverageReason, ExtractorCapability, FileInfo, Project};

mod declaration_bindings;
mod opaque_bindings;
use declaration_bindings::declaration_binding_occurs;
use opaque_bindings::{commonjs_export_binding_occurs, export_clause_binds, member_binding_occurs};

pub(super) fn definition_extractor_capability(
    project: &Project,
    file: &FileInfo,
    query: &str,
) -> Result<ExtractorCapability, (CoverageReason, String)> {
    if file.content_hash.is_none() {
        return Err((
            CoverageReason::UnsupportedConstruct,
            "definition source could not be read".to_string(),
        ));
    }
    if matches!(file.ext.as_str(), "vue" | "svelte") {
        return Err((
            CoverageReason::UnsupportedConstruct,
            format!(".{} component-container definition extraction", file.ext),
        ));
    }
    if !matches!(
        file.ext.as_str(),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs"
    ) {
        let reason = if matches!(file.ext.as_str(), "py" | "rs" | "go" | "swift") {
            CoverageReason::UnsupportedConstruct
        } else {
            CoverageReason::UnsupportedLanguage
        };
        return Err((reason, format!("partial .{} definition grammar", file.ext)));
    }
    audit_javascript_query(project, file, query)?;
    Ok(ExtractorCapability {
        extractor_id: "codemap.indexed-symbol-table".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        language: file.language.clone(),
        constructs: vec!["query_specific_javascript_definition".to_string()],
    })
}

/// Closure is query-specific. An independent lexical binding pass checks the
/// declaration forms which the line-oriented symbol table may not index (for
/// example `using`, generators, and later comma declarators). Ordinary imports
/// and calls do not make definition coverage unknown.
fn audit_javascript_query(
    project: &Project,
    file: &FileInfo,
    query: &str,
) -> Result<(), (CoverageReason, String)> {
    let text = std::fs::read_to_string(project.root.join(&file.rel)).map_err(|_| {
        (
            CoverageReason::UnsupportedConstruct,
            "definition source could not be read".to_string(),
        )
    })?;
    if let Some(construct) = text
        .lines()
        .find_map(crate::map::runtime_generated_code_line)
    {
        return Err((CoverageReason::UnsupportedConstruct, construct.to_string()));
    }
    let code = crate::repo::code_without_comments_or_strings(&text, &file.ext);
    if code.contains("\\u") {
        return Err((
            CoverageReason::UnsupportedConstruct,
            "escaped javascript identifier spelling".to_string(),
        ));
    }
    let indexed = file.symbols.iter().any(|symbol| symbol.name == query);
    if !indexed && commonjs_export_binding_occurs(&text, &code, query) {
        return Err((
            CoverageReason::UnsupportedConstruct,
            format!("queried CommonJS export binding `{query}` is not indexed"),
        ));
    }
    if !indexed && export_clause_binds(&code, query) {
        return Err((
            CoverageReason::UnsupportedConstruct,
            format!("queried export binding `{query}` is present but not indexed"),
        ));
    }
    if !indexed && member_binding_occurs(&code, query) {
        return Err((
            CoverageReason::UnsupportedConstruct,
            format!("queried member binding `{query}` is present but not indexed"),
        ));
    }
    if !indexed && declaration_binding_occurs(&code, query) {
        return Err((
            CoverageReason::UnsupportedConstruct,
            format!("queried identifier `{query}` is present but not indexed"),
        ));
    }
    Ok(())
}
