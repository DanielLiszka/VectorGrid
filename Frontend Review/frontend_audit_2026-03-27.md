# Frontend Audit

Date: 2026-03-27

Scope: Vue 3 frontend only under `web/`. No backend, Rust, CUDA, or Tauri command implementation changes were made during this review.

## Review Method

- Reviewed the current frontend structure via `REPO_MAP.md`, `web/README.md`, and the Vue shell/panel layout.
- Used the `build-web-apps:web-design-guidelines` skill as the primary review rubric.
- Used multiple sub-agents to audit separate page groups in parallel:
  - shell, overlays, and workflow architecture
  - authoring/data/strategy pages
  - results/analysis pages
  - history/settings/execution/shared styling
- Ran browser-side inspection against a local Vite dev server with a mocked Tauri runtime and captured screenshots under `output/playwright/ui-review/`.
- Ran frontend verification commands:
  - `npm run test`
  - `npm run typecheck`

## Executive Summary

The frontend is functional and structurally coherent, but it is carrying four major classes of debt:

1. build and test health is not fully green
2. primary interaction surfaces are still mouse-first rather than accessible desktop UI
3. execution and destructive actions are exposed with weak safety UX
4. the visual system is consistent enough to ship, but too compressed and not yet intentional enough to feel like a designed product

There is no canonical Figma design or design system to validate against today. Figma would be useful in the next phase to define a target design system and replacement comps, not to score the current UI against an existing source of truth.

## Current Remediation Status

The findings below capture the original audit snapshot. The codebase has moved materially since that review. As of the latest remediation pass:

- frontend verification is green again:
  - `npm run typecheck` passed
  - `npm run test` passed with 68 files and 480 tests
  - `npm run build` passed
  - `npm run baseline:ui` passed
- shell workspace and workflow-step navigation now use accessible tab semantics with keyboard support and an explicit live selection summary
- the command palette now exposes combobox/listbox semantics with an announced active option instead of a purely visual selection model
- the task center now ships as an explicitly mounted, named panel with focus handoff and focus restoration on close
- TaskCenterOverlay and context docks are mounted explicitly in the shipped shell
- the run footer is constrained to workflow contexts
- warning-state tokens are defined in the shared theme
- heavy panels now use active-only mounting rather than staying alive indefinitely after first visit
- validation now has stronger section hierarchy and keeps raw JSON behind disclosure instead of treating it like a first-class pane
- overview and top-results now use the newer header/section hierarchy with summary chips and less inline raw detail
- the dev workflow quick-switch now exposes grouped selection semantics with keyboard movement instead of only visual active states
- shared panel typography utilities now centralize page titles, section titles, metadata text, status/support copy, and metric key/value styling across the main workspace panels
- Drill, Pareto, Heatmap, and Debug now use that same shared panel type system instead of carrying separate local title/meta/status rules
- repeated workspace and dock timestamp rendering now uses a shared UI time formatter instead of per-panel `toLocaleTimeString` copies
- shared UTC date/count formatting now backs the repeated service/store helpers that previously duplicated `toISOString().slice(0, 10)` and `toLocaleString()` logic
- simple shell Teleport surfaces now use a shared mount helper instead of each panel hand-rolling the same `nextTick + querySelector + dataset flag` bootstrap
- Strategy and Advanced now render directly from the Vue-owned surface and no longer carry their stale compat-host passthrough bridges
- disclosure summaries and scrollable table wrappers now have explicit shared focus-visible treatment
- the execution review pane now has clearer internal structure for runtime status, chart context, and broker-order review
- Top Results rows and Run Library lists now support Arrow/Home/End keyboard movement in addition to activation with Enter/Space
- the Top Results table now uses grid-style row selection semantics and announces the current selection through a live status region
- cached Polygon dataset cards now expose selection state semantically and announce the active cached dataset programmatically
- history and execution confirmations now use modal alert dialogs with focus management instead of inline warning notices
- passive chart wrappers no longer create dead-end focus stops in Drill, Pareto, and Heatmap
- chart-heavy panes now describe the keyboard-safe Top-results path explicitly instead of relying on click-only instructions
- the task center now has an explicit close action, Escape dismissal, and an internal keyboard loop in addition to focus handoff and restoration
- shared typography, spacing, and intent-ink tokens now live in `app.css`, and a repo-wide CSS variable audit found no undefined token references in `web/src/*.vue` or `web/src/*.css`
- a canonical repo-local design-system spec and target-comp set now exist under `Frontend Review/`, so Phase 10 is no longer blocked on Figma auth
- browser-side audits across the major workspace states now show no unlabeled form controls and no heading-level jumps in the rendered main panels
- the densest Advanced ranking diagnostics now move route-behavior guidance behind disclosure summaries instead of always competing with the primary controls
- rendered browser checks now confirm shared focus-visible treatment on both buttons and checkboxes, and confirm the task-center dialog closes on Escape

The remaining work is now optional follow-up rather than checklist debt: bundle-size reduction, any later Figma mirroring once connector access is restored, and future visual refinement beyond the current implementation target.

## Consolidated Findings

Note: this section reflects the audit baseline from the initial review pass. Use the remediation status above and the checklist below as the source of truth for what is still open.

### High

#### 1. Frontend build health is currently broken

The frontend does not typecheck or build from a clean state. `validationRequestBuilder.ts` uses `String.prototype.replaceAll`, but the current TS/lib target does not provide it.

References:

- `web/src/services/validationRequestBuilder.ts:213`
- `web/tsconfig.app.json`

Observed verification:

- `npm run typecheck` fails with `TS2550`
- `npm run build` also fails on the same issue

#### 2. Core navigation and selection surfaces are still mouse-first

The app shell, result tables, and several selection surfaces are implemented as visually tabbed or selectable controls without full tab or keyboard semantics. This is the single biggest accessibility gap in the current frontend.

References:

- `web/src/components/VueWorkspaceShell.vue:278`
- `web/src/components/VueWorkspaceShell.vue:323`
- `web/src/components/VueWorkspaceShell.vue:366`
- `web/src/components/WorkflowQuickSwitch.vue:75`
- `web/src/components/TopWorkspacePanel.vue:250`
- `web/src/components/TopWorkspacePanel.vue:266`
- `web/src/components/ValidationWorkspacePanel.vue:893`
- `web/src/components/MetaWorkspacePanel.vue:509`
- `web/src/components/MetaWorkspacePanel.vue:621`

Concrete issues:

- shell tab rows are plain button groups rather than `tablist` / `tab` / `tabpanel`
- Top Results sorting and row selection are click-only
- validation cross-dataset and meta batch selection use clickable container rows with weak semantics
- there is no consistent keyboard interaction model for dense desktop workflows

#### 3. Execution safety UX is too weak for risky actions

The execution surface exposes sensitive actions with the same visual weight as normal controls and without confirmation or step-up UX.

References:

- `web/src/components/ExecutionWorkspacePanel.vue:330`
- `web/src/stores/executionStore.ts:391`
- `web/src/stores/executionStore.ts:427`
- `web/src/components/OverviewWorkspacePanel.vue:79`

Concrete issues:

- `Clear keys` executes immediately
- `Start live` and `Start runner` are one-click actions
- the overview CTA labeled as execution review actually routes users back to workflow strategy setup instead of the execution/settings surface

#### 4. Some intended frontend UI is not mounted in the shipped shell

The shell currently mounts the main panel surfaces, but not `TaskCenterOverlay` or the various `*ContextDock` components even though those components exist and are tested.

References:

- `web/src/App.vue:128`
- `web/src/components/TaskCenterOverlay.vue:267`
- `web/src/components/AnalysisContextDock.vue:95`
- `web/src/components/ValidationContextDock.vue:68`
- `web/src/components/ExecutionContextDock.vue:66`

Impact:

- dead UI exists in the codebase
- test coverage overstates what the real shell currently renders
- contributors need to reason about surfaces that are present in source but absent in the shipped app

### Medium

#### 5. Warning-state styling depends on an undefined design token

Warning callouts use `var(--warning)` even though the shared theme never defines that token.

References:

- `web/src/components/OverviewWorkspacePanel.vue:247`
- `web/src/components/StrategyWorkspacePanel.vue:2981`
- `web/src/styles/app.css`

Impact:

- warning/risk banners can render with degraded or inconsistent emphasis
- the design token system is incomplete

#### 6. The run footer is always present, even in contexts where it dilutes page intent

The run-control footer remains visible in Library, Pro, and Settings contexts and still shows optimization controls there.

References:

- `web/src/App.vue:144`
- `web/src/components/RunFooterPanel.vue:152`

Observed in browser sweep:

- the footer remains present in the screenshots for Library, Pro, and Settings
- it weakens the distinction between page purposes and creates misleading affordances

#### 7. Panels remain mounted after first visit, which is a likely long-session performance drag

The frontend lazily mounts panels once and then keeps them mounted for the rest of the session.

References:

- `web/src/App.vue:83`
- `web/src/App.vue:101`
- `web/src/App.vue:133`

Impact:

- hidden charts, tables, watchers, observers, and teleports stay alive off-screen
- memory and responsiveness risk grows with session length and page coverage

#### 8. History and Run Library UX is fragile at scale

The library flow depends heavily on native `<select>` controls for browsing artifact sets, while destructive actions dispatch immediately.

References:

- `web/src/components/HistoryWorkspacePanel.vue:254`
- `web/src/components/HistoryWorkspacePanel.vue:307`
- `web/src/components/HistoryWorkspacePanel.vue:296`
- `web/src/components/HistoryWorkspacePanel.vue:411`
- `web/src/stores/historyStore.ts:874`
- `web/src/stores/historyStore.ts:979`

Impact:

- long lists are hard to scan and compare
- metadata is pushed outside the selection surface
- clear/delete actions are too easy to trigger accidentally

#### 9. Status and async feedback are not announced consistently

Important progress and state changes are often rendered as plain text without `aria-live`.

References:

- `web/src/components/OverviewWorkspacePanel.vue:211`
- `web/src/components/HistoryWorkspacePanel.vue:500`
- `web/src/components/ValidationWorkspacePanel.vue:996`
- `web/src/components/ExecutionWorkspacePanel.vue:349`
- `web/src/components/RunFooterPanel.vue:186`

Impact:

- background workflow changes are easy to miss
- screen-reader users do not get consistent async status announcements

#### 10. Dense authoring pages are too compressed for the amount of decision-making they require

The Strategy and Advanced surfaces pack a lot of controls into long, low-relief forms with limited sectional hierarchy.

References:

- `web/src/components/StrategyWorkspacePanel.vue:1035`
- `web/src/components/AdvancedWorkspacePanel.vue:873`
- `web/src/components/AdvancedWorkspacePanel.vue:933`

Impact:

- high scan cost
- route-policy explanations compete with actual inputs
- important decisions get buried below the fold

#### 11. Chart-heavy analysis pages are visually strong but weakly described

Drill, Pareto, Heatmap, and execution-chart surfaces rely on canvases and pointer-first interaction cues with limited textual fallback.

References:

- `web/src/components/DrillWorkspacePanel.vue:71`
- `web/src/components/ParetoWorkspacePanel.vue:165`
- `web/src/components/HeatmapWorkspacePanel.vue:81`
- `web/src/components/ExecutionWorkspacePanel.vue:355`

Impact:

- non-visual inspection is weak
- exact-value exploration is less robust than it should be
- keyboard-only workflows are incomplete

#### 12. The shell/panel mounting architecture is more brittle than it needs to be

The frontend uses Teleport plus manual DOM lookup and dataset markers across many panels.

References:

- `web/src/components/TopWorkspacePanel.vue:142`
- `web/src/components/ValidationWorkspacePanel.vue:357`
- `web/src/components/MetaWorkspacePanel.vue:458`
- `web/src/components/HistoryWorkspacePanel.vue:184`
- `web/src/components/VueWorkspaceShell.vue:337`

Impact:

- layout changes are harder
- accessibility fixes are harder
- test ergonomics are worse
- surface ownership is more difficult to reason about

#### 13. Frontend verification is drifting from current behavior

The frontend test suite is not fully green, and some tooling still targets older shell structures.

References:

- `web/src/tauri/commands.test.ts:6`
- `web/src/utils/strategyOptimizationCapabilities.test.ts:95`
- `web/scripts/capture-baseline-ui.mjs:154`
- `web/src/components/VueWorkspaceShell.test.ts:29`

Observed verification:

- `npm run test` currently ends with 2 failing files
- the checked-in baseline capture script still tries to drive `#workflowTabBar`, which no longer exists
- service tests emit repeated Pinia `activePinia` warnings, which suggests store usage patterns that are too dependent on global active state in tests

### Low

#### 14. Heading semantics are inconsistent across the app

The shell uses a real heading, but many page titles are plain `div.title` elements.

References:

- `web/src/components/VueWorkspaceShell.vue:272`
- `web/src/components/TopWorkspacePanel.vue:170`
- `web/src/components/ValidationWorkspacePanel.vue:385`
- `web/src/components/HistoryWorkspacePanel.vue:226`
- `web/src/components/SettingsWorkspacePanel.vue:131`

Impact:

- weaker heading navigation for assistive tech
- weaker page structure in a dense multi-panel application

#### 15. Focus styling is incomplete

The app removes native outlines and replaces them for some input types, but not all real controls in use.

References:

- `web/src/styles/app.css:616`
- `web/src/styles/app.css:619`
- `web/src/components/DataWorkspacePanel.vue:321`
- `web/src/components/DataWorkspacePanel.vue:404`

Impact:

- some controls such as password/date-style inputs can end up with inconsistent visible focus treatment

#### 16. Copy, formatting, and visual rhythm still read as utilitarian rather than intentional

The frontend is consistent enough to use, but the typography, density, and copy polish are still closer to an internal tool than a designed desktop product.

References:

- `web/src/styles/app.css:98`
- `web/src/styles/app.css:602`
- `web/src/styles/app.css:712`
- `web/src/components/VueWorkspaceShell.vue:463`
- `web/src/components/VueWorkspaceShell.vue:492`
- `web/src/components/CommandPaletteOverlay.vue:396`
- `web/src/components/HistoryWorkspacePanel.vue:315`
- `web/src/components/TaskCenterOverlay.vue:326`

Impact:

- visual hierarchy is weak on dense pages
- some copy still reads as placeholder/debug UI
- the app lacks a strong product-level UI voice

## Verification Notes

### Browser-side review

A temporary Vite server was started locally and reviewed through Playwright against a mocked Tauri runtime. Screenshots were captured under:

- `output/playwright/ui-review/workflow-default.png`
- `output/playwright/ui-review/workflow-optimize.png`
- `output/playwright/ui-review/workflow-optimize-top.png`
- `output/playwright/ui-review/workspace-library.png`
- `output/playwright/ui-review/workspace-aggregate.png`
- `output/playwright/ui-review/workspace-pro.png`
- `output/playwright/ui-review/workspace-settings.png`

The shell rendered without console errors or page exceptions during that pass, but the screenshots confirmed:

- shell workspace and workflow-step bars now expose tab semantics, and shell selection changes are announced through a live status summary
- the command palette now uses combobox/listbox semantics, and the task center returns focus to its launcher on close
- the workflow-only footer behavior now holds across Library, Aggregate Analysis, Pro, and Settings
- dense authoring surfaces have stronger sectioning and summary treatment, but the deepest strategy-specific forms are still compact
- overview and top-results now read more consistently with the newer summary-chip and section-header treatment
- chart-heavy views now expose clearer non-pointer action paths, and the heaviest raw debug/output payloads are now tucked behind explicit disclosure
- the validation surface now has stronger section hierarchy, summary chips, and collapsed raw output instead of a competing debug pane
- rendered browser checks also confirm the task-center dialog closes on Escape and that both buttons and checkboxes pick up the shared focus-visible ring

### Command results

- `npm run typecheck`
  - passed
- `npm run test`
  - 68 test files passed
  - 480 tests passed
  - repeated Pinia `activePinia` warnings were eliminated from the current service/store test path
- `npm run build`
  - passed
  - Vite still reports a large main-bundle chunk warning at roughly `706 kB`
- `npm run baseline:ui`
  - passed
  - baseline screenshots and metrics were refreshed under `scripts/baselines/ui_capture`

## Full Implementation Checklist

Use this as the implementation checklist for a full frontend remediation pass.

### Phase 1: Build and Verification Health

- [x] Replace or polyfill `replaceAll` usage so `npm run typecheck` passes on the current TS/lib target.
- [x] Restore a clean `npm run build`.
- [x] Fix `web/src/tauri/commands.test.ts` so command registry expectations match current generated commands.
- [x] Fix `web/src/utils/strategyOptimizationCapabilities.test.ts` so route-preview expectations match current behavior.
- [x] Eliminate repeated Pinia `activePinia` warnings in service/store tests.
- [x] Update browser capture tooling to current shell structure, including removal of the stale `#workflowTabBar` assumption.
- [x] Re-run and document:
  - [x] `npm run test`
  - [x] `npm run typecheck`
  - [x] `npm run build`

### Phase 2: Navigation and Interaction Accessibility

- [x] Convert shell tab rows to accessible tab patterns:
  - [x] workspace tabs
  - [x] workflow step tabs
  - [x] controls tabs
  - [x] results tabs
- [x] Add `role="tablist"`, `role="tab"`, `aria-selected`, and correct tabpanel relationships where appropriate.
- [x] Add keyboard navigation for tab groups with arrow keys, Home, and End.
- [x] Refactor Top Results sorting and row selection to keyboard-accessible controls.
- [x] Add `aria-sort` to sortable columns.
- [x] Add Arrow/Home/End keyboard movement for Top Results rows and Run Library list items.
- [x] Refactor validation cross-dataset selection to proper checkbox list semantics.
- [x] Refactor meta batch selection to proper checkbox or listbox semantics.
- [x] Ensure all selection states are visible and announced programmatically.

### Phase 3: Execution Safety

- [x] Add confirmation or step-up UX for:
  - [x] `Clear keys`
  - [x] `Start live`
  - [x] `Start runner`
- [x] Visually separate destructive/risky actions from routine actions.
- [x] Rework the overview CTA so it routes to the actual execution/settings surface.
- [x] Add explicit live-mode state treatment so paper and live modes are visually and semantically distinct.
- [x] Add guardrail copy for live/trading-impacting actions.
- [x] Consider split action groups:
  - [x] account/setup actions
  - [x] paper actions
  - [x] live actions
  - [x] destructive actions

### Phase 4: Shell and Information Architecture

- [x] Decide whether `TaskCenterOverlay` and the `*ContextDock` surfaces should ship.
- [x] If yes, mount them explicitly in `App.vue`.
- [x] If no, remove dead UI and stale tests.
  Not applicable after the explicit ship decision to keep mounted overlays/docks.
- [x] Re-evaluate whether the run footer should appear in:
  - [x] workflow
  - [x] library
  - [x] aggregate
  - [x] pro
  - [x] settings
- [x] Hide or restyle run-control affordances in contexts where they are not primary.
- [x] Standardize heading structure across all major panels.
- [x] Replace major `div.title` surfaces with real heading semantics.

### Phase 5: Visual System and Design Tokens

- [x] Define a complete shared semantic token set, including warning-state tokens.
- [x] Audit all token usage for undefined variables.
- [x] Centralize shared panel typography utilities in `app.css` for:
  - [x] page titles
  - [x] section titles
  - [x] metadata labels
  - [x] status/support text
  - [x] metric key/value summaries
- [x] Create stronger typography hierarchy for:
  - [x] page titles
  - [x] section titles
  - [x] metadata labels
  - [x] status text
- [x] Increase legibility of dense surfaces by reducing overuse of sub-0.8rem text.
- [x] Review spacing/rhythm on:
  - [x] Strategy
  - [x] Advanced
  - [x] Validation
  - [x] History
  - [x] Settings
- [x] Remove placeholder-feeling copy and replace with product-quality text.
- [x] Centralize repeated UI timestamp formatting with a shared helper.
- [x] Centralize remaining date/time/number formatting with shared `Intl` helpers.
  Generic UI timestamps, counts, bytes, durations, and selection summaries now use shared helpers. Domain-specific analysis strings remain local where they encode metric semantics rather than generic UI formatting.

### Phase 6: Dense Page UX

- [x] Rework Strategy page sectional hierarchy so input clusters are easier to scan.
- [x] Move long route-policy/help explanations into expandable help, tooltips, or side notes instead of inline blocks.
- [x] Rework Advanced page so explanation density does not crowd primary controls.
- [x] Rebuild Run Library browsing around richer list/table surfaces instead of relying mainly on `<select>`.
- [x] Add clearer compare/detail affordances for saved runs.
- [x] Add confirmation or undo flows for history clear/delete actions.

### Phase 7: Charts, Results, and Status Surfaces

- [x] Add stronger textual fallback and summaries for chart-heavy pages:
  - [x] Drill
  - [x] Pareto
  - [x] Heatmap
  - [x] Execution chart
- [x] Add keyboard-accessible alternate action paths where interaction is currently pointer-only.
- [x] Improve empty/loading/error-state messaging for chart panels that depend on imperative renderer handoff.
- [x] Add `aria-live` for important async status surfaces.
- [x] Demote or collapse raw JSON/debug dumps so they do not dominate production workflows.
- [x] Revisit fixed docks so they do not obscure controls on smaller windows.

### Phase 8: Architecture and Performance Hygiene

- [x] Reassess the Teleport-plus-manual-mount architecture.
- [x] Decide whether panels should unmount on exit or use `KeepAlive` selectively rather than indefinitely.
- [x] Audit hidden-panel watchers, chart instances, and subscriptions for off-screen cost.
- [x] Introduce a shared shell-mount helper for simple Teleport panels and dataset-flag surfaces.
- [x] Reduce reliance on DOM query selectors and dataset markers for basic layout ownership.
- [x] Make surface ownership more explicit so page-level refactors are safer.

### Phase 9: Accessibility Completion Pass

- [x] Audit all form fields for programmatic labels.
- [x] Audit all controls for visible focus treatment.
- [x] Add focus treatment for input types not currently covered by the CSS reset.
- [x] Add shared focus-visible treatment for disclosure summaries and scrollable table wrappers.
- [x] Audit modal/dialog semantics and focus trapping.
- [x] Audit heading order across all rendered panels.
- [x] Audit color contrast of muted/status text after token cleanup.
- [x] Remove dead-end keyboard focus from passive chart-surface wrappers.
- [x] Audit keyboard-only completion of the primary workflows:
  - [x] data load
  - [x] optimization setup
  - [x] validation setup
  - [x] results review
  - [x] saved run review
  - [x] execution settings

### Phase 10: Design System / Figma Follow-up

- [x] Create a canonical frontend design system or Figma source of truth.
- [x] Define:
  - [x] semantic color roles
  - [x] typography scale
  - [x] spacing scale
  - [x] component states
  - [x] desktop layout rules
- [x] Produce target comps for:
  - [x] shell
  - [x] strategy authoring
  - [x] results review
  - [x] history/library
  - [x] execution/settings
- [x] Align implementation to those target comps after structural issues above are fixed.

## Recommended Implementation Order

1. build/test health
2. shell and selection accessibility
3. execution safety
4. history/library destructive-action UX
5. token cleanup and visual hierarchy
6. dense page restructuring
7. chart/status accessibility
8. panel architecture/perf cleanup
9. design system/Figma formalization
