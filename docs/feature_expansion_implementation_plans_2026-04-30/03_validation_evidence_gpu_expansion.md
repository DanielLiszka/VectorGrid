# Validation, Evidence, And GPU Validation Expansion Plan

Date: 2026-04-30

## Goal

Expand validation as the product's evidence layer. The priority is clearer validation semantics, GPU route transparency, replay-compatible artifact fields, and user-facing verdicts that distinguish optimized candidates from validated candidates.

## Current Owner Files

Start here:

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/stores/validationStore.ts`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/sharpe_stats.rs`

Related docs:

- `docs/artifact_replay_report_evidence_contract.md`
- `docs/verification_benchmark_release_authority.md`
- `docs/NUMERICAL_REPRODUCIBILITY.md`

## Research Basis

- Backtest overfitting research supports CSCV/PBO-style evidence when selecting from many strategy variants: <https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253>
- The same research family discusses deflating Sharpe-style results for multiple testing and non-normality: <https://academic.oup.com/jrssig/article/18/6/22/7038278>
- Hansen SPA and related data-snooping tests are a useful research basis for "many models tried" disclosure, but should be added as evidence metadata before changing pass/fail rules: <https://www.ssc.wisc.edu/~bhansen/papers/ecnmt_05.html>
- CUDA profiling docs distinguish system timeline analysis from kernel metric analysis, which matters for GPU validation residency claims: <https://docs.nvidia.com/nsight-systems/UserGuide/> and <https://docs.nvidia.com/nsight-compute/NsightComputeCli/index.html>

## Expansion Track A: Validation Evidence Ledger

### Problem

Validation results contain rich data, but the UI and artifacts need a consistent summary of what was actually proven.

### Implementation

Create a derived evidence summary, not a second persisted truth source:

- new: `web/src/utils/validationEvidenceSummary.ts`
- new: `web/src/utils/validationEvidenceSummary.test.ts`
- possible backend helper if needed: `crates/app_core/src/validation_evidence_summary.rs`

Fields:

- validation mode;
- dataset identity;
- candidate source;
- train/holdout split description;
- walk-forward coverage;
- cross-ticker coverage;
- Monte Carlo availability;
- PBO/CSCV availability;
- DSR/PSR availability;
- realized backend;
- fallback reason;
- artifact replay class;
- report state.

### Acceptance Criteria

- Existing artifacts can be summarized without migration.
- Missing fields become `unknown` or `not_available`, not false claims.
- Summary is shown in Advanced validation and compact Simple Mode validation summary.

## Expansion Track B: GPU Validation Route Transparency

### Problem

GPU validation performance/residency is valuable, but route labels alone are not enough evidence.

### Implementation

Current owners:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/commands_validation.rs`
- `web/src/services/validationService.ts`

Add or normalize route fields:

- requested backend;
- resolved backend;
- realized backend;
- fallback reason;
- GPU validation stage coverage;
- CPU tail reason, if any;
- device identity when available;
- timing boundary label;
- residency evidence class.

Do not imply cross-stage optimization-to-validation VRAM persistence. Current target is GPU-executed validation within the validation stage, rerunning backtests as needed.

### Tests

- CPU-only build returns CPU route metadata.
- CUDA build with unsupported route returns explicit fallback.
- GPU validation route metadata survives artifact save/load.
- Frontend shows fallback reason without treating it as failure when fallback is expected.

## Expansion Track C: Walk-Forward And Window Evidence

### Problem

Users need to see whether a validation result is broad or concentrated.

### Implementation

Enhance derived frontend models:

- `web/src/utils/workflowEvidenceModel.ts`
- `web/src/utils/core/stageCompletion.ts`
- `web/src/services/validationService.ts`
- `web/src/components/ValidationWorkspacePanel.vue`

Add:

- per-window summary table;
- window pass/fail/attention status;
- train/holdout coverage chart model;
- low-sample warning;
- regime concentration warning where existing data supports it.

Backend only if required:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_stats.rs`

### Acceptance Criteria

- No validation math changes in UI-only phase.
- New backend fields are optional in serialized artifacts.
- Validation reports include the same window evidence if report shape changes.

## Expansion Track D: Cross-Ticker And Replacement Dataset Semantics

### Problem

Validation can span datasets, tickers, or replacement scopes. These need explicit replay labels.

### Implementation

Use replay classes from `docs/artifact_replay_report_evidence_contract.md`:

- `exact_evidence_review`
- `compatible_recompute`
- `replacement_scoped_validation`
- `review_only_saved_result`
- `blocked`

Current owners:

- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `web/src/services/historyService.ts`
- `web/src/stores/workflowArtifactsStore.ts`

Add:

- reasoned blocked state for incompatible dataset replacement;
- compact matrix of source artifact vs replacement dataset compatibility;
- preserved source identity in reports.

### Tests

- replacement validation cannot silently use stale source;
- blocked replay labels roundtrip;
- report generation does not mutate source artifact except documented report-path update cases.

## Expansion Track E: Simple Mode Validation Recommendation

### Problem

Simple Mode should offer a clear recommendation without exposing every statistical detail.

### Implementation

Current owners:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/services/simpleSummaryService.ts`
- `src-tauri/src/commands_simple_summary.rs`
- `web/src/utils/simpleWorkflowModel.ts`

Add:

- validation status: not run, running, passed, attention, failed, inconclusive;
- reason list limited to high-signal evidence;
- copyable summary line;
- link/transition to Advanced detail.

### Acceptance Criteria

- Simple Mode does not imply live trading readiness.
- Summary names dataset, strategy, candidate, validation mode, and evidence status.
- Advanced route still owns full tables.

## Second-Pass Expansion: Cancellation And SPA Semantics

### Problem

Validation is long-running and can produce partial intermediate state. Cancellation must not be confused with statistical failure, and SPA/data-snooping style evidence must not be implied unless the exact test was run.

### Current Anchors

- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `web/src/services/validationService.ts`
- `web/src/stores/validationStore.ts`
- `web/src/stores/taskStore.ts`
- `web/src/services/runControlService.ts`
- `web/src/utils/core/stageCompletion.ts`

### Implementation

Add validation run state:

- `queued`;
- `preparing`;
- `running_train_windows`;
- `running_holdout_windows`;
- `running_monte_carlo`;
- `saving_artifact`;
- `completed`;
- `cancel_requested`;
- `cancelled`;
- `failed`.

Artifact/report status:

- completed validation artifacts can be replayed normally;
- cancelled validations can keep a diagnostic record only if marked incomplete;
- incomplete records must never be promoted into portfolio feasibility or Simple Mode recommendations;
- cancellation stage and elapsed time should be visible in history.

SPA/data-snooping metadata:

- `many_models_disclosed`: search-space/candidate count available;
- `spa_test_run`: only true when a concrete implemented test ran;
- `pbo_dsr_available`: existing evidence availability;
- `not_run_reason`: unsupported, insufficient data, disabled, legacy artifact.

### Tests

- cancellation before artifact save leaves no completed artifact;
- cancellation after artifact write marks the artifact incomplete or rolls it back;
- Simple Mode treats cancelled validation as `not_completed`, not `failed`;
- report generation blocks incomplete validation artifacts by default.

## Second-Pass Expansion: Monte Carlo And Trade-Bootstrap Detail

### Problem

Monte Carlo and trade-bootstrap evidence can be useful, but the current plan needs more explicit output shape and replay rules.

### Implementation

Add optional detail payloads:

- bootstrap method label;
- sample count;
- seed policy;
- input trade count;
- replacement policy;
- metric distribution summaries;
- percentile cutoffs;
- low-sample warning;
- compatibility with replacement dataset.

Frontend:

- show a compact "distribution evidence" panel in Advanced validation;
- keep raw sample rows out of the default artifact unless specifically exported;
- include only summary stats in Simple Mode.

Reports:

- include route metadata and bootstrap method labels;
- add output parity only after report shape stabilizes;
- never infer Monte Carlo pass/fail when the optional payload is absent.

## Verification

Targeted:

- `npm --prefix web run test -- src/services/validationRequestBuilder.test.ts src/services/validationService.test.ts src/components/ValidationWorkspacePanel.test.ts src/stores/validationStore.actions.test.ts`
- `cargo test -p ta_app_core double_ma_validate`
- `cargo test -p ta_app_core validation`

Artifact/report changes:

- `node scripts/run_output_parity_check.mjs`

GPU validation changes:

- `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`

Broad:

- `powershell -File scripts/run_repo_quality_gate.ps1`
