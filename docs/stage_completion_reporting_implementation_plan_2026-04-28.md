# Stage Completion Reporting Implementation Plan

Date: 2026-04-28

Status: proposed implementation plan after static code audit, web research, and three read-only sub-agent audits.

## Intent

Add a shared stage-completion contract so VectorGrid can clearly say what finished, what it means, what evidence supports it, what remains blocked or advisory, and what the next workflow action should be.

The current application is not shallow at the stage-local reporting level. Optimization has ranked tables, heatmaps, Pareto/drill views, TradingView Lightweight Charts, stability, and trade detail. Validation has a much stronger reporting spine: explicit run verdicts, candidate verdicts, report exports, an XLSX dashboard, a decision ledger, and robustness/audit sheets.

The gap is narrower and more structural: completion semantics are fragmented across stage-specific status strings, table visibility, artifact presence, report paths, and saved-run hydration. The implementation should add a small, durable `stage_completions` contract to existing artifacts and then let workflow UI, history, reports, and later automation consume that contract.

## Findings Validated

The deeper audit validates the original finding with important caveats:

- Valid: there is no single canonical stage-completion/reporting contract. `workflowArtifactsStore` models `currentRun` and `currentValidation` snapshots, not stage conclusions.
- Valid: frontend readiness mostly infers completion from data/result/table/status presence in `useUiReadiness`, `useGuidedWorkflowReadiness`, and `useResearchCockpitModel`.
- Valid: optimization completion currently persists a run artifact with `validation: null` and blank `report_xlsx_path`; the meaning of the run is communicated elsewhere through optimization views and status copy.
- Valid: validation is the strongest existing reference model. It has backend `run_verdict` semantics and a rich report workbook.
- Valid: Meta selection is score/ranking oriented. It consumes validation approval, but its output should not be rebranded as validation approval.
- Valid with nuance: ML Signal backend has validation-grade `run_verdict` and `deployability` metadata, but frontend persistence and saved-run reporting expose less than the validation workbook path.
- Important counterpoint: the repo already has a useful run/validation artifact and hydration shape. This plan should extend it, not replace it.

## Local Evidence Anchors

- `web/src/stores/workflowArtifactsStore.ts:13` and `web/src/stores/workflowArtifactsStore.ts:31` model run and validation snapshots, but no completion packet.
- `web/src/composables/useUiReadiness.ts:130`, `web/src/composables/useGuidedWorkflowReadiness.ts:61`, and `web/src/composables/useResearchCockpitModel.ts:218` infer readiness from result presence, rows, status text, and table state.
- `web/src/services/optimizeService.ts:2724` builds optimization artifacts with `validation: null` and blank report path.
- `crates/app_core/src/validation_pipeline.rs:75` defines the validation `run_verdict` contract used as the strongest existing model.
- `src-tauri/src/commands_reports_validation_xlsx.rs:3846` builds the validation Dashboard, and `src-tauri/src/commands_reports_validation_xlsx.rs:2325` builds the Decision Ledger.
- `crates/app_core/src/meta_select.rs:558` consumes validation approval when selecting artifacts, but `crates/app_core/src/meta_select.rs:365` has no equivalent run approval/verdict field for Meta output.
- `web/src/components/MetaWorkspacePanel.vue:821` explicitly frames Meta workbook output as score-first and points to JSON/saved artifact detail as the stronger evidence surface.
- `src-tauri/src/commands_meta.rs:1162` and `src-tauri/src/commands_meta.rs:2321` preserve Stage 5 semantics as advisory review labels, not deployability proof.
- `web/src/services/mlSignalService.ts:1777` and `web/src/services/mlSignalService.ts:1858` persist ML Signal run/validation artifacts with blank `report_xlsx_path`.
- `src-tauri/src/run_artifacts.rs:481` summarizes run artifacts without a general verdict/deployability completion contract.

## Research Basis

External research supports a contract-first, status-first implementation:

- W3C WCAG 2.2 status messages guidance defines status messages as information about success, results, progress, waiting state, or errors without a focus/context change. This supports making completion feedback programmatically available rather than only visible in scattered text. Source: <https://www.w3.org/WAI/WCAG22/Understanding/status-messages.html>
- W3C ARIA technique ARIA22 recommends `role="status"` and `aria-atomic="true"` for advisory dynamic status updates. This is relevant for the eventual Stage Conclusion card and status line updates. Source: <https://www.w3.org/WAI/WCAG21/Techniques/aria/ARIA22>
- GOV.UK task-list guidance treats tasks as named actions with short statuses that indicate whether the user can start or move on. This maps well to a multi-stage research workflow where stages can be complete, blocked, warning, or inspect-only. Source: <https://design-system.service.gov.uk/components/task-list/>
- USWDS step-indicator guidance says step indicators fit mostly linear, multi-page processes and are not appropriate for nonlinear flows that can be completed in any order. VectorGrid has both linear and nonlinear paths, so the canonical contract should be status data, not just a stepper UI. Source: <https://designsystem.digital.gov/components/step-indicator/>
- AWS ADR guidance emphasizes recording architecturally significant decisions with context and consequences. This supports documenting the `stage_completions` contract as an explicit architectural decision, because it becomes a durable artifact/interface. Source: <https://docs.aws.amazon.com/prescriptive-guidance/latest/architectural-decision-records/adr-process.html>
- Google Model Cards and NIST AI RMF both reinforce that ML outputs need transparent reporting of intended use, evaluation context, performance, and risk. This supports surfacing ML Signal `run_verdict`, `deployability`, score scope, and caveats in completion packets rather than only raw artifact fields. Sources: <https://research.google/pubs/model-cards-for-model-reporting/> and <https://www.nist.gov/itl/ai-risk-management-framework>

## Non-Goals

- Do not rewrite optimization, validation, Meta, ML Signal, or options-expression math.
- Do not create a new artifact family for completion packets in the first pass.
- Do not turn Meta or Stage 5 advisory labels into validation approval or deployability proof.
- Do not make CUDA, ML, Meta, or validation reports mandatory for a basic optimization artifact.
- Do not invalidate legacy saved artifacts that lack `stage_completions`.
- Do not make the UI depend only on the new packet until legacy backfill and tests are in place.

## Proposed Contract

Persist one optional top-level array on existing artifacts:

```json
{
  "stage_completions": [
    {
      "schema_version": 1,
      "stage": "optimization",
      "status": "success",
      "completed_at": "2026-04-28T12:00:00Z",
      "artifact_kind": "vectorgrid_run_artifact"
    }
  ]
}
```

Use snake_case on disk and camelCase in TypeScript.

```ts
export type StageCompletionStage =
  | "optimization"
  | "validation"
  | "meta"
  | "stage5_ml_confidence"
  | "ml_signal"
  | "options_expression";

export type StageCompletionStatus =
  | "success"
  | "failed"
  | "cancelled"
  | "blocked"
  | "stale";

export type StageCompletionVerdict =
  | "search_complete_requires_validation"
  | "passed"
  | "approved"
  | "approved_with_warnings"
  | "review"
  | "rejected"
  | "inspect_only"
  | "not_applicable";

export interface StageCompletionPacketV1 {
  schemaVersion: 1;
  stage: StageCompletionStage;
  substage?: string;
  status: StageCompletionStatus;
  verdict: StageCompletionVerdict;
  completedAt: string;
  durationMs?: number | null;
  artifactKind?: string;
  artifactPath?: string;
  reportXlsxPath?: string;
  csvPath?: string;
  strategyKind?: string;
  timeframe?: string;
  configFingerprint?: string;
  backendUsed?: string;
  route?: string;
  recommendation?: string;
  summary?: Record<string, unknown>;
  warnings?: string[];
  blockers?: string[];
  caveats?: string[];
  nextActions?: string[];
  backfilled?: boolean;
}
```

Implementation guidance:

- Keep `summary` small. It should not duplicate `req`, `res`, candidates, tables, or full report payloads.
- Use `substage` for families that contain multiple actionable phases, such as `ml_signal` dataset export, score cache, validation, and policy sweep.
- Use `verdict` for user-facing decision semantics. Use `status` for execution outcome.
- Use `approved` only where an existing backend contract supports approval semantics, such as validation `run_verdict.approved`.
- Use `review` or `inspect_only` for Meta and Stage 5 decisions unless the backend later defines a stronger approval contract.
- Use `backfilled: true` for packets derived from legacy artifacts rather than originally persisted.

## Storage Strategy

Store `stage_completions` on existing artifact families first:

- `vectorgrid_run_artifact`: optimization, validation, ML Signal stage packets.
- `vectorgrid_meta_selection`: Meta selection packet.
- `vectorgrid_meta_stage5_ml_confidence` and legacy stage4/stage5 aliases: Stage 5 packet.
- `vectorgrid_ml_confidence_policy_eval`: policy-evaluation packet if included in this first slice.

Do not create a new `ArtifactFamily` in the first pass. A new artifact family would force changes through artifact library snapshots, catalog rows, deletion/retention logic, UI library consumers, and repair paths without adding clear value for a small summary object.

## Backfill And Compatibility

Legacy artifacts remain valid.

Read-time backfill should synthesize packets when `stage_completions` is absent:

- Add an optimization packet if `/run/res` exists.
- Add a validation packet if `/validation/res` exists.
- Add an ML Signal packet if `/ml_signal/stage` exists.
- Add Meta/Stage 5 packets for their artifact kinds when loaded through the aggregate artifact APIs.
- Use artifact `ts` as `completed_at` when no better timestamp exists.
- Copy paths, report paths, config fingerprints, strategy, timeframe, route, backend, and report existence from existing fields.
- Mark backfilled packets with `backfilled: true`.

Write-time canonicalization should gradually materialize missing packets in repaired or re-saved artifacts. The likely Rust hook is `normalize_run_artifact_value_pure` in `src-tauri/src/run_artifacts.rs`.

## Stage Mapping

| Stage | Source Fields | Packet Verdict Rules |
| --- | --- | --- |
| Optimization | `run.req`, `run.res`, `optimization_metric_surface`, route provenance | `search_complete_requires_validation`; warnings from route fallback/artifact persistence |
| Validation | `validation.res.run_verdict`, report path, evidence policy summary | `approved`, `approved_with_warnings`, `passed`, or `rejected` from existing `run_verdict` |
| Meta | `res.groups`, warnings, holdout coverage, report path | `review`; never validation approval |
| Stage 5 ML confidence | `decision_counts`, evidence policy summary, model health | `review` or `inspect_only`; preserve "not deployability proof" caveats |
| ML Signal | `validation.res.run_verdict`, `ml_signal.deployability`, score scope, score contract | Use existing validation/deployability metadata, but do not invent production approval |
| Options Expression | acceptance summary, validation verdict fields | Defer until conventional and ML Signal packets are stable, unless options scope is pulled in explicitly |

## Implementation Phases

### Phase 0 - Fixture Lock And Contract ADR

- Add this plan as the implementation reference.
- Optionally add a short ADR under `docs/` once the schema is accepted.
- Capture representative legacy artifacts for optimization-only, validation, Meta, Stage 5, ML Signal dataset export, ML Signal validation, and ML Signal policy sweep.
- Confirm no existing artifact fields are dropped by save/repair/hydration.

Acceptance criteria:

- The packet contract is reviewed and accepted.
- Fixture list exists and covers legacy missing-packet cases.

### Phase 1 - Frontend Type And Store Support

Touchpoints:

- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/historyService.ts`
- `web/src/utils/core/workflowRunRestore.ts`

Work:

- Add `StageCompletionPacketV1` type and `stageCompletions` to `WorkflowRunArtifact`.
- Normalize both `stage_completions` and `stageCompletions`.
- Include packet data in artifact signatures so packet-only updates are not ignored.
- Add a read-time `deriveStageCompletionsFromWorkflowArtifact` fallback for legacy artifacts.
- Preserve existing packets when validation or ML Signal services rebuild artifacts from known fields.

Tests:

- `workflowArtifactsStore.test.ts`
- `workflowArtifactsSync.test.ts`
- `historyService.test.ts`

### Phase 2 - Rust Hydration, Detail, And Repair Support

Touchpoints:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports.rs`

Work:

- Add optional `stage_completions` to run detail and hydration payloads.
- Backfill packets in detail/hydration responses for legacy artifacts.
- Preserve unknown future packet fields where possible.
- Add canonicalization in repair/save paths so re-saved artifacts carry packets.
- Keep report export behavior stable; only pass packet metadata through where it helps report summaries.

Tests:

- Rust unit tests in `run_artifacts.rs` for packet parse, backfill, preservation, and repair.
- Report export tests only where packet data is intentionally surfaced.

### Phase 3 - Conventional Optimization And Validation Packets

Touchpoints:

- `web/src/services/optimizeService.ts`
- `web/src/services/validationService.ts`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`

Work:

- Write an optimization packet when optimization completes.
- Write or update a validation packet when validation completes.
- Map `run_verdict` into packet verdicts without changing backend semantics.
- Include `reportXlsxPath`, auto-export warning state, artifact save state, backend route, and key warnings.
- Add a small "Stage Completion" block or row to the validation workbook only if it does not duplicate the existing Dashboard and Decision Ledger.

Tests:

- `optimizeService.test.ts`
- `validationService.test.ts`
- existing validation XLSX tests for no regression in run verdict display.

### Phase 4 - Shared UI Consumption

Touchpoints:

- `web/src/composables/useUiReadiness.ts`
- `web/src/composables/useGuidedWorkflowReadiness.ts`
- `web/src/composables/useResearchCockpitModel.ts`
- `web/src/utils/workbenchActionModel.ts`
- new `web/src/components/workbench/StageConclusionCard.vue` if a reusable component is justified.

Work:

- Add `useStageCompletions` or similar derived model.
- Let readiness prefer packet verdict/status while retaining legacy fallbacks.
- Add a compact Stage Conclusion surface in the Research Cockpit and Guided workflow.
- Use `role="status"` plus `aria-atomic="true"` for dynamic conclusion updates.
- Keep labels short and plain: "Search complete", "Validation approved", "Review required", "Blocked", "Inspect only".

Tests:

- `useUiReadiness.test.ts`
- `useGuidedWorkflowReadiness` tests or `GuidedWorkspacePanel.test.ts`
- `useResearchCockpitModel.test.ts`
- `WorkbenchInspector.test.ts` if the inspector consumes packet state.

### Phase 5 - Meta And Stage 5 Reporting Parity

Touchpoints:

- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/run_artifacts.rs`

Work:

- Persist Meta selection packets with `verdict: "review"`.
- Add packet summaries to saved Meta artifact listings.
- Add a Meta workbook decision/evidence sheet or section that explains ranking, warnings, holdout coverage, and why this is not validation approval.
- Persist Stage 5 packets with decision counts and explicit advisory caveats.
- Keep Stage 5 decision buckets as review labels.

Tests:

- `metaService.test.ts`
- Meta XLSX tests for decision/evidence sheet coverage.
- `run_artifacts.rs` tests for Meta and Stage 5 summary hydration.

### Phase 6 - ML Signal Reporting Parity

Touchpoints:

- `web/src/services/mlSignalService.ts`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/run_artifacts.rs`

Work:

- Persist ML Signal packets for dataset export, score cache, validation, and policy sweep.
- Surface existing `run_verdict`, `deployability`, score scope, score contract, and evidence policy metadata in packet summaries.
- Keep `deployability` framed as review metadata, not production proof.
- Consider a lightweight ML Signal summary export after packets are stable. A full XLSX equivalent can come later.

Tests:

- `mlSignalService.test.ts`
- `MlSignalWorkspacePanel` tests where current artifact summaries are asserted.
- Rust ML Signal tests only if backend output shape changes.

### Phase 7 - Options Expression Follow-Up

Options expression already has stronger verdict vocabulary than the conventional navigation layer in some places, including validation acceptance summaries. Add packets after conventional, Meta, and ML Signal semantics stabilize.

Touchpoints:

- `web/src/stores/optionsExpressionStore.ts`
- `web/src/services/optionsExpressionService.ts`
- `web/src/components/OptionsExpressionPanel.vue`
- `src-tauri/src/commands_options_expression.rs`

Risk:

- Options expression artifacts are more typed. Only add optional/default fields, and avoid cross-contaminating options-specific verdicts with conventional validation terminology.

## UI Principles

- Show a conclusion before showing all details.
- Keep the conclusion short, then link to the detailed table/report/artifact.
- Distinguish "execution success" from "decision approval".
- Distinguish "validation approved" from "Meta/Stage 5 review label".
- Announce dynamic completion updates through status regions.
- Avoid a single linear stepper as the sole model because parts of VectorGrid are nonlinear and artifact-driven.

## Risk Register

| Risk | Mitigation |
| --- | --- |
| Legacy artifacts lack packets | Backfill at read time and mark `backfilled: true` |
| Packet-only changes ignored by sync signatures | Include `stageCompletions` in signatures and sync comparisons |
| Validation or ML services drop packets while rebuilding artifacts | Preserve existing packets and append/update by stage |
| Meta "accept" is mistaken for deployability approval | Use `review` and explicit caveats; never map Meta to validation approval |
| Status lines and packet summaries diverge | Generate major status-line conclusions from packet helpers where practical |
| Report exports duplicate too much data | Put only decision summary and links in reports; keep detailed evidence in existing sheets/artifacts |
| Screen readers get noisy updates | Use `role="status"` for completion changes, not for high-frequency progress ticks |

## Acceptance Criteria

- A legacy optimization-only artifact hydrates with a backfilled optimization packet.
- A legacy validation artifact hydrates with optimization and validation packets.
- A new optimization run persists a non-backfilled optimization packet.
- A new validation run updates/persists a validation packet with `run_verdict` semantics.
- Research Cockpit and Guided workflow can show stage conclusions from packets with legacy fallback.
- Meta packets never claim validation approval.
- Stage 5 packets retain advisory/deployability caveats.
- ML Signal packets expose existing validation/deployability summaries without changing backend math.
- Existing validation XLSX report behavior remains intact.
- Existing artifact library, restore, repair, and report export tests pass.

## Recommended First Slice

Implement only these in the first PR or task batch:

1. Add `StageCompletionPacketV1` TypeScript type and store normalization.
2. Add legacy backfill helpers for optimization and validation.
3. Persist new optimization and validation packets from `optimizeService` and `validationService`.
4. Add Rust hydration/detail preservation/backfill for run artifacts.
5. Update Research Cockpit and Guided workflow to read the packet while keeping fallbacks.
6. Add targeted tests for store, sync, history, optimization, validation, and Rust hydration.

Defer Meta, Stage 5, ML Signal report parity, options expression, and new report sheets until the first slice proves the contract is stable.
