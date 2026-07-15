// Responsibility: runtime-lens-facts
use crate::map::{
    directory_has_files, files_under_directory, package_for_rel, route_guard_owner,
    route_visit_locations, runtime_path_context, runtime_route_path_analysis,
    runtime_routes_for_file,
};
use crate::model::{EvidenceLocation, FileInfo, Project, RuntimeRoute, StructuralEdge, Unknown};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub(crate) struct RuntimeFactIndex {
    pub(crate) routes: Vec<RuntimeRoute>,
    pub(crate) routes_by_file: BTreeMap<String, Vec<RuntimeRoute>>,
    pub(crate) route_visits: Vec<RouteVisitFact>,
    route_paths: BTreeMap<String, Vec<StructuralEdge>>,
    route_unknowns: BTreeMap<String, Vec<Unknown>>,
}

pub(crate) struct RouteVisitFact {
    pub(crate) file: String,
    pub(crate) path: String,
    pub(crate) locations: Vec<EvidenceLocation>,
}

pub(crate) fn runtime_fact_index(project: &Project) -> RuntimeFactIndex {
    runtime_fact_index_for_files(project, project.files.values())
}

pub(crate) fn runtime_fact_index_for_paths(
    project: &Project,
    paths: &[String],
) -> RuntimeFactIndex {
    let route_files = paths
        .iter()
        .filter_map(|path| project.files.get(path))
        .collect::<Vec<_>>();
    runtime_fact_index_for_files(project, route_files)
}

pub(crate) fn proof_map_route_index_paths(
    project: &Project,
    scope: Option<&str>,
    seeds: &[String],
) -> Vec<String> {
    let mut paths = BTreeSet::new();
    if let Some(scope) = scope
        && directory_has_files(project, scope)
    {
        paths.extend(
            files_under_directory(project, scope)
                .into_iter()
                .map(|file| file.rel.clone()),
        );
        return paths.into_iter().collect();
    }
    for seed in seeds {
        if let Some(package) = package_for_rel(project, seed) {
            paths.extend(
                files_under_directory(project, &package.path)
                    .into_iter()
                    .map(|file| file.rel.clone()),
            );
        } else if project.files.contains_key(seed) {
            paths.insert(seed.clone());
        }
    }
    paths.into_iter().collect()
}

pub(crate) fn runtime_fact_index_for_files<'a, I>(
    project: &Project,
    route_files: I,
) -> RuntimeFactIndex
where
    I: IntoIterator<Item = &'a FileInfo>,
{
    let mut routes = Vec::new();
    let mut routes_by_file: BTreeMap<String, Vec<RuntimeRoute>> = BTreeMap::new();
    let mut route_visits = Vec::new();
    let mut route_paths = BTreeMap::new();
    let mut route_unknowns = BTreeMap::new();
    let path_context = runtime_path_context(project);

    for file in route_files {
        let mut file_routes = runtime_routes_for_file(project, file);
        for route in &mut file_routes {
            for guard in &mut route.middleware_or_guards {
                guard.owner = route_guard_owner(project, &route.file, &guard.name);
            }
            let analysis = runtime_route_path_analysis(project, route, &path_context);
            route.middleware_or_guards.extend(analysis.guards);
            route.middleware_or_guards.sort_by(|a, b| {
                a.owner
                    .cmp(&b.owner)
                    .then_with(|| a.name.cmp(&b.name))
                    .then_with(|| a.kind.cmp(&b.kind))
            });
            route
                .middleware_or_guards
                .dedup_by(|a, b| a.owner == b.owner && a.kind == b.kind);
            let key = runtime_route_key(route);
            route_paths.insert(key.clone(), analysis.edges);
            route_unknowns.insert(key, analysis.unknowns);
        }
        if !file_routes.is_empty() {
            routes_by_file.insert(file.rel.clone(), file_routes.clone());
            routes.extend(file_routes);
        }
    }

    for file in project.files.values() {
        if file.has_role("test") {
            for path in &file.visited_route_paths {
                route_visits.push(RouteVisitFact {
                    file: file.rel.clone(),
                    path: path.clone(),
                    locations: route_visit_locations(project, &file.rel, path),
                });
            }
        }
    }

    RuntimeFactIndex {
        routes,
        routes_by_file,
        route_visits,
        route_paths,
        route_unknowns,
    }
}

impl RuntimeFactIndex {
    pub(crate) fn routes_for_file(&self, rel: &str) -> Vec<RuntimeRoute> {
        self.routes_by_file.get(rel).cloned().unwrap_or_default()
    }

    pub(crate) fn has_routes_for_file(&self, rel: &str) -> bool {
        self.routes_by_file
            .get(rel)
            .is_some_and(|routes| !routes.is_empty())
    }

    pub(crate) fn paths_for_route(&self, route: &RuntimeRoute) -> Vec<StructuralEdge> {
        self.route_paths
            .get(&runtime_route_key(route))
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn unknowns_for_route(&self, route: &RuntimeRoute) -> Vec<Unknown> {
        self.route_unknowns
            .get(&runtime_route_key(route))
            .cloned()
            .unwrap_or_default()
    }
}

fn runtime_route_key(route: &RuntimeRoute) -> String {
    format!(
        "{}\u{0}{}\u{0}{}",
        route.file,
        route.method.as_deref().unwrap_or("ANY"),
        route.path
    )
}
