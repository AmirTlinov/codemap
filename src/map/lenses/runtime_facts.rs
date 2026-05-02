struct RuntimeFactIndex {
    routes: Vec<RuntimeRoute>,
    routes_by_file: BTreeMap<String, Vec<RuntimeRoute>>,
    route_visits: Vec<RouteVisitFact>,
}

struct RouteVisitFact {
    file: String,
    path: String,
    locations: Vec<EvidenceLocation>,
}

fn runtime_fact_index(project: &Project) -> RuntimeFactIndex {
    runtime_fact_index_for_files(project, project.files.values())
}

fn runtime_fact_index_for_paths(project: &Project, paths: &[String]) -> RuntimeFactIndex {
    let route_files = paths
        .iter()
        .filter_map(|path| project.files.get(path))
        .collect::<Vec<_>>();
    runtime_fact_index_for_files(project, route_files)
}

fn proof_map_route_index_paths(
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

fn runtime_fact_index_for_files<'a, I>(project: &Project, route_files: I) -> RuntimeFactIndex
where
    I: IntoIterator<Item = &'a FileInfo>,
{
    let mut routes = Vec::new();
    let mut routes_by_file: BTreeMap<String, Vec<RuntimeRoute>> = BTreeMap::new();
    let mut route_visits = Vec::new();

    for file in route_files {
        let file_routes = runtime_routes_for_file(project, file);
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
    }
}

impl RuntimeFactIndex {
    fn routes_for_file(&self, rel: &str) -> Vec<RuntimeRoute> {
        self.routes_by_file.get(rel).cloned().unwrap_or_default()
    }

    fn has_routes_for_file(&self, rel: &str) -> bool {
        self.routes_by_file
            .get(rel)
            .is_some_and(|routes| !routes.is_empty())
    }
}
