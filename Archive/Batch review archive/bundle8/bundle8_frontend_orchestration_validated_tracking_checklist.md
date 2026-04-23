# Bundle 8 Validated Tracking Checklist

This is the authoritative execution tracker for Bundle 8:

- `gpt54_feature_review_map.md` Section `## 8. Frontend orchestration and defaults`
- `bundle8_frontend_orchestration_implementation_plan.md`
- `bundle8_frontend_orchestration_plan_addendum.md`
- `bundle8_codex_implementation_prompt.md`

This checklist has already been validated against the current repo state.
It tracks only the repo-valid portions of the older Bundle 8 plan/addendum documents.

Rules:

- Only implement items in the unchecked required sections below.
- Treat the older Bundle 8 plan/addendum prose as background only.
- Do not re-implement items marked `[x]` in the preserve sections.
- Use this file, not the older plan prose, as the progress tracker.

## Validation Completed

- [x] Re-read `gpt54_feature_review_map.md` Section 8.
- [x] Re-read `bundle8_frontend_orchestration_implementation_plan.md`.
- [x] Re-read `bundle8_frontend_orchestration_plan_addendum.md`.
- [x] Compare the Bundle 8 docs against the current optimize orchestration, defaults helper, frontend tests, and command-layer code.
- [x] Identify already-correct route/default surfaces that should be preserved rather than re-implemented.
- [x] Update `bundle8_codex_implementation_prompt.md` so it only directs valid current-repo work.

## Already Present: Preserve, Extend, Do Not Re-implement

- [x] `resolveOptimizationCommand()` routing is already correct and should remain unchanged.
- [x] Rust product-mode normalization in `src-tauri/src/commands_optimize.rs` is already correct and already has baseline tests.
- [x] `DynamicDoubleMa` preview-before-run behavior is already implemented and should be preserved.
- [x] Route/fallback summary text already exists in `optimize.statusLine`; Bundle 8 should preserve it and stop later phases from overwriting it.
- [x] `applyRecommendedOptimizationDefaults()` already exists and is already called from `web/src/main.ts` and `web/src/components/AdvancedWorkspacePanel.vue`.
- [x] Legacy-wrapper-disabled preset coverage already exists in `web/src/services/optimizeService.test.ts`.
- [x] Existing workflow artifact payload shape and analysis refresh wiring remain in scope to preserve, not redesign.
- [x] Quick / Balanced / Deep preset values remain fixed in this bundle.
- [x] `RECOMMENDED_TOP_K` and `RECOMMENDED_HEATMAP_BINS` remain fixed in this bundle.
- [x] Bundle 8 does not own `MlSignal` shell-mode work, Bundle 9 validation-service redesign, or cross-service cancel-domain redesign.

## Required Implementation Scope

### Recommended Defaults Semantics

- [x] Add a pure resolver or equivalent quiet-resolution path for recommended optimize defaults while preserving the existing public mutating helper.
- [x] Keep the existing startup/UI helper call sites behavior-compatible in `web/src/main.ts` and `web/src/components/AdvancedWorkspacePanel.vue`.
- [x] Fix fallback semantics so missing/invalid `topK` repairs to `RECOMMENDED_TOP_K`.
- [x] Fix fallback semantics so blank/missing Grid heatmap bins repair to `RECOMMENDED_HEATMAP_BINS`.
- [x] Preserve explicit `heatmapBins = "0"` / `0` as a valid heatmap-disable choice instead of auto-repairing it.
- [x] Keep non-Grid modes from auto-filling heatmap bins.
- [x] Add or update a short comment clarifying these are fallback defaults, not Quick / Balanced / Deep preset values.
- [x] Update `web/src/services/recommendedOptimizationDefaults.test.ts` so blank-vs-explicit-zero behavior and helper/UI semantics are frozen.

### Optimize Orchestration and Status Semantics

- [x] Gate the legacy DOM bridge behind a legacy-boot check so Vue/Pinia remains the default runtime path.
- [x] Make direct DOM helper writes no-op when the legacy optimize bridge is not booted.
- [x] Add explicit optimize busy-state helpers for command-running, post-run busy, and idle phases.
- [x] Convert `applyPreset()`, `presetKernel()`, and `runGpuParityCheck()` guards from `canCancel`-only checks to real busy checks.
- [x] Add a request-resolution path for `runOptimization()` that validates injected requests and truthfully resolves auto-built requests.
- [x] Add clear injected-request validation for missing `strategy_kind` and missing `data_id`.
- [x] Stop swallowing auto-build errors inside `runOptimization()` and surface the real request-build error message.
- [x] Keep `DynamicDoubleMa` preview/build work counted as request-build time rather than command-completion time.
- [x] Replace misleading `Done in Xms (frontend)` wording with command-boundary wording that matches what is actually being timed.
- [x] Keep `statusLine` as the stable optimization command completion summary plus backend/route/fallback evidence after command success.
- [x] Move phase timing and non-fatal follow-up notes such as validation warnings and `artifact=unsaved` into `progressMeta`.
- [x] Replace unconditional full-result DOM stringify with a compact preview that preserves the key route/result fields users need to see.
- [x] Reuse artifact context instead of rebuilding duplicate dataset/config context in multiple post-run steps.
- [x] Treat thrown `save_run_artifact` failures as unsaved runs without making the run unusable.
- [x] Treat blank or whitespace-only `save_run_artifact` paths as unsaved runs rather than false success.
- [x] Keep workflow artifact sync and analysis refresh active even when artifact save is unsaved/non-fatal.
- [x] Preserve the command-completion `statusLine` when auto-validation fails or is cancelled.
- [x] Switch workflow result tabs only once per optimize run when auto-validation is enabled.
- [x] Apply the same busy-state correctness to parity runs.
- [x] Fix the unsupported-strategy parity message so it matches the currently supported parity strategy kinds.

### Focused Test Coverage

- [x] Add optimize-service coverage for request-build error transparency.
- [x] Add optimize-service coverage proving validation failure does not erase route/fallback summary text.
- [x] Add optimize-service coverage for thrown `save_run_artifact` failures being surfaced as non-fatal unsaved warnings.
- [x] Add optimize-service coverage for blank `save_run_artifact` paths being surfaced as non-fatal unsaved warnings.
- [x] Add optimize-service coverage for command-running vs post-run busy-state transitions.
- [x] Add optimize-service coverage for parity busy-state behavior and corrected unsupported-strategy messaging.
- [x] Add optimize-service coverage for injected-request validation failures.
- [x] Add optimize-service coverage for truthful completion wording and stable `progressMeta`/`statusLine` roles.
- [x] Add optimize-service coverage proving there is only one final tab switch decision when auto-validation is enabled.

### Rust Command-Layer Hygiene

- [x] Add an NVTX scope to `run_composite_cpu_optimization()` matching the current double-MA command profiling pattern.
- [x] Add an NVTX scope to `run_composite_mo_optimization()` matching the current double-MA command profiling pattern.
- [x] Add a Rust unit test proving whitespace-only `export_all_csv_path` does not force Grid mode.

### Verification and Signoff

- [x] Run the narrow Vitest command covering `web/src/services/recommendedOptimizationDefaults.test.ts`.
- [x] Run the narrow Vitest command covering `web/src/services/optimizeService.test.ts`.
- [x] Run the narrow Rust test command covering `src-tauri/src/commands_optimize.rs`.
- [x] Run the closest practical frontend typecheck or compile-equivalent for the touched frontend files.
- [x] Record any intentional deviations from the older Bundle 8 plan/addendum caused by current repo state.

Verification notes:

- `npm --prefix web test -- src/services/recommendedOptimizationDefaults.test.ts src/services/optimizeService.test.ts`
- `npm --prefix web run typecheck`
- `cargo check -p ta_desktop_demo_app --lib`
- `cargo test -p ta_desktop_demo_app --lib commands_optimize -- --nocapture`
- The narrow Rust test command was attempted as required, but it is currently blocked by an unrelated pre-existing lib-test compile error in `src-tauri/src/commands_validation.rs` involving `DoubleMaValidationCandidateResult` field drift. Bundle 8 command-layer code still compiled successfully under `cargo check -p ta_desktop_demo_app --lib`.
- No intentional deviations from the validated Bundle 8 scope were needed beyond adapting the implementation to current repo structure.

## Optional Follow-up Only

- [ ] GPU-status TTL or selective-refresh caching polish, only if it remains additive and low-risk.
- [ ] Broader cancel-domain audit across `runControlService.ts` / `validationService.ts` versus `cancel_current_run` / `cancel_background_task`.

## Final Acceptance

- [x] Command routing remains unchanged for Double-MA, CompositeCpu-family, and CompositeMo requests.
- [x] Rust product-mode normalization semantics remain unchanged apart from the added whitespace-path test.
- [x] `DynamicDoubleMa` still previews before run and uses the normalized preview payload.
- [x] Request-build failures surface their real cause instead of collapsing to the generic load-data message.
- [x] `statusLine` now truthfully reflects optimization command completion rather than a fake full-frontend boundary.
- [x] `progressMeta` carries phase timing plus non-fatal follow-up notes.
- [x] Auto-validation failure/cancel no longer erases backend route/fallback evidence from `statusLine`.
- [x] Artifact save failure is visible and non-fatal, including blank/whitespace save-path responses.
- [x] The optimize hot path no longer does unconditional full-result stringify.
- [x] Busy/cancel semantics are correct for optimize and parity flows.
- [x] Preset values and recommended-default numeric constants remain unchanged.
- [x] Existing startup/UI helper behavior for recommended defaults remains compatible.
