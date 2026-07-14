// Responsibility: repo-js-jsx-accessible-names
use crate::repo::{
    JsxOpeningTagSpan, SurfaceExtraction, find_jsx_closing_tag_start,
    jsx_byte_is_inside_custom_component_boundary, jsx_byte_is_inside_expression,
    jsx_element_visible_text, jsx_opening_tag_spans, jsx_single_static_attr_value,
    normalize_surface_phrase, quoted_prefix_has_jsx_attr, quoted_strings,
    strip_js_comments_from_text, surface_label_literal_is_structural, surface_literal_phrases,
    surface_literal_terms, surface_phrase_terms,
};
use std::collections::BTreeSet;

pub(crate) fn accessible_name_surfaces_from_native_labelled_ids(text: &str) -> SurfaceExtraction {
    let stripped = strip_js_comments_from_text(text);
    let mut surfaces = SurfaceExtraction::default();
    for opening in jsx_opening_tag_spans(&stripped) {
        let Some(role) = jsx_accessible_role_for_opening(&opening.tag, &opening.source) else {
            continue;
        };
        let Some(id) = jsx_single_static_attr_value(&opening.source, "aria-labelledby") else {
            continue;
        };
        let mut roles = BTreeSet::new();
        roles.insert(role);
        add_accessible_name_surface_from_label_in_opening_scope(
            &mut surfaces,
            &stripped,
            &opening,
            &id,
            &roles,
        );
    }
    surfaces
}

pub(crate) fn jsx_accessible_role_for_opening(tag: &str, opening: &str) -> Option<String> {
    if tag != tag.to_ascii_lowercase() {
        return None;
    }
    let mut role_attrs = 0usize;
    let mut roles = Vec::new();
    for quoted in quoted_strings(opening) {
        if quoted_prefix_has_jsx_attr(&quoted.prefix, "role") {
            role_attrs += 1;
            if let Some(role) = normalize_accessible_role(&quoted.value) {
                roles.push(role);
            }
        }
    }
    if role_attrs > 0 {
        return match roles.as_slice() {
            [role] if role_attrs == 1 => Some(role.clone()),
            _ => None,
        };
    }
    normalize_accessible_role(tag)
}

pub(crate) fn normalize_accessible_role(value: &str) -> Option<String> {
    let role = value.trim().to_ascii_lowercase();
    match role.as_str() {
        "alertdialog" => Some("alertdialog".to_string()),
        "dialog" => Some("dialog".to_string()),
        _ => None,
    }
}

pub(crate) fn add_accessible_name_surface_from_label_in_opening_scope(
    surfaces: &mut SurfaceExtraction,
    text: &str,
    opening: &JsxOpeningTagSpan,
    id: &str,
    roles: &BTreeSet<String>,
) {
    if opening.self_closing {
        return;
    }
    let Some(close_start) = find_jsx_closing_tag_start(text, &opening.tag, opening.opening_end)
    else {
        return;
    };
    if close_start < opening.opening_end {
        return;
    }
    let body = &text[opening.opening_end..close_start];
    let mut matching_label_count = 0usize;
    let mut matching_labels = Vec::new();
    for candidate in jsx_opening_tag_spans(body) {
        if jsx_single_static_attr_value(&candidate.source, "id").as_deref() != Some(id) {
            continue;
        }
        matching_label_count += 1;
        if jsx_byte_is_inside_expression(body, candidate.start) {
            continue;
        }
        if jsx_byte_is_inside_custom_component_boundary(body, candidate.start) {
            continue;
        }
        if candidate.self_closing {
            continue;
        }
        if let Some(text) = jsx_element_visible_text(body, &candidate) {
            matching_labels.push(text);
        }
    }
    if matching_label_count == 1
        && let [text] = matching_labels.as_slice()
    {
        add_accessible_name_parts_surface(surfaces, roles, std::slice::from_ref(text));
    }
}

fn add_accessible_name_parts_surface(
    surfaces: &mut SurfaceExtraction,
    roles: &BTreeSet<String>,
    parts: &[String],
) {
    let text = parts
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if !text.is_empty() {
        add_accessible_role_name_surfaces(surfaces, roles, &text);
    }
}

fn add_accessible_role_name_surfaces(
    surfaces: &mut SurfaceExtraction,
    roles: &BTreeSet<String>,
    name: &str,
) {
    surfaces.tokens.extend(surface_literal_terms(name));
    surfaces.phrases.extend(surface_literal_phrases(name, true));
    for role in roles {
        add_accessible_role_name_surface(surfaces, role, name);
    }
}

pub(crate) fn add_accessible_role_name_surface(
    surfaces: &mut SurfaceExtraction,
    role: &str,
    name: &str,
) {
    if !surface_label_literal_is_structural(name) {
        return;
    }
    let Some(name_phrase) = normalize_surface_phrase(name) else {
        return;
    };
    let terms = surface_phrase_terms(&name_phrase);
    if terms.is_empty() {
        return;
    }
    surfaces.tokens.extend(terms);
    surfaces
        .phrases
        .insert(format!("a11y-role-{role}-name-{name_phrase}"));
}
