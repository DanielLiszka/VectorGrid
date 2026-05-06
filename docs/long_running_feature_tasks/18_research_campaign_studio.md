# Research Campaign Studio

Date: 2026-05-06
Status: researched implementation plan
Risk: medium-high
Primary value: turn single-run optimization and validation into reproducible multi-run research programs

## Summary

Research Campaign Studio should be the first broad orchestration expansion after candidate-level review surfaces are strong. The repo already has a real campaign runner, campaign summaries, progress events, resume/skip-existing logic, batch summary artifacts, meta workflows, stage completion packets, artifact retention, and task lifecycle UI. The missing product layer is a dedicated campaign design and review workspace that makes those capabilities visible, comparable, and safe to rerun.

The correct first slice is not a new workflow engine. It is a typed campaign template and review layer over the existing `run_research_campaign` path. The campaign artifact should become the user's durable object: it records what was requested, which datasets ran, which were skipped/failed/retried, what artifacts were produced, which validation profile was used, and whether results are comparable.

This plan intentionally keeps resume semantics conservative. Existing code already has campaign resume paths and evidence states, but the studio should not imply full replay from summary artifacts unless the saved request, dataset identities, artifact identities, and command compatibility checks all pass.

## External Research Basis

- W3C PROV Overview: https://www.w3.org/TR/prov-overview/
  - PROV frames provenance around entities, activities, and agents, with reproducibility, versioning, derivation, and trust assessment as core use cases. Campaign artifacts should mirror that shape without importing PROV as a runtime dependency.
- RO-Crate 1.2 introduction: https://www.researchobject.org/ro-crate/specification/1.2/introduction.html
  - RO-Crate packages research data with contextual metadata such as software, equipment, workflows, technical metadata, and provenance. VectorGrid campaign summaries should borrow the idea of a portable research package without adopting JSON-LD in the app path.
- OpenLineage specification: https://github.com/OpenLineage/OpenLineage/blob/main/spec/OpenLineage.md
  - OpenLineage separates runs, jobs, datasets, and run events. Campaign Studio should similarly keep static campaign definition separate from per-run outcome events and dataset facets.
- MLflow Tracking: https://www.mlflow.org/docs/latest/ml/tracking
  - MLflow organizes work around experiments, runs, metrics, params, tags, artifacts, and child runs. That maps well to a VectorGrid campaign with parent campaign plus per-dataset child run rows.
- DVC Experiments: https://dvc.org/doc/user-guide/experiment-management
  - DVC experiments track variations, parameters, metrics, plots, queued runs, and reproducible local experiment state. The useful lesson for this repo is queue/compare/reproduce, not adding Git-like hidden refs.
- Common Workflow Language user guide: https://www.commonwl.org/user_guide/
  - CWL shows the value of explicit workflow descriptions for portable computational analysis. VectorGrid should borrow the principle of explicit step inputs/outputs, not adopt CWL syntax.
- Prefect flows and states: https://docs.prefect.io/concepts/flows and https://docs.prefect.io/latest/concepts/states
  - Prefect's flow/task state model reinforces that failed, cancelled, skipped, and completed are separate terminal states. Campaign Studio should not collapse them into a single success/failure flag.
- Dagster partitions and backfills: https://document-sensor-result.dagster.dagster-docs.io/concepts/partitions-schedules-sensors/partitions and https://release-1-5-9.dagster.dagster-docs.io/concepts/partitions-schedules-sensors/backfills
  - Partitioned assets and backfills are useful analogies for dataset-by-dataset campaign rows and retrying subsets. The repo should borrow the subset-retry discipline, not Dagster's runtime.

## Current Repo Anchors

- Tauri command registration: `src-tauri/src/commands.rs`
- Campaign command boundary: `src-tauri/src/commands_campaign.rs`
- Frontend campaign orchestration: `web/src/services/metaService.ts`
- Campaign resume tests: `web/src/services/metaServiceCampaignResume.test.ts`
- Campaign strategy routing tests: `web/src/services/metaServiceCampaignStrategy.test.ts`
- Task progress lifecycle: `web/src/stores/taskStore.ts`, `web/src/stores/taskStoreCampaignProgress.test.ts`
- Stage completion packets: `web/src/utils/core/stageCompletion.ts`, `src-tauri/src/run_artifacts.rs`
- Meta workspace host: `web/src/components/MetaWorkspacePanel.vue`
- Artifact library: `src-tauri/src/commands_artifacts.rs`, `src-tauri/src/run_artifacts.rs`
- Existing planning: `docs/feature_expansion_implementation_plans_2026-04-30/08_artifacts_reports_history_expansion.md`
- Existing warning: no campaign resume from summary artifacts alone.

## Existing Behavior To Preserve

- `ResearchCampaignRequest` already supports multiple datasets, optimization template, execution mode, validation config, preload policy, skip-existing, and output tagging.
- Backend summaries already contain counts, item summaries, evidence state, resume state, diagnostics, dataset identity, and manifest metadata.
- Frontend already blocks unsupported strategy kinds such as `MlSignal` and `DynamicDoubleMa` for campaigns.
- Task store already consumes `research_campaign_progress` events.
- Campaign summaries are a saved artifact class with retention settings.

## Product Shape

The studio should have three panes:

1. Campaign Designer
   - Template selector.
   - Dataset set builder.
   - Strategy family and backend preference.
   - Validation profile.
   - Follow-up/output pack switches.
   - Compatibility preview before run.

2. Campaign Monitor
   - DAG-like status view.
   - Dataset rows with pending/running/skipped/failed/succeeded/retried.
   - Current task progress.
   - Runtime diagnostics such as preload mode, VRAM staging feedback, skip-existing decision, and artifact match status.

3. Campaign Review
   - Comparable result table.
   - Per-dataset winners and validation verdicts.
   - Evidence pack availability.
   - Failure and skipped-dataset summary.
   - Links into Candidate Explainability Workbench, validation workspace, report export, and artifact library.

## State Model

The UI should use a strict state model so campaign rows are explainable and retry-safe.

| State | Meaning | Retry behavior |
|---|---|---|
| `draft` | Template/config exists but has not been invoked | no retry; user can edit |
| `queued` | Row is accepted into the campaign request | cancellable before start |
| `preloading` | Dataset load/prefetch is in progress | retry only from dataset load |
| `running_optimization` | Optimization command is active | retry from optimization after cancellation/failure |
| `running_validation` | Validation follow-up is active | retry validation-only if optimization artifact is compatible |
| `skipped_existing` | Existing artifact matched skip policy | reviewable, not fresh evidence |
| `succeeded` | Row completed requested stages | reviewable and comparable if basis matches |
| `failed` | Row failed with recorded reason | retry allowed with same request/dataset |
| `cancelled` | User or app cancelled before terminal success | retry allowed only from last safe checkpoint |
| `blocked` | Compatibility or policy failed before run | no retry until request is changed |

These state labels are frontend-facing. They can be derived from existing backend summary fields in the first slice.

## Template Presets

Use templates as request builders, not as hard-coded alternate engines.

| Template | Purpose | Required inputs | Default validation |
|---|---|---|---|
| Single-symbol deep dive | Run one dataset with deeper top-k and evidence follow-up | dataset, strategy, objective | walk-forward plus evidence pack when available |
| Cross-ticker robustness | Same request across ticker datasets | dataset set, symbol labels, strategy | preserved-finalists cross validation |
| Multi-timeframe sweep | Same symbol across resampled or imported timeframes | dataset group, timeframe set | per-timeframe validation labels |
| Strategy-family bakeoff | Compare Double-MA-like and supported composite profiles | dataset set, allowed strategy families | shared objective plus comparable-only warnings |
| Validate-only replay | Revalidate previously saved campaign finalists | campaign summary/artifact identity | strict compatibility checks |

## Proposed Data Contracts

Keep this additive. The existing `ResearchCampaignSummary` can remain the backend authority while a frontend studio model normalizes it.

### CampaignTemplateV1

```ts
export interface CampaignTemplateV1 {
  schemaVersion: 1;
  id: string;
  name: string;
  family:
    | "single_symbol_deep_dive"
    | "cross_ticker_robustness"
    | "multi_timeframe_sweep"
    | "strategy_family_bakeoff"
    | "validate_only_replay";
  description: string;
  defaultExecutionMode: "optimize_and_validate" | "optimize_only" | "validate_only";
  requiredInputs: string[];
  defaultValidationProfileId: string;
  supportedStrategyKinds: string[];
  blockedStrategyKinds: string[];
}
```

### CampaignStudioRunGraphV1

```ts
export interface CampaignStudioRunGraphV1 {
  schemaVersion: 1;
  campaignId: string;
  manifestHash?: string;
  nodes: CampaignStudioNodeV1[];
  edges: CampaignStudioEdgeV1[];
  comparability: CampaignComparabilitySummaryV1;
}
```

### CampaignStudioNodeV1

```ts
export interface CampaignStudioNodeV1 {
  id: string;
  kind:
    | "campaign"
    | "dataset"
    | "load"
    | "optimize"
    | "validate"
    | "meta"
    | "artifact"
    | "report";
  label: string;
  status: "pending" | "running" | "succeeded" | "skipped" | "failed" | "blocked";
  summary?: string;
  artifactPath?: string;
  runArtifactId?: string;
  warnings?: string[];
}
```

### CampaignComparabilitySummaryV1

```ts
export interface CampaignComparabilitySummaryV1 {
  comparable: boolean;
  basis: Array<
    | "same_objective"
    | "same_strategy_kind"
    | "same_timeframe"
    | "same_validation_profile"
    | "same_data_provider"
    | "same_price_semantics"
  >;
  blockers: string[];
  warnings: string[];
}
```

### CampaignRetryPlanV1

```ts
export interface CampaignRetryPlanV1 {
  schemaVersion: 1;
  campaignId: string;
  sourceSummaryPath?: string;
  retryMode:
    | "failed_only"
    | "cancelled_only"
    | "failed_and_cancelled"
    | "selected_rows"
    | "validate_only_compatible";
  selectedItemIds: string[];
  compatible: boolean;
  blockers: string[];
  warnings: string[];
}
```

### CampaignEvidencePacketV1

```ts
export interface CampaignEvidencePacketV1 {
  schemaVersion: 1;
  campaignId: string;
  templateId?: string;
  requestHash: string;
  manifestHash?: string;
  datasetIdentityBasis: string[];
  stageCounts: Record<string, number>;
  comparableResultCount: number;
  nonComparableResultCount: number;
  generatedArtifactIds: string[];
  generatedReportPaths: string[];
  warnings: string[];
}
```

## Backend And Tauri Scope

Phase 1 should avoid changing the backend campaign executor unless a missing field blocks safe display.

Likely backend/Tauri changes:

- Add an optional campaign template id/name into campaign manifest metadata if absent.
- Add a compact campaign graph summary builder in `src-tauri/src/commands_campaign.rs` or `src-tauri/src/run_artifacts.rs` only if the frontend cannot derive it safely.
- Add artifact summary fields for campaign template, comparability, and available follow-up artifact counts.
- Preserve all existing campaign request validation and unsupported-strategy behavior.

Avoid:

- New generic DAG executor.
- Resume from partial artifacts without original request compatibility.
- Hidden reruns when the user only asked to review a campaign.
- Parallel benchmark-like campaign execution that contaminates perf measurements.

## Resume And Retry Safety Matrix

| Scenario | Safe first-slice behavior |
|---|---|
| Same saved summary, same original request hash, same datasets present | allow review and explicit retry of failed/cancelled rows |
| Summary marks evidence unsafe/stale | block resume; allow new campaign from template only |
| Dataset path exists but hash/identity differs | block resume; show dataset identity mismatch |
| Existing artifact matches skip policy | mark as `skipped_existing`; do not count as fresh run |
| Validation profile changed | require new validation-only campaign or full rerun |
| Strategy kind now unsupported | block before invoke |
| Backend command version changed with no compatibility verdict | block resume; allow copy-to-new draft |

This matrix should be encoded in tests before adding one-click retry actions.

## Frontend Scope

Recommended files:

- New `web/src/utils/campaignStudioModel.ts`
- New `web/src/utils/campaignTemplates.ts`
- New `web/src/services/campaignStudioService.ts` only if keeping this logic out of `metaService.ts` is cleaner.
- Either a new `CampaignWorkspacePanel.vue` or a dedicated section inside `MetaWorkspacePanel.vue`.
- Tests near existing campaign tests:
  - `web/src/services/metaServiceCampaignStrategy.test.ts`
  - `web/src/services/metaServiceCampaignResume.test.ts`
  - new `web/src/utils/campaignStudioModel.test.ts`

Frontend requirements:

- Use explicit compatibility preview before invoke.
- Render skipped and failed items as first-class outcomes.
- Link each result row to existing artifact and drilldown surfaces.
- Show "not comparable" states instead of sorting incomparable rows together.
- Keep campaign creation dense and utilitarian. This is an operational research tool, not a marketing screen.

## Artifact And Report Scope

Campaign summary should be the first-class artifact. Add report/export only after the campaign review model is stable.

First export:

- `campaign_summary.md` or compact JSON copy summary.
- Fields:
  - campaign id
  - template id
  - request hash
  - dataset count
  - succeeded/skipped/failed counts
  - validation profile
  - comparability summary
  - top finalists by comparable metric
  - artifact links/paths
  - warnings

Later export:

- XLSX workbook with:
  - Campaign Overview
  - Dataset Outcomes
  - Optimization Finalists
  - Validation Evidence
  - Failures And Skips
  - Artifact Links

## Implementation Phases

### Phase 1: Inventory And Template Model

- [ ] Inventory existing `ResearchCampaignRequest`, manifest, summary, progress event, and resume states.
- [ ] Add `CampaignTemplateV1` definitions in frontend utilities.
- [ ] Add template presets listed above.
- [ ] Add compatibility preview builder.
- [ ] Add tests for supported/blocked strategies and validation profile defaults.

### Phase 2: Studio Review Model

- [ ] Normalize backend campaign summary into `CampaignStudioRunGraphV1`.
- [ ] Derive comparability summary from objective, strategy kind, timeframe, validation profile, data provider, and price semantics where present.
- [ ] Mark missing data as `unknown`, not comparable.
- [ ] Add row model tests for full, partial, failed, skipped, and legacy campaign summaries.

### Phase 3: UI Integration

- [ ] Add campaign studio entry point in Advanced or Meta workspace.
- [ ] Render template picker and request preview.
- [ ] Render live progress from task store events.
- [ ] Render run graph/status view.
- [ ] Render campaign result table with comparable-only default sorting.
- [ ] Add links to artifacts, validation evidence, and candidate explainability.
- [ ] Add row-level actions: review, retry, validate-only retry, open artifact, copy failure reason.
- [ ] Add campaign-level actions: duplicate draft, export compact summary, compare completed rows.

### Phase 4: Saved Campaign Summary Enhancements

- [ ] Add optional campaign template metadata to saved summary if not already available.
- [ ] Add optional `comparison_summary` block to saved campaign artifacts.
- [ ] Preserve old summaries with tolerant parsing.
- [ ] Add artifact library badges for campaign status and comparability.

### Phase 5: Reports And Follow-Up Actions

- [ ] Add compact Markdown or JSON summary export first.
- [ ] Add XLSX only after row model stabilizes.
- [ ] Add "open candidate review" action from campaign finalists.
- [ ] Add "retry failed datasets" only when original request and dataset identities are still compatible.

### Phase 6: Campaign Evidence Packet

- [ ] Add a frontend-derived `CampaignEvidencePacketV1` first.
- [ ] Promote to backend artifact summary only if frontend derivation proves insufficient.
- [ ] Include template id, request hash, dataset identity basis, comparability basis, stage counts, and output artifact ids.
- [ ] Add report/export parity checks if packet becomes persisted or exported.

## Verification Plan

Rust:

- `cargo test -p ta_desktop_demo_app --lib campaign`
- targeted tests in `src-tauri/src/commands_campaign.rs` if backend fields change
- artifact summary tests if `run_artifacts.rs` changes

Frontend:

- `npm --prefix web run test -- metaServiceCampaignStrategy`
- `npm --prefix web run test -- metaServiceCampaignResume`
- `npm --prefix web run test -- campaignStudioModel`
- `npm --prefix web run typecheck`
- retry/resume matrix tests before enabling row-level retry actions

Broad gates:

- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`
- Browser QA only if a new visible workspace is added.

## Regression Watchpoints

- Do not create a hidden second source of truth for campaign requests.
- Do not treat skipped existing artifacts as fresh evidence without a match decision.
- Do not merge rows with different objectives, timeframes, validation profiles, or price semantics into one comparable ranking.
- Do not infer dataset identity from label alone.
- Do not implement automatic broker/live follow-up actions from a campaign.
- Do not let UI template defaults silently change existing campaign request behavior.
- Do not allow retry actions to mutate completed successful rows unless the user explicitly creates a new campaign.
- Do not treat failed row absence as success; missing rows should be `unknown` or `incomplete`.

## Definition Of Done

- Users can define a campaign from a template and see the exact request before running.
- Campaign progress is visible as a graph and dataset table.
- Saved campaign summaries reopen into the studio.
- Comparable and non-comparable rows are clearly separated.
- Failed and skipped datasets are first-class, not hidden in logs.
- Campaign artifacts link into candidate review, validation evidence, reports, and history.
