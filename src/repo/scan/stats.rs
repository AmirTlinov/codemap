// Responsibility: repo-scan-stats
use crate::model::{ScanGroup, ScanInventoryBoundary, ScanStats};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct ScanStatsBuilder {
    pub(crate) files_visited: usize,
    pub(crate) files_scanned: usize,
    pub(crate) bytes_scanned: u64,
    ignored: BTreeMap<String, ScanGroupBuilder>,
    skipped: BTreeMap<String, ScanGroupBuilder>,
    generated: BTreeMap<String, ScanGroupBuilder>,
    inventory_boundaries: BTreeSet<ScanInventoryBoundary>,
}

impl ScanStatsBuilder {
    pub(crate) fn merge(&mut self, other: ScanStatsBuilder) {
        self.files_visited += other.files_visited;
        self.files_scanned += other.files_scanned;
        self.bytes_scanned += other.bytes_scanned;
        self.merge_groups(GroupKind::Ignored, other.ignored);
        self.merge_groups(GroupKind::Skipped, other.skipped);
        self.merge_groups(GroupKind::Generated, other.generated);
        self.inventory_boundaries.extend(other.inventory_boundaries);
    }

    fn merge_groups(&mut self, kind: GroupKind, groups: BTreeMap<String, ScanGroupBuilder>) {
        for (reason, group) in groups {
            for rel in group.seen {
                self.record_group(kind, &reason, &rel);
            }
        }
    }

    pub(crate) fn record_ignored(&mut self, reason: &str, rel: &str) {
        self.record_group(GroupKind::Ignored, reason, &ignored_group_unit(reason, rel));
    }

    pub(crate) fn record_skipped(&mut self, reason: &str, rel: &str) {
        self.record_group(GroupKind::Skipped, reason, rel);
    }

    pub(crate) fn record_generated(&mut self, reason: &str, rel: &str) {
        self.record_group(GroupKind::Generated, reason, rel);
    }

    pub(crate) fn record_inventory_boundary(&mut self, boundary: ScanInventoryBoundary) {
        self.inventory_boundaries.insert(boundary);
    }

    fn record_group(&mut self, kind: GroupKind, reason: &str, rel: &str) {
        let groups = match kind {
            GroupKind::Ignored => &mut self.ignored,
            GroupKind::Skipped => &mut self.skipped,
            GroupKind::Generated => &mut self.generated,
        };
        let group = groups.entry(reason.to_string()).or_default();
        if group.seen.insert(rel.to_string()) {
            group.count += 1;
            if group.examples.len() < 5 {
                group.examples.push(rel.to_string());
            }
        }
    }

    pub(crate) fn finish(self) -> ScanStats {
        ScanStats {
            files_visited: self.files_visited,
            files_scanned: self.files_scanned,
            files_skipped: self.skipped.values().map(|group| group.count).sum(),
            bytes_scanned: self.bytes_scanned,
            ignored: finish_groups(self.ignored),
            generated: finish_groups(self.generated),
            inventory_boundaries: self.inventory_boundaries.into_iter().collect(),
        }
    }
}

#[derive(Clone, Copy)]
enum GroupKind {
    Ignored,
    Skipped,
    Generated,
}

#[derive(Default)]
struct ScanGroupBuilder {
    count: usize,
    examples: Vec<String>,
    seen: BTreeSet<String>,
}

fn finish_groups(groups: BTreeMap<String, ScanGroupBuilder>) -> Vec<ScanGroup> {
    groups
        .into_iter()
        .map(|(reason, group)| ScanGroup {
            reason,
            count: group.count,
            examples: group.examples,
        })
        .collect()
}

fn ignored_group_unit(reason: &str, rel: &str) -> String {
    let Some(dir) = reason.strip_prefix("common_ignore_dir:") else {
        return rel.to_string();
    };
    let mut parts = Vec::new();
    for part in rel.split('/') {
        parts.push(part);
        if part == dir {
            return parts.join("/");
        }
    }
    rel.to_string()
}
