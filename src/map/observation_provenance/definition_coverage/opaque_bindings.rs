// Responsibility: javascript-opaque-public-binding-closure-audit

use super::declaration_bindings::{declaration_name_after, exact_identifier_occurs};

pub(super) fn commonjs_export_binding_occurs(text: &str, code: &str, query: &str) -> bool {
    let assignment = crate::map::identifier_ranges(code, query).any(|(start, end)| {
        let owner = code[..start].trim_end();
        let assigned = code[end..].trim_start().starts_with('=');
        assigned && (owner.ends_with("exports.") || owner.ends_with("module.exports."))
    });
    assignment
        || crate::map::runtime_code_lines(text)
            .into_iter()
            .any(|(_, line)| {
                let shape = crate::map::code_shape_without_literal_content(&line);
                let define_property = shape.contains("Object.defineProperty(exports,")
                    || shape.contains("Object.defineProperty(module.exports,");
                let bracket_assignment = (shape.contains("exports[")
                    || shape.contains("module.exports["))
                    && shape.contains("] =");
                (define_property || bracket_assignment)
                    && crate::map::quoted_literal_contents(&line)
                        .iter()
                        .any(|literal| literal == query)
            })
}

pub(super) fn member_binding_occurs(code: &str, query: &str) -> bool {
    crate::map::identifier_ranges(code, query).any(|(start, end)| {
        if javascript_brace_depth(&code[..start]) == 0 {
            return false;
        }
        let after = &code[end..];
        let Some(next_offset) = after.find(|ch: char| !ch.is_whitespace()) else {
            return false;
        };
        match after.as_bytes().get(next_offset).copied() {
            Some(b'=' | b':') => true,
            Some(b'(') => crate::map::matching_close_paren(code, end + next_offset)
                .is_some_and(|close| member_body_starts(&code[close + 1..])),
            _ => false,
        }
    })
}

fn member_body_starts(after_parameters: &str) -> bool {
    let tail = after_parameters.trim_start();
    if tail.starts_with('{') {
        return true;
    }
    if !tail.starts_with(':') {
        return false;
    }
    tail.lines().next().is_some_and(|line| line.contains('{'))
}

fn javascript_brace_depth(code: &str) -> usize {
    code.bytes().fold(0usize, |depth, byte| match byte {
        b'{' => depth.saturating_add(1),
        b'}' => depth.saturating_sub(1),
        _ => depth,
    })
}

pub(super) fn export_clause_binds(code: &str, query: &str) -> bool {
    crate::repo::js_keyword_positions(code, "export")
        .into_iter()
        .any(|start| {
            let tail = code[start + "export".len()..].trim_start();
            if let Some(after_star) = tail.strip_prefix('*') {
                return crate::repo::js_keyword_positions(after_star, "as")
                    .into_iter()
                    .any(|as_start| {
                        declaration_name_after(after_star, as_start + "as".len()) == Some(query)
                    });
            }
            let tail = tail.strip_prefix("type ").unwrap_or(tail).trim_start();
            if !tail.starts_with('{') {
                return false;
            }
            let Some(end) = crate::repo::js_balanced_pattern_end(tail, 0) else {
                return exact_identifier_occurs(tail, query);
            };
            crate::repo::js_split_top_level_commas(&tail[1..end])
                .iter()
                .any(|binding| export_binding_name(binding) == Some(query))
        })
}

fn export_binding_name(binding: &str) -> Option<&str> {
    let binding = binding
        .trim()
        .strip_prefix("type ")
        .unwrap_or(binding.trim());
    if let Some(as_start) = crate::repo::js_keyword_positions(binding, "as")
        .into_iter()
        .next()
    {
        declaration_name_after(binding, as_start + "as".len())
    } else {
        declaration_name_after(binding, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::{commonjs_export_binding_occurs, export_clause_binds, member_binding_occurs};

    #[test]
    fn export_clause_audit_finds_public_aliases_and_namespace_bindings() {
        assert!(export_clause_binds(
            "function internal() {} export { internal as PublicTarget };",
            "PublicTarget"
        ));
        assert!(export_clause_binds(
            "export { internal as PublicTarget } from             ;",
            "PublicTarget"
        ));
        assert!(export_clause_binds(
            "export * as PublicTarget from             ;",
            "PublicTarget"
        ));
        assert!(!export_clause_binds(
            "export { PublicTarget as internal };",
            "PublicTarget"
        ));
    }

    #[test]
    fn member_audit_finds_methods_accessors_fields_and_object_properties() {
        let code = r#"
            class Api {
                target() {}
                async AsyncTarget() {}
                *GeneratorTarget() {}
                get GetterTarget() { return 1; }
                set SetterTarget(value) {}
                FieldTarget = 1;
            }
            const object = { ObjectTarget() {}, ArrowTarget: () => 1 };
        "#;
        for query in [
            "target",
            "AsyncTarget",
            "GeneratorTarget",
            "GetterTarget",
            "SetterTarget",
            "FieldTarget",
            "ObjectTarget",
            "ArrowTarget",
        ] {
            assert!(member_binding_occurs(code, query), "missing {query}");
        }
        assert!(!member_binding_occurs(
            "function owner() { return target(); }",
            "target"
        ));
    }

    #[test]
    fn commonjs_audit_finds_public_assignment_and_property_bindings() {
        for (source, query) in [
            ("exports.PublicTarget = function() {};", "PublicTarget"),
            ("module.exports.OtherTarget = class {};", "OtherTarget"),
            (
                "Object.defineProperty(exports, \"GetterTarget\", { get() {} });",
                "GetterTarget",
            ),
            ("exports[\"BracketTarget\"] = value;", "BracketTarget"),
        ] {
            let code = crate::repo::code_without_comments_or_strings(source, "cjs");
            assert!(
                commonjs_export_binding_occurs(source, &code, query),
                "missing {query}"
            );
        }
    }
}
