// Responsibility: repo-constants
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+surface-cache-v9");

pub(crate) const ROOT_MARKERS: &[&str] = &[
    ".codemap.yml",
    ".codemap.yaml",
    ".codemap.json",
    "package.json",
    "pnpm-workspace.yaml",
    "yarn.lock",
    "package-lock.json",
    "Cargo.toml",
    "go.mod",
    "go.work",
    "pyproject.toml",
    "requirements.txt",
    "Package.swift",
    "Makefile",
    "justfile",
];

pub(crate) const COMMON_IGNORE_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".idea",
    ".vscode",
    "node_modules",
    ".pnpm-store",
    ".yarn",
    "bower_components",
    "dist",
    "build",
    "out",
    "target",
    ".next",
    ".next-dev",
    ".nuxt",
    ".output",
    ".svelte-kit",
    ".turbo",
    ".vercel",
    ".vite",
    ".cache",
    ".parcel-cache",
    "coverage",
    "playwright-report",
    "test-results",
    "storybook-static",
    ".pytest_cache",
    "__pycache__",
    ".mypy_cache",
    ".ruff_cache",
    "vendor",
    "tmp",
    "temp",
    "logs",
];

pub(crate) const BINARY_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "pdf", "zip", "gz", "tgz", "rar", "7z", "exe",
    "dll", "so", "dylib", "wasm", "woff", "woff2", "ttf", "otf", "mp3", "mp4", "mov", "avi", "mkv",
    "bin", "class", "jar",
];

pub(crate) const ASSET_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "svg", "woff", "woff2", "ttf", "otf", "mp3", "mp4",
    "mov", "webm", "wav",
];

pub(crate) const SOURCE_EXTS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "rs", "go", "java", "kt", "kts", "swift", "c",
    "cc", "cpp", "h", "hpp", "cs", "rb", "php", "vue", "svelte",
];

pub(crate) const SCRIPT_EXTS: &[&str] = &["sh", "bash", "zsh"];

pub(crate) const TEXT_EXTS: &[&str] = &[
    "json", "toml", "yaml", "yml", "md", "txt", "sql", "prisma", "graphql", "gql", "proto", "avsc",
    "lock", "css", "scss", "sass", "less", "svg", "snap", "snapshot", "sh", "bash", "zsh",
];

pub(crate) const DOMAIN_HINT_DIRS: &[&str] = &[
    "domains",
    "packages",
    "apps",
    "services",
    "libs",
    "crates",
    "modules",
    "cmd",
    "components",
];
