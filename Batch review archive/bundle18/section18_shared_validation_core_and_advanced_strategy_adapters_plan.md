# Section 18 Implementation Plan

## Shared validation core and advanced-strategy adapters

This plan is grounded in the current repo state for Section 18 of `gpt54_feature_review_map.md`.

It keeps the work focused on the remaining validation-architecture gaps that still matter today:

- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `src-tauri/src/commands_validation.rs`
- `web/src/services/validationRequestBuilder.ts`

Supporting evidence and compatibility surfaces:

- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

Older validation-throughput docs are still useful as background, but this plan treats the current code as authoritative:

- `validation_review/validation_throughput_cuda_plan.md`
- `validation_review/validation_throughput_cuda_plan_addendum.md`
- `validation_review/validation_throughput_plan_disposition_matrix.md`

Those documents mostly describe Double-MA validation work that has already landed. Section 18 keeps only the still-open shared-core and advanced-adapter follow-up.

## Current repo-state validation

### Already implemented and not Section 18 open work

- [x] `validation_core.rs` already owns the generic post-candidate validation loop through `run_validation_candidates(...)`.
- [x] `validation_pipeline.rs` already owns shared probability metrics, robust-score computation, hard-gate application, run-verdict building, and reusable walk-forward / reopt / SPA / bootstrap helpers.
- [x] Both `run_double_ma_validation_blocking_with_candles(...)` and `run_composite_validation_blocking_with_candles(...)` already route through `validation_core::run_validation_candidates(...)`.
- [x] Double-MA preserved-finalists cross reruns already keep the verdict-relevant stack through `ValidationAuxWorkMask` and `run_double_ma_validation_blocking_with_candles_with_aux_mask(...)`.
- [x] `commands_validation.rs` already has a shared cross-ticker aggregation loop via `apply_cross_ticker_validation_generic(...)`.
- [x] `validation_common.rs` already has canonical Tauri-side candidate key helpers and normalized cross-dataset dedupe.
- [x] `validationRequestBuilder.ts` already handles candidate dedupe, normalized cross-dataset dedupe, `validation_context`, and truthful Double-MA preserved-finalists summaries.
- [x] `ta_double_ma_validation_cost.rs` and `double_ma_validation_workflow.rs` already provide the primary Double-MA validation timing and route/work-shape evidence.
- [x] `ta_headless_validate.rs` already provides headless product-path parity/report coverage with `validation_context`, preserved-finalists cross mode, and JSON/XLSX export.
- [x] `commands_validation.rs` already has substantial headless/cross-ticker/composite smoke coverage, including preserved-finalists GPU-route coverage and composite cross-ticker smoke tests.
- [x] `composite_validation_workflow.rs` already provides supplementary advanced/composite route/work-shape evidence, including route probes and exact GPU-route assertions for GPU-labeled benches.
- [x] `validationRequestBuilder.test.ts` already covers candidate dedupe, cross-dataset dedupe, preserved-finalists summaries, and current composite lightweight-rerun warning text.
- [x] `commands_reports_validation_common.rs` and `validationService.ts` already mirror the current composite lightweight-rerun summary/warning policy, so they only need touch if backend rerun behavior changes.

### Still-open repo-valid problems

- [ ] Shared validation DTO/config/result types still live in `composite_validate.rs`, and many non-composite surfaces import them from a strategy-specific module.
- [ ] `ValidationAuxWorkMask` is still Double-MA-specific; composite/advanced cross reruns still fall back to an unconditional lightweight rerun shape even when `preserved_finalists` is requested.
- [ ] `composite_validate.rs` still uses raw serialization-based equality/keying (`serialized_eq`, `serialize_composite_candidate_key`, and `serde_json::to_string(...)` finalist keys) in several adapter paths.
- [ ] `composite_validate.rs` still repeats advanced-strategy dispatch ladders across GPU fixed-candidate prep, walk-forward, reopt walk-forward, CSCV, and related rerun paths instead of routing through a shared adapter descriptor.
- [ ] Shared validation ownership is still split awkwardly across `validation_core.rs`, `validation_pipeline.rs`, `double_ma_validate.rs`, and `composite_validate.rs`, which makes advanced-strategy adapter behavior harder to extend without editing the giant composite file directly.
- [ ] `commands_validation.rs`, `ta_headless_validate.rs`, `ta_double_ma_validation_cost.rs`, `double_ma.rs`, `gpu_backtest_kernel.rs`, `meta_select.rs`, `ml_signal.rs`, and `profile_gpu_dynamic_runner.rs` still import shared validation types from `composite_validate.rs`.

### Explicit non-goals for the first implementation wave

- [x] Do not re-open the already-landed Double-MA validation-throughput work from `validation_review/`.
- [x] Do not rewrite strategy-local validation math into one giant generic engine.
- [x] Do not add a brand-new composite/advanced headless CLI unless shared-core changes make it clearly necessary.
- [x] Do not turn this into a frontend redesign; `validationRequestBuilder.ts` should change only if backend preserved-finalists behavior changes.
- [x] Do not widen this into report/XLSX ownership or artifact-persistence work.
- [x] Do not change route policy or capability policy; this section is about shared validation ownership and adapter behavior, not new GPU promises.

## Implementation strategy

### Phase 1: move truly shared validation types and helpers under shared ownership

Goal:

- make the shared validation contract live in the shared validation layer rather than in `composite_validate.rs`

Checklist:

- [ ] Move the shared validation DTO/config/result types out of `composite_validate.rs` into `validation_core.rs` or a dedicated shared validation module that is owned by the shared validation layer.
- [ ] Include the currently shared types used by both Double-MA and composite paths, such as `ValidationArtifactContext`, walk-forward config/stats, reopt walk-forward config/stats, SPA config/stats, perturb config/stats, trade-bootstrap config/stats, and cross-ticker config/summary types.
- [ ] Re-export those shared types from `ta_app_core::lib.rs` with compatibility-preserving names so current callers do not need semantic changes.
- [ ] Update `validation_pipeline.rs`, `double_ma_validate.rs`, `commands_validation.rs`, `ta_headless_validate.rs`, `ta_double_ma_validation_cost.rs`, `double_ma.rs`, `gpu_backtest_kernel.rs`, `meta_select.rs`, `ml_signal.rs`, and any bench/bin importers so they no longer depend on `composite_validate.rs` for shared validation ownership.
- [ ] Keep request/response serde shape stable for Tauri and report consumers.

Notes:

- This is an ownership cleanup, not a schema redesign.
- `CompositeValidationRequest` and `CompositeValidationResult` can remain strategy-local. The shared pieces should move, not the whole composite module.

### Phase 2: add shared canonical identity helpers and eliminate raw serialization identity in composite validation

Goal:

- make advanced/composite validation identity deterministic and parity-safe the same way Double-MA was tightened

Checklist:

- [ ] Add one shared canonical validation key/equality helper in the shared validation layer and reuse it instead of ad hoc `serde_json::to_vec(...)` / `serde_json::to_string(...)` identity in `composite_validate.rs`.
- [ ] Replace `serialized_eq(...)` and `serialize_composite_candidate_key(...)` usage with the shared canonical helper.
- [ ] Replace raw finalist key generation for composite reopt walk-forward and SPA paths with the same canonical helper.
- [ ] Add regression coverage proving logically equal advanced/composite candidates with map-field insertion-order differences do not spuriously fail uniformity checks or fork reopt/SPA identity.

Notes:

- Reuse the existing Tauri-side canonical-key behavior as the semantic model; do not introduce multiple competing notions of validation identity.

### Phase 3: introduce an advanced-strategy adapter seam inside composite validation

Goal:

- stop duplicating the same strategy-kind branching logic across prep and rerun paths

Checklist:

- [ ] Add a lightweight advanced/composite validation adapter descriptor that captures the strategy kind once and routes strategy-local prep / walk-forward / CSCV / reopt behavior through named adapter hooks.
- [ ] Keep strategy-local compute functions where they are; this phase is about central dispatch and shared orchestration, not flattening every strategy into one file.
- [ ] Replace the repeated `if p.asr.is_some() / else if p.mkcs.is_some() / ...` ladders in the main composite validation flow with adapter-driven dispatch.
- [ ] Keep the plain base `CompositeCpu` fixed-candidate path and the advanced-strategy GPU paths behaviorally unchanged while centralizing dispatch.
- [ ] Add regression coverage for adapter selection and for at least one representative strategy from each current advanced group:
  - ASR / ROCB / MKCS / VEMR
  - VCC / VCET / CARM / CDMC
  - KQSR / KVDF

Notes:

- The goal is to make the shared flow extensible without forcing future strategy work through repeated copy-edit ladders in `composite_validate.rs`.

### Phase 4: extend preserved-finalists-style cross reruns to advanced/composite validation

Goal:

- let advanced/composite cross reruns reuse the shared verdict-relevant validation core instead of always degrading to the current lightweight-only shape

Checklist:

- [ ] Move `ValidationAuxWorkMask` out of `double_ma_validate.rs` into the shared validation layer, or add an equivalent shared validation-module mask owned outside the Double-MA module.
- [ ] Add composite/advanced support for a preserved-finalists rerun shape that keeps the verdict-relevant stack and skips only the non-gating aux modules.
- [ ] Keep the current lightweight composite rerun shape as the fallback/default only where the preserved-finalists path is still unsupported.
- [ ] Update `commands_validation.rs` so composite cross-ticker reruns can request the shared preserved-finalists mask when supported instead of always stripping the entire validation stack.
- [ ] Update `validationRequestBuilder.ts` capability/warning summaries only if backend behavior actually changes.
- [ ] Add command-layer tests for advanced/composite preserved-finalists reruns, including route/fallback truthfulness and verdict recomputation after cross filtering.

Notes:

- Non-gating aux modules are the same class of work already masked on Double-MA preserved-finalists reruns: regime/friction-style post-pass analytics, not the hard-gate stack.

### Phase 5: evidence and compatibility cleanup

Goal:

- keep the command/headless/bench surfaces truthful after the shared-core move

Checklist:

- [ ] Update `ta_headless_validate.rs` and `ta_double_ma_validation_cost.rs` imports and type ownership to the shared validation module without changing their timing-boundary semantics.
- [ ] Update `double_ma_validation_workflow.rs` and `composite_validation_workflow.rs` only where shared type ownership or route-summary wording changes require it.
- [ ] Update `commands_reports_validation_common.rs` and `validationService.ts` only if advanced/composite rerun behavior or wording actually changes.
- [ ] Keep `ta_double_ma_validation_cost.rs` as the primary Double-MA timing authority; do not make a new headless composite harness a first-wave requirement.
- [ ] If shared execution-profile or rerun-mode wording changes, update route/work-shape printouts in lockstep so the bench/bin evidence remains truthful.

## File ownership

### Primary implementation files

- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `src-tauri/src/commands_validation.rs`

### Inspect-only or lightly-touched supporting files

- `crates/app_core/src/lib.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/services/validationService.ts`

### Out-of-scope unless evidence forces escalation

- report/XLSX ownership files
- artifact persistence and hydration files
- optimizer request builders
- broad frontend UI changes
- new validation products or strategy families

## Verification checklist

### Behavior and regression coverage

- [ ] Add or update focused `validation_core.rs` tests for shared DTO/helper ownership and canonical identity behavior.
- [ ] Add or update focused `composite_validate.rs` tests for canonical-key uniformity and adapter dispatch.
- [ ] Add or update focused `commands_validation.rs` tests for advanced/composite cross rerun behavior.
- [ ] Update `validationRequestBuilder.test.ts` only if preserved-finalists composite summary behavior changes.
- [ ] Update `commands_reports_validation_common.rs` tests and `validationService.ts` expectations only if advanced/composite rerun wording changes.
- [ ] Preserve or extend Double-MA preserved-finalists tests to confirm no regression from moving the aux-mask ownership.

### Build and bench surfaces

- [ ] `cargo test -p ta_app_core validation_core`
- [ ] `cargo test -p ta_app_core composite_validate`
- [ ] `cargo test -p ta_app_core double_ma_validate`
- [ ] `cargo test -p ta_desktop_demo_app commands_validation`
- [ ] `cargo test -p ta_desktop_demo_app --bin ta_headless_validate` if that binary changes
- [ ] `cargo test -p ta_desktop_demo_app --bin ta_double_ma_validation_cost` if that binary changes
- [ ] `cargo test -p ta_desktop_demo_app commands_reports_validation_common` if report-summary wording changes
- [ ] `cargo bench -p ta_app_core --bench double_ma_validation_workflow --no-run`
- [ ] `cargo bench -p ta_app_core --bench composite_validation_workflow --no-run`
- [ ] `npm --prefix web run test -- --run src/services/validationRequestBuilder.test.ts` if `validationRequestBuilder.ts` or its tests change
- [ ] `npm --prefix web run typecheck` if frontend request-shaping or summary surfaces change

### Performance and workflow evidence

- [ ] Keep `ta_double_ma_validation_cost.rs` as the authority for Double-MA validation timing claims.
- [ ] Use `double_ma_validation_workflow.rs` and `composite_validation_workflow.rs` only as supplementary route/work-shape evidence.
- [ ] Run `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` after shared validation changes land.
- [ ] If the shared validation move touches broader performance-sensitive paths, pair it with `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`.

### Measurement rules

- [ ] Do not run multiple benchmarks at the same time.
- [ ] Do not claim a new advanced/composite timing authority unless a dedicated headless surface is deliberately added.
- [ ] Keep route/fallback wording truthful when rerun behavior changes.

## Acceptance criteria

- [ ] Shared validation DTO/config/result ownership no longer depends on `composite_validate.rs`.
- [ ] Composite/advanced validation no longer relies on raw serialization identity for uniformity or finalist keying.
- [ ] Advanced/composite validation dispatch is centralized enough that new strategy adapters do not require editing multiple repeated ladder sites.
- [ ] Double-MA preserved-finalists behavior remains intact after shared-core extraction.
- [ ] Advanced/composite cross reruns can reuse a verdict-relevant preserved-finalists path where supported, with truthful fallback when still unsupported.
- [ ] Headless, command-layer, and bench surfaces still build and report truthful timing boundaries.
- [ ] No route-policy, capability, or CPU-only build regression is introduced.

## Recommended implementation order

1. Move shared validation-owned types/helpers out of `composite_validate.rs`.
2. Add shared canonical identity helpers and replace raw composite serialization identity.
3. Introduce the advanced/composite adapter seam and remove repeated dispatch ladders.
4. Extend preserved-finalists-style cross reruns to advanced/composite validation.
5. Update command/headless/bench/frontend compatibility surfaces and rerun verification.
