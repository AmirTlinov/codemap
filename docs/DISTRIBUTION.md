# Distribution and compatibility

## Release identity

A release is one signed Git tag `vX.Y.Z`. The tag version, `Cargo.toml`, packaged crate,
downloaded binary, `codemap --version`, `doctor.build_identity`, checksums, and flagship
benchmark receipt must identify the same source commit and version.

GitHub Releases is the binary/package registry because the unrelated crates.io package named
`codemap` predates this project. Every release contains the source `.crate`, platform archives,
per-archive checksums, one global `SHA256SUMS`, machine-readable identity receipts, and GitHub
artifact attestations. Release archives contain only the binary, license, and README. The Cargo
package excludes fixtures, tests, benchmark corpora, and internal execution plans.

## Binary support matrix

| Platform | Target | Release gate |
| --- | --- | --- |
| macOS Apple Silicon | `aarch64-apple-darwin` | build, archive verification, fresh-download smoke |
| macOS Intel | `x86_64-apple-darwin` | build, archive verification, fresh-download smoke |
| Linux x86-64 glibc | `x86_64-unknown-linux-gnu` | build, archive verification, fresh-download smoke |
| Windows x86-64 MSVC | `x86_64-pc-windows-msvc` | build, archive verification, fresh-download smoke |

Other Rust targets may build from source but are not release promises. The CLI has no runtime
network dependency. Native Windows `proof --run` remains limited where a planned command requires
a POSIX shell; map and plan generation remain supported.

## Installation

Checksummed archives are published at GitHub Releases. Homebrew uses the same immutable macOS
archive through `AmirTlinov/homebrew-tap`:

```bash
brew tap AmirTlinov/tap
brew install codemap
codemap doctor
```

Source installation stays available from an exact tag:

```bash
cargo install --git https://github.com/AmirTlinov/codemap --tag vX.Y.Z --locked --force
```

## Upgrade, downgrade, cache, and schemas

The cache is external and derived. Every artifact carries cache-format, schema, repository-root,
and fingerprint identity. A newer or older binary that cannot prove compatibility treats the
artifact as a miss and rebuilds or quarantines it; it never migrates data inside the target repo.
Release CI runs `previous -> current -> previous -> current` against one shared external cache and
verifies the repository tree remains byte-identical.

Public JSON kinds are selected through `codemap schema manifest`. Schema versions never decrease
for an existing kind. Additive or changed report contracts advance their report version; breaking
agent-envelope or exit-taxonomy changes require a new agent protocol version. See
`SCHEMA_POLICY.md` for the complete policy and `CACHE.md` for privacy, retention, corruption, and
write-failure behavior.

## Release boundary

Maintenance builds may publish with explicit limitations. A release is called flagship only when
its exact binary passes the frozen 144-run acceptance and the released-download scenario.
