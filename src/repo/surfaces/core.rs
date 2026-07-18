// Responsibility: repo-surfaces-core
use crate::repo::{
    accessible_name_surfaces_from_native_labelled_ids, add_playwright_role_name_surfaces,
    apply_js_brace_delta, line_accepts_plain_label_surface,
    line_declares_disabled_playwright_describe, line_declares_local_identifier,
    line_declares_local_page_binding, line_declares_pending_nested_body,
    line_declares_playwright_page_fixture, line_ends_js_statement, line_has_arrow_callback,
    line_has_jsx_surface_container, line_has_playwright_scope_terminator_before_role_name_call,
    line_has_surface_context, line_opens_control_flow_body, line_opens_pending_control_flow_body,
    line_opens_pending_nested_body, line_starts_arrow_callback_body,
    line_starts_nested_playwright_body, line_starts_playwright_describe_callback_body,
    line_starts_unparsed_playwright_control_flow, line_terminates_playwright_page_scope,
    merge_surface_extraction, normalize_route_path, page_goto_url_binding,
    playwright_test_bindings, quoted_prefix_has_object_key, quoted_prefix_is_page_goto_argument,
    quoted_strings, quoted_value_is_module_specifier_context, static_jsx_visible_text,
    static_url_route_binding, strip_js_comments_from_line, surface_label_literal_is_structural,
    surface_literal_is_structural, surface_literal_phrases, surface_literal_terms,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub(crate) struct SurfaceExtraction {
    pub(crate) tokens: BTreeSet<String>,
    pub(crate) phrases: BTreeSet<String>,
    pub(crate) visited_routes: BTreeSet<String>,
}

pub(crate) fn extract_surfaces(text: &str, ext: &str) -> SurfaceExtraction {
    if !matches!(
        ext,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte"
    ) {
        return SurfaceExtraction::default();
    }
    let mut surfaces = SurfaceExtraction::default();
    let mut in_block_comment = false;
    let mut jsx_visible_text_context = 0usize;
    let mut js_brace_depth = 0usize;
    let mut playwright_page_scope_depth: Option<usize> = None;
    let mut playwright_page_shadowed = false;
    let mut playwright_nested_body_depth: Option<usize> = None;
    let mut playwright_nested_expression_arrow = false;
    let mut playwright_pending_nested_body = false;
    let mut disabled_playwright_scope_depth: Option<usize> = None;
    let mut pending_disabled_playwright_describe = false;
    let mut playwright_control_flow_depth: Option<usize> = None;
    let mut playwright_pending_control_flow = false;
    let mut playwright_page_scope_terminated = false;
    let playwright_test_bindings = playwright_test_bindings(text);
    let mut static_url_routes = BTreeMap::new();
    let mut shadowed_playwright_test_bindings = BTreeSet::new();
    let mut pending_playwright_role_names = BTreeMap::new();
    for raw_line in text.lines() {
        let line = strip_js_comments_from_line(raw_line, &mut in_block_comment);
        if let Some((binding, route)) = static_url_route_binding(&line) {
            static_url_routes.insert(binding, route);
        }
        if let Some(binding) = page_goto_url_binding(&line)
            && let Some(route) = static_url_routes.get(&binding)
        {
            surfaces.visited_routes.insert(route.clone());
        }
        for binding in &playwright_test_bindings {
            if line_declares_local_identifier(&line, binding) {
                shadowed_playwright_test_bindings.insert(binding.clone());
            }
        }
        let active_playwright_test_bindings;
        let active_playwright_test_bindings = if shadowed_playwright_test_bindings.is_empty() {
            &playwright_test_bindings
        } else {
            active_playwright_test_bindings = playwright_test_bindings
                .difference(&shadowed_playwright_test_bindings)
                .cloned()
                .collect::<BTreeSet<_>>();
            &active_playwright_test_bindings
        };
        if disabled_playwright_scope_depth.is_none()
            && line_declares_disabled_playwright_describe(&line, active_playwright_test_bindings)
        {
            pending_disabled_playwright_describe = true;
        }
        if pending_disabled_playwright_describe
            && line_starts_playwright_describe_callback_body(&line)
        {
            disabled_playwright_scope_depth = Some(js_brace_depth + 1);
            pending_disabled_playwright_describe = false;
        }
        let disabled_playwright_context =
            disabled_playwright_scope_depth.is_some() || pending_disabled_playwright_describe;
        let mut pending_control_flow_set_this_line = false;
        let entered_playwright_page_scope = !disabled_playwright_context
            && line_declares_playwright_page_fixture(&line, active_playwright_test_bindings);
        if entered_playwright_page_scope {
            playwright_page_scope_depth = Some(js_brace_depth + 1);
            playwright_page_shadowed = false;
            playwright_nested_body_depth = None;
            playwright_nested_expression_arrow = false;
            playwright_pending_nested_body = false;
            playwright_control_flow_depth = None;
            playwright_pending_control_flow = false;
            playwright_page_scope_terminated = false;
            pending_playwright_role_names.clear();
        }
        if !entered_playwright_page_scope
            && playwright_page_scope_depth.is_some()
            && playwright_pending_control_flow
            && line_opens_pending_control_flow_body(&line)
        {
            playwright_control_flow_depth = Some(js_brace_depth + 1);
            playwright_pending_control_flow = false;
            pending_playwright_role_names.clear();
        }
        if !entered_playwright_page_scope
            && playwright_page_scope_depth.is_some()
            && playwright_pending_nested_body
            && line_opens_pending_nested_body(&line)
        {
            playwright_nested_body_depth = Some(js_brace_depth + 1);
            playwright_pending_nested_body = false;
            pending_playwright_role_names.clear();
        }
        if !entered_playwright_page_scope
            && playwright_page_scope_depth.is_some()
            && playwright_nested_body_depth.is_none()
            && !playwright_nested_expression_arrow
            && !playwright_pending_nested_body
            && line_has_arrow_callback(&line)
        {
            if line_starts_arrow_callback_body(&line) {
                playwright_nested_body_depth = Some(js_brace_depth + 1);
            } else {
                playwright_nested_expression_arrow = true;
            }
            pending_playwright_role_names.clear();
        }
        if !entered_playwright_page_scope
            && playwright_page_scope_depth.is_some()
            && playwright_nested_body_depth.is_none()
            && !playwright_nested_expression_arrow
            && !playwright_pending_nested_body
            && line_starts_nested_playwright_body(&line)
        {
            playwright_nested_body_depth = Some(js_brace_depth + 1);
            pending_playwright_role_names.clear();
        }
        if !entered_playwright_page_scope
            && playwright_page_scope_depth.is_some()
            && playwright_nested_body_depth.is_none()
            && !playwright_nested_expression_arrow
            && !playwright_pending_nested_body
            && line_declares_pending_nested_body(&line)
        {
            playwright_pending_nested_body = true;
            pending_playwright_role_names.clear();
        }
        if !entered_playwright_page_scope
            && playwright_page_scope_depth.is_some()
            && line_declares_local_page_binding(&line)
        {
            playwright_page_shadowed = true;
            pending_playwright_role_names.clear();
        }
        if !entered_playwright_page_scope
            && playwright_page_scope_depth.is_some()
            && playwright_nested_body_depth.is_none()
            && !playwright_nested_expression_arrow
            && !playwright_pending_nested_body
            && line_starts_unparsed_playwright_control_flow(&line)
        {
            if line_opens_control_flow_body(&line) {
                playwright_control_flow_depth = Some(js_brace_depth + 1);
                playwright_pending_control_flow = false;
            } else {
                playwright_pending_control_flow = true;
                pending_control_flow_set_this_line = true;
            }
            pending_playwright_role_names.clear();
        }
        if !entered_playwright_page_scope
            && !disabled_playwright_context
            && playwright_page_scope_depth.is_some()
            && !playwright_page_shadowed
            && playwright_nested_body_depth.is_none()
            && !playwright_nested_expression_arrow
            && !playwright_pending_nested_body
            && playwright_control_flow_depth.is_none()
            && !playwright_pending_control_flow
            && !playwright_page_scope_terminated
            && !line_has_playwright_scope_terminator_before_role_name_call(
                &line,
                active_playwright_test_bindings,
            )
        {
            add_playwright_role_name_surfaces(
                &mut surfaces,
                &mut pending_playwright_role_names,
                &line,
            );
        }
        let has_surface_context = line_has_surface_context(&line);
        if (jsx_visible_text_context > 0 || line_has_jsx_surface_container(&line))
            && let Some(text) = static_jsx_visible_text(&line)
        {
            surfaces
                .phrases
                .extend(surface_literal_phrases(&text, true));
        }
        if line_has_jsx_surface_container(&line) {
            jsx_visible_text_context = 4;
        } else {
            jsx_visible_text_context = jsx_visible_text_context.saturating_sub(1);
        }
        if has_surface_context {
            let plain_label_context = line_accepts_plain_label_surface(&line);
            for quoted in quoted_strings(&line) {
                if quoted_prefix_has_object_key(&quoted.prefix, "name")
                    && line.to_ascii_lowercase().contains("getbyrole")
                {
                    continue;
                }
                if quoted_value_is_module_specifier_context(&quoted.prefix) {
                    continue;
                }
                let value = quoted.value;
                if quoted_prefix_is_page_goto_argument(&quoted.prefix)
                    && let Some(route) = normalize_route_path(&value)
                {
                    surfaces.visited_routes.insert(route);
                }
                let structural_literal = surface_literal_is_structural(&value)
                    || (plain_label_context && surface_label_literal_is_structural(&value));
                if !structural_literal {
                    continue;
                }
                surfaces.tokens.extend(surface_literal_terms(&value));
                surfaces
                    .phrases
                    .extend(surface_literal_phrases(&value, plain_label_context));
            }
        }
        if !entered_playwright_page_scope
            && !disabled_playwright_context
            && playwright_page_scope_depth.is_some()
            && playwright_nested_body_depth.is_none()
            && !playwright_nested_expression_arrow
            && !playwright_pending_nested_body
            && line_terminates_playwright_page_scope(&line, active_playwright_test_bindings)
        {
            playwright_page_scope_terminated = true;
            pending_playwright_role_names.clear();
        }
        js_brace_depth = apply_js_brace_delta(js_brace_depth, &line);
        if disabled_playwright_scope_depth
            .map(|scope_depth| js_brace_depth < scope_depth)
            .unwrap_or(false)
        {
            disabled_playwright_scope_depth = None;
            pending_disabled_playwright_describe = false;
        }
        if playwright_page_scope_depth
            .map(|scope_depth| js_brace_depth < scope_depth)
            .unwrap_or(false)
        {
            playwright_page_scope_depth = None;
            playwright_page_shadowed = false;
            playwright_nested_body_depth = None;
            playwright_nested_expression_arrow = false;
            playwright_pending_nested_body = false;
            playwright_page_scope_terminated = false;
            pending_playwright_role_names.clear();
        }
        if playwright_nested_body_depth
            .map(|scope_depth| js_brace_depth < scope_depth)
            .unwrap_or(false)
        {
            playwright_nested_body_depth = None;
        }
        if playwright_control_flow_depth
            .map(|scope_depth| js_brace_depth < scope_depth)
            .unwrap_or(false)
        {
            playwright_control_flow_depth = None;
        }
        if playwright_pending_control_flow && !pending_control_flow_set_this_line {
            playwright_pending_control_flow = false;
        }
        if playwright_nested_expression_arrow && line_ends_js_statement(&line) {
            playwright_nested_expression_arrow = false;
        }
    }
    merge_surface_extraction(
        &mut surfaces,
        accessible_name_surfaces_from_native_labelled_ids(text),
    );
    surfaces
}
