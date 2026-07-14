// Responsibility: route-service-test-triplet-grouping
use crate::map::{RuntimeFactIndex, file_matches_place_kind, route_from_path};
use crate::model::{EvidenceStrength, FileInfo, Surface};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Default)]
struct TripletParts {
    routes: Vec<String>,
    services: Vec<String>,
    tests: Vec<String>,
}

pub(crate) fn route_service_test_triplets(
    scope: &str,
    runtime_facts: &RuntimeFactIndex,
    files: &[&FileInfo],
) -> Vec<Surface> {
    let mut groups: BTreeMap<String, TripletParts> = BTreeMap::new();
    for file in files {
        let key = feature_stem(&file.rel);
        let group = groups.entry(key).or_default();
        if route_from_path(&file.rel) || runtime_facts.has_routes_for_file(&file.rel) {
            group.routes.push(file.rel.clone());
        }
        if file_matches_place_kind(file, "service") {
            group.services.push(file.rel.clone());
        }
        if file.has_role("test") {
            group.tests.push(file.rel.clone());
        }
    }
    let mut out = Vec::new();
    for (key, mut group) in groups {
        group.routes.sort();
        group.services.sort();
        group.tests.sort();
        let mut examples = Vec::new();
        examples.extend(group.routes.iter().take(2).cloned());
        examples.extend(group.services.iter().take(2).cloned());
        examples.extend(group.tests.iter().take(2).cloned());
        examples.sort();
        examples.dedup();
        let role_count = [
            !group.routes.is_empty(),
            !group.services.is_empty(),
            !group.tests.is_empty(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count();
        if role_count < 2 {
            continue;
        }
        let total_count = group.routes.len() + group.services.len() + group.tests.len();
        let hidden_count = total_count.saturating_sub(examples.len());
        out.push(Surface {
            id: format!("surface:triplet:{scope}:{key}"),
            kind: "route_service_test_triplet".to_string(),
            path: None,
            role: Some("local_convention".to_string()),
            evidence: "same_scope_stem_and_role".to_string(),
            strength: EvidenceStrength::Medium,
            count: Some(total_count),
            examples,
            hidden_count,
        });
    }
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.id.cmp(&b.id)));
    out
}

fn feature_stem(rel: &str) -> String {
    let file_name = Path::new(rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(rel);
    let without_ext = file_name
        .trim_end_matches(".tsx")
        .trim_end_matches(".jsx")
        .trim_end_matches(".ts")
        .trim_end_matches(".js")
        .trim_end_matches(".py")
        .trim_end_matches(".go")
        .trim_end_matches(".rs");
    without_ext
        .replace(".test", "")
        .replace(".spec", "")
        .replace("-test", "")
        .replace("_test", "")
        .replace("-route", "")
        .replace("_route", "")
        .replace("-routes", "")
        .replace("_routes", "")
        .replace("-service", "")
        .replace("_service", "")
        .replace("-controller", "")
        .replace("_controller", "")
}
