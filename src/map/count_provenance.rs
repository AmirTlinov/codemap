// Responsibility: map-consumer-count-provenance
use crate::model::{
    CountFact, CoverageCertificate, CoverageClosure, CoverageLocation, CoverageReason,
    ObservationLedger, ObservedCount, Project, UnsupportedObservation,
};

mod blind_spots;
use blind_spots::{
    consumer_blind_spots, consumer_extractor_capabilities, consumer_universe,
    supports_import_language,
};

/// Provenance-carrying consumer counter: counted(n) only for observed edges,
/// proven_zero only when the import flow around `rel` is fully supported,
/// otherwise a typed unknown naming the blind spot.
pub(crate) fn consumer_count_fact(
    project: &Project,
    rel: &str,
    symbol: Option<&str>,
    raw: usize,
) -> CountFact {
    if raw > 0 {
        return CountFact::counted(raw);
    }
    match consumer_zero_blind_spot(project, rel, symbol) {
        Some(reason) => CountFact::unknown(reason),
        None => CountFact::proven_zero(),
    }
}

/// The observation-backed count path. Unlike `CountFact` above, this keeps
/// an observed lower bound even when a static blind spot leaves traversal open.
/// Registration and count creation are one ledger operation, so the returned
/// certificate id cannot dangle in its owning report.
pub(crate) struct ConsumerObservationInput<'a> {
    pub rel: &'a str,
    pub symbol: Option<&'a str>,
    pub raw: usize,
    pub shown: usize,
    pub group: &'a str,
    pub expand: Option<String>,
    pub include_local: bool,
    pub observed_sources: Option<&'a std::collections::BTreeSet<String>>,
}

pub(crate) fn consumer_observed_count(
    project: &Project,
    input: ConsumerObservationInput<'_>,
    ledger: &mut ObservationLedger,
) -> ObservedCount {
    let ConsumerObservationInput {
        rel,
        symbol,
        raw,
        shown,
        group,
        expand,
        include_local,
        observed_sources,
    } = input;
    let scope = symbol
        .map(|name| format!("{rel}#{name}"))
        .unwrap_or_else(|| rel.to_string());
    let query_kind = if include_local {
        "symbol_incoming_relations"
    } else if symbol.is_some() {
        "symbol_consumers"
    } else {
        "file_consumers"
    };
    let universe = consumer_universe(project, rel);
    let anchor_is_eligible = include_local && project.files.contains_key(rel);
    let eligible_files = universe.len() as u64 + u64::from(anchor_is_eligible);
    let mut reasons = Vec::new();
    let mut dynamic_stops = Vec::new();
    let mut unresolved_stops = Vec::new();
    let mut unsupported = Vec::new();

    let closure = match project.files.get(rel) {
        None => {
            reasons.push(CoverageReason::AnchorNotIndexed);
            CoverageClosure::Unavailable
        }
        Some(info) if !supports_import_language(&info.language) => {
            reasons.push(CoverageReason::UnsupportedLanguage);
            unsupported.push(UnsupportedObservation {
                file: rel.to_string(),
                construct: format!("{} import flow", info.language),
                location: Some(CoverageLocation::path(rel)),
            });
            CoverageClosure::Unavailable
        }
        Some(info) if info.content_hash.is_none() => {
            reasons.push(CoverageReason::UnsupportedConstruct);
            unsupported.push(UnsupportedObservation {
                file: rel.to_string(),
                construct: "consumer anchor source could not be read".to_string(),
                location: Some(CoverageLocation::path(rel)),
            });
            CoverageClosure::Unavailable
        }
        Some(_) => {
            for blind_spot in
                consumer_blind_spots(project, rel, symbol, include_local, observed_sources)
            {
                let stop = blind_spot.stop;
                reasons.push(stop.kind);
                if let Some(observation) = blind_spot.unsupported {
                    unsupported.push(observation);
                }
                if stop.kind == CoverageReason::DynamicImportFlow {
                    dynamic_stops.push(stop);
                } else {
                    unresolved_stops.push(stop);
                }
            }
            if reasons.is_empty() {
                CoverageClosure::Closed
            } else {
                CoverageClosure::Open
            }
        }
    };

    let unsupported_files = unsupported
        .iter()
        .map(|observation| observation.file.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let traversed_files = universe
        .iter()
        .filter(|file| {
            file.content_hash.is_some() && !unsupported_files.contains(file.rel.as_str())
        })
        .count() as u64
        + u64::from(
            anchor_is_eligible
                && project.files.get(rel).is_some_and(|file| {
                    file.content_hash.is_some() && !unsupported_files.contains(file.rel.as_str())
                }),
        );
    let visited_files = if closure == CoverageClosure::Unavailable {
        0
    } else {
        traversed_files
    };
    let mut certificate = CoverageCertificate::new(
        query_kind,
        scope.clone(),
        crate::cache::fingerprint(project, None),
        eligible_files,
        visited_files,
        closure,
        reasons,
    );
    certificate.extractor_capabilities =
        consumer_extractor_capabilities(project, rel, include_local);
    certificate.unsupported = unsupported;
    certificate.dynamic_stops = dynamic_stops;
    certificate.unresolved_stops = unresolved_stops;
    let mut excluded = universe
        .iter()
        .filter(|file| {
            closure == CoverageClosure::Unavailable
                || file.content_hash.is_none()
                || unsupported_files.contains(file.rel.as_str())
        })
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    if anchor_is_eligible
        && (closure == CoverageClosure::Unavailable
            || project.files.get(rel).is_none_or(|file| {
                file.content_hash.is_none() || unsupported_files.contains(file.rel.as_str())
            }))
    {
        excluded.push(rel.to_string());
    }
    if !excluded.is_empty() {
        let reason = if closure == CoverageClosure::Unavailable
            && certificate
                .reasons
                .contains(&CoverageReason::UnsupportedLanguage)
        {
            CoverageReason::UnsupportedLanguage
        } else {
            CoverageReason::UnsupportedConstruct
        };
        certificate
            .excluded_files_by_reason
            .insert(reason, excluded);
    }
    ledger.record(group, &scope, raw as u64, shown as u64, certificate, expand)
}

fn consumer_zero_blind_spot(project: &Project, rel: &str, symbol: Option<&str>) -> Option<String> {
    let Some(info) = project.files.get(rel) else {
        return Some("anchor is not indexed".to_string());
    };
    if !supports_import_language(&info.language) {
        return Some(format!("{} import flow is not indexed", info.language));
    }
    consumer_blind_spots(project, rel, symbol, false, None)
        .into_iter()
        .next()
        .map(|blind_spot| {
            let stop = blind_spot.stop;
            let location = stop
                .location
                .map(|location| format!(" via `{}`", location.path))
                .unwrap_or_default();
            format!("{}{location}", stop.kind.label())
        })
}
