# Guided Mode Implementation Plan

## Intent

Add a focused, layman-friendly Guided Mode for the conventional workflow:

1. optimize a normal backtest strategy,
2. validate the selected candidate,
3. prepare the validated candidate for paper/live trading.

This must be additive. The current Advanced Mode remains the full product surface for Meta analysis, Stage 5 ML confidence, ML Signal/CatBoost workflows, options-expression optimization, campaigns, dynamic strategy tools, diagnostics, artifact replay, and every existing backend command currently exposed by the UI.

## Implementation Status

Last updated: 2026-04-24.

- [x] Phase 0: Confirm current advanced capability inventory and mode boundaries.
- [x] Phase 1: Add the mode model, persistence, shell switcher, and Advanced Mode preservation tests.
- [x] Phase 2: Build the Guided Mode conventional optimize flow.
- [x] Phase 3: Build the Guided Mode validation flow and plain-language verdict summaries.
- [x] Phase 4: Build the Guided Mode paper/live readiness flow.
- [x] Phase 5: Add Advanced Mode bridges, command-palette reachability, and capability preservation checks.
- [x] Phase 6: Run frontend, accessibility, browser, desktop, and baseline verification.
- [x] Phase 7: Apply post-MVP visual polish for charts, heatmap rendering, tooltip timing, and Guided Mode browser UX coverage.

Implementation note: Guided Mode has been implemented as an additive frontend mode. Advanced Mode remains the default and still owns the complete research surface.

## Scope

Guided Mode covers only the primary end-user journey:

- Load or choose market data.
- Choose a curated conventional strategy template.
- Run optimization with conservative defaults.
- Review the best candidates in plain language.
- Run validation on the selected candidate.
- Review whether validation supports or rejects the candidate.
- Prepare for paper/live execution with explicit broker, data-stream, strategy-support, and risk-limit checks.

Guided Mode should hide advanced knobs by default, but it should never hide the existence of Advanced Mode. When a user reaches a boundary, the UI should offer a clear handoff such as "Open Advanced Mode for Meta Analysis" rather than trying to embed the advanced workflow in the guided path.

## Non-Goals

Do not move these workflows into Guided Mode for the first implementation:

- Meta selection and aggregate analysis.
- Stage 5 ML confidence training, application, replay, or policy evaluation.
- ML Signal/CatBoost dataset export, score-cache generation, validation, or policy sweep.
- Options-expression optimization, feasibility analysis, tuple/event-path inspection, or options validation.
- Research campaigns, auto-meta, or auto-stage5 campaign follow-ups.
- Dynamic strategy builder/IR authoring.
- GPU route diagnostics, benchmark tooling, and low-level backend provenance controls.

These capabilities remain first-class Advanced Mode features.

## Backend Capability Preservation Matrix

| Capability family | Guided Mode first slice | Advanced Mode requirement |
| --- | --- | --- |
| CSV/sample data load | Expose a simplified data chooser | Preserve the full current data panel and command-palette access |
| Polygon historical data | Expose only the minimum download/load path if already stable in the current UI | Preserve cache, settings, validation, repair, and detailed provider controls |
| Conventional optimization | Expose curated templates and safe defaults | Preserve full strategy/config/backend/objective controls |
| Conventional validation | Expose selected-candidate validation and a simple verdict | Preserve all validation settings, reports, artifacts, and comparison tools |
| Paper/live execution | Expose readiness checks and supported dry-run/live setup only | Preserve the full execution workspace, broker controls, event details, and advanced runner state |
| Meta analysis / Stage 5 ML confidence | Show only contextual "Open Advanced Mode" links when relevant | Preserve all current Meta/Stage 5 surfaces and backend command access |
| ML Signal / CatBoost | Not included | Preserve the dedicated ML Signal panel and all bundle/dataset/cache/validation/sweep actions |
| Options-expression workflow | Not included | Preserve the options-expression panel and all artifact/inspection/feasibility actions |
| Campaigns and batch research | Not included | Preserve current campaign orchestration and artifact behavior |
| Dynamic strategy builder | Not included | Preserve current builder/preview/runtime surfaces |
| Diagnostics and GPU route detail | Not included except user-safe status copy | Preserve full diagnostics and route/provenance detail |

## Strategy Eligibility

Guided Mode should not imply a strategy can be traded if the live execution backend cannot run it.

Initial strategy eligibility should be conservative:

- Prefer strategies that already use the normal `optimizeService` and `validationService` request paths.
- For the full optimize -> validate -> paper/live guided journey, start with strategies supported by live execution, currently `DoubleMa` and `CompositeCpu`.
- Other conventional optimizer strategies can be added later as validate-only guided templates if the UI clearly stops at validation and explains that live execution is not currently supported for that strategy.
- Exclude `MlSignal`, options-expression, Meta/Stage 5, campaign-only, and dynamic-builder workflows from Guided Mode.

## UX Model

Guided Mode should behave like a workflow assistant, not like a reduced copy of every panel.

Primary steps:

1. **Choose Data**
   - Show loaded dataset status.
   - Offer sample CSV, CSV import, and optionally Polygon download.
   - Hide cache/repair/provider details behind Advanced Mode.

2. **Choose Strategy**
   - Offer a small set of strategy templates with plain-language descriptions.
   - Keep backend/objective/range defaults conservative.
   - Provide an Advanced Mode link for full configuration.

3. **Optimize**
   - Run the existing optimization path.
   - Show progress, best candidate, basic risk/performance summary, and blocked-state reasons.
   - Avoid exposing backend route names unless the user expands technical details.

4. **Validate**
   - Run validation for the selected candidate.
   - Explain pass/fail/uncertain in practical language.
   - Keep detailed validation tables/reports available through Advanced Mode.

5. **Prepare Trading**
   - Check live-execution strategy support.
   - Check active data stream requirements.
   - Check broker credential readiness.
   - Prefer dry-run/paper setup first.
   - Require explicit acknowledgement before any non-dry-run path.

## Architecture Guardrails

Avoid duplicating the existing app.

- Add a UI mode layer, not a second app.
- Reuse existing Pinia stores, request builders, services, artifact stores, and Tauri commands.
- Extract shared readiness/view-model logic where the advanced and guided surfaces need the same answer.
- Keep Guided Mode components thin: they should assemble steps, summaries, and primary actions from existing state.
- Keep Advanced Mode as the authority for full configuration and detailed inspection.
- Do not fork backend request contracts for Guided Mode unless a real backend contract gap is discovered and approved.

Suggested frontend shape:

- `settingsStore`: persisted `uiMode: "guided" | "advanced"` plus a first-run/default policy.
- `workflowStore` or a new guided composable: current guided step and step completion state.
- `useGuidedWorkflowReadiness`: derives step status, primary action, blocked reason, and next destination from existing stores.
- `GuidedWorkspacePanel.vue`: top-level guided route.
- `GuidedDataStep.vue`, `GuidedStrategyStep.vue`, `GuidedOptimizeStep.vue`, `GuidedValidateStep.vue`, `GuidedTradeReadinessStep.vue`: small step components.
- `GuidedVerdictSummary.vue`: plain-language validation summary from existing validation results.
- `GuidedAdvancedBridge.vue`: consistent links into Advanced Mode surfaces.

Candidate files:

- `web/src/App.vue`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/composables/useUiReadiness.ts`
- `web/src/stores/settingsStore.ts`
- `web/src/stores/workflowStore.ts`
- `web/src/services/optimizeService.ts`
- `web/src/services/validationService.ts`
- `web/src/services/executionService.ts`
- `web/src/utils/core/workflowModel.ts`

## Default Mode Policy

To minimize risk, implement the mode switch additively before changing first-launch behavior.

Recommended rollout:

1. Preserve the current Advanced Mode as the default while Guided Mode is being built and verified.
2. Add an explicit shell-level mode switch.
3. After Guided Mode passes browser, accessibility, desktop, and baseline verification, decide whether new installs should default to Guided Mode.
4. Preserve existing users' selected mode through settings migration.

## Phase 0: Capability Inventory And Boundaries

- [x] Confirm all Advanced Mode workspaces remain reachable from the shell.
- [x] Confirm command-palette entries still expose advanced-only surfaces.
- [x] Inventory current conventional optimize, validation, and execution prerequisites from the existing frontend stores/services.
- [x] Confirm the live-execution supported strategy list at the frontend boundary and Rust command boundary.
- [x] Document which guided templates are full-path eligible and which are validate-only.

Exit criteria:

- A short capability map exists in code comments, tests, or this plan before implementation begins.
- No advanced-only backend workflow is listed as part of Guided Mode MVP.

## Phase 1: Mode Model And Shell Integration

- [x] Add a persisted UI mode setting.
- [x] Add a shell-level Guided/Advanced switch.
- [x] Route Guided Mode to a new guided workspace without deleting or renaming Advanced Mode workspaces.
- [x] Keep current titlebar, command palette, settings, and task-center behavior available in Advanced Mode.
- [x] Add unit tests for mode persistence and shell rendering.

Exit criteria:

- Users can switch between Guided and Advanced without losing loaded app state.
- Advanced Mode renders the current workbench with all existing major workspaces.

## Phase 2: Guided Optimization Flow

- [x] Build the guided data step using existing data-store state and actions.
- [x] Build the guided strategy step with curated templates.
- [x] Build the guided optimize step using the existing optimization service and task/progress state.
- [x] Show selected candidate summary with plain labels for return, drawdown, trade count, and robustness signals already available from the optimizer result.
- [x] Add blocked states for missing data, unsupported strategy, missing candidate, or in-progress task.

Exit criteria:

- A user can run a guided conventional optimization without opening the advanced configure panels.
- The same optimization result remains available in Advanced Mode.

## Phase 3: Guided Validation Flow

- [x] Build the guided validation step using the existing validation request builder/service.
- [x] Derive a plain-language verdict from existing validation metrics and report state.
- [x] Show "continue", "revise strategy", or "open advanced validation" as the next action.
- [x] Preserve advanced validation tables, reports, exports, and artifact behavior.

Exit criteria:

- A guided user can validate the selected candidate and understand whether it is worth considering further.
- Advanced Mode still exposes the full validation workspace for the same run.

## Phase 4: Paper/Live Readiness Flow

- [x] Build a readiness checklist for execution support, symbol/data stream, broker credentials, dry-run mode, and risk settings.
- [x] Use existing execution store/service actions for dry-run or live runner setup.
- [x] Gate non-dry-run actions behind explicit warnings and required readiness.
- [x] Explain unsupported strategy handoffs without presenting them as failures.

Exit criteria:

- A supported validated strategy can reach dry-run/paper readiness from Guided Mode.
- Unsupported strategies stop at a clear validate-only boundary.
- Advanced execution controls remain unchanged.

## Phase 5: Advanced Preservation And Bridges

- [x] Add consistent "Open in Advanced Mode" links from Guided steps.
- [x] Ensure Meta, Stage 5 ML confidence, ML Signal, options-expression, campaigns, dynamic builder, history, reports, diagnostics, and settings remain discoverable in Advanced Mode.
- [x] Add regression tests that fail if major advanced workspaces disappear from the shell or command palette.
- [x] Keep advanced-only backend functions out of Guided Mode except for contextual bridge text.

Exit criteria:

- Guided Mode simplifies the first-run path without reducing product capability.
- Advanced Mode remains the canonical complete interface.

## Phase 6: Verification

Run at minimum:

- [x] `npm --prefix web run lint`
- [x] `npm --prefix web run typecheck`
- [x] `npm --prefix web run test`
- [x] `npm --prefix web run build`
- [x] `npm --prefix web run a11y:smoke:managed`
- [x] `npm --prefix web run test:browser:ux`
- [x] `npm --prefix web run baseline:ui`
- [x] `npm --prefix web run test:desktop:smoke`

For any frontend-to-command contract changes, also run the relevant parity checks from `web/package.json`.

## Phase 7: Post-MVP UI Polish

- [x] Make the TradingView Lightweight Charts frame more visually dominant without changing chart runtime contracts.
- [x] Improve heatmap canvas sharpness with responsive HiDPI backing-store sizing.
- [x] Move dense heatmap requirement copy into progressive details.
- [x] Standardize help tooltip timing to appear faster while preserving delayed full help.
- [x] Add browser UX coverage for Guided Mode's focused first-run shell and Advanced Mode handoff.
- [x] Refresh UI baselines after the intentional layout changes.

Exit criteria:

- Guided Mode remains visually streamlined and Advanced Mode remains the complete workbench.
- Heatmap and chart presentation changes are frontend-only and covered by unit, browser, desktop, and baseline checks.

## Acceptance Criteria

- Guided Mode provides a complete optimize -> validate -> paper/live readiness path for at least one supported conventional strategy.
- Advanced Mode keeps all existing backend functionality reachable.
- No Meta, ML Signal, options-expression, campaign, dynamic-builder, diagnostics, or artifact-replay capability is removed or silently hidden from advanced users.
- Guided Mode reuses existing services/stores and does not fork backend request contracts.
- Browser, desktop, accessibility, and baseline verification pass.

## Risks And Mitigations

| Risk | Mitigation |
| --- | --- |
| Guided Mode becomes a duplicated UI | Keep guided components thin and reuse stores/services/request builders |
| Advanced features become harder to find | Add explicit Advanced Mode bridges and command-palette preservation tests |
| Users assume validation means live readiness | Split validation verdict from execution readiness and show both |
| Users select a strategy that cannot be traded live | Filter guided full-path templates to live-supported strategies or mark validate-only clearly |
| Mode switch corrupts workflow state | Store mode separately from run state and add persistence tests |
| First-launch default disrupts existing users | Roll out mode switch first, then decide new-install default separately |
