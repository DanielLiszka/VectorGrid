# Artifact Replay Report Evidence Contract

Last updated: 2026-04-28

## Purpose

This document defines the shared evidence language for VectorGrid artifacts,
reports, and replay/reopen flows. It is intentionally a derived contract over
the heterogeneous artifact payloads that already exist in the repository. It is
not a requirement to write one generic `artifact_evidence` object into every
workflow.

Use this contract when changing:

- saved run history and hydration;
- XLSX/CSV/JSON report save and open behavior;
- replay, reopen, repair, or rerun labels;
- output parity checks for report or artifact shape;
- frontend evidence/status labels derived from current artifact fields.

## Common Evidence Summary

A common evidence summary may be derived from existing payloads and service
responses. Do not duplicate these fields into a second persisted source of
truth unless a later migration plan explicitly versions that schema.

Recommended derived fields:

| Evidence area | Existing sources |
| --- | --- |
| Artifact identity | `kind`, recognized artifact family, `ts`, artifact path |
| Config identity | `config_fingerprint`, `config_fingerprint_version` |
| Input data | `csv_path`, dataset metadata, timeframe/base timeframe, resample fields |
| Source fingerprint | source file size, mtime, content fingerprint when already present |
| Execution route | `route_provenance`, requested/resolved/realized backend, fallback reason |
| Lineage | parent path, parent kind, relation, source artifact/hash when present |
| Reports | `report_xlsx_path`, `report_xlsx_exists` / `report_path_exists`, state label |
| ML score scope | score space, score scope, native backend, fallback notes, schema path/version |
| User labels | tags and workflow-local schema/version fields |

Report state is path/existence based:

| Derived state | Meaning |
| --- | --- |
| `not_exported` | No report path is recorded. |
| `available` | A report path is recorded and existence is known true. |
| `missing_on_disk` | A report path is recorded and existence is known false. |
| `unknown` | A report path is recorded but existence was not checked or was not returned. |

Routine history refresh must not compute new file hashes or scan report/cache
trees to fill this summary. Use the fields already carried by artifact list,
detail, hydration, and report-save responses.

## Workflow Carve-Outs

The shared evidence summary is only a language for common labels. Workflow
payloads remain local:

- Optimization and validation run artifacts own optimization request/result,
  validation request/result, route provenance, config fingerprint, report path,
  dataset metadata, metric-surface fields, and tags.
- Meta, Stage 5, and ML policy artifacts remain saved-result review surfaces
  unless a workflow-specific recompute contract exists.
- ML Signal evidence keeps score-cache, feature-schema, score-scope, and native
  backend fields in the ML/run artifact surfaces.
- Options-expression evidence keeps exactness, parent artifact identity,
  dataset manifest/provenance, price semantics, quote quality, replacement data,
  and split-plan fields in the options workflow.
- Live-session artifacts are broker/session evidence. They are not replayable
  broker actions without the live execution state-machine contract.
- Batch and campaign summaries do not imply resumable campaign replay by
  themselves.

## Replay Classes

Use explicit replay/reopen labels so users do not confuse saved evidence review
with exact recomputation.

| Replay class | Meaning |
| --- | --- |
| `exact_evidence_review` | Reopen or hydrate the saved payload/report exactly as persisted. |
| `compatible_recompute` | Recompute from compatible request/data/context, with current engine behavior. |
| `replacement_scoped_validation` | Validate against an explicitly chosen replacement dataset or holdout scope. |
| `review_only_saved_result` | Inspect saved result payloads where recompute semantics are not defined. |
| `blocked` | Replay is unsafe or unsupported; show the blocking reason. |

Blocked examples include live broker-action replay, campaign resume from summary
artifacts alone, and options-expression replay when required parent/data/source
hashes do not match.

## Report Mutation Semantics

Reports are derived evidence outputs. XLSX reports are saved under the report
surface and may be pointed to by run artifacts, but they are not a separate JSON
artifact family.

| Command or helper | Mutation contract |
| --- | --- |
| `save_report_xlsx` | Writes an XLSX report. Does not mutate a source artifact. `artifact_updated=false` with an empty warning is normal. |
| `save_report_xlsx_from_artifact` | Validates a run artifact, writes an XLSX report, then narrowly attempts to persist `report_xlsx_path` back into that run artifact. |
| `build_report_xlsx` | Builds report bytes for fallback download. Does not mutate artifacts. |
| `open_path` / `reveal_path` | Shell conveniences only. Do not mutate artifacts. |
| `openSavedRunReport` | Opens or reveals the report path from saved-run detail after validating report-path state. Does not mutate artifacts. |
| `open_reports_dir` / `open_runs_dir` | Directory shell conveniences only. Do not mutate artifacts. |
| `repair_run_artifact` | Existing migration/repair path for run artifacts. Must remain dry-run/write-target/path scoped. |

`artifact_update_warning` is operation-scoped. It means a report operation
completed with a caveat, such as the report file being written while artifact
path persistence failed. The first implementation surfaces this warning in
immediate validation/export status and performance metadata; it does not create
a durable operation log.

## Output Parity Authority

Current parity authority:

- `scripts/run_output_parity_check.mjs` regenerates deterministic headless JSON
  and XLSX outputs and runs structural checks.
- `scripts/check_output_parity.mjs` performs structural parity checks. It is
  not a semantic diff and is not a performance gate.
- `scripts/check_output_semantic_parity.mjs` owns configured semantic JSON
  parity.

Current configured output parity includes JSON/XLSX report pairs and JSON
semantic parity. CSV structural checking exists in the script layer, but CSV is
not a configured output-parity claim until a headless CSV output and parity pair
are added.

Do not require byte-for-byte XLSX equality. If XLSX semantics need stronger
coverage, add sheet/cell invariants instead of raw ZIP-byte equality.

## Artifact Compatibility Summary

Run-history list/detail/hydration responses expose a derived
`artifact_compatibility` summary. It is not persisted as a duplicate source of
truth; it is resolved from the artifact family/kind, top-level artifact schema
version, optional report schema version, and command response version.

Compatibility states:

- `supported`: current command surface can load/replay the artifact.
- `legacy_unknown`: no top-level artifact schema version was recorded; the run
  remains inspectable and replay-compatible under legacy fallback rules.
- `future_unsupported`: artifact schema is newer than this command surface;
  history detail/report review stays available, but load/apply/rerun hydration is
  blocked until an adapter is added.
- `invalid`: artifact schema metadata is malformed; review-only handling is
  required.

The compatibility summary is intentionally part of saved-run history responses
only. It must not be added to headless report JSON or output-parity baselines
unless a separate report schema/versioning plan is approved.

## Migration Policy

Artifact evidence changes should be additive and read-compatible:

- preserve legacy `vectorbt_*` aliases and current artifact-family recognition;
- keep CPU-only builds free of CUDA or native dependency requirements;
- avoid destructive migrations;
- use existing repair commands for run-artifact normalization;
- do not persist duplicate evidence fields that can drift from the workflow
  owner fields.

Any future persisted evidence schema must have a versioned migration/repair
plan and an output-parity impact plan before it is added.

## Blocked Or Deferred Areas

The following require separate workflow-owner review before implementation:

- durable operation/audit log for report warnings, repairs, deletes, tag edits,
  and blocked replay attempts;
- live broker-action replay from saved session evidence;
- campaign resume/replay from summary artifacts;
- exact Polygon cache/file provenance field naming;
- configured validation-shortlist CSV output parity;
- external Open XML validators or new report-validation dependencies;
- cache/report/data-root scanning during normal history refresh.

## First-PR Regression Checklist

For a narrow evidence/report-status change, verify:

- report-save warnings are visible in manual and auto-export validation status;
- no-warning direct report saves remain quiet;
- report open/reveal operations do not mutate artifacts;
- `save_report_xlsx` does not mutate source artifacts;
- `save_report_xlsx_from_artifact` mutates only the run artifact report path;
- saved-run history labels remain path/existence based;
- output parity scripts do not require new ignore paths;
- no new cache/report root scans are added to history refresh.
