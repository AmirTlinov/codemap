# agent-context-cli npm wrapper

This package installs the native `ctx` binary from the matching GitHub Release.

Install from the npm registry after this package is published there:

```bash
npm install -g agent-context-cli
ctx doctor
```

Published GitHub Releases also include the packed wrapper tarball:

```bash
gh release download v0.2.0 --repo AmirTlinov/ctx --pattern 'agent-context-cli-0.2.0.tgz'
GH_TOKEN="$(gh auth token)" npm install -g ./agent-context-cli-0.2.0.tgz
ctx doctor
```

The npm package is only a distribution wrapper. `ctx` remains a standalone Rust binary and does not add dependencies to projects where it is used.

Supported prebuilt targets:

- `linux-x64` -> `x86_64-unknown-linux-gnu`
- `darwin-arm64` -> `aarch64-apple-darwin`

For local testing, set `CTX_NPM_INSTALL_ARCHIVE=/path/to/ctx-v<version>-<target>.tar.gz`.

For private GitHub releases, expose `GH_TOKEN`, `GITHUB_TOKEN`, or `CTX_NPM_GITHUB_TOKEN` during install.
