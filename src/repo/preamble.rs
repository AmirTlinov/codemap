use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use anyhow::{Context, Result};
use globset::GlobBuilder;
use ignore::WalkBuilder;
use regex::Regex;

use crate::cache;
use crate::model::{
    AnchorDomain, ConfigLoadError, CodemapConfig, Domain, FileInfo, GitChange, ImportBindingsBySpec,
    PackageDependency, PackageInfo, Project, ProjectTimings, ScanGroup, ScanStats, ScriptInfo,
    SymbolInfo,
};
