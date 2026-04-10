# ML Signal Report Path Semantics Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for ML-signal run/report path semantics so artifact detail, report opening, and workflow rehydration behave consistently when a saved run includes ML-specific attachments and status fields.

## Scope Boundaries

In scope:

- `src-tauri/src/run_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/services/analysisService.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- ML-signal-related saved-run detail/report fields exposed through artifact summaries and detail payloads

Out of scope:

- training or scoring logic for ML signals
- options-related artifact surfaces
- redesigning the ML workspace itself

## Current User-Facing Behavior

When a run artifact includes ML-signal data, users can already see more than a generic saved run:

- saved-run detail exposes ML stage, bundle id, score-cache path, manifest path, feature-schema path/version, and policy summaries
- history/apply flows can load ML-signal artifacts and set the shell into ML-signal mode
- report opening still depends on the same `report_xlsx_path` concept used by conventional runs

The main risk is semantic drift:

- when `report_xlsx_path` exists but ML-specific files are the more relevant artifact
- when ML fields are present but path expectations are missing or stale
- when history/workflow surfaces do not agree on which path is the authoritative “latest” report

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/services/analysisService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

Key current ownership:

- `run_artifacts.rs` extracts `report_xlsx_path`, ML metadata, and ML-specific paths into summary/detail responses
- `historyService.ts` loads/apply/rerun flows and presents ML-signal artifact status messages
- `analysisService.ts` and workflow artifact state consume report-path and artifact payload semantics downstream

## Current Verification Surface

- tests in `src-tauri/src/run_artifacts.rs`
- tests in `web/src/services/historyService.test.ts`
- tests in `web/src/services/analysisService.test.ts`
- tests around workflow artifact application in `web/src/stores/workflowArtifactsStore.test.ts`

## Strengths Already Present

- ML-signal fields are already surfaced explicitly in artifact detail responses
- the saved-run detail contract distinguishes report path from ML cache/schema paths
- history apply/load flows already recognize ML-signal runs and switch shell mode appropriately
- report-path absence already degrades to clear status messaging instead of silent failure

## Weaknesses / Failure Modes

- the repo does not yet state clearly which path is authoritative for which user task
- report-path semantics are shared with conventional runs, but ML-signal runs often need additional artifact-path clarity
- history, analysis, and workflow-artifact surfaces can each carry overlapping path state
- it is easy for UI wording to imply that “the report” is the whole artifact story when ML-signal runs also carry cache/schema/policy attachments

## Regressions To Avoid

- losing the conventional `report_xlsx_path` behavior while improving ML-specific detail
- introducing conflicting precedence rules for report-path resolution across history, workflow, and quick-action surfaces
- hiding ML-specific artifact paths behind generic summaries that make troubleshooting harder
- widening the ML artifact contract without corresponding detail/report tests
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because ML-signal run detail is already a structured artifact contract with multiple path-bearing fields.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because the Rust artifact layer transports ML attachments as structured JSON fragments that must remain stable for UI consumers.
- [rust_xlsxwriter docs](https://rustxlsxwriter.github.io/index.html) are relevant because `report_xlsx_path` remains part of the expected artifact surface even for ML-signal runs.
- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because path precedence and workflow rehydration depend on shared client state rather than isolated component-local state.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because all saved-run detail and path-opening behavior ultimately depends on stable command payloads crossing the Rust/TS boundary.

## Proposed Improvement Approach

1. Document path semantics by task, not just by field name:
   - open latest report
   - inspect ML score cache/schema attachments
   - rerun or rehydrate an ML-signal workflow
2. Keep `report_xlsx_path` as the report contract, but make ML attachment paths explicitly secondary artifacts rather than accidental side channels.
3. Audit all consumers to ensure the same precedence order is used for report-path resolution.
4. Tighten detail/report wording so users can distinguish:
   - report path
   - artifact path
   - ML cache/schema paths
5. Add targeted tests for missing-path and mixed-path cases.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the authoritative meaning of `report_xlsx_path` for ML-signal runs.
- [x] Document which ML-specific paths are troubleshooting attachments versus primary workflow outputs.
- [x] Audit report-path precedence across history, analysis, and workflow artifact stores.
- [x] Add or tighten tests for ML-signal saved-run detail payloads in `run_artifacts.rs`.
- [x] Add or tighten tests for history/workflow behavior when ML paths exist but report paths do not.
- [x] Tighten UI wording where ML runs currently read too much like conventional report-only runs.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run relevant tests in `src-tauri/src/run_artifacts.rs`
- run relevant tests in `web/src/services/historyService.test.ts`
- run relevant tests in `web/src/services/analysisService.test.ts`
- run workflow-artifact store tests if report-path precedence changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- keep existing `report_xlsx_path` behavior stable
- introduce clearer ML path semantics as additive detail, not as a replacement for the conventional report contract
- if a path-semantic cleanup causes confusion in saved-run reopening or quick actions, revert to the simpler explicit wording and re-test
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/06_artifacts/20_report_json_contract_2026-04-09.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-08.md`
- `REPO_Improvement/08_ml/21_mlsignal_artifact_attachment_2026-04-08.md`
- `REPO_Improvement/04_results/16_report_path_reopen_flow_2026-04-08.md`

## Execution Notes

- Closed as an ML-signal report-path semantics item. The current code already keeps `report_xlsx_path` as the primary report contract while surfacing ML-specific attachments separately through saved-run detail, workflow artifacts, and analysis/report consumers.
- Re-verified ML path semantics through `run_artifacts` ML detail tests, `historyService.test.ts`, `analysisService.test.ts`, and `workflowArtifactsStore.test.ts`, ensuring ML attachment fields do not replace the conventional report-path contract.
- Acceptance evidence for closure: `npm.cmd --prefix web run test -- src/services/historyService.test.ts src/services/analysisService.test.ts src/stores/workflowArtifactsStore.test.ts`, `npm.cmd --prefix web run lint`, and `npm.cmd --prefix web run typecheck`.

