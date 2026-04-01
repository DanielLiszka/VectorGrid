# Codex implementation prompt: Bundle 8 frontend orchestration and defaults

You are implementing **bundle 8: frontend orchestration and defaults** in this repo.

## Read these documents first, then follow the validated scope in this prompt

1. `gpt54_feature_review_map.md`  
   Focus specifically on the section: **`## 8. Frontend orchestration and defaults`**
2. `bundle8_frontend_orchestration_implementation_plan.md`
3. `bundle8_frontend_orchestration_plan_addendum.md`

If these docs are attached in the workspace, their likely paths are:

- `/mnt/data/gpt54_feature_review_map.md`
- `/mnt/data/bundle8_frontend_orchestration_implementation_plan.md`
- `/mnt/data/bundle8_frontend_orchestration_plan_addendum.md`

Read them fully before editing code.  
Use the review-map section as the **scope guard** and the two bundle-8 markdown documents as **background implementation guidance only**.  
When the current repo state disagrees with the older bundle-8 docs, the validated scope and checklist in this prompt win.

Use `bundle8_frontend_orchestration_validated_tracking_checklist.md` as the authoritative progress tracker for this bundle.

## Repo-state validation findings

- [x] `resolveOptimizationCommand()` routing is already correct and should be preserved, not reworked.
- [x] Rust product-mode normalization is already correct and already has baseline tests.
- [x] `DynamicDoubleMa` preview-before-run is already implemented and should be preserved, not reintroduced.
- [x] backend/route/fallback summary text already exists in `statusLine`; Bundle 8 should preserve it and stop later phases from overwriting it.
- [x] `applyRecommendedOptimizationDefaults()` already exists and is already used at app boot and Advanced panel mount; preserve the public helper and existing call sites unless a tightly-coupled compatibility fix is truly required.
- [x] legacy-wrapper-disabled preset coverage already exists in the optimize service tests; extend focused coverage instead of rebuilding the entire suite around that case.

## Authoritative validated checklist

Only the unchecked items below are required Bundle-8 implementation scope. Any later detail in this prompt that is not represented by an unchecked checklist item is background-only and must not be treated as required work.

### Scope locks

- [x] Keep Quick / Balanced / Deep numeric values unchanged in this bundle.
- [x] Do not retune `RECOMMENDED_TOP_K` / `RECOMMENDED_HEATMAP_BINS` or claim they are benchmark-optimal in this bundle.
- [x] Do not redesign workflow artifact payload shape, replay shape, or analysis hydration in this bundle.
- [x] Do not broaden this prompt into `MlSignal` shell-mode work, Bundle 9 validation-service redesign, or cross-service cancel-domain redesign.
- [x] Preserve current command routing, Rust product normalization, `DynamicDoubleMa` preview-first behavior, route-summary formatting, and startup/UI uses of the recommended-default helper.

### Required implementation scope

- [ ] `web/src/services/recommendedOptimizationDefaults.ts`: fix fallback semantics so blank/missing Grid heatmap bins repair to `64`, explicit `heatmapBins = "0"` remains a valid disable choice, and the optimize run path does not call the noisy mutating helper directly.
- [ ] `web/src/services/recommendedOptimizationDefaults.test.ts`: add/update focused tests for blank-vs-explicit-zero heatmap behavior while preserving current helper/UI semantics.
- [ ] `web/src/services/optimizeService.ts`: gate the legacy DOM bridge behind a legacy-boot check so Vue/Pinia remains the default runtime path.
- [ ] `web/src/services/optimizeService.ts`: replace swallow-and-null auto-build behavior with truthful request-resolution and clear injected-request validation.
- [ ] `web/src/services/optimizeService.ts`: split command-running vs post-run busy state, convert `canCancel`-only guards to real busy checks, and apply the same busy correctness to parity.
- [ ] `web/src/services/optimizeService.ts`: make `statusLine` the stable optimization command completion summary, move phase timings and non-fatal warnings to `progressMeta`, and remove misleading `(frontend)` wording.
- [ ] `web/src/services/optimizeService.ts`: replace unconditional full-result stringify with a compact preview and reuse artifact context so the post-run path does less unnecessary work.
- [ ] `web/src/services/optimizeService.ts`: treat thrown or blank `save_run_artifact` results as unsaved, keep the run usable, surface the warning non-fatally, and avoid double tab switching when auto-validation is enabled.
- [ ] `web/src/services/optimizeService.test.ts`: add focused coverage for request-build error transparency, preserved route summary on validation failure, unsaved artifact warnings, busy-phase semantics, parity message/busy behavior, injected-request validation, truthful completion wording, and single final tab selection.
- [ ] `src-tauri/src/commands_optimize.rs`: keep route and normalization semantics unchanged, but add the minimal remaining profiling/test hygiene (`run_composite_cpu_optimization` and `run_composite_mo_optimization` NVTX scopes plus a whitespace-only export-path normalization test).

### Optional follow-up only

- [ ] GPU-status TTL or selective-refresh caching polish.
- [ ] Cancel-domain audit across `runControlService.ts` / `validationService.ts` versus `cancel_current_run` / `cancel_background_task`.

## Primary repo files to modify

Modify these files as needed to implement the guides:

- `web/src/services/optimizeService.ts`
- `web/src/services/optimizeService.test.ts`
- `web/src/services/recommendedOptimizationDefaults.ts`
- `web/src/services/recommendedOptimizationDefaults.test.ts`
- `src-tauri/src/commands_optimize.rs`

## Adjacent files you may inspect for compatibility, but do not expand scope into them unless required

Read-only unless needed for compile/test fixes or tightly-coupled compatibility:

- `web/src/main.ts`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/services/runControlService.ts`
- `web/src/services/validationService.ts`
- `web/src/services/analysisService.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/stores/workflowStore.ts`
- any directly-imported store/service/type helpers required to keep the above primary files correct

If you must touch any adjacent file, keep the change minimal and explain why.

---

## Mission

Fully implement the bundle-8 guides with priority on:

- **performance**
- **optimization**
- **correctness**
- **truthful timing/status reporting**
- **no feature removal**
- **no route or parity regressions**
- **no broad redesign**

This is not a feature-ideation task. It is an implementation task.

---

## Non-negotiable constraints

1. **Do not change command routing semantics unless required by a clear bug.**
   - Preserve the current `resolveOptimizationCommand()` behavior.
   - `CompositeMo` must continue to route to `run_composite_mo_optimization`.
   - The standalone composite CPU family must continue to route to `run_composite_cpu_optimization`.
   - The Double-MA family must continue to route to `run_double_ma_optimization`.

2. **Do not change the Rust product-mode normalization semantics except for adding tests/profiling hygiene.**
   - Preserve `normalize_product_double_ma_optimization_mode()`.
   - Preserve `normalize_product_composite_cpu_optimization_mode()` behavior.

3. **Do not remove DynamicDoubleMa preview-before-run.**
   - Preserve preview-first behavior and continue to build the runtime request from the normalized preview payload.

4. **Do not remove or degrade route/fallback transparency.**
   - Keep backend route/fallback info visible to the user.
   - After command success, route/fallback evidence must remain preserved.

5. **Do not remove artifact persistence, workflow artifact sync, or analysis refresh.**
   - You may make them cheaper / more truthful / more robust.
   - You may surface save failure more clearly.
   - You must not remove the feature.

6. **Do not slim or redesign the stored run/result artifact shape in this task.**
   - Keep `run: { req, res }` in the artifact payload.
   - Keep the workflow artifact store behavior compatible.
   - Do not attempt a broader replay/analysis refactor in this pass.

7. **Do not change Quick / Balanced / Deep numeric preset values in this task.**
   - The docs explicitly say the semantics should be fixed first.
   - Numeric re-tuning requires bundle-7 benchmark evidence and is out of scope here.

8. **Do not remove the legacy DOM bridge entirely.**
   - Gate it so Vue/Pinia is the default path.
   - Keep legacy compatibility available when the legacy runtime is actually booted.

9. **Prefer small, parity-safe changes over broad refactors.**

10. **Use the code snippets in the two bundle-8 markdown documents as strong implementation guidance only for items that survive the validated checklist above.**
    - Adapt signatures/types to the actual repo state if needed.
    - Preserve intent even if exact snippet text needs adjustment.

---

## High-level outcome you need to achieve

Implement the bundle-8 plan so that:

- the optimize hot path does less unnecessary synchronous work
- timing/status text is truthful about the boundary being reported
- request-build failures surface their real cause
- artifact save failure is visible but non-fatal
- `statusLine` remains the stable command-completion summary + route/fallback evidence
- `progressMeta` becomes the place for full phase timing and non-fatal warnings
- busy/cancel semantics are correct and consistent between optimize and parity flows
- recommended defaults become safe fallback repair logic rather than noisy hot-path mutation
- profiling consistency improves in the Rust command layer

Use this behavioral contract unless there is a strong local reason not to:

- `statusLine` = stable optimization command completion summary + backend/route/fallback evidence
- `progressMeta` = phase timing summary + non-fatal follow-up notes such as validation warnings or `artifact=unsaved`

---

## Required implementation work by file

The validated checklist above is authoritative. The detailed file-by-file sections below are background guidance and should be interpreted through that checklist rather than treated as an independent source of required work.

# 1) `web/src/services/recommendedOptimizationDefaults.ts`

Implement the guide from the original plan **plus** the addendum refinement.

### Required changes

1. Implement semantics-correct fallback repair.
   - A **pure resolver** plus **optional mutating UI helper** split is the preferred structure.
   - A very close equivalent is acceptable if it preserves current startup/UI helper behavior and gives `runOptimization()` a quiet repair path.

2. Preserve the existing public repair capability, but make the optimize hot path use a **quiet / pure resolution** rather than a noisy status-mutating helper.

3. Recommended structure:
   - `parseOptionalNonNegativeInt(value): number | null`
   - `modeSupportsRecommendedGridHeatmap(value): boolean`
   - `RecommendedOptimizationDefaultsResolution` interface/type
   - `resolveRecommendedOptimizationDefaults({ topK, heatmapBins, mode })`
   - `applyRecommendedOptimizationDefaults()` remains as the mutating UI helper

### Required semantics

- `topK`
  - blank / null / NaN / negative / zero => treat as invalid/missing and repair to `RECOMMENDED_TOP_K`
- `heatmapBins`
  - blank / null / missing in `Grid` mode => repair to `RECOMMENDED_HEATMAP_BINS`
  - explicit `"0"` or numeric `0` => preserve as a valid “disable heatmap” choice
  - non-Grid mode => do not auto-fill heatmap bins

### Required behavior notes

- Keep the mutating helper updating:
  - `advancedStore.topK`
  - `advancedStore.heatmapBins`
  - `advancedStore.statusLine`
  - `advancedStore.source`
  - `advancedStore.updatedAtUnixMs`
- But **do not** call this noisy UI helper directly from the optimize hot path.
- Instead, export and use the pure resolver or a quiet wrapper from `optimizeService.ts`.

### Add a small comment
Add a short comment clarifying that these are **fallback defaults for missing/invalid values**, not the same as Quick / Balanced / Deep preset profiles.

---

# 2) `web/src/services/optimizeService.ts`

This is the main implementation target.

## A. Legacy DOM bridge gating

### Required changes

Add a helper like:

- `legacyOptimizeDomBridgeEnabled()`

The helper should return `true` only when the legacy runtime bridge is actually booted, e.g. via:

- `window.__VTA_LEGACY_BOOTED__`

Then gate these helpers so they no-op when legacy is not booted:

- `writeDomInputValue()`
- `writeDomCheckboxValue()`
- `writeDomTextContent()`

### Intent

Vue/Pinia should be the default runtime path.  
The direct DOM bridge should remain compatibility-only.

Do **not** remove legacy support. Gate it.

---

## B. Busy/cancel lifecycle helpers

### Required changes

Introduce a more explicit busy-state model inside `optimizeService.ts`, using the docs’ approach.

Add helpers/types such as:

- `OptimizeBusyState`
- `setOptimizeBusyState(...)`
- `markOptimizationCommandRunning()`
- `markOptimizationPostRunBusy()`
- `markNonCancelableBusy()`
- `markOptimizeIdle()`
- `isOptimizeBusy()`

### Required semantics

During the backend optimization command:
- `runDisabled = true`
- `validateDisabled = true`
- `canCancel = true`

During post-run follow-up after the command returns:
- `runDisabled = true`
- `validateDisabled = true`
- `canCancel = false`

When idle:
- all three false

### Required guard updates

Replace early guards that currently only check `canCancel` with real busy checks where appropriate, including at least:

- `applyPreset()`
- `presetKernel()`
- `runGpuParityCheck()`

### Intent

Do not let:
- presets
- parity
- other optimize launches

start while the service is still busy, even if the command is no longer cancelable.

---

## C. Timing helpers and truthful phase reporting

### Required changes

Add local helpers/types similar to the guide:

- `RunPhaseTimings`
- `nowMs()`
- `elapsedMs(startedAt)`
- `appendProgressNote(base, note)`
- `buildCommandCompletionStatus(commandMs, backendSummary)`
- `buildRunPhaseSummary(strategyKind, timings)`

Track at least these timings:

- request build
- optimize command
- artifact persistence
- analysis refresh
- validation
- GPU status refresh
- total

### Required status semantics

- `statusLine` must describe the **optimization command completion boundary**, not false “full frontend done” time
- `progressMeta` must contain the phase timing breakdown and non-fatal notes

### Required wording change

Remove the current misleading wording of the form:

- `Done in Xms (frontend)`

Replace it with wording that clearly reflects command time only, e.g. “Optimization command completed in Xms …” or equivalent.

---

## D. Replace unconditional full-result DOM stringify with a compact preview

### Required changes

Do not keep unconditional, full raw DOM stringify on the hot path.

Add:

- `RAW_OUTPUT_PREVIEW_CHARS`
- truncating `safePrettyJson(value, maxChars)`
- `buildOptimizationOutputPreview(value)`

The output preview should preserve the most important information such as:

- `best_params`
- `best_metrics`
- `backend_used`
- `backend_route`
- `backend_fallback_reason`
- `indicator_route_plan`
- any other route-related fields that are useful

### Intent

Keep the visible output useful while reducing hot-path serialization cost.

Do **not** redesign artifact storage around this.  
This is only for the user-visible output preview path.

---

## E. Request resolution and error transparency

### Required changes

Add a request resolver for `runOptimization()` that handles both injected requests and auto-built requests cleanly.

Recommended structure:

- `assertInjectedOptimizationRequestShape(req)`
- `resolveRunOptimizationRequest(options): Promise<{ req; requestBuildMs }>`

### Required semantics

1. **Injected requests**
   - validate they include `strategy_kind` (or compatible equivalent)
   - validate they include `data_id`
   - throw a clear error if either is missing
   - do **not** mutate advanced defaults for injected requests

2. **Auto-built requests**
   - perform quiet recommended-default repair first
   - call `buildOptimizationRequestFromVueStores()`
   - measure true request-build time
   - do **not** swallow thrown errors

3. **Error surfacing**
   - if request build fails, surface the real error message in `optimize.statusLine`
   - do not collapse everything into the generic “Load data first…” message unless there truly is no better message

### Critical requirement

Stop using the current swallow-and-null behavior around `tryBuildAutoOptimizationRequest()`.

The guides explicitly require preserving real build errors, especially for DynamicDoubleMa preview/build failures.

---

## F. Artifact context reuse and explicit persistence status

### Required changes

Implement the artifact-context reuse pattern from the guide.

Recommended structure:

- `RunArtifactContext`
- `PersistRunArtifactResult`
- `buildRunArtifactContext(req)`
- updated `buildRunArtifactPayload(req, res, ts, context)`
- updated `persistRunArtifact(req, res, ts, context)`
- updated `applyVueRunArtifacts(ts, req, res, persisted)`

### Required behavior

1. Compute these once and reuse them:
   - `csvPath`
   - `datasetMeta`
   - `configFingerprint`

2. Treat both of these as unsaved artifact cases:
   - thrown `save_run_artifact` error
   - successful response with blank / empty / whitespace-only `path`

3. Continue to:
   - update workflow artifacts
   - refresh analysis
   - keep the run usable in memory

4. Surface save failure as a non-fatal warning in `progressMeta`, for example:
   - `artifact=unsaved`
   - optionally include the error detail if safe

### Critical requirement

Do **not** falsely treat `{ path: "" }` or `{}` as a successful saved artifact.

---

## G. Single final tab switch

### Required changes

Avoid unnecessary tab churn in `runOptimization()` when auto-validation is enabled.

Current problem from the guide:
- switching to `top`
- then later switching again to `overview`

### Required behavior

Choose the final results tab once after validation outcome is known.

Do not introduce extra tab churn.

If you need to preserve existing UX semantics, do it with one final decision rather than two sequential switches.

---

## H. Rewrite `runOptimization()` around the new lifecycle

Rework `runOptimization()` so it follows the docs’ intent.

### Required flow

1. Resolve request via the new request resolver
2. Measure request-build time
3. Resolve command from `strategy_kind`
4. Build artifact context
5. Mark command-running busy state
6. Set initial status/progress text for the running command
7. Invoke the optimize command and measure command time
8. Immediately after command returns:
   - mark post-run busy (non-cancelable)
   - compute backend summary
   - write stable completion `statusLine`
   - update preview output
9. Persist artifact and measure persistence time
10. Apply workflow artifacts and refresh analysis
11. Handle optional validation and measure validation time
12. Handle GPU status refresh (or a low-risk cached/selective equivalent if implemented)
13. Build final `progressMeta` from phase timings + non-fatal notes
14. Return to idle state in `finally`

### Required semantics

- After the optimize command succeeds, `statusLine` must remain the command completion summary plus route/fallback evidence.
- Validation failure/cancel must **not** overwrite that route summary.
- Validation status should go into `progressMeta`, not replace the stable `statusLine`.
- Artifact save warnings should also go into `progressMeta`.

### Error behavior

- Optimization command cancellation/error itself should still behave as a real command failure/cancel path.
- Post-run non-fatal follow-up should not erase successful command completion status.

### Explicit rule from the docs

Use this contract unless local constraints force a very close equivalent:

- `statusLine` = stable command completion summary + route/fallback evidence
- `progressMeta` = full phase timing summary + non-fatal warnings

---

## I. Preserve DynamicDoubleMa behavior, but count preview as build time

Do **not** change the preview-before-run contract.

What you should change is the accounting:

- DynamicDoubleMa preview and normalized request build belong to **request-build time**, not command time

Keep the existing normalized-preview request construction behavior intact.

---

## J. GPU status refresh and parity orchestration

### Required changes

Apply the same busy-state correctness to parity.

`runGpuParityCheck()` should:

- refuse to start if optimize is already busy
- set a non-cancelable busy state while parity is running
- restore idle state afterward

### Required message fix

Fix the unsupported-strategy message so it correctly reflects the actual supported parity kinds currently handled in code:

- `DoubleMa`
- `Pit`
- `Set`
- `Mts`
- `Orc`
- `Marrt`
- `Vfct`

Do not keep the narrower, misleading text from before.

### Optional warm-path improvement

A short TTL cache or selective refresh policy for GPU status is acceptable **only if it is low risk and easy to test**.

If you implement it:
- keep the code simple
- avoid brittle timing tests
- do not allow stale cache behavior to break correctness

If it feels risky or noisy, leave it out and keep the behavior correct.

---

## K. Leave these parts unchanged unless a real bug forces a change

Do **not** change in this pass:

- `resolveOptimizationCommand()` mapping
- Double-MA routing behavior
- Rust normalization behavior
- DynamicDoubleMa preview-before-run semantics
- Quick / Balanced / Deep preset numeric values
- artifact shape beyond explicit unsaved signaling
- backend result contracts beyond lightweight profiling hygiene

---

# 3) `src-tauri/src/commands_optimize.rs`

This file only needs focused, low-risk changes.

## Required changes

1. Add NVTX scopes to the blocking command closures for:
   - `run_composite_cpu_optimization()`
   - `run_composite_mo_optimization()`

Mirror the double-MA path’s profiling hygiene as closely as possible.

2. Add a Rust unit test for whitespace-only export paths in product-mode normalization:
   - whitespace-only `export_all_csv_path` should **not** force Grid mode
   - preserve current semantics for non-empty path / include_all / heatmap_bins

## Do not change

- route selection behavior
- normalization semantics other than adding the whitespace test if needed for clarity
- cancellation semantics in this file

This file is for profiling consistency and a small normalization test only.

---

# 4) `web/src/services/optimizeService.test.ts`

Extend the tests so the new behavior is frozen.

## Required test coverage

### A. Request-build error transparency
Add a test showing that a real request-build error is surfaced, especially for auto-built requests / DynamicDoubleMa preview failure.

The optimize status should reflect the real failure message, not the generic load-data message.

### B. Validation failure does not erase route summary
Add a test where:
- optimization command succeeds with backend/route/fallback metadata
- auto-validation fails

Expected result:
- `statusLine` still contains the backend/route/fallback summary
- validation failure/cancel is recorded in `progressMeta` instead

### C. Artifact save warning behavior
Add tests for both:
- thrown `save_run_artifact` failure
- blank save path response

Expected result:
- optimize completion still succeeds
- analysis still refreshes
- `progressMeta` or equivalent non-fatal status clearly shows unsaved artifact warning

### D. Optimize busy-state phase behavior
Add a test proving:
- `canCancel = true` only while the optimize command is in flight
- after the command returns, post-run work remains busy but no longer cancelable

You may need a deferred-promise test shape to verify the transition.

### E. Parity busy-state behavior
Add a test that parity:
- enters a busy state
- does not expose stale cancel state
- restores idle state after completion

### F. Legacy DOM bridge disabled behavior
Preserve and/or expand coverage showing the service works when:

- `window.__VTA_LEGACY_BOOTED__ = false`

The new DOM-bridge gating must not break current Vue-store-driven behavior.

### G. Recommended defaults behavior
Add focused tests for:
- invalid/missing `topK` -> auto-fill `50`
- blank Grid heatmap bins -> auto-fill `64`
- explicit `heatmapBins = "0"` -> preserved
- non-Grid mode -> no automatic heatmap fill

You can place these either:
- in `optimizeService.test.ts`, or
- in a new `recommendedOptimizationDefaults.test.ts`

Choose the least disruptive option for this repo.

### H. Injected request validation
Add a test showing that an injected request missing:
- `strategy_kind`, or
- `data_id`

fails with a clear error instead of silently misrouting.

### I. Completion wording expectations
Update/add tests so the completion wording no longer depends on the misleading `(frontend)` label.

The test should reflect:
- truthful command-completion wording
- preserved route summary

### J. Single final tab switch
If you implement the tab-switch optimization, add a focused test/spies proving that the run does not churn through multiple unnecessary tab switches.

### K. GPU status cache
Only add tests for GPU status caching if you implement the cache **and** can test it stably without brittle wall-clock timing.

Do not add flaky cache tests.

---

## Implementation order you should follow

### Phase 0: read and map
- Read the three docs listed at the top
- Inspect the four primary files
- Understand current tests before editing

### Phase 1: tests for the new cases
Add the highest-value missing tests first:
- request-build error transparency
- validation failure preserving route summary
- unsaved artifact warning
- injected request validation
- defaults behavior

### Phase 2: defaults hardening
Implement the pure resolver + mutating wrapper pattern in `recommendedOptimizationDefaults.ts`.

Then wire a quiet/default-repair path into `optimizeService.ts` for auto-built requests only, without changing the existing startup/UI helper call sites unless a tightly-coupled compatibility fix forces it.

### Phase 3: optimizeService orchestration rewrite
Implement:
- DOM bridge gating
- busy helpers
- request resolver
- output preview
- artifact context reuse
- stable `statusLine` + richer `progressMeta`
- post-run non-cancelable state
- single final tab switch

### Phase 4: parity cleanup
Implement:
- busy-state gating in parity
- unsupported-strategy message fix
- any low-risk GPU status refresh cleanup

### Phase 5: Rust profiling/test hygiene
Implement:
- NVTX scopes
- whitespace normalization test

### Phase 6: verification
Run the smallest relevant test commands available in this repo and confirm nothing regressed.

---

## Acceptance criteria

Your implementation is done only when all of these are true:

1. **Route contract unchanged**
   - existing route-selection tests still pass
   - no strategy-kind command mapping regressed

2. **Request-build failures are explicit**
   - DynamicDoubleMa preview/build failures surface their real message

3. **Timing language is honest**
   - completion message clearly refers to command completion, not fake total frontend time
   - `progressMeta` includes phase timing

4. **Route summary is preserved**
   - validation failure/cancel does not erase backend route/fallback evidence

5. **Busy/cancel semantics are correct**
   - cancel available only during backend optimize command
   - post-run follow-up remains busy but non-cancelable
   - parity also uses a correct busy model

6. **Artifact save failures are visible and non-fatal**
   - optimize still completes
   - analysis still refreshes
   - unsaved artifact condition is visible

7. **Recommended defaults are safe**
   - `topK <= 0` repaired
   - blank Grid heatmap bins repaired
   - explicit `heatmapBins = 0` preserved
   - non-Grid does not auto-fill heatmap bins

8. **Profiling consistency improves**
   - composite CPU and composite MO commands have matching NVTX-style profiling scope coverage
   - Rust normalization test for whitespace path exists and passes

---

## Explicit risks / things you should not overreach on

1. **Do not redesign workflow artifact payloads or analysis hydration.**
   - Keep changes limited to reuse and warning visibility.
   - Do not drop `res` from artifacts/store in this task.

2. **Do not change preset numbers or claim benchmark-optimal defaults.**
   - That belongs to bundle 7 / benchmark evidence, not this implementation pass.

3. **Do not turn this into a validation-service rewrite.**
   - Validation routing/cancellation-domain follow-up is adjacent scope.
   - Only touch adjacent validation/control files if required for compatibility.

4. **Do not redesign result tabs, UI surface layout, or user flows beyond eliminating unnecessary tab churn.**

---

## Tests / verification to run

Run the narrowest relevant test commands available in this repo. Prefer targeted verification.

At minimum, run:
- the Vitest tests covering `web/src/services/optimizeService.test.ts`
- the Vitest tests covering `web/src/services/recommendedOptimizationDefaults.test.ts`
- the Rust tests covering `src-tauri/src/commands_optimize.rs` normalization logic

If available and cheap, also run:
- a targeted typecheck/build step for the touched frontend files
- any focused Rust check/build for the touched command file

If exact command names differ in this repo, choose the closest targeted equivalents.

Do not skip test execution.

---

## Final deliverable format

When you finish, provide:

1. **Files changed**
   - list every modified file

2. **Behavioral summary**
   - what changed in each file
   - why it changed

3. **Tests run**
   - exact commands you ran
   - pass/fail results

4. **Open follow-ups not implemented**
   - especially if you intentionally did not implement GPU status caching
   - and whether a future adjacent audit is still needed for:
     - `runControlService.ts`
     - `validationService.ts`
     - `cancel_current_run` vs `cancel_background_task`

5. **Any deviations from the markdown guides**
   - explain why a local repo difference required an adaptation

---

## Short reminder

This is an implementation task, not a review task.

Implement the bundle-8 guides fully and carefully, with emphasis on:
- performance
- optimization
- correctness
- truthful timing/status semantics
- parity-safe behavior
- no feature removal
- no route regressions
