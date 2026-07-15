use std::collections::{BTreeMap, BTreeSet};

pub struct CacheFileDelta {
    pub cached_fingerprint: String,
    pub cached_content_hashes: BTreeMap<String, Option<String>>,
    pub unchanged: BTreeSet<String>,
    pub changed_or_added: BTreeSet<String>,
    pub removed: BTreeSet<String>,
}

impl CacheFileDelta {
    pub fn is_exact_hit(&self) -> bool {
        self.changed_or_added.is_empty() && self.removed.is_empty()
    }
}
