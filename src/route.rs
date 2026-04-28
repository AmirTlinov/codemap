use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use serde::Serialize;

use crate::cache;
use crate::model::{
    BoundaryFinding, BoundaryReport, CacheInfo, Candidate, Confidence, DoNotRead, Domain,
    DomainRef, ExplainReport, FileInfo, FileRisk, ImpactReport, LocateCandidate, LocateReport,
    Project, Risk, TaskCapsule, VerificationPlan, VerifyReport, WidenReport,
};
use crate::repo;

mod graph_lens;
pub use graph_lens::graph_lens;

#[derive(Debug, Serialize)]
pub struct StatusReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub root: String,
    pub cwd: String,
    pub vcs: Option<String>,
    pub config: Option<String>,
    pub config_errors: Vec<String>,
    pub nearest_agents: Option<String>,
    pub cache_dir: String,
    pub cache_state: String,
    pub cache_artifacts: Vec<crate::model::CacheArtifactStatus>,
    pub zero_footprint_default: bool,
    pub package_manager: String,
    pub languages: Vec<String>,
    pub files_scanned: usize,
    pub domains: Vec<DomainStatus>,
    pub scripts: Vec<String>,
    pub fingerprint: String,
    pub boundary_findings: usize,
    pub unclassified_source_files: Vec<String>,
    pub unclassified_count: usize,
}

#[derive(Debug, Serialize)]
pub struct DomainStatus {
    pub id: String,
    pub path: String,
    pub config: Option<String>,
}

pub fn status_report(project: &Project) -> StatusReport {
    let unclassified: Vec<String> = project
        .files
        .values()
        .filter(|file| repo::is_source_ext(&file.ext) && file.roles.is_empty())
        .map(|file| file.rel.clone())
        .collect();
    StatusReport {
        kind: "status_report",
        schema_version: "1",
        root: project.root.to_string_lossy().to_string(),
        cwd: project.cwd.to_string_lossy().to_string(),
        vcs: project.vcs.clone(),
        config: project.config_path.clone(),
        config_errors: project
            .config_errors
            .iter()
            .map(|error| format!("{}: {}", error.path, error.error))
            .collect(),
        nearest_agents: project.nearest_agents.clone(),
        cache_dir: project.cache_dir.to_string_lossy().to_string(),
        cache_state: project.cache_state.clone(),
        cache_artifacts: project.cache_artifacts.clone(),
        zero_footprint_default: true,
        package_manager: project.package_manager.clone(),
        languages: project.languages.iter().cloned().collect(),
        files_scanned: project.files.len(),
        domains: project
            .domains
            .iter()
            .map(|d| DomainStatus {
                id: d.id.clone(),
                path: d.path.clone(),
                config: d.config_path.clone(),
            })
            .collect(),
        scripts: project.scripts.iter().map(|s| s.command.clone()).collect(),
        fingerprint: cache::fingerprint(project, None),
        boundary_findings: boundary_findings(project, None).len(),
        unclassified_count: unclassified.len(),
        unclassified_source_files: unclassified.into_iter().take(30).collect(),
    }
}

pub fn locate_report(project: &Project, task: &str, limit: usize) -> LocateReport {
    let task_tokens = route_text_tokens(task);
    let mut scored = Vec::new();
    for domain in &project.domains {
        let (kind, kind_conf, mut reasons) = task_kind(project, domain, task);
        let mut score = 0.0;
        let domain_tokens = route_text_tokens(&format!("{} {}", domain.id, domain.path));
        let overlap = token_overlap(&task_tokens, &domain_tokens);
        let has_domain_overlap = !overlap.is_empty();
        if !overlap.is_empty() {
            score += 2.0 * overlap.len() as f64;
            reasons.push(format!("task/domain overlap: {}", overlap.join(", ")));
        }
        let candidates = select_read_first(project, domain, task, &kind, 3, &BTreeSet::new());
        if let Some(best) = candidates.first() {
            if candidate_has_specific_evidence(best, &kind) {
                score += best.score;
                reasons.push(format!("best local file: `{}`", best.path));
            } else if kind == "general" {
                score += 0.4;
                reasons.push(format!("orientation file available: `{}`", best.path));
            } else {
                reasons.push("no task-specific file evidence found".to_string());
            }
        }
        let has_semantic_anchor = domain.config_path.is_some() || project.config_path.is_some();
        if has_semantic_anchor {
            score += 1.2;
            reasons.push("has semantic anchors".to_string());
        }
        let has_specific_file_evidence = candidates
            .iter()
            .any(|candidate| candidate_has_specific_evidence(candidate, &kind));
        let has_configured_route = reasons
            .iter()
            .any(|reason| reason.starts_with("matched configured route"));
        let mut confidence_score = (kind_conf + score / 22.0).min(1.0);
        if !has_specific_file_evidence
            && !has_domain_overlap
            && !has_semantic_anchor
            && !has_configured_route
        {
            confidence_score = confidence_score.min(0.45);
        } else if !has_specific_file_evidence && !has_semantic_anchor && !has_configured_route {
            confidence_score = confidence_score.min(0.68);
        }
        let confidence = confidence_from_score(confidence_score);
        scored.push(LocateCandidate {
            domain: domain.into(),
            score: round2(score),
            task_kind: kind,
            confidence: confidence.as_str().to_string(),
            reasons: unique(reasons).into_iter().take(6).collect(),
            start_command: format!(
                "ctx start --path {} --task {}",
                shell_quote(&domain.path),
                shell_quote(task)
            ),
        });
    }
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.domain.path.cmp(&b.domain.path))
    });
    LocateReport {
        kind: "location_candidates",
        schema_version: "1",
        task: task.to_string(),
        candidates: scored.into_iter().take(limit).collect(),
    }
}

pub fn start_capsule(
    project: &Project,
    task: &str,
    path: Option<&str>,
    limit: usize,
) -> TaskCapsule {
    let domain = primary_domain(project, task, path);
    let (kind, score_conf, mut reasons) = task_kind(project, &domain, task);
    let route = route_for_kind(project, &kind);
    let mut matched_configured_route = false;
    let mut read = Vec::<Candidate>::new();
    if let Some(route) = route {
        for rel in &route.read_first {
            let full = resolve_domain_pattern(&domain, rel);
            if project.files.contains_key(&full) {
                read.push(Candidate {
                    path: full,
                    score: 25.0,
                    reasons: vec!["configured read_first".to_string()],
                });
            }
        }
        if !read.is_empty() {
            matched_configured_route = true;
            reasons.push(format!("matched configured task route `{kind}`"));
        }
    }
    if read.is_empty() {
        let seed_limit = if kind == "build_ci" {
            limit
        } else {
            limit.min(5)
        };
        read = select_read_first(project, &domain, task, &kind, seed_limit, &BTreeSet::new());
    }
    let mut read_paths: Vec<String> = read.iter().map(|c| c.path.clone()).collect();
    for test in test_files_for(
        project,
        &read_paths,
        Some(&domain),
        7 - read_paths.len().min(7),
    ) {
        if read_paths.len() >= limit {
            break;
        }
        if !read_paths.contains(&test) {
            read.push(Candidate {
                path: test.clone(),
                score: 3.0,
                reasons: vec!["related test".to_string()],
            });
            read_paths.push(test);
        }
    }
    read.truncate(limit);
    read_paths = read.iter().map(|c| c.path.clone()).collect();
    let has_specific_read_evidence = read
        .iter()
        .any(|candidate| candidate_has_specific_evidence(candidate, &kind));
    let general_orientation_route =
        kind == "general" && !matched_configured_route && !has_specific_read_evidence;
    if !matched_configured_route && !has_specific_read_evidence && kind != "general" {
        read.clear();
        read_paths.clear();
    }
    let related_tests = test_files_for(project, &read_paths, Some(&domain), 3);
    let source_of_truth =
        if matched_configured_route || has_specific_read_evidence || general_orientation_route {
            source_truths(project, &domain)
        } else {
            Vec::new()
        };
    let public_boundaries = public_boundaries(project, &domain);
    let invariants = invariants_for(project, &domain, &read_paths);
    let mut conf_score = score_conf;
    if read.is_empty() {
        conf_score -= 0.25;
    }
    if !matched_configured_route && !has_specific_read_evidence {
        conf_score -= 0.45;
        reasons.push("no task-specific file or anchor evidence found".to_string());
    }
    if project.config_path.is_some() {
        conf_score += 0.08;
    }
    if path.is_none() && project.domains.len() == 1 && domain.path == "." {
        conf_score -= 0.05;
    }
    let confidence = if matched_configured_route {
        Confidence::High
    } else {
        confidence_from_score(conf_score.clamp(0.1, 1.0))
    };
    let risk = if confidence == Confidence::Low {
        Risk::MediumHigh
    } else {
        read_paths
            .iter()
            .map(|path| risk_for_file(project, path).0)
            .max()
            .unwrap_or(Risk::Medium)
    };
    let mut verification = verification_plan(project, &read_paths, &[]);
    if let Some(route) = route_for_kind(project, &kind)
        && !route.verify.is_empty()
    {
        verification.minimal = route.verify.clone();
    }
    let mut provenance = BTreeMap::new();
    provenance.insert(
        "structural".to_string(),
        "imports/manifests/filesystem/git".to_string(),
    );
    provenance.insert(
        "semantic".to_string(),
        if project.config_path.is_some() {
            "optional .ctx.yml anchors".to_string()
        } else {
            "heuristic only; no .ctx anchors found".to_string()
        },
    );
    TaskCapsule {
        kind: "task_context_capsule",
        schema_version: "1",
        task: task.to_string(),
        domain: (&domain).into(),
        task_kind: kind.clone(),
        confidence: confidence.as_str().to_string(),
        risk: risk.as_str().to_string(),
        read_first: read,
        related_tests,
        source_of_truth,
        public_boundaries,
        do_not_read_yet: do_not_read_yet(project, &domain, &kind, task, 8),
        forbidden_moves: forbidden_moves(project, &domain, &kind),
        invariants,
        verification,
        expansion_triggers: expansion_triggers(&kind),
        stop_conditions: stop_conditions(),
        provenance,
        cache: CacheInfo {
            path: project.cache_dir.to_string_lossy().to_string(),
            fingerprint: cache::fingerprint(project, Some(&domain.path)),
        },
    }
}

pub fn verify_report(
    project: &Project,
    changed: Vec<String>,
    depth: usize,
    limit: usize,
) -> VerifyReport {
    let impact = impact_report(project, changed, depth, limit);
    VerifyReport {
        kind: "verification_plan",
        schema_version: "1",
        changed: impact.changed,
        risk: impact.risk,
        impacted: impact.impacted,
        related_tests: impact.related_tests,
        verification: VerificationPlan {
            minimal: impact.minimal_verification,
            recommended: impact.recommended_verification,
            full_only_if_triggered: impact.full_verification,
        },
        expansion_triggers: impact.expansion_triggers,
    }
}

pub fn impact_report(
    project: &Project,
    changed: Vec<String>,
    depth: usize,
    limit: usize,
) -> ImpactReport {
    let changed: Vec<String> = changed
        .into_iter()
        .map(|f| repo::normalize_rel_path(&f))
        .filter(|f| f != ".")
        .collect();
    let mut impacted = impacted_files(project, &changed, depth, limit);
    let package_seed = [changed.clone(), impacted.clone()].concat();
    let package_impacted = package_consumer_manifests(
        project,
        &package_seed,
        depth,
        limit.saturating_sub(impacted.len()),
    );
    for rel in &package_impacted {
        if impacted.len() >= limit {
            break;
        }
        if !changed.contains(rel) && !impacted.contains(rel) {
            impacted.push(rel.clone());
        }
    }
    let related_tests = test_files_for(
        project,
        &[changed.clone(), impacted.clone()].concat(),
        None,
        10,
    );
    let mut max_risk = Risk::Low;
    let mut files = Vec::new();
    for file in &changed {
        let (risk, reasons) = risk_for_file(project, file);
        max_risk = max_risk.max(risk);
        files.push(FileRisk {
            path: file.clone(),
            risk: risk.as_str().to_string(),
            reasons,
        });
    }
    let domains = impacted_domains(project, &[changed.clone(), impacted.clone()].concat());
    let changed_domains: BTreeSet<String> = changed
        .iter()
        .filter_map(|f| domain_by_rel(project, f))
        .map(|d| d.id.clone())
        .collect();
    let external_domains: Vec<DomainRef> = domains
        .iter()
        .filter(|d| !changed_domains.contains(&d.id))
        .map(|d| (*d).into())
        .collect();
    let verification = verification_plan(project, &changed, &impacted);
    let mut triggers = Vec::new();
    let impacted_and_changed = [changed.clone(), impacted.clone()].concat();
    if matches!(max_risk, Risk::High | Risk::Critical) {
        triggers.push("high/critical risk change".to_string());
    }
    if has_role(project, &changed, "public_boundary") {
        triggers.push("public boundary changed".to_string());
    } else if has_role(project, &impacted, "public_boundary") {
        triggers.push("impact reaches public boundary".to_string());
    }
    if has_role(project, &changed, "schema_contract") {
        triggers.push("DTO/schema contract changed".to_string());
    }
    if has_role(project, &changed, "source_of_truth") {
        triggers.push("source of truth changed".to_string());
    }
    if has_unclassified_source(project, &impacted_and_changed) {
        triggers.push("unclassified source file participates".to_string());
    }
    if !external_domains.is_empty() {
        triggers.push("impact crosses domain boundary".to_string());
    }
    if !package_impacted.is_empty() {
        triggers.push("package consumers affected".to_string());
    }
    if changed.iter().any(|f| {
        project
            .files
            .get(f)
            .map(|info| info.has_role("generated"))
            .unwrap_or(false)
    }) {
        triggers.push("generated file changed".to_string());
    }
    triggers = unique(triggers);
    ImpactReport {
        kind: "impact_report",
        schema_version: "1",
        changed,
        risk: max_risk.as_str().to_string(),
        files,
        impacted,
        related_tests,
        domains: domains.into_iter().map(Into::into).collect(),
        external_domains,
        minimal_verification: verification.minimal,
        recommended_verification: verification.recommended,
        full_verification: verification.full_only_if_triggered,
        expansion_triggers: triggers,
    }
}

fn has_role(project: &Project, files: &[String], role: &str) -> bool {
    files.iter().any(|file| {
        project
            .files
            .get(file)
            .map(|info| info.has_role(role))
            .unwrap_or(false)
    })
}

fn has_unclassified_source(project: &Project, files: &[String]) -> bool {
    files.iter().any(|file| {
        project
            .files
            .get(file)
            .map(|info| repo::is_source_ext(&info.ext) && info.roles.is_empty())
            .unwrap_or(false)
    })
}

pub fn verification_plan(
    project: &Project,
    changed: &[String],
    impacted: &[String],
) -> VerificationPlan {
    let all_files: Vec<String> = changed
        .iter()
        .chain(impacted.iter())
        .cloned()
        .collect::<Vec<_>>();
    let domains = if all_files.is_empty() {
        project.domains.iter().collect::<Vec<_>>()
    } else {
        impacted_domains(project, &all_files)
    };
    let max_risk = all_files
        .iter()
        .map(|f| risk_for_file(project, f).0)
        .max()
        .unwrap_or(Risk::Low);

    let mut minimal = project.anchors.verification.default.clone();
    if minimal.is_empty() {
        minimal = infer_minimal_commands(project, &domains);
    }
    let mut recommended = Vec::new();
    if matches!(max_risk, Risk::MediumHigh | Risk::High | Risk::Critical)
        && let Some(typecheck) = find_script(project, &["typecheck", "tsc", "check"])
    {
        recommended.push(typecheck);
    }
    if matches!(max_risk, Risk::High | Risk::Critical) {
        recommended.push("ctx boundaries --changed".to_string());
    }
    let mut full = Vec::new();
    if matches!(max_risk, Risk::Critical)
        && let Some(test) = find_script(project, &["test"])
    {
        full.push(test);
    }
    VerificationPlan {
        minimal: unique(minimal).into_iter().take(3).collect(),
        recommended: unique(recommended).into_iter().take(3).collect(),
        full_only_if_triggered: unique(full).into_iter().take(3).collect(),
    }
}

pub fn explain_target(project: &Project, target: &str) -> ExplainReport {
    let rel = repo::normalize_rel_path(target);
    if let Some(info) = project.files.get(&rel).or_else(|| {
        project
            .files
            .iter()
            .find(|(path, _)| {
                path.to_ascii_lowercase()
                    .contains(&target.to_ascii_lowercase())
            })
            .map(|(_, info)| info)
    }) {
        let domain = domain_by_rel(project, &info.rel).map(Into::into);
        let (risk, risk_reasons) = risk_for_file(project, &info.rel);
        return ExplainReport {
            kind: "file".to_string(),
            schema_version: "1",
            path: Some(info.rel.clone()),
            id: None,
            domain,
            roles: info.roles.iter().cloned().collect(),
            risk: Some(risk.as_str().to_string()),
            risk_reasons,
            imports: info.resolved_imports.iter().cloned().collect(),
            imported_by: project
                .reverse_imports
                .get(&info.rel)
                .map(|x| x.iter().cloned().collect())
                .unwrap_or_default(),
            exports: info.exports.iter().cloned().collect(),
            related_tests: test_files_for(
                project,
                std::slice::from_ref(&info.rel),
                domain_by_rel(project, &info.rel),
                10,
            ),
            invariants: Vec::new(),
            files: Vec::new(),
            provenance: "filesystem+imports+heuristics".to_string(),
            confidence: "high".to_string(),
            target: None,
        };
    }

    for (id, concept) in &project.anchors.concepts {
        if id == target
            || id
                .to_ascii_lowercase()
                .contains(&target.to_ascii_lowercase())
        {
            let domain = primary_domain(project, "", None);
            let files = concept
                .files
                .iter()
                .map(|f| resolve_domain_pattern(&domain, f))
                .collect();
            return ExplainReport {
                kind: "concept".to_string(),
                schema_version: "1",
                path: None,
                id: Some(id.clone()),
                domain: Some((&domain).into()),
                roles: concept.role.iter().cloned().collect(),
                risk: None,
                risk_reasons: Vec::new(),
                imports: Vec::new(),
                imported_by: Vec::new(),
                exports: Vec::new(),
                related_tests: Vec::new(),
                invariants: concept.invariants.clone(),
                files,
                provenance: "ctx_anchor".to_string(),
                confidence: "hard".to_string(),
                target: None,
            };
        }
    }

    ExplainReport {
        kind: "missing".to_string(),
        schema_version: "1",
        path: None,
        id: None,
        domain: None,
        roles: Vec::new(),
        risk: None,
        risk_reasons: Vec::new(),
        imports: Vec::new(),
        imported_by: Vec::new(),
        exports: Vec::new(),
        related_tests: Vec::new(),
        invariants: Vec::new(),
        files: Vec::new(),
        provenance: "none".to_string(),
        confidence: "low".to_string(),
        target: Some(target.to_string()),
    }
}

pub fn widen_context(
    project: &Project,
    task: &str,
    path: Option<&str>,
    reason: &str,
    already: &[String],
    limit: usize,
) -> WidenReport {
    let domain = primary_domain(project, task, path);
    let (kind, conf, _) = task_kind(project, &domain, task);
    let exclude: BTreeSet<String> = already
        .iter()
        .filter_map(|f| normalize_path_in_repo(project, f))
        .collect();
    let mut add: Vec<String> = select_read_first(project, &domain, task, &kind, limit, &exclude)
        .into_iter()
        .map(|c| c.path)
        .collect();
    for file in impacted_files(
        project,
        &exclude.iter().cloned().collect::<Vec<_>>(),
        1,
        limit,
    ) {
        if add.len() >= limit {
            break;
        }
        if !exclude.contains(&file) && !add.contains(&file) {
            add.push(file);
        }
    }
    WidenReport {
        kind: "widened_context",
        schema_version: "1",
        reason: reason.to_string(),
        domain: (&domain).into(),
        add,
        still_do_not_read_yet: do_not_read_yet(project, &domain, &kind, task, 8),
        confidence: confidence_from_score((conf - 0.05).max(0.1)).as_str().to_string(),
        stop_rule: "Stop after this widened set plus minimal verification unless a new expansion trigger fires.".to_string(),
    }
}

pub fn boundary_report(
    project: &Project,
    changed_only: Option<&BTreeSet<String>>,
) -> BoundaryReport {
    BoundaryReport {
        kind: "boundary_report",
        schema_version: "1",
        findings: boundary_findings(project, changed_only),
    }
}

pub fn boundary_findings(
    project: &Project,
    changed_only: Option<&BTreeSet<String>>,
) -> Vec<BoundaryFinding> {
    let mut findings = Vec::new();
    let root_domain = primary_domain(project, "", None);
    for file in project.files.values() {
        if let Some(changed) = changed_only
            && !changed.contains(&file.rel)
        {
            continue;
        }
        if file.has_role("generated")
            && let Some(changed) = changed_only
            && changed.contains(&file.rel)
        {
            findings.push(BoundaryFinding {
                from: file.rel.clone(),
                to: String::new(),
                status: "forbidden".to_string(),
                reason: "generated file edited directly".to_string(),
                recovery: vec!["Edit the source input or generator, then regenerate.".to_string()],
                provenance: "heuristic".to_string(),
                confidence: "medium".to_string(),
            });
        }
        for target in &file.resolved_imports {
            for rule in &project.anchors.boundaries.forbidden {
                if rule.from.is_empty() || rule.to.is_empty() {
                    continue;
                }
                let from = resolve_domain_pattern(&root_domain, &rule.from);
                let to = resolve_domain_pattern(&root_domain, &rule.to);
                if glob_match(&from, &file.rel) && glob_match(&to, target) {
                    findings.push(BoundaryFinding {
                        from: file.rel.clone(),
                        to: target.clone(),
                        status: rule
                            .status
                            .clone()
                            .unwrap_or_else(|| "forbidden".to_string()),
                        reason: rule.reason.clone(),
                        recovery: rule.recovery.clone(),
                        provenance: "ctx_anchor".to_string(),
                        confidence: "hard".to_string(),
                    });
                }
            }
        }
    }
    for edge in &project.package_edges {
        if let Some(changed) = changed_only
            && !changed.contains(&edge.from_manifest)
        {
            continue;
        }
        for rule in &project.anchors.boundaries.forbidden {
            if rule.from.is_empty() || rule.to.is_empty() {
                continue;
            }
            let from = resolve_domain_pattern(&root_domain, &rule.from);
            let to = resolve_domain_pattern(&root_domain, &rule.to);
            if package_edge_matches_rule(&from, &edge.from)
                && package_edge_matches_rule(&to, &edge.to)
            {
                let mut reason = rule.reason.clone();
                if !reason.is_empty() {
                    reason.push_str("; ");
                }
                reason.push_str(&format!(
                    "package manifest dependency `{}` from {}",
                    edge.dependency, edge.source
                ));
                findings.push(BoundaryFinding {
                    from: edge.from_manifest.clone(),
                    to: edge.to_manifest.clone().unwrap_or_else(|| edge.to.clone()),
                    status: rule
                        .status
                        .clone()
                        .unwrap_or_else(|| "forbidden".to_string()),
                    reason,
                    recovery: rule.recovery.clone(),
                    provenance: "package_manifest+ctx_anchor".to_string(),
                    confidence: "hard".to_string(),
                });
            }
        }
    }
    for path in package_transitive_paths(project, 4) {
        if let Some(changed) = changed_only
            && !path
                .manifests
                .iter()
                .any(|manifest| changed.contains(manifest))
        {
            continue;
        }
        for rule in &project.anchors.boundaries.forbidden {
            if rule.from.is_empty() || rule.to.is_empty() {
                continue;
            }
            let from = resolve_domain_pattern(&root_domain, &rule.from);
            let to = resolve_domain_pattern(&root_domain, &rule.to);
            if package_edge_matches_rule(&from, &path.from)
                && package_edge_matches_rule(&to, &path.to)
            {
                let mut reason = rule.reason.clone();
                if !reason.is_empty() {
                    reason.push_str("; ");
                }
                reason.push_str(&format!(
                    "transitive package manifest dependency path `{}`",
                    path.dependencies.join(" -> ")
                ));
                findings.push(BoundaryFinding {
                    from: path.from_manifest.clone(),
                    to: path.to_manifest.clone().unwrap_or_else(|| path.to.clone()),
                    status: rule
                        .status
                        .clone()
                        .unwrap_or_else(|| "forbidden".to_string()),
                    reason,
                    recovery: rule.recovery.clone(),
                    provenance: "package_manifest_transitive+ctx_anchor".to_string(),
                    confidence: "hard".to_string(),
                });
            }
        }
    }
    findings
}

struct PackageGraphPath {
    from: String,
    from_manifest: String,
    to: String,
    to_manifest: Option<String>,
    dependencies: Vec<String>,
    manifests: Vec<String>,
}

fn package_transitive_paths(project: &Project, max_depth: usize) -> Vec<PackageGraphPath> {
    let mut outgoing: BTreeMap<&str, Vec<&crate::model::PackageDependency>> = BTreeMap::new();
    for edge in &project.package_edges {
        outgoing.entry(&edge.from).or_default().push(edge);
    }
    let mut paths = Vec::new();
    for first in &project.package_edges {
        let mut queue = VecDeque::from([(
            first.to.clone(),
            vec![first.dependency.clone()],
            vec![first.from_manifest.clone()],
            BTreeSet::from([first.from.clone(), first.to.clone()]),
            1usize,
        )]);
        while let Some((current, dependencies, manifests, seen, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            let Some(next_edges) = outgoing.get(current.as_str()) else {
                continue;
            };
            for edge in next_edges {
                if seen.contains(&edge.to) {
                    continue;
                }
                let mut next_dependencies = dependencies.clone();
                next_dependencies.push(edge.dependency.clone());
                let mut next_manifests = manifests.clone();
                next_manifests.push(edge.from_manifest.clone());
                let next_depth = depth + 1;
                paths.push(PackageGraphPath {
                    from: first.from.clone(),
                    from_manifest: first.from_manifest.clone(),
                    to: edge.to.clone(),
                    to_manifest: edge.to_manifest.clone(),
                    dependencies: next_dependencies.clone(),
                    manifests: next_manifests.clone(),
                });
                let mut next_seen = seen.clone();
                next_seen.insert(edge.to.clone());
                queue.push_back((
                    edge.to.clone(),
                    next_dependencies,
                    next_manifests,
                    next_seen,
                    next_depth,
                ));
            }
        }
    }
    paths
}

fn package_edge_matches_rule(pattern: &str, package_path: &str) -> bool {
    let package_path = package_path.trim_end_matches('/');
    let probes = if package_path == "." {
        vec![
            "package.json".to_string(),
            "Cargo.toml".to_string(),
            "go.mod".to_string(),
            "pyproject.toml".to_string(),
            "src/__package_dependency__".to_string(),
            "__package_dependency__".to_string(),
        ]
    } else {
        vec![
            format!("{package_path}/package.json"),
            format!("{package_path}/Cargo.toml"),
            format!("{package_path}/go.mod"),
            format!("{package_path}/pyproject.toml"),
            format!("{package_path}/src/__package_dependency__"),
            format!("{package_path}/__package_dependency__"),
        ]
    };
    probes.iter().any(|probe| glob_match(pattern, probe))
}

fn primary_domain(project: &Project, task: &str, path: Option<&str>) -> Domain {
    if let Some(path) = path {
        return explicit_domain_for_path(project, path, task);
    }
    task_domain(project, task)
}

fn task_domain(project: &Project, task: &str) -> Domain {
    if task.is_empty() {
        return project
            .domains
            .iter()
            .find(|d| d.path == ".")
            .unwrap_or(&project.domains[0])
            .clone();
    }
    locate_report(project, task, 1)
        .candidates
        .first()
        .and_then(|candidate| {
            project
                .domains
                .iter()
                .find(|d| d.id == candidate.domain.id && d.path == candidate.domain.path)
        })
        .cloned()
        .unwrap_or_else(|| {
            project
                .domains
                .iter()
                .find(|d| d.path == ".")
                .unwrap_or(&project.domains[0])
                .clone()
        })
}

fn explicit_domain_for_path(project: &Project, path: &str, task: &str) -> Domain {
    let rel = match normalize_path_in_repo(project, path) {
        Some(rel) => rel,
        None => return domain_for_path(project, path).clone(),
    };
    let best = domain_for_path(project, &rel);
    if rel == "." {
        return task_domain(project, task);
    }
    if best.path != "." && path_is_in_domain(&rel, best) {
        return best.clone();
    }
    if project.files.contains_key(&rel)
        && let Some(package_domain) = enclosing_package_domain_for_path(project, &rel)
    {
        return package_domain;
    }
    let scoped_path = explicit_scope_path(project, &rel);
    if scoped_path == "." {
        return best.clone();
    }
    if !task.is_empty()
        && let Some(package_domain) = nested_package_domain_for_task(project, &scoped_path, task)
    {
        return package_domain;
    }
    if let Some(package_domain) = enclosing_package_domain_for_path(project, &scoped_path) {
        return package_domain;
    }
    Domain {
        id: Path::new(&scoped_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("scope")
            .to_string(),
        path: scoped_path,
        config_path: None,
    }
}

fn nested_package_domain_for_task(
    project: &Project,
    scope_path: &str,
    task: &str,
) -> Option<Domain> {
    let task_tokens = route_text_tokens(task);
    let scope_prefix = format!("{}/", scope_path.trim_end_matches('/'));
    let mut best: Option<(f64, &crate::model::PackageInfo)> = None;
    for package in &project.packages {
        if package.path == scope_path || !package.path.starts_with(&scope_prefix) {
            continue;
        }
        let package_tokens = route_text_tokens(&format!("{} {}", package.name, package.path));
        let overlap = token_overlap(&task_tokens, &package_tokens).len();
        if overlap == 0 {
            continue;
        }
        let score = overlap as f64 * 2.0 + package.path.matches('/').count() as f64 * 0.05;
        match best {
            Some((best_score, _)) if best_score >= score => {}
            _ => best = Some((score, package)),
        }
    }
    best.map(|(_, package)| package_domain(package))
}

fn path_is_in_domain(rel: &str, domain: &Domain) -> bool {
    let prefix = domain.path.trim_end_matches('/');
    prefix == "." || rel == prefix || rel.starts_with(&format!("{prefix}/"))
}

fn enclosing_package_domain_for_path(project: &Project, rel: &str) -> Option<Domain> {
    project
        .packages
        .iter()
        .filter(|package| {
            package.path != "."
                && (rel == package.path
                    || rel == package.manifest
                    || rel.starts_with(&format!("{}/", package.path.trim_end_matches('/'))))
        })
        .max_by_key(|package| package.path.len())
        .map(package_domain)
}

fn package_domain(package: &crate::model::PackageInfo) -> Domain {
    Domain {
        id: Path::new(&package.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("scope")
            .to_string(),
        path: package.path.clone(),
        config_path: None,
    }
}

fn normalize_path_in_repo(project: &Project, path: &str) -> Option<String> {
    if Path::new(path).is_absolute() {
        let absolute = Path::new(path)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(path).to_path_buf());
        absolute
            .strip_prefix(&project.root)
            .ok()
            .map(|p| repo::normalize_rel_path(&p.to_string_lossy()))
    } else {
        Some(repo::normalize_rel_path(path))
    }
}

fn explicit_scope_path(project: &Project, rel: &str) -> String {
    let rel = rel.trim_end_matches('/');
    if project.files.contains_key(rel) {
        return Path::new(rel)
            .parent()
            .map(|p| repo::normalize_rel_path(&p.to_string_lossy()))
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| ".".to_string());
    }
    let prefix = format!("{rel}/");
    if project.files.keys().any(|file| file.starts_with(&prefix)) {
        rel.to_string()
    } else {
        ".".to_string()
    }
}

fn domain_for_path<'a>(project: &'a Project, path: &str) -> &'a Domain {
    let rel = if Path::new(path).is_absolute() {
        let absolute = Path::new(path)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(path).to_path_buf());
        absolute
            .strip_prefix(&project.root)
            .map(|p| repo::normalize_rel_path(&p.to_string_lossy()))
            .unwrap_or_else(|_| repo::normalize_rel_path(path))
    } else {
        repo::normalize_rel_path(path)
    };
    let mut best = project
        .domains
        .iter()
        .find(|domain| domain.path == ".")
        .unwrap_or(&project.domains[0]);
    let mut best_len = 0usize;
    for domain in &project.domains {
        if path_is_in_domain(&rel, domain) {
            let prefix = domain.path.trim_end_matches('/');
            let len = if prefix == "." { 0 } else { prefix.len() };
            if len >= best_len {
                best = domain;
                best_len = len;
            }
        }
    }
    best
}

fn domain_by_rel<'a>(project: &'a Project, rel: &str) -> Option<&'a Domain> {
    Some(domain_for_path(project, rel))
}

fn domain_files<'a>(project: &'a Project, domain: &'a Domain) -> Vec<&'a FileInfo> {
    let prefix = if domain.path == "." {
        None
    } else {
        Some(format!("{}/", domain.path.trim_end_matches('/')))
    };
    project
        .files
        .values()
        .filter(|file| {
            prefix
                .as_ref()
                .map(|p| file.rel.starts_with(p))
                .unwrap_or(true)
        })
        .collect()
}

fn task_kind(project: &Project, domain: &Domain, task: &str) -> (String, f64, Vec<String>) {
    let task_tokens = route_text_tokens(task);
    let mut best: (String, f64, Vec<String>) = ("general".to_string(), 0.0, Vec::new());
    for (name, route) in &project.anchors.task_routes {
        let mut score = 0.0;
        let mut reasons = Vec::new();
        for word in &route.matches {
            let word_l = word.to_ascii_lowercase();
            if task_keyword_matches(&task_tokens, &word_l) {
                score += 2.7;
                reasons.push(format!("matched configured route `{name}` via `{word}`"));
            }
        }
        if score > best.1 {
            best = (name.clone(), score, reasons);
        }
    }
    if best.1 > 0.0 {
        return (best.0, (0.60 + best.1 / 12.0).min(1.0), best.2);
    }

    let keyword_sets: &[(&str, &[&str])] = &[
        (
            "playback_session",
            &[
                "frame", "seek", "cursor", "playback", "replay", "timeline", "scrub",
            ],
        ),
        (
            "persistence",
            &[
                "save", "saved", "load", "loaded", "reopen", "restore", "persist", "storage",
            ],
        ),
        (
            "parser",
            &[
                "parse",
                "parser",
                "fixture",
                "input",
                "deserialize",
                "decoder",
            ],
        ),
        (
            "serialization_schema",
            &[
                "serialize",
                "schema",
                "migration",
                "format",
                "dto",
                "contract",
            ],
        ),
        (
            "public_api",
            &["api", "export", "public", "interface", "sdk", "breaking"],
        ),
        (
            "ui_rendering",
            &[
                "ui",
                "screen",
                "button",
                "view",
                "page",
                "component",
                "render",
                "visual",
                "css",
                "style",
            ],
        ),
        (
            "auth",
            &[
                "auth",
                "login",
                "logout",
                "token",
                "session",
                "permission",
                "role",
                "oauth",
            ],
        ),
        (
            "data_storage",
            &[
                "database",
                "db",
                "query",
                "repository",
                "model",
                "entity",
                "store",
                "cache",
            ],
        ),
        (
            "test",
            &["test", "spec", "failing", "coverage", "assert", "snapshot"],
        ),
        (
            "build_ci",
            &[
                "build",
                "bundle",
                "compile",
                "typecheck",
                "lint",
                "ci",
                "workflow",
            ],
        ),
        (
            "context_routing",
            &[
                "ctx",
                "route",
                "routing",
                "locate",
                "start",
                "capsule",
                "context",
                "impact",
                "verify",
                "widen",
                "boundary",
                "boundaries",
            ],
        ),
    ];
    for (kind, words) in keyword_sets {
        let mut score = 0.0;
        let mut reasons = Vec::new();
        for word in *words {
            if task_keyword_matches(&task_tokens, word) {
                score += 1.0;
                reasons.push(format!("task mentions `{word}`"));
            }
        }
        if score > best.1 {
            best = ((*kind).to_string(), score, reasons);
        }
    }
    if best.1 == 0.0 {
        let domain_tokens = route_text_tokens(&format!("{} {}", domain.id, domain.path));
        best.1 += token_overlap(&task_tokens, &domain_tokens).len() as f64 * 0.5;
    }
    if best.1 == 0.0 {
        (
            "general".to_string(),
            0.42,
            vec!["no strong task route matched".to_string()],
        )
    } else {
        (best.0, (0.55 + best.1 / 8.0).min(0.95), best.2)
    }
}

fn task_keyword_matches(task_tokens: &BTreeSet<String>, keyword: &str) -> bool {
    let keyword_tokens = route_text_tokens(keyword);
    !keyword_tokens.is_empty()
        && keyword_tokens
            .iter()
            .all(|token| task_tokens.contains(token))
}

fn route_text_tokens(text: &str) -> BTreeSet<String> {
    repo::tokenize(&text.replace('_', " "))
}

fn token_overlap(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.intersection(right).cloned().collect()
}

fn route_for_kind<'a>(
    project: &'a Project,
    kind: &str,
) -> Option<&'a crate::model::AnchorTaskRoute> {
    project.anchors.task_routes.get(kind)
}

fn select_read_first(
    project: &Project,
    domain: &Domain,
    task: &str,
    kind: &str,
    limit: usize,
    exclude: &BTreeSet<String>,
) -> Vec<Candidate> {
    let include_fixtures = task_mentions_fixture(task) || domain.path.contains("fixtures");
    let include_examples = task_mentions_example(task)
        || domain.path.contains("examples")
        || domain.path.contains("samples");
    let mut candidates = domain_files(project, domain)
        .into_iter()
        .filter(|file| {
            !exclude.contains(&file.rel)
                && !file.has_role("generated")
                && (include_fixtures || !file.has_role("fixture"))
                && (include_examples || !file.has_role("example"))
        })
        .map(|file| score_file(project, file, task, kind))
        .filter(|candidate| candidate.score >= 1.0)
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });
    let mut out = Vec::new();
    for candidate in candidates {
        if out.len() >= limit {
            break;
        }
        if !project
            .files
            .get(&candidate.path)
            .map(|f| f.has_role("test"))
            .unwrap_or(false)
        {
            out.push(candidate);
        }
    }
    if out.is_empty() {
        out = domain_files(project, domain)
            .into_iter()
            .filter(|file| {
                repo::is_source_ext(&file.ext)
                    && !file.has_role("test")
                    && !file.has_role("generated")
                    && (include_fixtures || !file.has_role("fixture"))
                    && (include_examples || !file.has_role("example"))
            })
            .take(limit)
            .map(|file| Candidate {
                path: file.rel.clone(),
                score: 1.0,
                reasons: vec!["fallback source file".to_string()],
            })
            .collect();
    }
    out
}

fn task_mentions_fixture(task: &str) -> bool {
    let tokens = route_text_tokens(task);
    tokens.contains("fixture") || tokens.contains("fixtures")
}

fn task_mentions_example(task: &str) -> bool {
    let tokens = route_text_tokens(task);
    tokens.contains("example")
        || tokens.contains("examples")
        || tokens.contains("sample")
        || tokens.contains("samples")
}

fn path_mentions_support(path: &str) -> bool {
    path.contains("fixtures")
        || path.contains("examples")
        || path.contains("samples")
        || path.contains("fixture")
        || path.contains("sample")
}

fn score_file(_project: &Project, file: &FileInfo, task: &str, kind: &str) -> Candidate {
    let mut score = 0.0;
    let mut reasons = Vec::new();
    let tokens = repo::tokenize(task);
    let overlap: Vec<_> = tokens
        .intersection(&file.tokens)
        .cloned()
        .collect::<Vec<String>>();
    if !overlap.is_empty() {
        score += 2.0 * overlap.len() as f64;
        reasons.push(format!("path matches: {}", overlap.join(", ")));
    }
    let boosts: &[(&str, &[(&str, f64)])] = &[
        (
            "playback_session",
            &[
                ("runtime_state", 6.0),
                ("source_of_truth", 4.0),
                ("parser", 1.0),
                ("schema_contract", 1.0),
                ("test", 1.0),
            ],
        ),
        (
            "persistence",
            &[
                ("persistence", 6.0),
                ("source_of_truth", 2.5),
                ("runtime_state", 1.5),
                ("schema_contract", 1.0),
                ("test", 1.0),
            ],
        ),
        (
            "parser",
            &[
                ("parser", 5.0),
                ("repo_discovery", 2.5),
                ("schema_contract", 2.0),
                ("test", 1.5),
            ],
        ),
        (
            "serialization_schema",
            &[
                ("schema_contract", 5.0),
                ("adapter", 2.0),
                ("source_of_truth", 1.5),
            ],
        ),
        (
            "public_api",
            &[("public_boundary", 6.0), ("schema_contract", 2.0)],
        ),
        ("ui_rendering", &[("renderer_ui", 5.0), ("adapter", 1.5)]),
        (
            "auth",
            &[
                ("runtime_state", 3.0),
                ("adapter", 2.0),
                ("source_of_truth", 2.0),
                ("schema_contract", 1.5),
            ],
        ),
        (
            "data_storage",
            &[
                ("source_of_truth", 4.0),
                ("adapter", 2.0),
                ("schema_contract", 1.5),
            ],
        ),
        ("test", &[("test", 5.0)]),
        (
            "general",
            &[
                ("source_of_truth", 2.5),
                ("public_boundary", 1.5),
                ("schema_contract", 1.0),
                ("persistence", 1.0),
            ],
        ),
        (
            "context_routing",
            &[
                ("routing", 6.0),
                ("repo_discovery", 2.5),
                ("cli_surface", 1.5),
                ("cache", 1.0),
            ],
        ),
        (
            "build_ci",
            &[
                ("build_ci", 6.0),
                ("public_boundary", 4.0),
                ("cli_surface", 2.0),
                ("repo_discovery", 1.0),
            ],
        ),
    ];
    let role_boosts = boosts
        .iter()
        .find(|(name, _)| *name == kind)
        .or_else(|| boosts.iter().find(|(name, _)| *name == "general"))
        .map(|(_, rules)| *rules)
        .expect("general boosts");
    for (role, boost) in role_boosts {
        if file.has_role(role) {
            score += boost;
            reasons.push(format!("role `{role}`"));
        }
    }
    if repo::is_source_ext(&file.ext) {
        score += 0.8;
        if kind == "context_routing"
            && ["routing", "repo_discovery", "cli_surface", "cache"]
                .iter()
                .any(|role| file.has_role(role))
        {
            score += 2.0;
            reasons.push("source implementation".to_string());
        }
    } else if kind == "context_routing" && file.has_role("schema_contract") {
        score -= 4.0;
        reasons.push("route contract, not implementation".to_string());
    }
    if file.has_role("agent_bootstrap") {
        score -= 1.5;
    }
    if file.has_role("semantic_anchor") {
        score -= 5.0;
    }
    if file.has_role("generated") {
        score -= 5.0;
    }
    if file.has_role("test") && kind != "test" {
        score -= 0.7;
    }
    let name = Path::new(&file.rel)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if matches!(
        name,
        "package.json" | "Cargo.toml" | "go.mod" | "pyproject.toml"
    ) && !matches!(kind, "public_api" | "build_ci" | "serialization_schema")
    {
        score -= 2.0;
    }
    if file.has_role("fixture") && kind != "test" && !task_mentions_fixture(task) {
        score -= 4.0;
    }
    if file.has_role("example") && kind != "test" && !task_mentions_example(task) {
        score -= 4.0;
    }
    Candidate {
        path: file.rel.clone(),
        score,
        reasons: unique(reasons).into_iter().take(3).collect(),
    }
}

fn candidate_has_specific_evidence(candidate: &Candidate, kind: &str) -> bool {
    if candidate.reasons.iter().any(|reason| {
        reason.starts_with("configured read_first") || reason.starts_with("path matches")
    }) {
        return true;
    }
    let specific_roles: &[&str] = match kind {
        "context_routing" => &["routing", "repo_discovery", "cli_surface", "cache"],
        "playback_session" => &["runtime_state"],
        "parser" => &["parser", "repo_discovery"],
        "serialization_schema" | "public_api" => &["schema_contract", "public_boundary"],
        "ui_rendering" => &["renderer_ui"],
        "auth" => &["runtime_state", "adapter"],
        "data_storage" | "persistence" => &["persistence"],
        "build_ci" => &["build_ci", "public_boundary", "cli_surface"],
        _ => &[],
    };
    candidate.reasons.iter().any(|reason| {
        specific_roles
            .iter()
            .any(|role| reason == &format!("role `{role}`"))
    })
}

fn test_files_for(
    project: &Project,
    rels: &[String],
    domain: Option<&Domain>,
    limit: usize,
) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    let stem_domains: Vec<(String, Option<String>)> = rels
        .iter()
        .filter_map(|r| {
            let stem = Path::new(r)
                .file_stem()
                .and_then(|s| s.to_str())?
                .replace(".test", "")
                .replace(".spec", "");
            if matches!(stem.as_str(), "index" | "mod" | "main" | "lib") {
                return None;
            }
            let source_domain = scoped_domain_path_for_rel(project, r, domain);
            Some((stem, source_domain))
        })
        .collect();
    let allow_fixture_tests = rels.iter().any(|rel| {
        project
            .files
            .get(rel)
            .map(|file| file.has_role("fixture"))
            .unwrap_or(false)
    });
    let allow_example_tests = rels.iter().any(|rel| {
        project
            .files
            .get(rel)
            .map(|file| file.has_role("example"))
            .unwrap_or(false)
    });
    let mut scored = Vec::new();
    for file in project.files.values() {
        if !file.has_role("test") {
            continue;
        }
        if file.has_role("fixture") && !allow_fixture_tests {
            continue;
        }
        if file.has_role("example") && !allow_example_tests {
            continue;
        }
        let mut score = 0.0;
        let test_domain = scoped_domain_path_for_rel(project, &file.rel, domain);
        for (stem, source_domain) in &stem_domains {
            if source_domain.is_some() && source_domain != &test_domain {
                continue;
            }
            if !stem.is_empty()
                && file
                    .rel
                    .to_ascii_lowercase()
                    .contains(&stem.to_ascii_lowercase())
            {
                score += 5.0;
            }
        }
        for rel in rels {
            if file.resolved_imports.contains(rel) {
                score += 4.0;
            }
        }
        if let Some(domain) = domain
            && domain.path != "."
            && file.rel.starts_with(&format!("{}/", domain.path))
        {
            score += 0.5;
        }
        if score > 0.0 {
            scored.push((score, file.rel.clone()));
        }
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().map(|(_, rel)| rel).take(limit).collect()
}

fn scoped_domain_path_for_rel(
    project: &Project,
    rel: &str,
    scope: Option<&Domain>,
) -> Option<String> {
    if let Some(domain) = scope
        && (domain.path == "."
            || rel == domain.path
            || rel.starts_with(&format!("{}/", domain.path.trim_end_matches('/'))))
    {
        return Some(domain.path.clone());
    }
    domain_by_rel(project, rel).map(|domain| domain.path.clone())
}

fn source_truths(project: &Project, domain: &Domain) -> Vec<String> {
    let mut out = Vec::new();
    for concept in project.anchors.concepts.values() {
        if concept.role.as_deref() == Some("source_of_truth")
            || concept.kind.as_deref() == Some("source_of_truth")
        {
            for file in &concept.files {
                let rel = resolve_domain_pattern(domain, file);
                if project.files.contains_key(&rel) {
                    out.push(rel);
                }
            }
        }
    }
    for file in domain_files(project, domain) {
        if file.has_role("fixture") || file.has_role("example") {
            continue;
        }
        if file.has_role("source_of_truth") || file.has_role("persistence") {
            out.push(file.rel.clone());
        }
    }
    unique(out).into_iter().take(8).collect()
}

fn public_boundaries(project: &Project, domain: &Domain) -> Vec<String> {
    domain_files(project, domain)
        .into_iter()
        .filter(|file| {
            file.has_role("public_boundary")
                && !file.has_role("fixture")
                && !file.has_role("example")
        })
        .map(|file| file.rel.clone())
        .take(8)
        .collect()
}

fn invariants_for(project: &Project, domain: &Domain, read_files: &[String]) -> Vec<String> {
    let read: BTreeSet<_> = read_files.iter().cloned().collect();
    let mut out = Vec::new();
    for concept in project.anchors.concepts.values() {
        let files: Vec<String> = concept
            .files
            .iter()
            .map(|f| resolve_domain_pattern(domain, f))
            .collect();
        if files.iter().any(|f| read.contains(f)) {
            out.extend(concept.invariants.clone());
        }
    }
    unique(out).into_iter().take(7).collect()
}

fn do_not_read_yet(
    project: &Project,
    domain: &Domain,
    kind: &str,
    task: &str,
    limit: usize,
) -> Vec<DoNotRead> {
    let task_tokens = route_text_tokens(task);
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for package in sibling_support_packages(project, domain) {
        push_do_not_read(
            &mut out,
            &mut seen,
            format!("{}/**", package.path),
            "sibling package inside the scoped support artifact; inspect only if the task or an expansion trigger points there",
            limit,
        );
    }
    if !task_mentions_fixture(task) && !domain.path.contains("fixtures") {
        for path in support_roots_for_role(project, domain, "fixture", &["fixtures"]) {
            push_do_not_read(
                &mut out,
                &mut seen,
                path,
                "fixture code is support evidence, not the task owner; inspect only if the task names fixtures",
                limit,
            );
        }
    }
    if !task_mentions_example(task)
        && !domain.path.contains("examples")
        && !domain.path.contains("samples")
    {
        for path in support_roots_for_role(project, domain, "example", &["examples", "samples"]) {
            push_do_not_read(
                &mut out,
                &mut seen,
                path,
                "example/sample code is support evidence, not the task owner; inspect only if the task names examples or samples",
                limit,
            );
        }
    }
    for other in &project.domains {
        if out.len() >= limit {
            break;
        }
        if other.id == domain.id && other.path == domain.path {
            continue;
        }
        if other.path == "." && domain.path != "." {
            continue;
        }
        let lname = format!("{} {}", other.id, other.path).to_ascii_lowercase();
        let other_tokens = route_text_tokens(&lname);
        if !token_overlap(&task_tokens, &other_tokens).is_empty() {
            continue;
        }
        let reason =
            if kind == "ui_rendering" && contains_any(&lname, &["ui", "web", "app", "renderer"]) {
                continue;
            } else if kind == "serialization_schema"
                && contains_any(&lname, &["storage", "db", "persistence"])
            {
                "inspect only if format/storage contract changes"
            } else if kind == "public_api" {
                "inspect only if public consumer impact points here"
            } else if contains_any(&lname, &["renderer"]) {
                "consumer/rendering path; inspect only if DTO/render-visible output changes"
            } else if contains_any(&lname, &["storage"]) {
                "persistence path; inspect only if package/schema format changes"
            } else if contains_any(&lname, &["web", "app", "ui"]) {
                "application/UI path; inspect only if public API or UI task requires it"
            } else {
                "not predicted by task route"
            };
        let path = if other.path == "." {
            ".".to_string()
        } else {
            format!("{}/**", other.path)
        };
        push_do_not_read(&mut out, &mut seen, path, reason, limit);
    }
    out
}

fn sibling_support_packages<'a>(
    project: &'a Project,
    domain: &Domain,
) -> Vec<&'a crate::model::PackageInfo> {
    let Some(container) = support_container_scope(&domain.path) else {
        return Vec::new();
    };
    let container_prefix = format!("{}/", container.trim_end_matches('/'));
    project
        .packages
        .iter()
        .filter(|package| {
            package.path != "."
                && package.path != domain.path
                && package.path.starts_with(&container_prefix)
        })
        .take(6)
        .collect()
}

fn support_container_scope(path: &str) -> Option<String> {
    let parts = path.split('/').collect::<Vec<_>>();
    for (idx, part) in parts.iter().enumerate() {
        if matches!(*part, "fixtures" | "examples" | "samples") {
            let end = (idx + 2).min(parts.len());
            return Some(parts[..end].join("/"));
        }
    }
    None
}

fn push_do_not_read(
    out: &mut Vec<DoNotRead>,
    seen: &mut BTreeSet<String>,
    path: String,
    reason: &str,
    limit: usize,
) {
    if out.len() >= limit || !seen.insert(path.clone()) {
        return;
    }
    out.push(DoNotRead {
        path,
        reason: reason.to_string(),
    });
}

fn support_roots_for_role(
    project: &Project,
    domain: &Domain,
    role: &str,
    markers: &[&str],
) -> Vec<String> {
    let mut roots = Vec::new();
    for file in domain_files(project, domain) {
        if !file.has_role(role) {
            continue;
        }
        let parts = file.rel.split('/').collect::<Vec<_>>();
        for (idx, part) in parts.iter().enumerate() {
            if markers.iter().any(|marker| part == marker) {
                roots.push(format!("{}/**", parts[..=idx].join("/")));
                break;
            }
        }
    }
    unique(roots).into_iter().take(4).collect()
}

fn forbidden_moves(project: &Project, domain: &Domain, kind: &str) -> Vec<String> {
    let mut moves = Vec::new();
    for edge in project.anchors.boundaries.forbidden.iter().take(5) {
        let from = resolve_domain_pattern(domain, &edge.from);
        let to = resolve_domain_pattern(domain, &edge.to);
        let reason = if edge.reason.is_empty() {
            String::new()
        } else {
            format!(" - {}", edge.reason)
        };
        moves.push(format!("Do not cross `{from}` -> `{to}`{reason}"));
    }
    let id = domain.id.to_ascii_lowercase();
    if id.contains("core") || id == "domain" {
        moves.push("Do not import application/UI layers from core/domain code.".to_string());
    }
    if id.contains("replay") {
        moves
            .push("Do not use wall-clock time or randomness directly in replay logic.".to_string());
        moves.push("Do not fix replay semantics by reading renderer state.".to_string());
    }
    if matches!(
        kind,
        "playback_session" | "data_storage" | "serialization_schema" | "persistence"
    ) {
        moves
            .push("Do not change public exports unless the impact report requires it.".to_string());
    }
    unique(moves).into_iter().take(7).collect()
}

fn expansion_triggers(kind: &str) -> Vec<String> {
    let mut triggers = vec![
        "public boundary changed",
        "test failure points outside the predicted files",
        "context confidence is medium/low",
        "changed file is unclassified or generated",
        "read-first set did not contain the cause",
    ];
    if matches!(kind, "ui_rendering" | "public_api") {
        triggers.push("external consumer or DTO/schema changed");
    }
    if matches!(kind, "serialization_schema" | "parser") {
        triggers.push("file/package format or migration schema changed");
    }
    if kind == "playback_session" {
        triggers.push("DTO shape, renderer-visible output, or timeline source of truth changed");
    }
    triggers.into_iter().map(str::to_string).take(8).collect()
}

fn stop_conditions() -> Vec<String> {
    [
        "minimal verification passes",
        "no expansion trigger fires",
        "no public boundary changed",
        "no forbidden boundary finding appears",
        "confidence remains medium/high",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn risk_for_file(project: &Project, rel: &str) -> (Risk, Vec<String>) {
    let Some(file) = project.files.get(rel) else {
        return (Risk::Medium, vec!["file not found in scan".to_string()]);
    };
    let mut risk = Risk::Low;
    let mut reasons = Vec::new();
    let mut bump = |level, reason: &str| {
        risk = risk.max(level);
        reasons.push(reason.to_string());
    };
    if file.has_role("generated") {
        bump(Risk::Critical, "generated file");
    }
    if file.has_role("semantic_anchor") {
        bump(Risk::High, "semantic context anchor");
    }
    if file.has_role("public_boundary") {
        bump(Risk::Critical, "public boundary");
    }
    if file.has_role("schema_contract") {
        bump(Risk::High, "schema/contract/DTO");
    }
    if file.has_role("source_of_truth") || file.has_role("persistence") {
        bump(Risk::High, "source of truth / persistence");
    }
    if file.has_role("runtime_state") {
        bump(Risk::MediumHigh, "runtime state / session/controller");
    }
    if file.has_role("routing") {
        bump(Risk::High, "context routing logic");
    }
    if file.has_role("cli_surface") {
        bump(Risk::High, "CLI command surface");
    }
    if file.has_role("build_ci") {
        bump(Risk::MediumHigh, "build/CI configuration");
    }
    if file.has_role("repo_discovery") {
        bump(Risk::MediumHigh, "repo discovery / inventory");
    }
    if file.has_role("cache") {
        bump(Risk::Medium, "external cache / fingerprint");
    }
    let fan_in = project
        .reverse_imports
        .get(rel)
        .map(|x| x.len())
        .unwrap_or(0);
    let fan_out = file.resolved_imports.len();
    if fan_in >= 8 {
        bump(Risk::Critical, &format!("high fan-in ({fan_in} importers)"));
    } else if fan_in >= 3 {
        bump(Risk::High, &format!("multiple importers ({fan_in})"));
    }
    if fan_out >= 12 {
        bump(Risk::Medium, &format!("high fan-out ({fan_out} imports)"));
    }
    if file.has_role("test") {
        bump(Risk::Low, "test file");
    }
    if reasons.is_empty() {
        reasons.push("isolated or low-risk implementation file".to_string());
    }
    (risk, unique(reasons))
}

fn impacted_files(
    project: &Project,
    changed: &[String],
    depth: usize,
    limit: usize,
) -> Vec<String> {
    let mut seen: BTreeSet<String> = changed.iter().cloned().collect();
    let mut queue: VecDeque<(String, usize)> = changed.iter().cloned().map(|f| (f, 0)).collect();
    let mut out = Vec::new();
    while let Some((rel, d)) = queue.pop_front() {
        if out.len() >= limit {
            break;
        }
        let mut neighbors = BTreeSet::new();
        if let Some(file) = project.files.get(&rel) {
            neighbors.extend(file.resolved_imports.iter().cloned());
        }
        if let Some(importers) = project.reverse_imports.get(&rel) {
            neighbors.extend(importers.iter().cloned());
        }
        for next in neighbors {
            if seen.insert(next.clone()) {
                out.push(next.clone());
                if d + 1 < depth {
                    queue.push_back((next, d + 1));
                }
                if out.len() >= limit {
                    break;
                }
            }
        }
    }
    out
}

fn package_consumer_manifests(
    project: &Project,
    changed: &[String],
    depth: usize,
    limit: usize,
) -> Vec<String> {
    if depth == 0 || limit == 0 {
        return Vec::new();
    }
    let mut roots = BTreeSet::new();
    for rel in changed {
        if !requires_package_consumer_expansion(project, rel) {
            continue;
        }
        if let Some(package) = package_for_rel(project, rel) {
            roots.insert(package.path.clone());
        }
    }
    if roots.is_empty() {
        return Vec::new();
    }
    let mut seen = roots.clone();
    let mut queue: VecDeque<(String, usize)> = roots.into_iter().map(|path| (path, 0)).collect();
    let mut out = Vec::new();
    while let Some((package_path, d)) = queue.pop_front() {
        if out.len() >= limit {
            break;
        }
        for edge in project
            .package_edges
            .iter()
            .filter(|edge| edge.to == package_path)
        {
            if seen.insert(edge.from.clone()) {
                out.push(edge.from_manifest.clone());
                if d + 1 < depth {
                    queue.push_back((edge.from.clone(), d + 1));
                }
                if out.len() >= limit {
                    break;
                }
            }
        }
    }
    out
}

fn requires_package_consumer_expansion(project: &Project, rel: &str) -> bool {
    let Some(file) = project.files.get(rel) else {
        return false;
    };
    file.has_role("public_boundary")
        || file.has_role("schema_contract")
        || matches!(
            Path::new(rel).file_name().and_then(|name| name.to_str()),
            Some("package.json" | "Cargo.toml" | "go.mod" | "pyproject.toml")
        )
}

fn package_for_rel<'a>(project: &'a Project, rel: &str) -> Option<&'a crate::model::PackageInfo> {
    let mut best = None;
    let mut best_len = 0usize;
    for package in &project.packages {
        let prefix = package.path.trim_end_matches('/');
        let matches = prefix == "."
            || rel == package.manifest
            || rel == prefix
            || rel.starts_with(&format!("{prefix}/"));
        if matches {
            let len = if prefix == "." { 0 } else { prefix.len() };
            if len >= best_len {
                best = Some(package);
                best_len = len;
            }
        }
    }
    best
}

fn impacted_domains<'a>(project: &'a Project, files: &[String]) -> Vec<&'a Domain> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for file in files {
        if let Some(domain) = domain_by_rel(project, file)
            && seen.insert(domain.id.clone())
        {
            out.push(domain);
        }
    }
    out
}

fn infer_minimal_commands(project: &Project, domains: &[&Domain]) -> Vec<String> {
    if let Some(test) = find_script(project, &["test"]) {
        if domains.len() == 1 && domains[0].path != "." && project.package_manager != "bun" {
            return vec![format!("{test} {}", domains[0].path)];
        }
        return vec![test];
    }
    match project.package_manager.as_str() {
        "cargo" => vec!["cargo test".to_string()],
        "go" => vec!["go test ./...".to_string()],
        "python" => vec!["pytest".to_string()],
        _ => vec!["run the nearest domain tests for the changed files".to_string()],
    }
}

fn find_script(project: &Project, names: &[&str]) -> Option<String> {
    project
        .scripts
        .iter()
        .find(|script| {
            let hay = format!("{} {}", script.name, script.command).to_ascii_lowercase();
            names.iter().any(|name| hay.contains(name))
        })
        .map(|script| script.command.clone())
}

pub fn resolve_anchor_path(project: &Project, pattern: &str) -> String {
    let domain = primary_domain(project, "", None);
    resolve_domain_pattern(&domain, pattern)
}

fn resolve_domain_pattern(domain: &Domain, pattern: &str) -> String {
    let p = pattern.trim().trim_start_matches("./");
    if p.starts_with('/') {
        return repo::normalize_rel_path(p);
    }
    if domain.path == "." {
        return repo::normalize_rel_path(p);
    }
    let domain_path = domain.path.trim_end_matches('/');
    if p == domain_path || p.starts_with(&format!("{domain_path}/")) {
        return repo::normalize_rel_path(p);
    }
    if p.starts_with("domains/")
        || p.starts_with("packages/")
        || p.starts_with("apps/")
        || p.starts_with("services/")
        || p.starts_with("libs/")
        || p.starts_with("crates/")
        || p.starts_with("modules/")
        || p.starts_with("cmd/")
        || p.starts_with("components/")
    {
        repo::normalize_rel_path(p)
    } else {
        repo::normalize_rel_path(&format!("{domain_path}/{p}"))
    }
}

fn confidence_from_score(score: f64) -> Confidence {
    if score >= 0.78 {
        Confidence::High
    } else if score >= 0.52 {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    let tokens = route_text_tokens(text);
    needles
        .iter()
        .any(|needle| task_keyword_matches(&tokens, needle))
}

fn glob_match(pattern: &str, value: &str) -> bool {
    glob_match_parts(
        &pattern.split('/').collect::<Vec<_>>(),
        &value.split('/').collect::<Vec<_>>(),
    )
}

fn glob_match_parts(pattern: &[&str], value: &[&str]) -> bool {
    if pattern.is_empty() {
        return value.is_empty();
    }
    if pattern[0] == "**" {
        return glob_match_parts(&pattern[1..], value)
            || (!value.is_empty() && glob_match_parts(pattern, &value[1..]));
    }
    if value.is_empty() {
        return false;
    }
    segment_match(pattern[0], value[0]) && glob_match_parts(&pattern[1..], &value[1..])
}

fn segment_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == value;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut rest = value;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 && !rest.starts_with(part) {
            return false;
        }
        if let Some(pos) = rest.find(part) {
            rest = &rest[pos + part.len()..];
        } else {
            return false;
        }
    }
    pattern.ends_with('*')
        || parts
            .last()
            .map(|last| value.ends_with(last))
            .unwrap_or(true)
}

fn unique(items: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        if !item.is_empty() && seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
