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
    let mut routes = Vec::new();
    let mut routes_by_file: BTreeMap<String, Vec<RuntimeRoute>> = BTreeMap::new();
    let mut route_visits = Vec::new();

    for file in project.files.values() {
        let file_routes = runtime_routes_for_file(project, file);
        if !file_routes.is_empty() {
            routes_by_file.insert(file.rel.clone(), file_routes.clone());
            routes.extend(file_routes);
        }
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
