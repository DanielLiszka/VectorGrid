# Simple Mode Public Workflow Polish

Status: implementation pass completed on 2026-04-29 for Simple summary, validation recommendation, copy summary, packaged synthetic sample, release-doc updates, and QA coverage
Risk: low to medium; the implemented pass kept optimizer/validation semantics unchanged and added only presentation, packaging-resource discovery, and documentation changes
Primary value: make Simple Mode feel like a complete public-user workflow for optimization plus optional validation without exposing Advanced Mode complexity

## Summary

Simple Mode is now a focused four-step path:

1. Data
2. Strategy
3. Optimize
4. Validate, optional

That shape is correct for the public-facing mode. The next improvement should not add more expert capability to Simple Mode. It should make the existing path more self-explanatory, more decision-oriented, and easier to trust after a run.

The highest-ROI work is:

- add a clear run summary after optimization;
- add a plain validation recommendation state;
- improve blocked, empty, and failure states;
- harden first-run sample/default behavior;
- add one obvious result-saving or handoff path;
- expand Simple Mode QA coverage so release candidates prove the public path still works.

## Implementation Pass Completed 2026-04-29

Implemented:

- shared `web/src/utils/simpleWorkflowModel.ts` with unit-tested labels, Simple strategy support, validation recommendations, metric normalization, and Markdown summary formatting;
- direct `useGuidedWorkflowReadiness` unit coverage for blocked, supported, optimized, warning, and structured stage-completion states;
- Simple Optimize run summary with metric strip and clipboard-backed "Copy summary";
- Simple Validate recommendation panel with blocked, recommended, passed, review, and rejected language based on existing frontend evidence;
- Simple/live boundary cleanup so visible Simple Mode remains Data -> Strategy -> Optimize -> Validate and broker/live controls stay in Advanced Mode;
- titlebar/readiness gates using Simple optimization support rather than live execution support;
- stable `data-qa-panel`, `data-qa-state`, and action-manifest coverage for new Simple states/actions;
- bundled synthetic sample CSV discovery for clean-machine Simple first-run smoke;
- release docs and release manifest resource tracking for the Simple sample path.

Deferred:

- backend optimizer or validation-scoring changes, by design;
- direct Simple file export beyond clipboard summary;
- full clean-machine signed installer smoke, because that requires the external release/signing environment;
- broader sample-market-data packaging, because redistribution rights and file size need a product decision.

This work should remain additive. Advanced Mode remains the full workstation for live execution, options, ML Signal, aggregate analysis, campaigns, diagnostics, detailed exports, and expert controls.

## Current Anchors

Primary Simple Mode implementation:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/composables/useGuidedWorkflowReadiness.ts`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/workbench/WorkbenchShell.vue`
- `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/stores/settingsStore.ts`

Data, optimization, validation, and artifact state:

- `web/src/stores/dataStore.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/validationStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/optimizeService.ts`
- `web/src/services/validationService.ts`
- `web/src/services/workflowArtifactsSync.ts`

Reusable UI primitives:

- `web/src/components/ui/AppEmptyState.vue`
- `web/src/components/ui/AppMetricStrip.vue`
- `web/src/components/ui/AppButton.vue`
- `web/src/components/ui/AppStatusChip.vue`

QA and release coverage:

- `web/src/components/GuidedWorkspacePanel.test.ts`
- `web/src/components/DesktopTitlebar.test.ts`
- `web/src/components/workbench/WorkbenchShell.test.ts`
- `web/e2e/browser/guided-mode-ux.spec.ts`
- `web/e2e/browser/qa/actionCoverageManifest.ts`
- `web/e2e/browser/qa/visualBaselines.spec.ts`
- `web/scripts/check-a11y-smoke.mjs`
- `web/e2e/browser/README.md`
- `docs/PUBLIC_RELEASE.md`
- `docs/WINDOWS_INSTALL.md`

Public docs and sample-resource anchors:

- `README.md`
- `web/README.md`
- `docs/guided_mode_implementation_plan.md` as ignored local historical context, not current authority
- `.gitignore` if a new tracked public doc under `docs/` is added
- `src-tauri/src/commands_data.rs`
- `src-tauri/tauri.windows.conf.json`
- `tauri.windows.conf.json`

Related planning context:

- `docs/frontend_workflow_evidence_decision_authority.md`
- `docs/frontend_end_to_end_qa_automation_plan.md`
- `docs/frontend_premium_desktop_ui_ux_implementation_plan_2026-04-26.md`
- `docs/verification_benchmark_release_authority.md`

## Existing Behavior

- Simple Mode hides the advanced workspace bar and run footer.
- Simple Mode exposes four visible steps and maps legacy stored `guidedStep = "trade"` back to validation.
- Simple Mode already has a recommended next action, a readiness checklist, sample CSV loading, two strategy templates, optimization, validation, and Advanced Mode handoffs.
- Paper/live runner controls are no longer visible in Simple Mode.
- Validation is framed as optional evidence rather than a mandatory step before the user can leave Simple Mode.
- Existing component, browser workflow, visual, accessibility, typecheck, build, and lint gates already cover the current shell.

## Original Audit Findings And Resolution

These findings came from a second pass across code, tests, and release docs. The implementation pass addressed the items noted below unless explicitly marked deferred.

### Public Documentation Drift

- `README.md` still describes the supported public surface as backend/CLI only and says the Vue/Tauri app is not the public quick-start path.
- `web/README.md` still describes the frontend as internal work-in-progress.
- The ignored historical `docs/guided_mode_implementation_plan.md` is stale: it refers to Guided Mode, says Advanced remains the default, and still describes paper/live readiness as part of the guided path.
- `docs/PUBLIC_RELEASE.md` defined strong Windows release evidence, but its manual public-user checklist needed an explicit Simple Mode first-run path.

Implementation note: the completed pass added the Simple first-run path to release documentation while preserving the root README's backend-preview public status. The ignored historical Guided Mode plan remains a non-authoritative local history file.

### Sample Data Packaging Gap

- `src-tauri/src/commands_data.rs` discovers samples from repo-local `data/10kCandles.csv`, `data/100kCandles.csv`, and `data/1MillionCandles.csv` style files.
- `src-tauri/tauri.windows.conf.json` bundled the CatBoost native DLL resource but did not include sample candle data.
- A packaged Windows build may therefore launch into Simple Mode with no sample data unless a bundled-resource sample path is added or the public release notes require the user to provide a CSV.

Implementation note: the completed pass added a small synthetic OHLCV CSV under `src-tauri/resources/sample-data/`, wired it into Windows Tauri resources, and made `list_sample_csvs` discover packaged-resource samples before falling back to dev and repo-local data samples.

### Simple Versus Live Boundary Drift

- Some Simple readiness names and gates still use live-trading language, especially `liveStrategySupported`.
- Simple optimization support should be separate from live execution support. A strategy can be allowed in Simple optimization without implying it can be live traded.
- The validation panel should not make "Open live workspace" feel like the primary next step. Live and broker controls belong in Advanced Mode and should be a secondary boundary handoff.
- Hidden `trade` code can remain temporarily for compatibility, but visible Simple Mode should be strictly Data -> Strategy -> Optimize -> Validate.

### QA Coverage Gap

- Current browser E2E proves first-run Simple shell and Advanced handoff, but not the full public path.
- Several Simple actions are marked covered by a handoff workflow that does not actually click them.
- `useGuidedWorkflowReadiness.ts` does not have direct unit tests even though it owns key readiness and validation verdict behavior.
- Visual and a11y coverage is concentrated on the empty/default guided shell, not loaded data, optimization-ready, validation-warning, or compact per-step states.
- Several Simple controls would benefit from `data-qa-panel` and `data-qa-state` markers in addition to `data-qa-action`.

## Product Target

Simple Mode should let a new public user answer four questions without learning the full workstation:

1. What data am I testing?
2. What strategy and objective am I using?
3. What did the optimization find?
4. Does optional validation support trusting the result, reject it, or require more review?

Simple Mode is complete when a user can load sample or CSV data, choose a supported strategy, run optimization, understand the best result, optionally validate it, and save or hand off the result without opening Advanced Mode unless they want expert details.

## Additive Boundary

Allowed in this plan:

- frontend-only view-model helpers;
- Simple Mode copy, layout, and summary changes;
- clearer disabled reasons and failure states;
- new component tests and mocked browser workflows;
- optional clipboard or existing-report handoffs;
- release documentation that points clean-machine testers through Simple Mode.

Avoid unless explicitly approved:

- backend scoring changes;
- optimizer ranking changes;
- validation verdict algorithm changes;
- new live execution controls in Simple Mode;
- new Tauri commands for export unless the frontend-only path proves insufficient;
- broad Advanced Mode refactors;
- new dependency additions.

## Non-Goals

Do not fold these into Simple Mode:

- live broker setup, paper-runner operation, emergency halt, or active feed control;
- options-expression optimization or validation;
- ML Signal/CatBoost workflows;
- Meta/Stage 5 confidence training or application;
- campaign orchestration;
- dynamic strategy authoring;
- GPU diagnostics, route profiling, or benchmark controls;
- artifact repair, report parity, or history library maintenance tools.

Advanced Mode should remain the place where these capabilities are visible and fully controllable.

## Implementation Plan

### Phase 0: Baseline, Vocabulary, And Public Scope Audit

Goal: freeze what Simple Mode means before adding more UI.

Checklist:

- [ ] Confirm all visible Simple Mode labels use "Simple", not "Guided".
- [ ] Confirm the visible step set remains `data`, `strategy`, `optimize`, `validate`.
- [ ] Confirm legacy `trade` persisted state still maps to validation.
- [ ] Confirm no broker runner actions are visible in Simple Mode.
- [ ] Confirm Advanced Mode still exposes full live, options, ML, aggregate, diagnostics, and artifact tools.
- [ ] Inventory all Simple Mode `data-qa-action` IDs and verify action coverage manifest entries remain current.
- [ ] Inventory missing Simple `data-qa-panel` and `data-qa-state` selectors before writing new E2E coverage.
- [ ] Update or explicitly supersede stale Guided Mode wording in ignored/local docs before using it as implementation guidance.

Files likely touched:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/components/GuidedWorkspacePanel.test.ts`
- `web/e2e/browser/guided-mode-ux.spec.ts`
- `web/e2e/browser/qa/actionCoverageManifest.ts`
- `docs/guided_mode_implementation_plan.md` if this ignored local doc remains useful to the maintainer

Acceptance criteria:

- Simple Mode still has exactly four user-visible steps.
- Existing users with stale `guidedStep = "trade"` land on validation/handoff, not a missing panel.
- Advanced Mode remains one click away from each step.
- Public-facing docs and current implementation do not disagree about Simple Mode being the recommended first-run UI.

### Phase 1: Add A Shared Simple Workflow Model

Goal: centralize public-facing run state so summary copy, step status, disabled reasons, and titlebar actions cannot drift.

Recommended shape:

Create a small frontend helper, preferably pure enough to unit test:

- `web/src/utils/simpleWorkflowModel.ts`, plus a thin composable wrapper if store access is needed; or
- `web/src/composables/useSimpleWorkflowSummary.ts` if the implementation stays store-bound.

Suggested model:

```ts
export type SimpleStep = "data" | "strategy" | "optimize" | "validate"

export interface SimpleWorkflowModel {
  activeStep: SimpleStep
  steps: Array<{
    key: SimpleStep
    label: string
    subtitle: string
    tone: "blocked" | "ready" | "done" | "warning"
    disabledReason: string
  }>
  dataset: {
    label: string
    timeframe: string
    rows: string
    range: string
    ready: boolean
  }
  strategy: {
    label: string
    objective: string
    searchDepth: string
    backendLabel: string
    ready: boolean
    simpleOptimizationSupported: boolean
    simpleUnsupportedReason: string
  }
  optimization: {
    ready: boolean
    status: string
    candidateTitle: string
    candidateSummary: string
    rankedCountLabel: string
    metricItems: Array<{ key: string; label: string; value: string; tone?: string }>
  }
  validation: {
    available: boolean
    ready: boolean
    recommendation: "not_run" | "recommended" | "passed" | "review" | "rejected" | "blocked"
    title: string
    body: string
    metricItems: Array<{ key: string; label: string; value: string; tone?: string }>
  }
  nextAction: {
    label: string
    detail: string
    disabledReason: string
    kind: "choose_data" | "load_sample" | "choose_strategy" | "run_optimization" | "run_validation" | "review_results"
  }
}
```

Data sources:

- dataset state from `dataStore`;
- strategy and run readiness from `optimizeStore`;
- candidate title, selected summary, `bestStats`, and top-row counts from `analysisStore`;
- validation rows, warning summary, route summary, output preview, and stage completion from `validationStore` and `workflowArtifactsStore`;
- existing readiness booleans from `useGuidedWorkflowReadiness`.

Rules:

- Keep backend route names out of the primary summary unless the label is user-safe.
- Prefer "Auto selected engine" or "CPU/GPU engine" in Simple Mode; keep exact route names in Advanced Mode.
- Do not invent metrics. Only display metrics already present in store state or artifact stage completion.
- Treat unknown metrics as unavailable, not zero.
- Separate `simpleOptimizationSupported` from live execution support. Do not use `liveStrategySupported` as the Simple optimization gate.
- Keep titlebar primary actions and panel primary actions derived from the same model.

Files likely touched:

- `web/src/utils/simpleWorkflowModel.ts`
- `web/src/utils/simpleWorkflowModel.test.ts`
- `web/src/composables/useSimpleWorkflowSummary.ts` if a composable wrapper is used
- `web/src/composables/useGuidedWorkflowReadiness.ts`
- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/GuidedWorkspacePanel.test.ts`
- `web/src/components/DesktopTitlebar.test.ts`

Acceptance criteria:

- Step state, next action, disabled reason, and summary state are testable without mounting the full Simple panel.
- Simple Mode can show the same run summary consistently in multiple places.
- No backend contract changes are required.
- Data/Strategy titlebar actions do not surprise-load or mutate user state without explicit interaction.

### Phase 2: Add A Post-Optimization Run Summary

Goal: after optimization, show a compact answer to "what did this run find?"

UI placement:

- In the Optimize step, replace or enhance the current candidate block with a "Run summary" section.
- In the Validate step, keep a compact candidate summary above the validation action.
- Keep the hero "Next" card action-oriented, not overloaded with metrics.

Recommended content:

- Dataset: label, timeframe, row count, range.
- Strategy: Double MA crossover or Composite signal.
- Search: objective and search depth.
- Engine: simplified backend label.
- Best candidate: selected title.
- Metrics: return, Sharpe/Sortino or objective metric, max drawdown, trade count where available.
- Result count: ranked candidates or top rows.

Component guidance:

- Reuse `AppMetricStrip.vue` for metric rows if it fits without nested card clutter.
- Keep cards at 8px radius or less if new local styles are added.
- Use icons only where they clarify action state; avoid decorative icons.
- Keep text compact enough for the 390px mobile visual baseline.

Files likely touched:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/components/ui/AppMetricStrip.vue` only if the existing primitive needs a small compatibility adjustment
- `web/src/components/GuidedWorkspacePanel.test.ts`
- `web/e2e/browser/guided-mode-ux.spec.ts`
- `web/e2e/browser/qa/visualBaselines.spec.ts-snapshots/mobile-guided-shell-chromium-win32.png`

Acceptance criteria:

- Optimization-ready Simple Mode shows a useful summary without requiring Advanced Mode.
- Missing metric values render as `-` or plain unavailable copy, never as misleading zeros.
- Mobile layout has no horizontal overflow and no clipped button text.

### Phase 3: Make Validation Optional And Outcome-Oriented

Goal: make optional validation meaningful by translating existing validation evidence into a clear next decision.

Recommended recommendation states:

| State | Trigger | Primary copy | Primary action |
| --- | --- | --- | --- |
| `not_run` | optimization ready, validation not run | "Validation is optional but recommended before trusting this result." | Run validation |
| `blocked` | no optimization result or validation disabled | concrete blocked reason | return to needed step |
| `passed` | backend/stage verdict approved or passed with no concerning warnings | "Validation supports this candidate." | Review report or save summary |
| `review` | warnings, partial evidence, approved with warnings, concerning text | "Validation needs review." | Open full validation report |
| `rejected` | backend/stage verdict rejected or failure-like state | "Validation does not support this candidate." | Revise strategy or open report |

Implementation notes:

- Start with frontend interpretation of existing `validationCompletion`, `validationWarningSummary`, `validationOutputPreview`, and row counts.
- Do not change backend validation thresholds.
- Do not claim "safe to trade". The best Simple Mode phrasing is "supports this candidate" or "needs review".
- A warning state is not a failure if the backend has approved with warnings.
- Include dataset/timeframe compatibility context if available from `previewValidationContextCompatibilityFromStores()`.
- Add a "skip for now" or "review optimization result" secondary action if it improves flow, but do not make live execution the default continuation.
- Demote live/broker handoff copy to a secondary Advanced Mode boundary note.

Files likely touched:

- `web/src/composables/useGuidedWorkflowReadiness.ts`
- `web/src/composables/useSimpleWorkflowSummary.ts` or equivalent
- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/services/validationService.ts` only if an existing preview helper can be reused without behavior changes
- `web/src/components/GuidedWorkspacePanel.test.ts`

Acceptance criteria:

- A user sees why validation is optional and when it is recommended.
- Passed, warning/review, rejected, blocked, and not-run states have distinct titles and actions.
- Tests cover all recommendation states with mocked store data.
- The primary validation CTA is validation/report/review oriented, not live-trading oriented.

### Phase 4: Improve Empty, Blocked, Failure, And Recovery States

Goal: make failures feel handled rather than technical.

Target states:

- no sample dataset available;
- no CSV path selected;
- CSV parse/load failure;
- dataset loaded but unsupported strategy selected;
- optimization blocked because data is missing;
- optimization returns no candidates;
- optimization failure;
- validation blocked because no candidate exists;
- validation blocked by dataset/timeframe mismatch;
- validation backend failure;
- validation returns warnings or zero admitted candidates.

Recommended UI behavior:

- Use `AppEmptyState.vue` for true empty/blocked states where a step has no usable content yet.
- Keep existing status lines visible but add an action-oriented blocked reason near the disabled action.
- Separate "what happened" from "what to do next".
- For recoverable failures, keep the next useful action available.
- Avoid raw backend stack text in Simple Mode. Use sanitized store status and leave full detail in Advanced/Task Center where available.

Files likely touched:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/composables/useGuidedWorkflowReadiness.ts`
- `web/src/composables/useSimpleWorkflowSummary.ts` or equivalent
- `web/src/components/GuidedWorkspacePanel.test.ts`
- `web/e2e/browser/guided-mode-ux.spec.ts`
- `web/e2e/browser/qa/regressionRepros.spec.ts` for any user-discovered failure path reproduced in browser

Acceptance criteria:

- Every disabled primary action has a visible reason in Simple Mode, not only a tooltip.
- Failure states clear busy indicators and preserve retry or handoff actions.
- Browser E2E covers at least one optimization failure and one validation failure/mismatch path in Simple Mode.

### Phase 5: Streamline Data And Strategy Choices

Goal: let a new user get a meaningful result quickly.

Checklist:

- [ ] Confirm release packaging includes at least one small, non-sensitive sample dataset if the sample loader depends on bundled files.
- [ ] If packaging sample data, update `list_sample_csvs` or a nearby command path so packaged builds can discover resource samples, not only repo-local `data/`.
- [ ] Prefer synthetic or redistributable sample candles unless real market-data redistribution rights are explicit.
- [ ] Store sample metadata: synthetic/real, source/license, symbol, timeframe, date range, and row count where available.
- [ ] Confirm the first enabled sample is selected by default.
- [ ] Confirm sample labels are meaningful to a public user, for example symbol, timeframe, and date range when known.
- [ ] Confirm "Load sample data" is the first recommended action when samples exist and no dataset is loaded.
- [ ] Confirm the default strategy is supported by Simple Mode and validation.
- [ ] Confirm the default objective and search depth are not surprising for a beginner.
- [ ] Consider hiding the backend select in Simple Mode or demoting it to a small "Engine: Auto" line with an Advanced handoff.
- [ ] Rename public strategy cards away from implementation names such as `CompositeCpu`.
- [ ] Consider strategy personas such as "Baseline crossover", "Composite trend signal", "Regime-aware trend", or "Breakout profile" only when the existing optimize request builder fully supports the exposed profile.

Recommended default posture:

- Strategy: Double MA crossover.
- Objective: Sharpe or the repo's current conventional default if different.
- Search depth: Balanced.
- Backend: Auto.
- Validation: optional but recommended after optimization.
- Advanced links: always available, never dominant.

Files likely touched:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/stores/dataStore.ts` only if sample labels require normalized metadata already returned by existing commands
- `src-tauri/src/commands_data.rs` only if existing sample metadata is insufficient and a backend change is approved
- `src-tauri/tauri.conf.json` and release manifest scripts only if sample packaging needs a resource change
- `src-tauri/tauri.windows.conf.json` and root mirror if Windows bundle resources change
- `web/src/utils/strategyOptimizationCapabilities.ts` if public strategy personas depend on capability summaries
- `web/src/services/optimizeRequestBuilder.ts` tests if new Simple personas map to existing strategy/profile requests

Acceptance criteria:

- A first-run user with bundled samples can reach an optimization result with minimal choices.
- CPU-only release builds do not depend on CUDA, Polygon, broker credentials, or private local paths for the sample path.
- Clean-machine release smoke includes the Simple sample path when sample data is packaged.
- Simple strategy labels do not imply live-trading support.

### Phase 6: Add One Simple Result-Saving Path

Goal: give Simple Mode one obvious way to preserve or share the result without exposing the full artifact/report machinery.

Preferred low-risk sequence:

1. Add "Copy summary" in Simple Mode using existing clipboard support where available, with a browser fallback for tests.
2. Add "Open full report/export tools" as the Advanced Mode handoff.
3. Only after that, consider a first-class Markdown save/export command if users need a file from Simple Mode directly.

Suggested copied summary content:

```md
# VectorGrid Simple Run Summary

Dataset: ...
Strategy: ...
Objective: ...
Search depth: ...
Optimization result: ...
Best candidate: ...
Validation: not run / passed / needs review / rejected
Warnings: ...
Generated: ...
```

Implementation constraints:

- Do not mutate saved artifacts just to copy a summary.
- Do not create a new report schema in this slice.
- If a Markdown file export is later added, use an existing safe save-dialog/service pattern and add command-capability checks together.

Files likely touched:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/services/desktopClipboard.ts` or existing clipboard helper if present
- `web/src/utils/simpleWorkflowModel.ts` for Markdown serialization if the summary formatter lives with the model
- `web/e2e/browser/qa/actionCoverageManifest.ts`
- `web/src/components/GuidedWorkspacePanel.test.ts`

Acceptance criteria:

- Simple Mode exposes one clear result-saving action after optimization.
- The action works in mocked browser tests and does not require a backend command.
- Advanced report export remains the authority for XLSX/report artifacts.

### Phase 7: Polish Mode Transitions And Visual Hierarchy

Goal: make the four-step path feel finished without turning it into a marketing page.

Recommended changes:

- Keep the first viewport focused on the usable workflow, not explanatory copy.
- Shorten the hero body if the run summary becomes more useful than the current explanation.
- Make the current step and next action visually stronger than secondary Advanced handoffs.
- Demote advanced bridge text to a small footer or secondary rail block.
- Keep the readiness checklist compact and readable at 390px width.
- Avoid nested card effects. Use section bands, metric strips, and compact panels.
- Use existing icon library icons for actions where useful.
- Preserve 24px minimum target-size behavior from the WCAG smoke expectations.
- Switching Simple/Advanced should reconcile the visible workspace and active Simple step predictably.
- If Settings from Simple currently forces Advanced Mode, document that behavior in tests or replace it with a smaller Simple settings affordance.
- Remove fixed 5-column assumptions in Simple step CSS if any remain; use the step metadata length or responsive grid tracks.

Files likely touched:

- `web/src/components/GuidedWorkspacePanel.vue`
- `web/src/components/workbench/WorkbenchShell.vue` only if shell subtitle or hidden chrome needs polish
- `web/src/components/DesktopTitlebar.vue` only if titlebar action copy changes
- `web/src/components/SettingsWorkspacePanel.vue` if Simple settings entry behavior changes
- `web/e2e/browser/qa/visualBaselines.spec.ts`

Acceptance criteria:

- Desktop and mobile screenshots show a clear current step, next action, and summary.
- Simple Mode does not show competing action clusters with equal weight.
- No text overlap, clipped labels, or unexpected horizontal document overflow.
- Mode switches preserve user state and do not strand users on hidden or impossible panels.

### Phase 8: Expand Simple Mode Browser QA

Goal: prove the public path, not just the presence of the shell.

Add or expand browser workflows for:

- first-run Simple Mode with no data;
- first-run Simple Mode with sample data available;
- sample load -> strategy -> optimize success -> validation recommended;
- optimization success -> validation run -> passed/review/rejected display, using fixture states;
- optimization failure recovery;
- validation dataset mismatch or backend failure recovery;
- Advanced handoff from each Simple step;
- legacy stored `guidedStep = "trade"` repair;
- compact/mobile Simple layout with no overflow.
- keyboard-only step navigation and primary action activation;
- actual coverage for each required guided action rather than only the generic handoff workflow.

Files likely touched:

- `web/e2e/browser/qa/simpleModeWorkflow.spec.ts` if a new focused spec is clearer than expanding `guided-mode-ux.spec.ts`
- `web/e2e/browser/guided-mode-ux.spec.ts`
- `web/e2e/browser/qa/workflowMatrix.spec.ts`
- `web/e2e/browser/qa/actionCoverageManifest.ts`
- `web/e2e/browser/qa/visualBaselines.spec.ts`
- `web/e2e/browser/support/mockTauri.ts`
- `web/e2e/browser/fixtures/*` if new named fixtures are introduced
- `web/scripts/check-a11y-smoke.mjs`
- `web/src/composables/useGuidedWorkflowReadiness.test.ts`

Recommended commands:

```powershell
npm --prefix web run test -- src/components/GuidedWorkspacePanel.test.ts src/composables/useGuidedWorkflowReadiness.test.ts src/components/DesktopTitlebar.test.ts src/components/workbench/WorkbenchShell.test.ts src/components/CommandPaletteOverlay.test.ts src/components/SettingsWorkspacePanel.test.ts src/stores/settingsStore.actions.test.ts src/services/settingsBootstrap.test.ts src/preflight.test.ts
npm --prefix web run typecheck
npm --prefix web run lint
npm --prefix web run build
npm --prefix web run check:qa-static
npm --prefix web run test:e2e:workflow:built
npm --prefix web run test:e2e:visual:built
npm --prefix web run a11y:smoke:wcag:built
```

For a release-facing frontend pass:

```powershell
powershell -File scripts/run_frontend_e2e_gate.ps1 -IncludeFullWorkflow -IncludeResponsive -IncludeRegressions -IncludeA11ySmoke -IncludeWcag22Smoke -IncludeVisualBaselines -IncludeQaStatic -IncludeQaReport -IncludeUiPerfBudget
```

Acceptance criteria:

- Simple Mode has browser coverage for success, blocked, failure, repair, and Advanced handoff states.
- Action coverage manifest stays current.
- Visual baseline changes are intentional and reviewed.

### Phase 9: Release Documentation And Clean-Machine Script Notes

Goal: make public release verification exercise the actual public path.

Documentation updates:

- Update `README.md` public status once the desktop app is actually a supported public release artifact.
- Update `web/README.md` so it reads as maintainer/developer frontend documentation, not as a warning that the frontend is unsupported.
- Update or replace `docs/guided_mode_implementation_plan.md` if it remains in active local use, because the current implementation is Simple Mode by default and no longer includes paper/live readiness in the Simple path.
- Add Simple Mode public smoke to `docs/PUBLIC_RELEASE.md`.
- Add a user-facing "first run" note to `docs/WINDOWS_INSTALL.md` if the installer is intended for non-developers.
- Consider adding `docs/SIMPLE_MODE_QUICKSTART.md` if public user docs need a single offline quickstart; because `docs/*` is ignored by default, update `.gitignore` allowlisting in the same change if this file should be tracked.
- Consider a short README note that the recommended first action is Simple Mode sample optimization.
- If sample data is packaged, document what it is and that it is not financial advice.

Release smoke additions:

- clean install;
- launch into Simple Mode;
- load sample data or a known local CSV;
- run CPU-only optimization;
- observe run summary;
- optionally run validation;
- copy summary or open full report/export tools;
- switch to Advanced Mode and confirm expert workspaces remain reachable.

Acceptance criteria:

- Public release docs do not require a new user to understand Advanced Mode first.
- CPU public release path does not require credentials, CUDA, or external providers.
- Advanced Mode remains documented as the full workstation.
- Public docs no longer say the desktop app is unsupported after a public desktop release is declared.

## Suggested Rollout Order

1. Phase 0 baseline and Simple/live boundary audit.
2. Phase 1 shared Simple workflow model plus unit tests.
3. Phase 5 data/strategy streamlining where it is frontend-only.
4. Phase 2 optimization run summary in Simple Mode.
5. Phase 3 validation recommendation state.
6. Phase 4 empty/failure recovery states.
7. Phase 8 focused browser QA for the new states.
8. Phase 7 visual polish and baseline refresh.
9. Phase 6 copy-summary action.
10. Phase 5 sample packaging/resource work if approved.
11. Phase 9 release docs.

This order keeps the first implementation slices frontend-only and testable. Sample packaging and direct file export are deferred because they may cross into Tauri/release-resource behavior.

## File-Level Implementation Checklist

### New frontend helper

- [ ] Add `web/src/utils/simpleWorkflowModel.ts`, with `web/src/composables/useSimpleWorkflowSummary.ts` only if a store-bound wrapper is useful.
- [ ] Add unit tests for dataset, strategy, optimization, validation, and Markdown/copy summary formatting if implemented.
- [ ] Keep helper output plain strings and simple arrays so UI rendering remains thin.

### `useGuidedWorkflowReadiness.ts`

- [ ] Keep boolean readiness in this composable.
- [ ] Add only lightweight derived validation recommendation fields if they are broadly reused.
- [ ] Avoid expanding this composable into a full presentation model if a new summary helper exists.
- [ ] Preserve `tradeChecklist` only for legacy/unreachable fallback if removing it would cause avoidable churn.
- [ ] Rename or wrap `liveStrategySupported` so Simple optimization support is not coupled to live execution support.
- [ ] Add direct unit tests for readiness and verdict-tone behavior.

### `GuidedWorkspacePanel.vue`

- [ ] Replace ad hoc candidate text with summary-helper output.
- [ ] Add post-optimization metric strip.
- [ ] Add validation recommendation panel.
- [ ] Add structured empty states.
- [ ] Add a copy-summary or Advanced export handoff action.
- [ ] Keep Simple Mode to four visible steps.
- [ ] Keep paper/live controls out of the visible Simple path.
- [ ] Demote live workspace handoff to secondary boundary copy or remove it from the primary validation action cluster.
- [ ] Add `data-qa-panel` and `data-qa-state` markers where Simple E2E needs stable state assertions.

### `DesktopTitlebar.vue`

- [ ] Keep the primary titlebar action aligned with the active Simple step.
- [ ] Add or update disabled reasons if validation recommendation copy changes.
- [ ] Preserve legacy `trade -> validate` mapping.

### Settings and command palette

- [ ] Keep Simple Mode label concise.
- [ ] Do not expose "Trade Ready" as a start step.
- [ ] Confirm command palette copy describes Simple Mode as optimization plus optional validation.

### Browser QA

- [ ] Update action coverage for any new Simple action IDs.
- [ ] Add Simple success and failure workflows.
- [ ] Refresh mobile visual baseline only after visual review.
- [ ] Extend a11y smoke if new visible states require interaction before scan.

## Copy Guidelines

Use:

- "Run optimization"
- "Best candidate"
- "Validation recommended"
- "Validation supports this candidate"
- "Validation needs review"
- "Open full validation report"
- "Open Advanced Mode for live tools"
- "Advanced Mode"

Avoid in primary Simple Mode copy:

- "route"
- "backend"
- "artifact"
- "dry run"
- "broker"
- "GPU kernel"
- "residency"
- "CSCV/PBO/SPA" unless inside Advanced Mode or a technical details disclosure
- "safe to trade"
- "approved for live trading"

When technical terms cannot be removed, put them in secondary detail:

- "Engine: Auto" can replace "Backend: Auto".
- "Detailed report" can replace "artifact-backed XLSX export".
- "Evidence rows" can replace "validation table rows" in primary copy.

## Regression Watchpoints

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| Simple Mode accidentally hides Advanced capability | Public polish must not reduce expert workflows | Advanced handoff tests and command-palette/workspace coverage |
| Validation recommendation overstates confidence | Financial decision UX must avoid false certainty | Use "supports", "needs review", or "rejected"; never "safe" |
| Missing metrics render as misleading values | Users may trust incorrect summaries | Unknown metrics render unavailable, not zero |
| Copy-summary action mutates artifacts | Low-risk Simple action should not alter saved evidence | Clipboard-only first, no artifact writes |
| Sample path fails on clean machine | First-run public workflow breaks | Release-resource audit and clean-machine smoke |
| Simple strategy support is confused with live execution support | Users may infer a strategy is tradable because it can optimize | Rename support fields and keep live support copy in Advanced handoff |
| Public docs contradict the desktop release | Users and maintainers follow the wrong quick-start path | Update root README, web README, public release docs, and quickstart together |
| Mobile layout regresses | Simple Mode is the public face | Mobile visual baseline, overflow checks, clipped-text audit |
| Legacy `trade` state breaks old installs | Existing localStorage/settings may persist old step | Preserve repair mapping until a migration removes it intentionally |
| QA manifest drift | New buttons can become untested | `check:qa-static` and action coverage updates in same change |

## Validation Checklist

For frontend-only Simple Mode changes:

- [ ] `npm --prefix web run test -- src/components/GuidedWorkspacePanel.test.ts src/components/DesktopTitlebar.test.ts src/components/workbench/WorkbenchShell.test.ts src/components/CommandPaletteOverlay.test.ts src/components/SettingsWorkspacePanel.test.ts`
- [ ] `npm --prefix web run typecheck`
- [ ] `npm --prefix web run lint`
- [ ] `npm --prefix web run build`
- [ ] `npm --prefix web run check:qa-static`
- [ ] `npm --prefix web run test:e2e:workflow:built`
- [ ] `npm --prefix web run a11y:smoke:wcag:built`
- [ ] `npm --prefix web run test:e2e:visual:built`
- [ ] `git diff --check`

For release-facing Simple Mode changes:

- [ ] `npm --prefix web run test:e2e:workflow:full:built`
- [ ] `powershell -File scripts/run_frontend_e2e_gate.ps1 -IncludeFullWorkflow -IncludeResponsive -IncludeRegressions -IncludeA11ySmoke -IncludeWcag22Smoke -IncludeVisualBaselines -IncludeQaStatic -IncludeQaReport -IncludeUiPerfBudget`
- [ ] `powershell -ExecutionPolicy Bypass -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowDirtyWorktree -AllowUnsignedBeta` for a local non-publishable rehearsal when packaging/sample-resource behavior changes

For backend or Tauri-resource changes:

- [ ] `cargo test -p ta_desktop_demo_app`
- [ ] `node scripts/check_tauri_config_sync.mjs`
- [ ] relevant release-resource or manifest checks
- [ ] clean-machine smoke when sample packaging, resources, or installer behavior changes

## Definition Of Done

This plan is complete when:

- Simple Mode still exposes only Data, Strategy, Optimize, and optional Validate.
- A first-run user can reach a meaningful optimization result from sample or CSV data.
- Optimization results include a compact, accurate, plain-language run summary.
- Validation has clear not-run, recommended, passed, review, rejected, and blocked states.
- Simple Mode has structured empty and failure states for the public path.
- A user can copy a simple summary or hand off to Advanced export/report tools.
- Public release docs and sample-data packaging either support the first-run sample path or clearly document that the user must provide a CSV.
- Advanced Mode remains the complete workstation.
- Component tests, browser workflows, accessibility smoke, visual baselines, typecheck, lint, build, and QA static checks pass.
- Public release docs tell clean-machine testers to verify the Simple path.

## Future Expansion Candidates

These are useful but should not block the first polish pass:

- a dedicated Simple Mode Markdown export command if clipboard summary is insufficient;
- a small sample dataset selector with richer metadata and preview stats;
- a "recent successful run" reopen tile in Simple Mode;
- a Simple Mode validation comparison summary for reopened artifacts;
- a first-run checklist that disappears after the user completes the path once;
- a short "what changed since last run" summary when the user edits strategy or data after optimization.
