# agent-context-cli npm wrapper

This package installs the native `ctx` binary from the matching GitHub Release.

```bash
npm install -g agent-context-cli
ctx doctor
```

The npm package is only a distribution wrapper. `ctx` remains a standalone Rust binary and does not add dependencies to projects where it is used.

Supported prebuilt targets:

- `linux-x64` -> `x86_64-unknown-linux-gnu`
- `darwin-arm64` -> `aarch64-apple-darwin`

For local testing, set `CTX_NPM_INSTALL_ARCHIVE=/path/to/ctx-v<version>-<target>.tar.gz`.

For private GitHub releases, expose `GH_TOKEN`, `GITHUB_TOKEN`, or `CTX_NPM_GITHUB_TOKEN` during install.
