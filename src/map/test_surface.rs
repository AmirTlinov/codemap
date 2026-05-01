fn test_imports_support_consuming_anchor(project: &Project, rel: &str, test: &FileInfo) -> bool {
    let mut seen = BTreeSet::new();
    let mut frontier = test
        .resolved_imports
        .iter()
        .filter_map(|import| project.files.get(import))
        .filter(|file| file.has_role("test_support"))
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    for _ in 0..2 {
        if frontier.is_empty() {
            return false;
        }
        let mut next = Vec::new();
        for support_rel in frontier {
            if !seen.insert(support_rel.clone()) {
                continue;
            }
            let Some(support) = project.files.get(&support_rel) else {
                continue;
            };
            if support.resolved_imports.contains(rel) {
                return true;
            }
            next.extend(
                support
                    .resolved_imports
                    .iter()
                    .filter_map(|import| project.files.get(import))
                    .filter(|file| file.has_role("test_support"))
                    .map(|file| file.rel.clone()),
            );
        }
        frontier = next;
    }
    false
}

fn swift_test_can_prove_anchor(project: &Project, rel: &str, test: &FileInfo) -> bool {
    let Some(anchor) = project.files.get(rel) else {
        return true;
    };
    if anchor.ext != "swift" || test.ext != "swift" {
        return true;
    }
    let Some((root, target)) = swift_source_scope(&anchor.rel) else {
        return false;
    };
    swift_test_package_root(&test.rel)
        .map(|test_root| test_root == root)
        .unwrap_or(false)
        && test.imports.contains(&target)
}

fn test_references_anchor_symbol(project: &Project, rel: &str, test: &FileInfo) -> bool {
    let Some(anchor) = project.files.get(rel) else {
        return false;
    };
    if anchor_symbol_reference_names(anchor).is_empty() {
        return false;
    }
    let source_domain = scoped_domain_path_for_rel(project, rel, domain_by_rel(project, rel));
    let test_domain = scoped_domain_path_for_rel(project, &test.rel, domain_by_rel(project, rel));
    if source_domain.is_some() && source_domain != test_domain {
        return false;
    }
    let source_package = package_for_rel(project, rel).map(|package| package.path.clone());
    let test_package = package_for_rel(project, &test.rel).map(|package| package.path.clone());
    if source_package.is_some() && source_package != test_package {
        return false;
    }
    if !same_symbol_reference_scope(anchor, test) {
        return false;
    }
    anchor_symbol_reference_names(anchor)
        .iter()
        .any(|name| test.references.contains(name))
}

fn anchor_symbol_reference_names(anchor: &FileInfo) -> BTreeSet<String> {
    anchor
        .symbols
        .iter()
        .filter(|symbol| symbol.kind != "method")
        .filter(|symbol| symbol.exported || structural_anchor_symbol_kind(&symbol.kind))
        .map(|symbol| symbol.name.clone())
        .filter(|name| meaningful_symbol_reference_name(name))
        .collect()
}

fn meaningful_symbol_reference_name(name: &str) -> bool {
    if name == "default" || name.len() < 4 {
        return false;
    }
    let terms = semantic_name_terms(name);
    !terms.is_empty()
}

fn shared_surface_phrases(project: &Project, rel: &str, test: &FileInfo) -> BTreeSet<String> {
    project
        .files
        .get(rel)
        .map(|file| {
            let mut shared = BTreeSet::new();
            for source_phrase in &file.surface_phrases {
                if !meaningful_surface_phrase(source_phrase) {
                    continue;
                }
                for test_phrase in &test.surface_phrases {
                    if !meaningful_surface_phrase(test_phrase) {
                        continue;
                    }
                    if surface_phrases_match(source_phrase, test_phrase) {
                        shared.insert(source_phrase.clone());
                    }
                }
            }
            shared
        })
        .unwrap_or_default()
}

fn surface_phrases_match(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    if accessible_role_name_surface(left) || accessible_role_name_surface(right) {
        return false;
    }
    let (shorter, longer) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    let shorter_terms = surface_phrase_terms(shorter);
    shorter_terms.len() >= 3 && phrase_contains_with_boundaries(longer, shorter)
}

fn accessible_role_name_surface(phrase: &str) -> bool {
    phrase.starts_with("a11y-role-")
}

fn phrase_contains_with_boundaries(longer: &str, shorter: &str) -> bool {
    longer.match_indices(shorter).any(|(start, _)| {
        let before = longer[..start].chars().next_back();
        let end = start + shorter.len();
        let after = longer[end..].chars().next();
        before.map(phrase_boundary_char).unwrap_or(true)
            && after.map(phrase_boundary_char).unwrap_or(true)
    })
}

fn phrase_boundary_char(ch: char) -> bool {
    ch == '-'
}

fn source_stem(rel: &str) -> String {
    Path::new(rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .replace(".test", "")
        .replace(".spec", "")
        .to_ascii_lowercase()
}

fn test_name_matches_source_stem(test_rel: &str, source_stem: &str) -> bool {
    test_stem(test_rel) == source_stem
}

fn test_stem(test_rel: &str) -> String {
    let mut stem = Path::new(test_rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    for suffix in [".test", ".spec", "_test"] {
        if let Some(stripped) = stem.strip_suffix(suffix) {
            stem = stripped.to_string();
        }
    }
    stem
}

fn meaningful_stem(stem: &str) -> bool {
    !stem.is_empty() && !matches!(stem, "index" | "mod" | "main" | "lib" | "types")
}

fn anchor_terms(project: &Project, rel: &str) -> BTreeSet<String> {
    let mut terms = semantic_path_terms(rel);
    if let Some(file) = project.files.get(rel) {
        for symbol in &file.symbols {
            if symbol.exported || structural_anchor_symbol_kind(&symbol.kind) {
                terms.extend(semantic_name_terms(&symbol.name));
            }
        }
        for export in &file.exports {
            terms.extend(semantic_name_terms(export));
        }
        terms.extend(file.surface_tokens.iter().cloned());
    }
    terms
}

fn anchor_core_terms(project: &Project, rel: &str) -> BTreeSet<String> {
    let mut terms = semantic_name_terms(&source_stem(rel));
    if let Some(file) = project.files.get(rel) {
        for symbol in &file.symbols {
            if symbol.exported {
                terms.extend(semantic_name_terms(&symbol.name));
            }
        }
        for export in &file.exports {
            terms.extend(semantic_name_terms(export));
        }
    }
    terms
}

fn structural_anchor_symbol_kind(kind: &str) -> bool {
    matches!(
        kind,
        "component"
            | "function"
            | "class"
            | "interface"
            | "type"
            | "struct"
            | "enum"
            | "trait"
            | "method"
    )
}

fn test_surface_terms(file: &FileInfo) -> BTreeSet<String> {
    let mut terms = semantic_path_terms(&file.rel);
    terms.extend(file.surface_tokens.iter().cloned());
    terms
}

fn semantic_path_terms(path: &str) -> BTreeSet<String> {
    let normalized = repo::normalize_rel_path(path);
    let without_ext = Path::new(&normalized)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(normalized.as_str());
    repo::tokenize(&normalized)
        .into_iter()
        .chain(semantic_name_terms(without_ext))
        .filter(|term| meaningful_surface_term(term))
        .collect()
}

fn semantic_name_terms(name: &str) -> BTreeSet<String> {
    let mut expanded = String::new();
    let mut previous_lower_or_digit = false;
    for ch in name.chars() {
        if ch == '-' || ch == '_' || ch == '.' || ch == '/' {
            expanded.push(' ');
            previous_lower_or_digit = false;
            continue;
        }
        if ch.is_ascii_uppercase() && previous_lower_or_digit {
            expanded.push(' ');
        }
        previous_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        expanded.push(ch);
    }
    repo::tokenize(&expanded)
        .into_iter()
        .filter(|term| meaningful_surface_term(term))
        .collect()
}

fn meaningful_surface_term(term: &str) -> bool {
    term.len() >= 3
        && !matches!(
            term,
            "app"
                | "apps"
                | "src"
                | "lib"
                | "libs"
                | "test"
                | "tests"
                | "spec"
                | "unit"
                | "e2e"
                | "tsx"
                | "jsx"
                | "mjs"
                | "cjs"
                | "typescript"
                | "javascript"
                | "component"
                | "components"
                | "feature"
                | "features"
                | "page"
                | "pages"
                | "hook"
                | "hooks"
                | "util"
                | "utils"
                | "index"
                | "main"
                | "type"
                | "types"
                | "support"
                | "setup"
                | "helper"
                | "helpers"
                | "fixture"
                | "fixtures"
                | "blueprint"
        )
}

fn meaningful_surface_phrase(phrase: &str) -> bool {
    let terms = surface_phrase_terms(phrase);
    terms.len() >= 2
        && terms
            .iter()
            .any(|term| !matches!(term.as_str(), "frame" | "title" | "canvas" | "node"))
}

fn surface_phrase_terms(phrase: &str) -> BTreeSet<String> {
    surface_terms(&phrase.replace(['.', '#', '/', '-', '_', ':'], " "))
        .into_iter()
        .filter(|term| term.len() >= 3)
        .filter(|term| {
            !matches!(
                term.as_str(),
                "the"
                    | "and"
                    | "for"
                    | "with"
                    | "from"
                    | "true"
                    | "false"
                    | "null"
                    | "undefined"
                    | "data"
                    | "test"
                    | "testid"
                    | "aria"
                    | "label"
                    | "role"
                    | "root"
                    | "blueprint"
                    | "nodrag"
                    | "nopan"
            )
        })
        .collect()
}

fn surface_terms(value: &str) -> BTreeSet<String> {
    value
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .map(str::to_lowercase)
        .filter(|term| term.len() >= 2)
        .collect()
}

fn same_parent_or_test_scope(source: &str, test: &str) -> bool {
    let source_parent = Path::new(source)
        .parent()
        .map(|path| repo::normalize_rel_path(&path.to_string_lossy()))
        .unwrap_or_else(|| ".".to_string());
    if test.starts_with(&format!("{}/", source_parent.trim_end_matches('/'))) {
        return true;
    }
    let source_stem = source_stem(source);
    meaningful_stem(&source_stem) && test.to_ascii_lowercase().contains(&source_stem)
}

fn surface_priority(kind: &str) -> usize {
    if kind == "domain" {
        return 0;
    }
    if kind.starts_with("package:") {
        return 1;
    }
    if kind == "dir" {
        return 2;
    }
    if kind == "script" {
        return 3;
    }
    if kind.starts_with("recursive:") {
        return 11;
    }
    if kind.starts_with("support_package:") {
        return 10;
    }
    match kind {
        "schema_contract" | "public_boundary" => 4,
        "runtime_state" | "persistence" | "adapter" | "parser" | "renderer_ui" => 5,
        "test" | "e2e_test" | "test_support" => 6,
        "source" => 7,
        "config" | "build_ci" => 8,
        _ => 9,
    }
}

