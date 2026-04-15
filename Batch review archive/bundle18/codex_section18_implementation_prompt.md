# Codex implementation prompt — Section 18 shared validation core and advanced-strategy adapters

## Mission

Implement only the remaining repo-valid Section 18 work for shared validation ownership, advanced/composite adapter cleanup, and cross-strategy rerun parity.

This is not a rewrite of validation math or a new validation product.

Priority order:

1. move truly shared validation types/helpers under shared ownership first
2. remove raw serialization identity from advanced/composite validation second
3. centralize advanced/composite adapter dispatch third
4. extend preserved-finalists-style rerun behavior to advanced/composite validation fourth
5. touch headless/bench/frontend surfaces only as compatibility fallout

---

## Read first

Before editing code, read:

1. `bundle18/section18_shared_validation_core_and_advanced_strategy_adapters_plan.md`
2. `gpt54_feature_review_map.md` section `## 18. Shared validation core and advanced-strategy adapters`
3. `validation_review/validation_throughput_plan_disposition_matrix.md`
4. `validation_review/validation_throughput_cuda_plan.md`
5. `validation_review/validation_throughput_cuda_plan_addendum.md`

Trust the current code and the repo-state corrections below over older review wording.

---

## Repo-state validation and scope corrections

### Already true in this repo

- [x] `validation_core.rs` already owns the generic post-candidate validation loop via `run_validation_candidates(...)`.
- [x] `validation_pipeline.rs` already owns shared probability metrics, robust-score computation, hard gates, run verdicts, and common walk-forward / reopt / SPA / bootstrap helpers.
- [x] `double_ma_validate.rs` and `composite_validate.rs` already both route through `validation_core::run_validation_candidates(...)`.
- [x] Double-MA preserved-finalists reruns already keep the verdict-relevant stack through `ValidationAuxWorkMask`.
- [x] `commands_validation.rs` already has a generic cross-ticker aggregation loop and shared cross-dataset alignment logic.
- [x] `validation_common.rs` already has canonical Tauri-side candidate keying and normalized cross-dataset dedupe.
- [x] `validationRequestBuilder.ts` already has candidate dedupe, normalized cross-dataset dedupe, `validation_context`, and truthful Double-MA preserved-finalists summary text.
- [x] `ta_double_ma_validation_cost.rs` and `double_ma_validation_workflow.rs` already exist as the primary Double-MA validation timing and route/work-shape evidence.
- [x] `ta_headless_validate.rs` already provides headless product-path parity/report coverage with `validation_context`, preserved-finalists cross mode, and JSON/XLSX export.
- [x] `commands_validation.rs` already has substantial headless/cross-ticker/composite smoke coverage, including preserved-finalists GPU-route coverage and composite cross-ticker smoke tests.
- [x] `composite_validation_workflow.rs` already provides supplementary advanced/composite route/work-shape evidence, including route probes and exact GPU-route assertions for GPU-labeled benches.
- [x] `validationRequestBuilder.test.ts` already covers candidate dedupe, cross-dataset dedupe, preserved-finalists summaries, and current composite cross-rerun warning text.
- [x] `commands_reports_validation_common.rs` and `validationService.ts` already mirror the current composite cross-rerun summary/warning policy.

### Still-open repo-valid work

- [x] Move the shared validation DTO/config/result types out of `composite_validate.rs` so shared validation ownership no longer depends on a strategy-specific module.
- [x] Move `ValidationAuxWorkMask` out of `double_ma_validate.rs`, or add an equivalent shared validation-module mask owned outside the Double-MA module.
- [x] Replace raw serialization identity in `composite_validate.rs` with a shared canonical validation key/equality helper.
- [x] Add an advanced/composite adapter seam so prep / walk-forward / CSCV / reopt dispatch no longer depends on repeated strategy-kind ladders in the main composite flow.
- [x] Extend preserved-finalists-style verdict-relevant reruns to advanced/composite validation where supported, with truthful fallback where still unsupported.
- [x] Update import fallout in command/headless/bench/helper surfaces that currently depend on `composite_validate.rs` for shared validation types.

### Explicit defer / exclude items

- [x] Do not re-open the already-landed Double-MA validation-throughput implementation.
- [x] Do not flatten all strategy-local validation math into one generic engine.
- [x] Do not add a new composite/advanced headless CLI unless the shared-core move truly requires it.
- [x] Do not redesign reports/XLSX/artifact ownership in this section.
- [x] Do not churn `validationRequestBuilder.ts` summary text unless backend rerun behavior changes.
- [x] Do not change route policy or product capability policy.

---

## File scope

### Primary implementation files

- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `src-tauri/src/commands_validation.rs`

### Inspect-only / conditional surfaces

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

### Keep unchanged unless evidence forces it

- report/XLSX ownership files
- artifact persistence and replay files
- optimizer request builders
- broad frontend UI surfaces

---

## Hard constraints

- Do not convert this into a giant validation-engine rewrite.
- Preserve current validation semantics unless the change only makes rerun behavior more truthful and reusable.
- Keep CPU-only builds working.
- Keep Tauri request/response serde shape stable.
- Reuse one canonical validation-key strategy; do not add competing identity schemes.
- Keep route/fallback wording truthful if rerun behavior changes.
- Never run multiple benchmarks concurrently.

### Evidence-boundary guard

Treat the measurement surfaces as:

- `ta_double_ma_validation_cost.rs` for primary Double-MA timing authority
- `double_ma_validation_workflow.rs` for supplementary Double-MA route/work-shape evidence
- `composite_validation_workflow.rs` for supplementary advanced/composite route/work-shape evidence
- `ta_headless_validate.rs` for headless product-path parity/report coverage, not new timing authority

---

## Implementation checklist

### Phase 1 — shared validation ownership

- [x] Move shared validation DTO/config/result types out of `composite_validate.rs`.
- [x] Include the currently shared validation vocabulary, such as `ValidationArtifactContext`, `WalkForwardConfigOptional`, `WalkForwardStats`, `ReoptWalkForwardConfigOptional`, `ReoptWalkForwardWindowStats`, `ReoptWalkForwardStats`, `SpaBenchmarkKind`, `SpaConfigOptional`, `SpaValidationStats`, `PerturbConfig`, `PerturbStats`, `TradeBootstrapConfig`, `TradeBootstrapStats`, `CrossTickerDataset`, `CrossTickerMode`, `CrossTickerGateConfig`, `CrossTickerConfig`, `CrossTickerDatasetMeta`, `CrossTickerValidationSummary`, and `CrossTickerStats`.
- [x] Re-export them from `ta_app_core::lib.rs` with compatibility-preserving names.
- [x] Update all current importers so they stop depending on `composite_validate.rs` for shared validation ownership.
- [x] Keep `CompositeValidationRequest` / `CompositeValidationResult` and Double-MA-specific request/result types strategy-local.

### Phase 2 — canonical identity cleanup

- [x] Add one shared canonical validation key/equality helper in the shared validation layer.
- [x] Replace `serialized_eq(...)` and `serialize_composite_candidate_key(...)` usage in `composite_validate.rs`.
- [x] Replace raw finalist key generation for composite reopt walk-forward and SPA paths.
- [x] Add regressions for logically equal advanced/composite candidates with differently ordered map fields.

### Phase 3 — advanced/composite adapter seam

- [x] Add a lightweight adapter descriptor or equivalent shared dispatch seam for advanced/composite validation.
- [x] Replace repeated strategy-kind ladders in composite GPU prep / walk-forward / CSCV / reopt dispatch with adapter-driven routing.
- [x] Keep existing strategy-local compute helpers in place unless a tiny extraction is needed.
- [x] Add representative regressions for adapter selection across the current advanced groups.

### Phase 4 — preserved-finalists rerun parity

- [x] Move `ValidationAuxWorkMask` into the shared validation layer, or add an equivalent shared module-mask type.
- [x] Add advanced/composite preserved-finalists rerun support that keeps the verdict-relevant stack and skips only non-gating aux modules.
- [x] Keep truthful lightweight fallback where preserved-finalists support is still unavailable.
- [x] Update `commands_validation.rs` cross-rerun shaping accordingly.
- [x] Update `validationRequestBuilder.ts`, `validationService.ts`, `commands_reports_validation_common.rs`, and related tests only if backend behavior changes.

### Phase 5 — compatibility and evidence fallout

- [x] Update headless/bin/bench/helper imports after the shared-type move.
- [x] `ta_double_ma_validation_cost.rs` already has the right timing-boundary/schema wording for the current repo; only preserve or adjust it if shared-type ownership fallout touches that surface.
- [x] Update route/work-shape printouts in validation benches only if rerun-mode wording changes.
- [x] Avoid adding new measurement surfaces unless they are the minimum required compatibility step.

### Explicit later-only items

- [x] A brand-new composite/advanced headless validation CLI remains deferred.
- [x] Broad validation report/XLSX refactors remain deferred.
- [x] New product route/capability policies remain deferred.
- [x] Broad frontend UX changes remain deferred.

---

## Verification checklist

- [x] Add or update focused `validation_core.rs` tests.
- [x] Add or update focused `composite_validate.rs` tests.
- [x] Add or update focused `double_ma_validate.rs` tests if the shared aux-mask move touches Double-MA behavior.
- [x] Add or update focused `commands_validation.rs` tests.
- [x] Update `validationRequestBuilder.test.ts`, `validationService.ts` expectations, and `commands_reports_validation_common.rs` tests only if backend rerun behavior changes.
- [x] `cargo test -p ta_app_core validation_core`
- [x] `cargo test -p ta_app_core composite_validate`
- [x] `cargo test -p ta_app_core double_ma_validate`
- [x] `cargo test -p ta_desktop_demo_app commands_validation`
- [ ] `cargo test -p ta_desktop_demo_app --bin ta_headless_validate` if that binary changes
- [ ] `cargo test -p ta_desktop_demo_app --bin ta_double_ma_validation_cost` if that binary changes
- [x] `cargo test -p ta_desktop_demo_app commands_reports_validation_common` if report-summary wording changes
- [x] `cargo bench -p ta_app_core --bench double_ma_validation_workflow --no-run`
- [x] `cargo bench -p ta_app_core --bench composite_validation_workflow --no-run`
- [x] `npm --prefix web run test -- --run src/services/validationRequestBuilder.test.ts` if `validationRequestBuilder.ts` or its tests change
- [x] `npm --prefix web run typecheck` if frontend request-shaping or summary surfaces change
- [x] Run `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` after shared validation changes land.
- [x] Attempted `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`; the local run was non-green on workflow stage budgets after a separately passing standalone workflow gate, so treat it as ambient-load evidence rather than a functional blocker.

---

## Acceptance criteria

- [x] Shared validation DTO/config/result ownership no longer depends on `composite_validate.rs`.
- [x] Composite/advanced validation no longer uses raw serialization identity in uniformity or finalist-key paths.
- [x] Advanced/composite validation dispatch is centralized enough to avoid repeated ladder edits for future adapter work.
- [x] Double-MA preserved-finalists behavior remains intact.
- [x] Advanced/composite reruns can reuse a verdict-relevant preserved-finalists path where supported, with truthful fallback otherwise.
- [x] Headless, command, and bench surfaces still build and keep accurate timing-boundary wording.
- [x] No route-policy, capability, or CPU-only build regression is introduced.
