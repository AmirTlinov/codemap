# Agent protocol v1

`codemap` is one read-only CLI tool. It does not route tasks, inject hidden
context, call a model, or require a daemon or network service.

## Invocation order

Choose the narrowest anchor already known:

```bash
codemap where <symbol> --format json
codemap cone <file-or-file#symbol> --format json
codemap ls <file-or-directory> --format json
```

Use `codemap ls .` only when the relevant scope is unknown. Execute entries from
`agent.expands` only when the current evidence leaves the corresponding question
open. After edits, use `codemap changed --format json`, then
`codemap proof changed --format json`.

## Transport

- stdout contains the requested report and nothing else;
- stderr contains diagnostics and execution progress;
- readable output is for people; integrations consume JSON and its bundled schema;
- `codemap schema manifest` returns the machine registry;
- `codemap schema <kind>` returns the exact report schema;
- `agent.expands` contains argv arrays, starts with `codemap`, and requests JSON;
- no report command writes into the target repository by default.

Every report has its own `kind`, `schema_version`, and `build_identity`. The
required `agent` envelope v1 provides the common fields:

```json
{
  "envelope_version": "1",
  "report_kind": "cone_report",
  "report_version": "18",
  "result": "success",
  "scope": {},
  "snapshot": { "state": "observed", "identities": ["..."] },
  "horizon": {
    "status": "open",
    "groups": 13,
    "reasons": ["dynamic_runtime_registration"],
    "certificate_count": 7
  },
  "expands": [["codemap", "ls", "src/a.ts", "--all", "--format", "json"]]
}
```

`scope`, `snapshot`, and `horizon` are summaries. The report-specific schema
remains authoritative for complete facts and certificates.

## Exit taxonomy

| Code | Meaning | stdout |
| ---: | --- | --- |
| 0 | success | requested report, when applicable |
| 10 | valid empty map | report with `agent.result=valid_empty_map` |
| 20 | invalid anchor or config | invalid-anchor report when one exists; otherwise empty |
| 21 | unsupported request | empty |
| 22 | stale/corrupt diagnostic or failed verification diagnostic | report if already produced |
| 23 | unsafe execution refused | empty |
| 70 | internal error | empty |

For codes 0, 10, and report-producing 20/22, parse stdout first and require the
schema. Never infer empty versus invalid from readable prose.

## Compatibility and shells

`schemas/manifest.json` owns protocol v1, exit codes, streams, and every report
schema version. Report changes advance only that report version. An incompatible
common-envelope or exit-semantic change advances the agent protocol version.

Generate completion source without loading a repository:

```bash
codemap completions bash
codemap completions zsh
codemap completions fish
codemap completions powershell
codemap completions elvish
```
