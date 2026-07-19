# Agent protocol v1

`codemap` is one read-only CLI tool. It does not route tasks, inject hidden
context, call a model, or require a daemon or network service.

## Map affordances

An integration may use the narrowest scope already known when a structural map is useful:

```bash
codemap where <symbol> --format json
codemap cone <file-or-file#symbol> --format json
codemap ls <directory> --format json
```

`codemap ls .` provides current-level orientation. `agent.expands` exposes exact
deeper views; `codemap changed --format json` maps the current diff and
`codemap proof changed --format json` maps nearby verification surfaces. These
commands are independent affordances. The protocol does not prescribe investigation
order, required calls, or project verification choices.

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
remains authoritative for complete facts. `certificate_count` is integrity metadata
for persisted observation bases; it is not a confidence score, verdict, or action.

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
