# Campaign Runner Implementation Plan

Date: 2026-04-02

## Purpose

This plan defines a low-debt path to introduce a one-click research campaign runner that:

1. runs batch backtest optimization across many datasets
2. runs validation for each optimization artifact that still needs validation
3. runs a single meta-analysis pass across the validated artifact corpus
4. optionally runs the post-meta AI/ML stage

It also covers dataset preloading for batch optimization:

- host preload for all strategies
- optional VRAM-resident staging only for the plain `DoubleMa` crossover strategy

The goal is to use as much of the existing optimize, validate, meta, artifact, and UI shell functionality as possible without turning the current frontend-driven batch loop into a long-term debt sink.

## Executive Summary

The repo already contains a partial batch runner under `Meta -> Batch`, but it is frontend-driven, limited to a narrow strategy set, and mutates the shared interactive stores while it runs. That is useful proof that the workflow is product-valid, but it is not the right final architecture for a full campaign runner.

The recommended architecture is:

- keep the existing `Meta -> Batch` aggregate surface as the initial UI shell
- add a dedicated backend campaign runner command in Tauri
- let the frontend build resolved campaign templates from the existing request builders
- let the backend own dataset iteration, artifact persistence, resume/skip logic, progress, and cancellation
- implement host-only preload for all strategies in v1
- implement optional VRAM staging only for plain `DoubleMa` on the kernel-backed route, behind a capability gate

This gives us a single product surface, a single orchestration owner, reuse of current stage logic, and a narrow performance specialization where it is most likely to pay off.

## Goals

- Support a one-click workflow over many datasets for the strategies already supported by the app.
- Reuse existing stage-specific optimize, validate, meta, and Stage 5 functionality rather than creating duplicate feature flows.
- Make the workflow resumable, skippable, and artifact-aware.
- Keep interactive single-run UX intact.
- Improve batch optimization throughput with preload-aware dataset handling.
- Keep higher-debt CUDA overlap isolated to plain `DoubleMa`.

## Non-Goals

- Full generic VRAM preloading for every strategy family.
- Multi-dataset parallel execution on one GPU in v1.
- Replacing the current interactive optimize / validate / meta flows.
- Rewriting the artifact schema for all existing saved runs.
- Shipping a new workspace or navigation model before the backend campaign runner exists.

## Current State

### Existing reusable surfaces

- `web/src/components/MetaWorkspacePanel.vue`
  - already exposes aggregate tabs `meta`, `batch`, `saved`, and `compare`
- `web/src/stores/analysisStore.ts`
  - already contains batch UI state, controls, and actions
- `web/src/services/metaService.ts`
  - already contains a partial batch loop
  - currently loads datasets, calls `runOptimization(...)`, applies tags, calls `runValidation(...)`, saves a `vectorgrid_batch_summary`, and can auto-run meta
- `web/src/services/optimizeService.ts`
  - already resolves strategy routing and builds resolved optimization requests
- `web/src/services/validationService.ts`
  - already builds validation from optimization top rows
- `src-tauri/src/commands_optimize.rs`
  - already routes optimization over in-memory candles
- `src-tauri/src/commands_validation.rs`
  - already supports Double-MA-family and Composite-family validation entrypoints
- `src-tauri/src/commands_meta.rs`
  - already runs meta selection and Stage 4 / Stage 5 ML confidence flows over saved artifacts
- `src-tauri/src/commands_artifacts.rs` and `src-tauri/src/run_artifacts.rs`
  - already save run artifacts and batch summary artifacts

### Important current limitations

- The existing batch loop in `metaService.ts` only supports `CompositeCpu`, `CompositeMo`, and `Vcet`.
- The current batch flow mutates the shared UI stores directly.
- The current batch flow loads datasets through the normal data workflow.
- Validation request shaping for campaign-like usage currently lives in frontend TypeScript.
- The current CUDA runtime is built around one active resident market cache and one stream per runtime entry.

### Existing backend batch remnants and how to treat them

The repo already contains backend batch-adjacent code. That code should be classified deliberately before the campaign runner starts depending on it.

#### Reuse directly as live backend primitives

- `src-tauri/src/commands_validation.rs`
  - `apply_cross_ticker_validation_generic(...)` already performs a real backend cross-dataset validation loop inside one validation request
  - treat this as a reusable validation-stage primitive, not as campaign orchestration
- `src-tauri/src/commands_meta.rs`
  - `run_meta_selection(...)` and the underlying artifact scan are already a real backend corpus pass over many saved artifacts
  - `apply_holdout_evaluation(...)` already reruns validation-like work across many holdout datasets and candidate groups
  - treat these as the existing backend meta / holdout engine that the campaign runner should call after corpus generation
- `crates/app_core/src/meta_select.rs`
  - already contains the real artifact-corpus ranking logic
  - this should remain the ranking engine rather than duplicating meta logic in the new campaign layer

#### Use as reference only, not as product dependencies

- `src-tauri/src/bin/ta_workflow_regression.rs`
  - already chains optimize -> validate -> meta -> holdout in a regression harness
- `src-tauri/src/bin/ta_headless_validate.rs`
  - already chains headless optimization and validation for DoubleMa-oriented workflows
- `crates/app_core/benches/e2e_workflow_ml.rs`
  - already proves that optimize -> validate -> meta -> optional ML can be chained in bench form

These are useful implementation references and verification surfaces, but the campaign runner should not be built by wrapping or invoking these utilities directly.

#### Keep only for migration compatibility, then reassess

- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
  - existing batch summary persistence is useful for the current frontend batch surface
  - once `vectorgrid_campaign_summary` is introduced, these legacy batch-summary helpers should be audited for either:
    - temporary dual-write / dual-read compatibility
    - or removal if no longer needed
- `src-tauri/src/commands_meta.rs`
  - meta artifact scanning currently knows about batch-summary filenames so it can ignore them
  - that ignore logic should be reviewed during migration because legacy and new summary names must not drift indefinitely

### Existing runtime constraints that matter

- `AppState` in `src-tauri/src/state.rs` has a single-run guard.
- Base dataset retention defaults to `1` loaded base dataset.
- `data_cache.rs` already provides reusable source identity and binary cache loading.
- `gpu_backtest_kernel.rs` keeps a thread-local CUDA runtime per device with:
  - one `Stream`
  - one `resident_market`
  - one scratch / cache ownership model oriented around one active dataset

Those constraints strongly support:

- batch-local host preload as a good v1 optimization
- backend-owned orchestration as the low-debt path
- DoubleMa-only device staging as a narrow optional extension

## Product Model

This feature should be modeled as a `campaign runner`, not as separate unrelated batch features.

The user-visible workflow is:

1. Select datasets.
2. Choose strategy, timeframe policy, optimization profile, validation profile, meta profile, and optional ML stage.
3. Click one action to run the campaign.
4. Observe progress per dataset and per stage.
5. Open the final campaign summary, saved run artifacts, meta artifact, and optional ML artifact.

The internal stage model is:

1. Dataset discovery and normalization
2. Optimization campaign
3. Validation campaign
4. Single meta-analysis
5. Optional Stage 5 / ML-confidence stage

The validation stage is intentionally not a new cross-asset ranking engine. It is simply:

- for each optimization artifact that still lacks validation under the current campaign settings, run one validation pass

Meta-analysis remains the cross-artifact ranking stage.

## Design Principles

### 1. Reuse existing stage functionality, not existing store choreography

We should reuse the current optimization, validation, and meta engines, but not rely on the current frontend batch loop as the permanent orchestration layer.

### 2. Frontend captures intent, backend owns execution

The frontend should build and send a campaign request. The backend should own:

- dataset iteration
- cancellation
- progress events
- artifact writes
- skip / resume
- host preload
- DoubleMa-only device staging

### 3. Generic path first, specialized path second

The generic campaign runner should work for all supported strategies without any CUDA specialization.

Only after that should we add DoubleMa-specific device staging.

### 4. Keep state isolated from interactive UI

Campaign execution should not depend on mutating the current interactive dataset / optimize / validate UI state as a control mechanism.

### 5. Prefer artifact-aware idempotence over ad hoc reruns

Campaign resume and skip behavior should come from artifact identity and per-stage fingerprints, not only from user memory.

## Recommended Architecture

## Frontend responsibilities

Frontend should:

- keep using `Meta -> Batch` as the initial shell
- gather dataset selection and campaign options
- reuse the current optimization request builder to create a resolved optimization template
- send a single campaign request to Tauri
- render progress and summary
- optionally reveal artifacts and switch tabs after completion

Frontend should not:

- loop datasets itself in the final design
- call `loadCsv`, `runOptimization`, `runValidation`, and `runMeta` as the long-term campaign implementation

## Backend responsibilities

Add a new Tauri command, for example:

- `run_research_campaign`
- `cancel_research_campaign`
- `load_campaign_summary`
- optionally `list_campaign_summaries`

The campaign runner should:

- acquire the run guard once for the whole campaign
- own the shared cancellation token
- load datasets through a batch-local loader, not the interactive store path
- call existing blocking optimization / validation / meta functionality directly
- persist stage results as artifacts
- emit campaign progress events
- write a campaign summary artifact that doubles as a resume checkpoint

## Campaign request shape

The exact types can evolve, but the request should look roughly like this:

```ts
interface ResearchCampaignRequest {
  name?: string
  strategy_kind: string
  dataset_paths: string[]
  timeframe_override?: string | null
  tags: string[]
  holdout_pct: number
  holdout_tag?: string | null
  skip_existing_optimization: boolean
  skip_existing_validation: boolean
  auto_meta: boolean
  auto_stage5: boolean
  optimization_template: Record<string, unknown>
  validation_config: Record<string, unknown>
  meta_config?: Record<string, unknown> | null
  stage5_config?: Record<string, unknown> | null
  preload_policy: "host_only" | "host_plus_doublema_vram"
}
```

### Why `optimization_template` should be frontend-built

This is the cleanest way to support all current strategies, including routes whose final request is already shaped in frontend logic.

The frontend already knows how to build:

- DoubleMa-family requests
- Composite CPU requests
- Composite MO requests
- preview-dependent flows such as DynamicDoubleMa resolved requests

The backend campaign runner should receive the resolved optimization template and patch only dataset-specific fields such as:

- `data_id`
- dataset metadata
- campaign tags / identity fields when needed

### Why `validation_config` should not be the full validation request

Validation is built from the optimization artifact output, especially the top candidate rows.

The backend runner should construct the actual per-dataset validation request from:

- the optimization request template
- the optimization result
- the campaign validation settings

That logic should live in Rust campaign helpers, not remain locked in the frontend batch loop.

## Campaign summary artifact

Do not overload the existing `vectorgrid_batch_summary` meaning indefinitely.

Add a new artifact kind:

- `vectorgrid_campaign_summary`

The file should be written early and updated after each significant stage so it can act as a checkpoint.

Suggested fields:

```json
{
  "kind": "vectorgrid_campaign_summary",
  "version": 1,
  "ts": "...",
  "state": "running|completed|cancelled|failed",
  "strategy_kind": "DoubleMa",
  "timeframe": "1h",
  "config": {
    "campaign_tag": "...",
    "optimization_fingerprint": "...",
    "validation_fingerprint": "...",
    "meta_enabled": true,
    "stage5_enabled": false,
    "preload_policy": "host_plus_doublema_vram"
  },
  "counts": {
    "datasets_total": 100,
    "optimized_ok": 95,
    "validated_ok": 92,
    "skipped_optimized": 10,
    "skipped_validated": 18,
    "failed": 3
  },
  "datasets": [
    {
      "csv_path": "...",
      "label": "...",
      "is_holdout": false,
      "optimization_status": "ok|skipped|failed|pending",
      "validation_status": "ok|skipped|failed|pending",
      "run_artifact_path": "...",
      "report_xlsx_path": "...",
      "error": null
    }
  ],
  "meta": {
    "status": "ok|skipped|failed|pending",
    "artifact_path": "...",
    "report_path": "..."
  },
  "stage5": {
    "status": "ok|skipped|failed|pending",
    "artifact_path": "..."
  }
}
```

## Stage-by-Stage Implementation Model

## Stage 1: dataset discovery and normalization

Reuse existing dataset enumeration patterns from the current batch UI, but move the actual load path to backend batch-local code.

Rules:

- require one effective timeframe across the campaign unless an explicit resample override is set
- preserve deterministic holdout tagging if `holdout_pct > 0`
- capture dataset identity from:
  - path
  - size
  - mtime
  - source content fingerprint if available

Recommended new helper:

- `src-tauri/src/campaign_dataset_loader.rs`

Responsibilities:

- discover and normalize selected datasets
- call `data_cache::identity_for_source(...)`
- call `data_cache::load_or_build_with_identity(...)`
- keep a batch-local ring buffer for host preload

Important constraint:

- do not rely on `AppState::load_price_data(...)` as the main batch preload path
- that API mutates the interactive loaded-dataset state and is optimized for the normal UI flow, not hidden campaign staging

## Stage 2: optimization campaign

The campaign runner should call the existing blocking optimization entrypoints directly with in-memory candles:

- DoubleMa-family routes
- Composite CPU routes
- Composite MO routes

This avoids nested Tauri command calls while the run guard is already held.

Recommended backend helper:

- `src-tauri/src/campaign_runner.rs`

Optimization responsibilities:

- clone and patch the resolved optimization template for the current dataset
- run the appropriate blocking optimization function
- persist the run artifact
- apply campaign tags
- record optimization fingerprint for skip / resume

### Skip rule

Optimization can be skipped when a run artifact already exists for the same:

- strategy kind
- dataset identity
- effective timeframe
- optimization fingerprint
- campaign tag policy

## Stage 3: validation campaign

Validation should run once per optimization artifact that still lacks compatible validation results.

This is not a new ranking engine. It is corpus generation.

Required new helper:

- build validation request from optimization request + optimization result + campaign validation config

Recommended placement:

- `src-tauri/src/campaign_validation_builder.rs`
  or
- shared helpers under `src-tauri/src/validation_common.rs`

Responsibilities:

- extract candidate rows from optimization output
- fall back to best params if needed
- apply campaign candidate cap
- set walk-forward / perturb / bootstrap / CSCV / cross-ticker / hard-gate config
- preserve validation artifact context

### Skip rule

Validation can be skipped when the run artifact already contains validation results matching the current validation fingerprint.

If optimization is skipped but validation is missing, the campaign runner should still validate that existing artifact.

## Stage 4: single meta-analysis

Run meta-analysis once after the dataset loop completes, not once per dataset.

This stage should reuse the existing artifact-driven backend flow:

- scan run artifacts
- filter by strategy kind, timeframe, required tag, and validation presence
- run meta selection

Meta should proceed only if there are enough validated artifacts to satisfy the configured minimum samples.

Meta failure should not erase a successful campaign corpus. It should mark the campaign summary as partial success.

## Stage 5: optional post-meta AI/ML

If enabled, run the current Stage 4 / Stage 5 ML-confidence flow after meta selection completes successfully.

This stage should remain optional and warning-tolerant:

- campaign success should not depend on Stage 5 unless explicitly configured that way
- failure should be recorded in the campaign summary, not destroy campaign completion

## Dataset Preload Strategy

## Generic preload for all strategies

This is the v1 optimization and should be implemented first.

### Design

Use a batch-local two-slot host ring:

- `current`
- `next`

While dataset `N` is optimizing:

- start loading and normalizing dataset `N+1`
- parse or binary-load through `data_cache`
- apply optional timeframe override or batch-local resample
- keep `Candles` ready in host memory

When optimization for `N` completes:

- swap `next` into `current`
- begin preloading `N+2`

### Why this is low debt

- it reuses existing data cache logic
- it does not require changes to the generic optimization engines
- it does not require new CUDA ownership models
- it avoids mutating the interactive data store during campaign staging

## Optional VRAM staging for plain `DoubleMa`

This is the only high-debt specialization recommended in this plan.

### Scope guard

Only enable this path when all of the following are true:

- `strategy_kind == "DoubleMa"`
- the request uses the plain crossover path, not another DoubleMa-family profile
- no dynamic strategy IR is involved
- no strategy-local overlay path that materially changes device prep ownership is involved
- backend route resolves to the kernel-backed GPU path

Everything else should fall back to host preload only.

### Why the scope should be this narrow

The current CUDA runtime in `gpu_backtest_kernel.rs` is built around one active runtime entry with:

- one stream
- one resident market cache
- one scratch ownership model

Generalizing that for all DoubleMa-family profiles would increase complexity quickly. Plain `DoubleMa` is the best candidate because:

- it is the most direct crossover path
- it most closely matches the resident market / kernel runtime assumptions
- it is the most likely place for the overlap benefit to be real

### Design for the specialized path

Add batch-only device staging support to the DoubleMa kernel runtime:

- keep the active resident market for dataset `N`
- add a second staging slot for dataset `N+1`
- add a transfer stream or equivalent async upload lane
- use an event-based handoff when current compute completes
- if staging fails due to VRAM pressure or unsupported shape, drop back to host-only preload

Recommended touchpoints:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- possibly `crates/app_core/src/double_ma.rs`
- possibly `crates/app_core/src/double_ma_backend.rs`

### Critical rule

This capability must be batch-runner-only in v1.

Do not entangle the normal single interactive optimization path with campaign staging ownership until the batch version is proven.

## How to Minimize Tech Debt

## 1. Extend the current batch surface, do not create a parallel product surface

Keep using the existing `Meta -> Batch` shell rather than adding a new workspace.

## 2. Replace the frontend loop, do not decorate it forever

`metaService.runBatch()` is a good prototype, not the final orchestration home.

Make it a thin frontend client around the new Tauri campaign command.

## 3. Keep batch-local dataset loading out of `AppState` interactive ownership

Use `data_cache` directly for preload instead of pretending the campaign is just a user clicking `load csv` very quickly.

## 4. Capture resolved optimization templates once

Let frontend request builders keep doing what they are already good at, especially for strategy-specific shaping.

## 5. Move validation campaign shaping into Rust

The backend campaign runner must not depend on frontend store state to figure out which candidate rows to validate.

## 6. Keep artifact identity explicit

Add campaign-level optimization and validation fingerprints to make skip / resume deterministic.

## 7. Keep DoubleMa staging optional and capability-gated

If it regresses, disable only that specialization and keep the whole campaign runner intact.

## 8. Classify legacy batch code before inheriting it

The new runner should reuse live backend primitives, borrow ideas from workflow harnesses, and remove or migrate legacy summary behavior intentionally.

Do not let the new campaign layer accidentally depend on:

- the old frontend store choreography
- dev-only regression binaries
- bench-only workflow harnesses
- stale batch-summary naming quirks

## Proposed File Touchpoints

## Frontend

- `web/src/components/MetaWorkspacePanel.vue`
  - rename or reframe `Batch Generation` around campaigns
- `web/src/stores/analysisStore.ts`
  - campaign controls, progress, summary state
- `web/src/services/metaService.ts`
  - convert current batch loop into thin client
- `web/src/services/metaBatchRequestBuilder.ts`
  - evolve into campaign request builder or add a new `campaignRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
  - expose resolved optimization template generation cleanly
- `web/src/services/validationService.ts`
  - likely still reused for single-run UX only

## Tauri

- `src-tauri/src/commands_campaign.rs` new
- `src-tauri/src/campaign_runner.rs` new
- `src-tauri/src/campaign_dataset_loader.rs` new
- `src-tauri/src/campaign_validation_builder.rs` new
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`

## Core / CUDA

- `crates/app_core/src/gpu_backtest_kernel.rs`
  - DoubleMa-only staging specialization
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_backend.rs`

## Artifact and Resume Strategy

The campaign runner should support:

- fresh run
- resume last incomplete campaign
- retry only failed datasets
- skip already optimized datasets
- skip already validated datasets

The existing retry-friendly behavior in the current batch summary is worth preserving, but it should move behind campaign-aware artifact semantics.

## Progress and Cancellation

Add a new progress channel, for example:

- `campaign_progress`

Payload should include:

- campaign stage
- dataset index / total
- dataset label
- stage status
- optimization / validation / meta counters
- whether host preload is active
- whether DoubleMa device staging is active or bypassed

Cancellation should be explicit and stop after the current safe point:

- current dataset load
- current optimization
- current validation
- current meta or Stage 5 call

## Testing Plan

## Unit tests

- campaign request normalization
- strategy routing matrix
- validation request building from optimization output
- artifact fingerprint and skip logic
- holdout split reproducibility

## Integration tests

- campaign summary writes and resumes
- skip optimization but run validation
- skip validation but include artifact in meta
- meta runs once across all validated artifacts
- Stage 5 optional behavior
- cancellation after current dataset

## GPU-specific tests

- DoubleMa staging path capability gate
- fallback to host-only preload on unsupported request shape
- fallback to host-only preload on VRAM budget failure

## Verification commands

- `cargo test --workspace`
- relevant frontend tests around `analysisStore`, `metaService`, and request builders
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- after DoubleMa staging lands:
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
  - only after confirming no other benchmark or profiling process is running

## Phased Implementation Order

## Phase 0: remnant audit and migration boundary

- classify each discovered backend batch remnant as:
  - reuse directly
  - reference only
  - migration compatibility only
  - removal candidate
- decide whether `vectorgrid_batch_summary` needs temporary compatibility with `vectorgrid_campaign_summary`
- review meta scan ignore rules so old and new summary names are both handled intentionally during migration
- confirm that dev-only workflow harnesses remain verification-only and are not called by product code

Exit criteria:

- the implementation has a written keep / migrate / remove decision for each known remnant class
- the campaign runner boundary is explicit enough that new product code does not inherit dead or legacy behavior by accident

## Phase 1: backend campaign skeleton, no preload specialization

- add campaign request / progress / summary types
- add `run_research_campaign`
- keep existing UI shell
- backend loops datasets sequentially
- no hidden preload yet
- no DoubleMa device staging yet

Exit criteria:

- one-click optimize -> validate -> meta works end to end for at least the currently supported strategies in the existing batch flow

## Phase 2: strategy-general support

- remove current frontend strategy restriction
- route all existing strategies through the resolved optimization template
- add backend validation request builders for DoubleMa-family and Composite-family validation

Exit criteria:

- campaign runner works for all strategy routes that already work in single-run mode

## Phase 3: artifact-aware resume and skip logic

- add optimization fingerprint
- add validation fingerprint
- add campaign summary checkpoint updates
- add retry failed / resume incomplete campaign behavior

Exit criteria:

- interrupted campaigns can resume without rerunning completed datasets unnecessarily

## Phase 4: generic host preload

- add batch-local dataset loader
- add two-slot host preload ring
- keep interactive data state untouched

Exit criteria:

- measurable reduction in inter-dataset idle time on batch optimization without route regressions

## Phase 5: plain `DoubleMa` VRAM staging

- batch-only CUDA staging path
- capability gate and fallback
- perf validation on the kernel-backed route

Exit criteria:

- clear measured batch throughput benefit on plain `DoubleMa`
- no regressions for non-DoubleMa campaign strategies

## Open Questions

- Should the new campaign artifact kind replace the current `vectorgrid_batch_summary` immediately, or should we support both during migration?
- Should legacy batch-summary writers and meta ignore rules be removed in the same rollout as the campaign runner, or only after one migration cycle?
- Should Stage 5 failure ever mark the whole campaign as failed, or should it remain warning-only by default?
- Do we want campaign-level dataset discovery to remain scoped to repo-local CSVs and Polygon-downloaded datasets first, or should any file path list be allowed equally from day one?
- Should the first version expose the DoubleMa staging toggle to users, or keep it automatic and diagnostic-only?

## Recommendation

Proceed with a split strategy:

- generic host preload for all strategies
- backend-owned campaign orchestration for optimize -> validate -> meta -> optional Stage 5
- plain `DoubleMa`-only VRAM staging as an optional later specialization

This is the best balance between:

- one-click workflow value
- reuse of existing code
- support for the current strategy set
- contained tech debt
- performance upside where the current architecture already gives us the best chance of success

## Related Follow-On Plans

The campaign runner should stay focused on orchestration. Two adjacent scopes should remain separate implementation tracks:

- `metric_contract_unification_plan_2026-04-02.md`
  - owns cross-strategy `Core17` / `Trade22` metric-surface work
  - matters most for artifact normalization, compare/export parity, and later ML feature quality
- `validation_shortlist_admission_plan_2026-04-02.md`
  - owns better candidate admission into validation
  - matters most for validation quality and meta-corpus quality, but does not belong inside campaign orchestration

Recommended sequencing:

- campaign runner can ship before either follow-on plan is complete
- improved validation admission can land independently of universal metric-contract support
- universal `Core17` support is a likely prerequisite for a strong campaign-wide Stage 5 / ML feature surface
