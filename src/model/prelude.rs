#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MapPrelude {
    pub root: String,
    pub cwd: String,
    pub cwd_rel: Option<String>,
    pub git_root: Option<String>,
    pub vcs: Option<String>,
    pub head: HeadPrelude,
    pub branch: BranchPrelude,
    pub remote: RemotePrelude,
    pub worktree: WorktreePrelude,
    pub freshness: FreshnessPrelude,
    pub unknowns: Vec<PreludeUnknown>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HeadPrelude {
    pub oid: Option<String>,
    pub short: Option<String>,
    pub detached: bool,
    pub unborn: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BranchPrelude {
    pub name: Option<String>,
    pub upstream: Option<String>,
    pub upstream_gone: bool,
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RemotePrelude {
    pub name: Option<String>,
    pub display: Option<String>,
    pub sanitized_url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WorktreePrelude {
    pub clean: bool,
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub conflicted: usize,
    pub renamed: usize,
    pub deleted: usize,
    pub typechanged: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FreshnessPrelude {
    pub network_used: bool,
    pub remote_refs: String,
    pub remote_currentness: String,
    pub fetch_head_mtime: Option<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PreludeUnknown {
    pub kind: String,
    pub effect: String,
}
