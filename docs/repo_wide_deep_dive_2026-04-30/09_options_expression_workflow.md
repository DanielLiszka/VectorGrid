# Options Expression Workflow Implementation Plan

## Surface

Primary files:

- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/commands_options_data.rs`
- `src-tauri/src/options_parent_adapters.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_types.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/services/optionsExpressionService.ts`
- `web/src/stores/optionsExpressionStore.ts`

## Current Strengths

- Options expression is a useful research lane with explicit provenance and validation replay.
- The workflow separates underlying data, options chain data, objective profiles, and expression evaluation.
- CUDA hooks and research plans exist, but CPU correctness remains the right baseline.
- The UI already treats options as a specialized workflow rather than folding it into simple stock backtests.

## Improvement Plan

### OPTX-01: Per-Window Walk-Forward Metrics

Risk: Low/medium.

Problem:
Anchored and rolling-session walk-forward modes exist today, but the current implementation evaluates the union of validation sessions as a holdout. That is a valid baseline, but users lose visibility into which windows worked and which failed.

Implementation:

- Add durable per-window result structs, for example `OptionsExpressionWalkForwardWindowResultV1` and `OptionsExpressionWalkForwardSummaryV1`.
- Persist per-window option strategy metrics:
  - window start/end
  - train range
  - validation range
  - selected expression tuple
  - objective score
  - realized outcome
  - quote/chain coverage
  - warning flags
- Add frontend table and report export.
- Preserve the existing union-holdout result as the backward-compatible headline result.

Files:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_types.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/services/optionsExpressionService.ts`
- `web/src/components/OptionsExpressionPanel.vue`

Verification:

- `cargo test -p ta_app_core --lib options_expression`
- `cargo test -p ta_desktop_demo_app --lib commands_options_expression`
- `npm --prefix web run test -- src/services/optionsExpressionService.test.ts`

### OPTX-02: Objective Profile CPU Oracle

Risk: Medium.

Problem:
Objective profiles are typed, and the current production route intentionally supports the balanced profile first. Any non-balanced profile should be promoted one at a time only after it has an executable CPU oracle and deterministic expected rankings.

Implementation:

- Define a CPU reference implementation for each objective profile before exposing it beyond research mode.
- Add fixture cases with known expected rankings.
- Require GPU implementations to pass parity against the CPU oracle before exposure.
- Add report fields for objective profile and version.

Files:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_types.rs`
- `src-tauri/src/commands_options_expression.rs`

Verification:

- `cargo test -p ta_app_core options_expression`
- `cargo test -p ta_app_core options_expression_gpu`
- `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1 -Mode candidate`

### OPTX-03: Replay Exactness Classes For Legacy Options Runs

Risk: Low.

Problem:
Options replay provenance already compares content hashes when present and keeps legacy artifacts readable. The improvement is to align options replay labels with the repo-wide artifact exactness classes so old artifacts remain inspectable without claiming exact replay.

Implementation:

- Classify replay as:
  - `exact_evidence_review`
  - `compatible_recompute`
  - `replacement_scoped_validation`
  - `review_only_saved_result`
  - `blocked`
- Show the class in options UI, history, and reports.
- Keep old artifacts readable.

Files:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/services/optionsExpressionService.ts`
- `web/src/components/OptionsExpressionPanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --lib run_artifacts`
- `cargo test -p ta_desktop_demo_app --lib commands_options_expression`
- `npm --prefix web run test -- src/services/optionsExpressionService.test.ts`

### OPTX-04: Family-Local Parent Adapters

Risk: Medium.

Problem:
Options expression should not depend on broad generic execution layers when family-local parent strategy adapters are clearer and safer.

Implementation:

- Extend the existing `src-tauri/src/options_parent_adapters.rs` adapter boundary rather than introducing a broad generic execution layer.
- Explicitly document adapter input/output contracts.
- Add tests for each adapter and reject unsupported parent outputs with structured errors.

Files:

- `crates/app_core/src/options_expression.rs`
- `src-tauri/src/options_parent_adapters.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/services/optionsExpressionService.ts`

Verification:

- `cargo test -p ta_app_core options_expression`
- `cargo test -p ta_desktop_demo_app --lib commands_options_expression`

### OPTX-05: Options Data Preview And Quote Gate

Risk: Low/medium.

Problem:
The options workflow already distinguishes aggregate, derived-mid, and top-of-book quote semantics in imported datasets. The next improvement is to make the preview and provider guidance stricter so quote-dependent objectives cannot be run on data that only looks complete at the aggregate level.

Implementation:

- Add an options data preview that reports:
  - symbol
  - expiration coverage
  - strike coverage
  - aggregate coverage
  - quote coverage if available
  - thin quote warnings
  - rate-limit/provider warnings
- Block quote-dependent objectives when quote coverage is insufficient.
- Persist the preview quality summary into artifacts so reopened runs keep their quote/aggregate context.

Files:

- `src-tauri/src/commands_options_data.rs`
- `src-tauri/src/polygon.rs`
- `web/src/services/optionsExpressionService.ts`
- `web/src/components/OptionsExpressionPanel.vue`

Verification:

- `cargo test -p ta_desktop_demo_app --lib commands_options_data`
- `cargo test -p ta_desktop_demo_app --lib polygon`
- `npm --prefix web run test -- src/services/optionsExpressionService.test.ts src/components/OptionsExpressionPanel.test.ts`

### OPTX-06: Options Data Quality Hard Gates

Risk: Medium.

Problem:
Thin options data can create misleading results if validation accepts it as complete enough. The current code has quote-aware fields and some runtime filters; this guide should harden them into visible gate policy and artifact evidence.

Implementation:

- Add configurable gates for:
  - minimum quote/aggregate coverage
  - maximum missing days
  - minimum contracts per window
  - minimum open interest/volume when available
  - maximum bid/ask spread if quote data exists
- Keep defaults conservative and visible.
- Store gate configuration in artifacts.

Files:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_types.rs`
- `src-tauri/src/commands_options_expression.rs`
- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/services/optionsExpressionService.ts`

Verification:

- `cargo test -p ta_app_core --lib options_expression`
- `cargo test -p ta_desktop_demo_app --lib commands_options_expression`
- `npm --prefix web run test -- src/services/optionsExpressionService.test.ts`

### OPTX-07: GPU Promotion Only With Evidence

Risk: Medium/high.

Problem:
Options workloads may benefit from GPU acceleration, but correctness and data movement can dominate. GPU support should be promoted only when benchmarks prove value.

Implementation:

- Keep CPU oracle as the source of truth.
- Add GPU parity tests for small deterministic options workloads.
- Benchmark candidate kernels with realistic chain sizes.
- Report whether time is spent in transfer, expression evaluation, reduction, or post-processing.
- Expose as research/experimental until stable.

Files:

- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/kernels/*`
- `scripts/run_options_expression_gpu_promotion_gate.ps1`
- `scripts/run_options_expression_phase05.ps1`

Verification:

- `cargo test -p ta_app_core options_expression_gpu`
- `powershell -File scripts/run_options_expression_gpu_promotion_gate.ps1 -Mode candidate`

## Acceptance Criteria For This Guide

- Current code ownership stays in `options_expression.rs`, `options_expression_gpu.rs`, `options_types.rs`, and Tauri options command modules unless a deliberate extraction is performed.
- CPU option-expression scoring remains the correctness oracle for any GPU promotion.
- Walk-forward additions preserve existing union-holdout behavior while adding per-window visibility.
- Options data gates distinguish aggregate, derived mid, and top-of-book quote semantics.
- Legacy options artifacts remain readable but cannot claim exact replay without required content identity.

## Deferred Higher-Risk Work

- Making options expression part of Simple Mode.
- Adding broker execution for generated options strategies.
- Assuming aggregate-only options data is sufficient for quote-sensitive strategies.
