# Security policy

## Supported releases

Security fixes target the latest published release. Older binaries remain available for
compatibility testing but do not receive backports unless a release note says otherwise.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting for
[`AmirTlinov/codemap`](https://github.com/AmirTlinov/codemap/security/advisories/new).
Do not open a public issue for an unpatched vulnerability. Include the affected version,
platform, reproduction, impact, and whether `proof --run` or repository-controlled commands
are involved.

## Trust boundary

`codemap` reads untrusted repositories. Map commands do not execute target-project code or use
the network. `proof` is a plan by default; `proof --run` is explicit execution consent and still
admits only the documented command allowlist. The external cache is local derived data, not a
secret store or authenticity boundary. See `docs/CACHE.md` and `docs/CURRENT_LIMITATIONS.md`.
