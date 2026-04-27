# Premium Desktop UI/UX Implementation Plan

Last updated: 2026-04-26

## Purpose

This document is the next frontend/UI planning slice after `docs/frontend_ui_ux_redesign_plan.md` and the automated browser/desktop QA work.

The current UI is credible for an internal quant workstation, but it is not yet competitive with the highest-polish desktop applications. The main issue is not missing backend capability. The issue is that too much state, navigation, and implementation structure is visible at once. The app needs a more deliberate desktop information architecture: fewer persistent rails of chrome, clearer workflow progression, a real inspector model, stronger command access, and higher-quality dense data surfaces.

This plan turns that into implementation work for the Vue 3/Tauri frontend while preserving the backend command surface.

## Implementation Status

2026-04-26 first implementation slice is complete; the workbench shell is now the production shell:

- Added `WorkbenchShell` as the desktop shell.
- Preserved the existing panel mount IDs, shell selectors, and QA action IDs so current workspace panels can teleport into the production shell.
- Added a workbench rail, condensed header, inspector, and footer action-bar host.
- Replaced floating context docks with the workbench inspector and action bar, avoiding duplicated context.
- Added model tests, component tests, selector coverage, action-coverage manifest entries, and Playwright workflow coverage.

2026-04-26 second implementation slice is also complete on the workbench shell:

- Added `ResearchCockpit` as the Research workbench overview, backed by `useResearchCockpitModel`.
- Connected the cockpit to the existing workflow store, readiness model, primary-action model, and task lifecycle summary instead of introducing a parallel workflow state.
- Expanded `WorkbenchInspector` from a placeholder into mode-specific context for selection, validation, history, tasks, and diagnostics.
- Added inspector action buttons for opening Data/Optimize/Analyze, jumping to Validation, opening the Task Center, and running existing History actions.
- Added roving keyboard behavior for inspector mode tabs.
- Added workbench QA selectors, action-coverage manifest entries, unit tests, and browser E2E coverage for cockpit steps, inspector modes, inspector actions, and responsive inspector visibility.
- Verified the workbench visually at `1366x768` and `390x844` from Playwright screenshots.

2026-04-26 dense data-surface slice is complete across the production workbench shell:

- Added a shared `AppTableDensityControl` segmented control and `usePersistedTableDensity` composable.
- Added a table-shell density contract to `AppDataTableShell`, including `data-table-density`, toolbar labeling, and compact/comfortable/spacious row-density classes.
- Wired persistent density controls into Top results, Validation results, and the History saved-run library.
- Moved History saved-run search/tag controls into the shared table-shell toolbar contract so saved-run filtering and table density are handled in one place.
- Added unit coverage for the density composable/control and extended the Validation browser workflow to click the density control and assert the shell state changes.

2026-04-26 table/command/native hardening slice is complete:

- Added persistent named view presets for Top results and Validation results.
- Extended persisted matrix column state with `columnOrderKeys` so saved/preset views can reorder columns without corrupting row-cell alignment.
- Extended persisted matrix column state with sanitized `columnWidths`, resize helpers, and reset/restore participation.
- Added source-column-index handling to the headless matrix table adapter and regression coverage for reordered columns.
- Added sortable Top/Validation table headers, client-side Validation sorting, persisted resizable column widths, pointer resize handles, and keyboard column resize controls.
- Added shared `AppDataTableShell` keyboard-grid navigation for slotted table surfaces, including Arrow/Home/End/PageUp/PageDown movement and scroll-into-view behavior.
- Added a selected-row detail inspector for Validation results, with row selection, keyboard activation, and QA selectors.
- Updated Validation selected-row detail and bounds logic to honor the sorted row model instead of raw input order.
- Added a central `shellCommandRegistry` while keeping the old shortcut-registry import path as a compatibility wrapper.
- Added a Tauri native menu that emits `shell-command` events for shared app commands, and routed those events through the same frontend command actions used by keyboard shortcuts and the command palette.
- Added frontend-to-Tauri native menu enabled-state synchronization through `set_shell_menu_command_states`, so disabled shared commands are reflected in the desktop menu while frontend guards remain authoritative.
- Added the missing workbench inspector History clear-session action.
- Added empty-library History actions so refresh/open-library remain reachable before any saved artifacts exist.
- Fixed leaked global sticky `header` styling so nested panel headers cannot intercept clicks across adjacent scroll lanes.
- Hardened the workbench shell with unit coverage for app-level shell routing.
- Extended browser regression coverage to click validation table view presets, verify the selected-row detail panel, exercise keyboard-only data-to-validation flow, and cover data-load, GPU-status, options-validation input, and options-validation preview failure paths.
- Updated layout regression coverage for the workbench inspector and updated the desktop smoke harness so WDIO accepts the workbench labels.

2026-04-26 final productization hardening slice is complete:

- Added `scripts/run_frontend_e2e_gate.ps1`, build-once Playwright `:built` scripts, repo-quality opt-in switches, and release-candidate browser QA wiring so the browser workbench/action/core-workflow lane is repeatable without rebuilding for every spec group, with responsive/regression/exploratory slices added explicitly.
- Added `typecheck:desktop` and made `-IncludeDesktopTypecheck` execute the WDIO/Tauri harness typecheck instead of accepting a skipped placeholder.
- Added release-candidate packaged desktop smoke wiring through `scripts/run_packaged_smoke.ps1` when `-IncludeDesktopSmoke` is requested.
- Added native `Cancel Run`, startup-disabled defaults for unavailable native menu entries, and a checked `Show Controls Panel` menu item synced from Vue state.
- Brought the Meta results table into the shared persisted table contract by applying saved column order and persisted resizable widths while preserving its backend-defined row order.
- Added browser regression coverage for Validation header sort plus keyboard resize, and desktop-width browser layout coverage for the workbench inspector/action-bar geometry.

2026-04-26 previous shell retirement slice is complete:

- Removed the previous Vue shell implementation, shell toggle code, explicit query/storage selector, and fallback-only tests.
- Removed the floating Analysis, Optimize, Validation, and History side panels after their run, validation, history, task, and diagnostics actions were confirmed in the workbench inspector/action bar or existing panels.
- Removed legacy-only browser scenarios and layout assertions.
- Updated repository routing docs and accessibility-review guidance to point future shell work at `WorkbenchShell`.

Verification completed for the current workbench implementation:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run typecheck:desktop`
- `npm --prefix web run test -- src/utils/inspectorModel.test.ts src/components/workbench/WorkbenchInspector.test.ts src/composables/useResearchCockpitModel.test.ts src/components/workbench/ResearchCockpit.test.ts src/components/workbench/WorkbenchShell.test.ts`
- `npm --prefix web run test -- src/components/ui/AppDataTableShell.test.ts src/components/ui/AppTableDensityControl.test.ts src/composables/usePersistedTableDensity.test.ts src/components/TopWorkspacePanel.test.ts src/components/ValidationWorkspacePanel.test.ts src/components/HistoryWorkspacePanel.test.ts`
- `npm --prefix web run test -- src/composables/usePersistedMatrixColumnState.test.ts src/utils/shellCommandRegistry.test.ts src/components/GlobalShortcutBridge.test.ts src/components/CommandPaletteOverlay.test.ts src/components/MetaWorkspacePanel.test.ts`
- `npm --prefix web run test`
- `npm --prefix web run test:e2e:workbench`
- `npm --prefix web run test:e2e:action-coverage`
- `npm --prefix web run test:e2e:responsive`
- `npm --prefix web run test:e2e:regressions`
- `npm --prefix web run test:e2e:workflow`
- `cargo check -p ta_desktop_demo_app`
- `cargo test -p ta_desktop_demo_app --lib desktop_menu --jobs 1`
- `npm --prefix web run test:desktop:smoke`

Remaining roadmap items are still intentionally open:

- Extend sortable/resizable/grid behavior beyond Top, Validation, History, and Meta where future table surfaces need the same workstation-grade interaction contract.
- Continue row-level action standardization for any table surfaces beyond the current Top/Validation/History scope.
- Add native desktop polish beyond the current menu bridge, especially tray/window-affordance decisions and any additional menu entries whose checked/enabled state depends on deeper runtime state.
- Keep shell parity documentation current when future backend commands or workspace panels are added.

## Research Basis

The recommendations below are based on current product/UI research and platform guidelines reviewed on 2026-04-26:

- Tauri positions itself around small, fast apps that use the operating system WebView instead of bundling a browser engine. That makes frontend polish and cross-WebView visual QA important, because rendering can vary by platform.
  - Source: <https://tauri.app/start/>
  - Source: <https://github.com/tauri-apps/awesome-tauri>
- Apple Human Interface Guidelines treat sidebars as a strong pattern for top-level app areas, recommend compact alternatives when space is limited, and recommend split views when hierarchy becomes deeper than two levels.
  - Source: <https://developer.apple.com/design/human-interface-guidelines/sidebars>
- Microsoft Fluent guidance emphasizes using layout to create relationships between components, highlight the most important information, and help users make decisions comfortably. Current Microsoft app guidance also favors streamlined navigation, condensed headers, and compact commands.
  - Source: <https://fluent2.microsoft.design/layout>
  - Source: <https://learn.microsoft.com/en-us/power-apps/user/modern-fluent-design>
  - Source: <https://learn.microsoft.com/en-us/windows/apps/design/controls/command-bar-flyout>
- Material adaptive-layout guidance separates navigation, body, and system regions and recommends changing navigation treatment by window size. Navigation rails are positioned as a medium/expanded-screen top-level destination pattern, typically for a small set of core destinations.
  - Source: <https://developer.android.com/codelabs/adaptive-material-guidance>
  - Source: <https://developer.android.com/reference/kotlin/androidx/compose/material3/NavigationRail.composable>
  - Source: <https://developer.android.com/develop/ui/views/layout/build-responsive-navigation>
- Carbon's data-table guidance is relevant for VectorGrid's dense result surfaces: table toolbars should hold global actions, sorting/search/filtering/settings belong in a consistent toolbar, row expansion supports progressive disclosure, and inline row actions should be visible when the action count is small.
  - Source: <https://v10.carbondesignsystem.com/components/data-table/usage/>
  - Source: <https://v10.carbondesignsystem.com/patterns/filtering/>
- WAI-ARIA APG guidance matters because this app uses custom tabs, overlays, command palette, and dense grids. Tabs should follow roving focus patterns, automatic activation is best only when panels are already present and instant, modal dialogs should trap focus and restore focus on close, and true data grids need explicit keyboard behavior.
  - Source: <https://www.w3.org/WAI/ARIA/apg/patterns/tabs/examples/tabs-automatic/>
  - Source: <https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/>
  - Source: <https://www.w3.org/WAI/ARIA/apg/patterns/grid/>
- WCAG 2.2 target-size guidance sets a practical floor for compact desktop controls: pointer targets should be at least 24 by 24 CSS pixels unless spacing exceptions apply. Compact density is acceptable, but tiny unlabeled targets need spacing and accessible names.
  - Source: <https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum>
- Nielsen Norman Group's usability heuristics remain directly applicable: system status must be visible, labels should match user language, consistency matters, recognition beats recall, and irrelevant information competes with relevant information.
  - Source: <https://media.nngroup.com/media/articles/attachments/Heuristic_Summary_A4_compressed.pdf>

### Second Research Pass Additions

Additional research reviewed on 2026-04-26 expanded the plan from "better layout polish" into a more explicit desktop workbench architecture:

- Tauri's desktop APIs support native-facing polish that the current plan should treat as product scope, not later garnish: custom titlebar behavior, drag regions, native window menus, menu item state, and system tray affordances.
  - Source: <https://v2.tauri.app/learn/window-customization/>
  - Source: <https://v2.tauri.app/learn/window-menu/>
  - Source: <https://v2.tauri.app/learn/system-tray/>
- Microsoft WinUI guidance is directly relevant to a Tauri Windows build. Custom titlebars must preserve drag/caption behavior, global search can live in a taller titlebar, NavigationView adapts across left/top/minimal states, and titlebar controls should not collide with navigation controls.
  - Source: <https://learn.microsoft.com/en-us/windows/apps/design/basics/titlebar-design>
  - Source: <https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/title-bar>
  - Source: <https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/navigationview>
- VS Code's workbench is a useful benchmark because it solves the same class of problem: a complex local desktop tool with a rail, sidebars, editor/work area, panel, status bar, command palette, and user-customizable visibility. Its public UX guidelines caution against overusing status bar items, encourage clear Activity Bar labels/icons, and make commands reachable from the command palette.
  - Source: <https://code.visualstudio.com/api/ux-guidelines/overview>
  - Source: <https://code.visualstudio.com/api/ux-guidelines/activity-bar>
  - Source: <https://code.visualstudio.com/api/ux-guidelines/status-bar>
  - Source: <https://code.visualstudio.com/docs/getstarted/userinterface>
- JetBrains IDEs reinforce the same desktop pattern: tool windows are dockable side/bottom panes, tool-window bars provide compact access, context menus expose local actions, and the status bar carries current state, progress, and configurable widgets.
  - Source: <https://www.jetbrains.com/help/rider/Guided_Tour_Around_the_User_Interface.html>
  - Source: <https://plugins.jetbrains.com/docs/intellij/ui-overview.html>
- Carbon's UI shell guidance adds a useful distinction between global navigation, local navigation, and product tasks. It also explicitly recommends preserving/restoring page state when users move through shell navigation, using left panels when secondary navigation is frequent, and limiting deeper left-nav hierarchy.
  - Source: <https://carbondesignsystem.com/patterns/global-header/>
  - Source: <https://carbondesignsystem.com/components/UI-shell-left-panel/usage/>
- Carbon filtering guidance adds an important rule for this app's slow or expensive result tables: batch filters are better when users make several selections or when every filter change would cause noticeable data reload cost. Hidden filters need a visible applied-count indicator and a clear reset path.
  - Source: <https://carbondesignsystem.com/patterns/filtering/>
- Apple list/table guidance confirms that multicolumn productivity tables should support clear headings, sort by column heading when valuable, column resizing, readable truncation, persistent selection feedback where it represents navigation, and alternating row backgrounds where they help wide-table scanning.
  - Source: <https://developer.apple.com/design/human-interface-guidelines/lists-and-tables>
- Material adaptive-layout guidance reinforces that responsive design should not simply scale a fixed page. It should switch navigation affordances by window class, use canonical list-detail/supporting-pane layouts, and let large layouts show multiple destinations or panes at once.
  - Source: <https://developer.android.com/codelabs/adaptive-material-guidance>
  - Source: <https://developer.android.com/jetpack/androidx/releases/compose-material3-adaptive>
- WAI-ARIA APG guidance makes the keyboard contract concrete. Data grids need explicit cell focus and arrow-key behavior; toolbars should use roving focus when grouping 3 or more controls; modal dialogs must trap focus, close on Escape, and restore focus; tabs should activate automatically only when the panel is already available without noticeable latency.
  - Source: <https://www.w3.org/WAI/ARIA/apg/patterns/grid/>
  - Source: <https://www.w3.org/WAI/ARIA/apg/patterns/toolbar/>
  - Source: <https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/>
  - Source: <https://www.w3.org/WAI/ARIA/apg/patterns/tabs/>
  - Source: <https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/>
- WCAG 2.2 adds constraints that matter specifically for a dense Tauri app: sticky bars and inspector overlays must not fully hide focused controls, reflow must avoid two-dimensional scrolling except where the content type needs it, and focus indicators need enough size and contrast to remain visible in compact density.
  - Source: <https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html>
  - Source: <https://www.w3.org/WAI/WCAG22/Understanding/reflow.html>
  - Source: <https://www.w3.org/WAI/WCAG22/Understanding/focus-appearance.html>
- Tableau's dashboard guidance is useful for the Analyze stage: chart choice should follow the user's question, device-specific layouts are preferable to automatic squeezing, and performance can be affected by visual complexity, number of data points, filters, calculations, and query cost.
  - Source: <https://help.tableau.com/current/blueprint/en-us/bp_visual_best_practices.htm>

### Research Synthesis

The second pass changes the bar for this plan in five ways:

1. The target shell should be a workbench, not a dashboard. The dominant layout should be rail + work area + supporting/inspector pane + status/action region.
2. The titlebar is a product surface. It must handle window behavior, app identity, global search/command access, task state, and settings without fighting native caption or drag affordances.
3. Navigation must have levels. Global destinations belong in a rail or app shell. Workflow stages belong in the active workbench. Object-specific details belong in an inspector. They should not be visually equivalent.
4. Dense tables are product-critical. Sorting, selection, applied filters, column resizing/presets, keyboard review, and row detail should be specified before implementation starts.
5. Accessibility is not just a compliance pass. Keyboard focus, visible focus, modal focus restoration, sticky-region behavior, and data-grid navigation must be part of the implementation acceptance criteria.

### Research-To-Implementation Traceability

| Research finding | Implementation consequence |
| --- | --- |
| Tauri custom titlebars require explicit drag regions and platform-safe controls | Titlebar migration must include drag-region QA and avoid placing interactive controls in draggable zones |
| WinUI NavigationView adapts between left, compact, minimal, and top patterns | Workbench rail must have explicit desktop, compact, and mobile behavior instead of one CSS layout |
| VS Code separates Activity Bar, sidebars, editor/work area, panel, and status bar | VectorGrid should separate destinations, work content, inspector/details, and execution/status |
| JetBrains uses tool-window bars for supplemental panes | History, validation, tasks, diagnostics, and options should become inspector/tool-window modes, not permanent docks |
| Carbon global header distinguishes global tasks from product tasks | Titlebar/global shell should hold app-wide tasks; stage toolbars should hold stage-local commands |
| Carbon filtering prefers batch filters for slow/multi-category filtering | Result tables should use batch filter panels when filter changes would recompute or reload data |
| Apple tables support sort, resize, truncation, and alternating row backgrounds for wide data | `AppDataTableShell` should support sortable headers, column sizing, truncation tooltips, and dense row scanning |
| Material supports list-detail/supporting-pane adaptive layouts | Inspector should become a right supporting pane on desktop and a sheet/drill-in view on compact windows |
| APG grid pattern requires managed cell focus | Tables that behave like interactive grids need explicit keyboard model tests, not just clickable row tests |
| WCAG focus-not-obscured warns about sticky headers/footers | Layout regression tests must tab through controls and assert focused elements are not hidden by titlebar/footer/inspector |
| Tableau warns about visual complexity and device layouts | Analyze cockpit should have curated chart presets and viewport-specific evidence, not automatic chart squeezing |

## Current State Assessment

### What Is Good Now

- The app exposes a serious workflow: data loading, strategy selection, optimization, validation, history, options follow-up, ML Signal, and live execution.
- Recent QA work added browser workflow coverage, action coverage, responsive workflow tests, desktop smoke geometry, and layout regression budgets.
- Recent UI work improved chrome density, mobile workflow visibility, validation inspector behavior, and history inspector placement.
- The frontend already has useful component primitives such as `AppButton`, `AppStatusChip`, `AppDataTableShell`, visualization components, command palette, task center, and desktop smoke selectors.
- The backend command surface is broad and mature enough to support a premium shell without backend churn.

### Why It Is Not Yet High-End

The current shell still reads like a technical control surface:

- It has too many simultaneously visible navigation layers: desktop titlebar, UI mode, workspace, workflow mode, task center, workflow stepper, run context, panel tabs, main panel, footer.
- Major information is distributed across chrome strips instead of composed into a primary workbench model.
- Fixed docks are operationally useful but visually and architecturally brittle.
- The first screen does not yet feel like a "research cockpit"; it feels like a sequence of panels.
- Dense result views are usable but need stronger saved views, column presets, keyboard behavior, and selection/detail pairing.
- The current palette and surface treatment are calmer than before, but the hierarchy is still too soft. Too many pills, chips, tabs, and buttons have similar weight.
- Advanced features are reachable, but the product does not yet teach the shortest path to a successful first run.

## Product Target

Target: a premium quant research desktop workstation.

The app should feel like a serious native desktop tool built with web technology, not a web dashboard wrapped in a desktop window.

The target user should be able to answer these questions within five seconds:

- What dataset is active?
- What strategy or workflow is active?
- What is the next valid action?
- What is blocked and why?
- What was the most recent meaningful result?
- Where do I inspect details without disrupting the main workflow?

## Competitive Quality Bar

For this project, "high-end desktop UI" means the app meets these product qualities simultaneously:

- Native fit: window controls, drag regions, titlebar density, menu behavior, and keyboard shortcuts feel intentional in a Tauri desktop shell.
- Durable orientation: users can always tell where they are, what object is active, what stage is active, and what action is available.
- Dense without feeling cramped: tables, charts, and controls use space efficiently while keeping target size, focus visibility, and labels intact.
- Progressive complexity: first-run and common flows are direct; advanced settings, route details, diagnostics, and options expressions are present but not front-loaded.
- Recoverable exploration: users can switch workspaces, inspect previous runs, and return to their prior state without losing context.
- Keyboard confidence: common operations are reachable through shortcuts, command palette, tab order, arrow-key navigation, and Escape behavior.
- Evidence-based visual QA: layout decisions are verified at representative desktop, compact, and narrow sizes before each migration phase is accepted.

### Benchmark Pattern Map

This plan does not attempt to copy any one product. It borrows mature patterns from several product classes:

| Benchmark class | Relevant pattern | VectorGrid adaptation |
| --- | --- | --- |
| VS Code-style workbench | Activity rail, command palette, sidebars, status bar | Rail destinations, command/search, inspector, action/status bar |
| JetBrains-style IDE | Dockable tool windows, compact tool stripes, contextual action lists | Inspector modes, optional task/diagnostic panes, context actions |
| Microsoft WinUI apps | Titlebar-integrated navigation/search, adaptive NavigationView | Desktop titlebar with command access, adaptive rail state |
| macOS productivity apps | Sidebar/split view, sortable/resizable tables | Research/library split model and wide result table behavior |
| Enterprise design systems | Global/local navigation distinction, shell state preservation | Stable app shell, stage-local navigation, preserved filters/selections |
| Analytics tools | Chart choice by question, responsive dashboard evidence | Analyze-stage presets and screenshot-backed viewport decisions |

### Non-Negotiable Acceptance Bar

Before implementation begins, each phase below must be judged against these rules:

- No backend command can become unreachable.
- No current E2E selector can be deleted without a same-commit replacement and action manifest update.
- No global region can be added without naming which region it replaces or why it must persist.
- No table redesign is complete until sorting, filtering, row selection, keyboard behavior, empty/loading/error states, and export visibility are specified.
- No overlay/dock pattern is complete until focus trap, Escape behavior, outside-click behavior, and focus restoration are specified.
- No compact viewport is complete until the same workflow can be completed without horizontal overflow or hidden primary actions.
- No "polish" change is accepted without a screenshot or Playwright evidence path for at least desktop and compact widths.

## Design Principles

1. One primary action per state.
   - The shell should expose the next valid action clearly and demote irrelevant actions.

2. Separate navigation from state.
   - Navigation chooses where the user is. Context/status explains what is loaded and ready. These should not compete visually.

3. Use persistent regions deliberately.
   - A left rail is for destinations. A right inspector is for selected context. A footer is for execution state and primary action. Avoid stacking more global regions.

4. Prefer panes over overlays for persistent work.
   - Validation/history/analysis context should be inspector content, not fixed overlay docks.

5. Make dense data efficient, not decorative.
   - Tables and charts should prioritize scan speed, sorting, selection, comparison, keyboard review, and export.

6. Keep advanced controls available but not front-loaded.
   - Use progressive disclosure for strategy knobs, options provenance, backend route details, and diagnostics.

7. Preserve backend capability.
   - Every current command must remain reachable through a stage, inspector, command palette, settings, or advanced drawer.

8. Make layout quality executable.
   - New shell work must extend Playwright/WDIO layout guards instead of relying on manual screenshot review only.

## Target Information Architecture

### Top-Level Layout

Replace the current stacked shell with a workbench layout:

```text
+---------------------------------------------------------------------+
| Desktop titlebar: app identity, command/search, tasks, settings      |
+---------+-----------------------------------------------+-----------+
| Rail    | Main workbench                                | Inspector |
|         |                                               |           |
| Research| Research cockpit or active workflow stage     | Context   |
| Library |                                               | Selected  |
| Runs    | Tables/charts/forms                           | Run       |
| Live    |                                               | Validation|
| Settings|                                               | Tasks     |
+---------+-----------------------------------------------+-----------+
| Contextual action/status bar: next action, progress, outcome         |
+---------------------------------------------------------------------+
```

### Workspaces

Recommended top-level rail destinations:

- Research
- Library
- Runs
- Live
- Settings

`Runs` is the proposed user-facing label for the current aggregate/history-oriented surface. If the existing `Aggregate` workspace id remains in code during migration, it should be treated as a compatibility route, not the final visible product label.

Diagnostics should move to command palette and Settings unless active diagnostics work requires a persistent workspace. Diagnostics can remain reachable but should not compete with primary destinations.

### Research Stages

Inside Research, use a single stage stepper or local stage rail:

- Data
- Configure
- Optimize
- Validate
- Analyze
- Finalize

The active stage owns its local tabs. For example:

- Configure: Strategy, Advanced, Options
- Optimize: Search, Results Preview, Runtime
- Validate: Setup, Results, Reports
- Analyze: Top, Candidate Detail, Heatmap, Pareto

This removes the current effect where global workflow tabs and panel tabs compete.

### Inspector

The right inspector is a persistent, reserved region with modes:

- Run context
- Selected candidate
- Validation context
- Options provenance
- History detail
- Task detail
- Diagnostics

Inspector behavior:

- Desktop/laptop: persistent right rail, resizable/collapsible.
- Tablet: collapsible side sheet.
- Mobile/narrow: modal sheet or inline section below stage summary.

The inspector replaces the former validation, history, and optimize context docks plus ad hoc context summaries inside individual panels where duplication exists.

### Footer

The footer should become a true action/status bar:

- left: active stage and readiness summary
- center: task/progress/outcome
- right: one primary action plus one secondary overflow

Avoid streaming many low-priority chips in the footer. If the user needs deeper task detail, open the task inspector.

## Implementation Phases

### Phase 0 - Baseline And Guard Tightening

Goal: make the current UI measurable before structural changes.

Tasks:

- Keep `web/e2e/browser/qa/layoutRegression.spec.ts` as the shell budget authority.
- Add one additional desktop-laptop layout guard for 1440x900 if the new shell targets common laptop resolution.
- Add explicit assertions for:
  - no document/body horizontal overflow
  - visible primary navigation targets
  - primary action visible and enabled/disabled with reason
  - inspector does not overlap main content or footer
  - command palette remains usable at 390px and 768px heights
- Capture current screenshot baselines in `output/playwright/` when needed, but keep pass/fail based on geometry and interaction assertions.

Acceptance criteria:

- `npm --prefix web run test:e2e:responsive` includes layout budgets.
- `npm --prefix web run test:e2e:workflow` remains green.
- `npm --prefix web run test:desktop:geometry` remains green.

### Phase 1 - Shell Primitives

Goal: create components that make the workbench possible without changing every workflow at once.

New or revised components:

- `web/src/components/workbench/WorkbenchShell.vue`
- `web/src/components/workbench/WorkbenchRail.vue`
- `web/src/components/workbench/WorkbenchHeader.vue`
- `web/src/components/workbench/WorkbenchInspector.vue`
- `web/src/components/workbench/WorkbenchActionBar.vue`
- `web/src/components/workbench/ResearchStageNav.vue`
- `web/src/components/workbench/StageSummary.vue`

New or revised state/helpers:

- `web/src/stores/workbenchLayoutStore.ts`
- `web/src/composables/useWorkbenchReadiness.ts`
- `web/src/utils/workbenchNavigationModel.ts`
- `web/src/utils/workbenchActionModel.ts`
- `web/src/utils/inspectorModel.ts`

Design system extensions:

- `AppPanel`
- `AppToolbar`
- `AppInspectorSection`
- `AppCommandButton`
- `AppSplitButton`
- `AppEmptyState`
- `AppMetricTile`
- `AppKeyboardHint`

Implementation notes:

- Use existing Pinia stores for workflow state; do not invent parallel business state.
- Use `WorkbenchShell` as the only production shell.
- Preserve backend command reachability through existing stores, services, panel mount slots, the workbench inspector, the command palette, and the action bar.
- Keep all existing `data-qa-action` values working where practical. If a selector must move, update the action manifest and E2E coverage in the same change.

Acceptance criteria:

- New primitives render in isolation with component tests where useful.
- New shell can mount in mocked-Tauri mode without changing backend commands.
- Existing advanced-mode panels remain available inside the workbench shell.

### Phase 2 - Workbench Shell Migration

Goal: move from stacked navigation to rail + stage + inspector.

Tasks:

- Move top-level workspace selection from wide tabs to `WorkbenchRail`.
- Keep desktop titlebar focused on:
  - VectorGrid identity
  - current workspace/stage subtitle
  - command palette/search
  - tasks
  - settings
  - primary run/validate shortcut only if it does not duplicate footer
- Move workflow stages into `ResearchStageNav` inside the Research workbench.
- Move the run-context strip into either:
  - `StageSummary`, when it is high-signal and stage-specific
  - `WorkbenchInspector`, when it is detail/provenance
- Convert the footer into `WorkbenchActionBar`.

Target geometry budgets:

| Viewport | Max global chrome before main | Min visible main | Notes |
| --- | ---: | ---: | --- |
| 1720x1080 | 170 px | 850 px | Rail and inspector do not count as vertical chrome |
| 1440x900 | 160 px | 690 px | Common laptop target |
| 1366x768 | 145 px | 560 px | Must feel usable |
| 1024x768 | 160 px | 540 px | Inspector collapses |
| 390x844 | 240 px | 520 px | Mobile/small window shell uses compact stage header |

Acceptance criteria:

- No workflow loses reachability.
- Main content receives materially more vertical space than the current shell.
- The responsive layout guard is updated to the new budgets.
- Desktop WDIO smoke still opens all major workspaces.

### Phase 3 - Inspector Rail Replaces Docks

Goal: eliminate fixed overlay docks as persistent UI.

Tasks:

- Introduce `inspectorStore` with:
  - active inspector mode
  - selected run/candidate/task context
  - collapsed/expanded state
  - per-workspace preferred mode
- Keep validation context in `WorkbenchInspector`.
- Keep history context in `WorkbenchInspector`.
- Keep high-signal optimize context in `WorkbenchInspector`.
- Replace dock visibility logic with inspector routing:
  - Research/Validate -> validation context if a run exists, otherwise readiness.
  - Library -> history detail.
  - Analyze -> selected candidate/candidate-detail context.
  - Live -> runner/feed/broker context.
- Keep mobile behavior as a sheet or inline collapsed panel.

Files likely touched:

- `web/src/App.vue`
- new `web/src/stores/inspectorStore.ts`
- new `web/src/components/workbench/WorkbenchInspector.vue`

Acceptance criteria:

- No fixed validation or history overlay is needed for normal workflow.
- Inspector never overlaps main content or footer.
- Layout E2E asserts inspector behavior in desktop and mobile.
- Component tests prove validation/history context routing.

### Phase 4 - Research Cockpit

Goal: make the first Research screen feel intentional and high-end.

Research cockpit layout:

- Active dataset card
- Strategy/readiness card
- Last run/result card
- Validation readiness card
- Recent artifacts card
- Primary action band

Recommended content:

- Dataset:
  - source
  - symbol/timeframe
  - row count/date range
  - quote/data quality if options data is in scope
- Strategy:
  - selected kind
  - backend policy
  - GPU route availability
  - objective
- Next action:
  - exact action label
  - blocked reason if disabled
  - secondary setup action
- Last result:
  - best metric summary
  - candidate count
  - route/runtime
  - open analyze/validate
- Validation:
  - ready/not ready
  - saved run compatibility
  - reports status

Files likely touched:

- New `web/src/components/workbench/ResearchCockpit.vue`
- `web/src/composables/useWorkbenchReadiness.ts`
- `web/src/stores/dataStore.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/validationStore.ts`
- `web/src/stores/historyStore.ts`

Acceptance criteria:

- Empty app shows exactly one primary next action.
- Loaded dataset but no run routes user to Configure/Optimize.
- Completed optimization routes user to Validate/Analyze.
- Completed validation routes user to Finalize/Reports.
- E2E verifies the happy path and blocked path.

### Phase 5 - Dense Data Surface Upgrade

Goal: make tables/charts feel like premium desktop tools.

Table upgrades:

- Saved table views:
  - column visibility
  - column order
  - density
  - sorting
  - filters
- Column presets:
  - default
  - risk
  - runtime
  - validation
  - options provenance
- Sticky table toolbar:
  - search
  - filter
  - columns
  - export
  - density
  - selected-row actions
- Row detail model:
  - single selection opens inspector
  - double click or Enter opens candidate details
  - multi-select uses batch action bar
- Keyboard behavior:
  - arrow-key row movement for true interactive grids
  - Enter opens primary row action
  - Escape clears selection or closes inspector mode

Files likely touched:

- `web/src/components/ui/AppDataTableShell.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`

Acceptance criteria:

- At least Top, Validation, and History use the same table toolbar contract.
- Existing exports still work.
- Layout and interaction E2E covers at least one dense table surface.
- No table action is only available by hover unless keyboard/focus also reveals it.

### Phase 6 - Desktop-Native Polish

Goal: make Tauri feel intentional as a desktop runtime.

Tasks:

- Add or refine native menu entries:
  - File: Open CSV, Open Recent, Reveal Runs Folder, Export
  - Edit: Copy, Find, Command Palette
  - View: Toggle Inspector, Toggle Task Center, Density
  - Run: Optimize, Validate, Cancel
  - Help: Diagnostics, About
- Add command palette command groups:
  - Navigation
  - Run actions
  - Data
  - Library
  - View/layout
  - Diagnostics
- Add shortcut registry:
  - `Ctrl+K`: command palette
  - `Ctrl+O`: open/load CSV
  - `Ctrl+Enter`: primary run action
  - `Esc`: close current overlay/sheet
  - `Ctrl+B`: toggle rail labels or sidebar if useful
  - `Ctrl+I`: toggle inspector
- Add drag/drop CSV affordance on Data stage.
- Add recent datasets/runs as first-class desktop affordances.
- Improve window-state and task notification copy.

Files likely touched:

- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/stores/commandPaletteStore.ts`
- `web/src/services/desktopProductization.ts`
- `src-tauri/src/commands_system.rs` if native menu/backend support is needed
- `web/e2e/desktop/specs/*`

Acceptance criteria:

- Command palette can reach every major action.
- Desktop smoke verifies native shell readiness and key commands where possible.
- Keyboard shortcuts do not conflict with form editing.
- Recent files/runs are visible in cockpit and command palette.

### Phase 7 - Visual System And Theming

Goal: make the app look expensive without making it decorative.

Palette direction:

- Keep a quiet operational base.
- Reduce beige/cream dominance.
- Use semantic accents for:
  - ready/success
  - warning
  - blocked/error
  - running/progress
  - selected/focused
- Increase contrast in tables and chart axes.
- Avoid broad gradients and decorative blobs.

Surface direction:

- Use fewer card borders.
- Reserve cards for repeated items, modals, and framed tools.
- Use full-width workbench bands or panes for major layout.
- Use clear focus rings and selection rails.
- Keep border radius modest.

Density direction:

- Desktop default: compact but not cramped.
- Mobile/narrow: progressive disclosure, not miniature desktop.
- Add density preference only after the default is stable.

Acceptance criteria:

- CSS token audit shows no one-note palette dominance.
- Focus/selected/active states are visually distinct.
- Screenshots at 1720, 1440, 1366, 1024, and 390 are reviewed before handoff.
- WCAG target-size exceptions are documented for any compact control under 24x24 CSS px.

### Phase 8 - Migration And Deletion

Goal: avoid keeping two shells forever.

Tasks:

- Keep the production workbench as the only active shell after migration.
- Keep a migration checklist of selectors, commands, and mounted panels.
- After parity:
  - remove obsolete dock components or keep them as inspector sections only
  - remove obsolete shell CSS
  - remove obsolete layout tests that assert retired chrome budgets
  - update `docs/REPO_MAP.md`
  - update browser/desktop README docs

Acceptance criteria:

- One production shell remains.
- E2E action coverage manifest has no stale selectors.
- `npm --prefix web run test:e2e:workflow` passes.
- `npm --prefix web run test:desktop:smoke` passes locally when desktop deps are available.

## Detailed Workstream Backlog

The phase list above defines sequencing. This backlog defines the concrete product surfaces that need design and implementation detail before coding begins.

### Workstream A - Shell And Navigation Model

Files likely involved:

- `web/src/App.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/styles/app.css`
- new `web/src/components/workbench/WorkbenchShell.vue`
- new `web/src/components/workbench/WorkbenchRail.vue`
- new `web/src/components/workbench/WorkbenchStageNav.vue`
- new `web/src/utils/workbenchNavigationModel.ts`

Implementation requirements:

- Define a single navigation model with:
  - global destination id
  - visible label
  - icon name
  - route/workspace mapping
  - enabled/disabled state
  - badge/count metadata
  - compact visibility
  - command palette command id
- Start with these global destinations:
  - Research
  - Library
  - Runs
  - Live
  - Settings
- Treat Diagnostics as a command/settings surface unless user research proves it deserves top-level space.
- Treat ML Signal as a Research mode until it has enough independent state to justify a top-level destination.
- Provide stable selectors:
  - `data-qa-shell="workbench"`
  - `data-qa-rail`
  - `data-qa-rail-item="<destination>"`
  - `data-qa-stage-nav`
  - `data-qa-stage="<stage>"`
  - `data-qa-inspector`
  - `data-qa-action-bar`
- Preserve current workspace state when switching destinations.
- Make compact behavior explicit:
  - desktop wide: rail visible, inspector can be pinned
  - laptop/medium: rail compact, inspector collapsible
  - narrow/mobile: rail becomes menu/sheet, inspector becomes drill-in sheet

Open implementation decisions:

- Whether rail labels are always visible at desktop width or appear in expanded/hover state.
- Whether Settings is always bottom-aligned in the rail.
- Whether Runs and Library are separate destinations or one "Library" destination with saved runs as the first view.

### Workstream B - Desktop Titlebar And Native Chrome

Files likely involved:

- `web/src/components/DesktopTitlebar.vue`
- `web/src/services/desktopProductization.ts`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/*.json`
- Tauri menu/tray wiring if native menus are adopted

Implementation requirements:

- Preserve minimize, maximize, close, and drag behavior.
- Separate draggable and interactive regions; no button, input, menu, or command control should sit inside a drag-only region.
- Promote global command/search access to the titlebar only if it remains responsive at compact desktop widths.
- Keep current task center access but demote noisy state into status/action bar where appropriate.
- Add a titlebar QA scenario that double-clicks the drag region for maximize/restore only if supported by the current platform harness.
- If native menus are added, mirror core commands:
  - Load data
  - Open command palette
  - Run optimization
  - Run validation
  - Open history/library
  - Open settings
  - Toggle inspector
  - Toggle theme/density when implemented

Acceptance details:

- Titlebar controls have accessible names.
- Titlebar controls meet the compact target-size rule or documented spacing exception.
- The command/search input does not overlap caption controls at 1024 px width.
- Titlebar inactive/active state is visually distinguishable without breaking theme contrast.

### Workstream C - Inspector And Tool-Window Modes

Files likely involved:

- `web/src/components/RunContextBar.vue`
- `web/src/components/OptionsExpressionPanel.vue`
- new `web/src/components/workbench/WorkbenchInspector.vue`
- new `web/src/utils/inspectorModel.ts`

Inspector modes:

- Selection: row/run/strategy detail for the currently selected object.
- Validation: validation run summary, blockers, out-of-sample context, route metadata.
- History: recent runs, saved runs, options follow-up, provenance.
- Tasks: running/background tasks and recent completion state.
- Diagnostics: backend route, provider, cache, and environment details.
- Help: context-sensitive empty-state and first-run guidance.

Implementation requirements:

- Only one inspector mode is active at a time.
- The inspector can be pinned, collapsed, and opened from command palette.
- The inspector remembers last active mode per global destination.
- Inspector sections have explicit headings and local actions.
- Inspector must not be used as a dump of every store field.
- Inspector width has min/max constraints and must not shrink the main work area below its usable minimum.
- In compact windows, inspector opens as a sheet or drill-in route with focus management and Escape/back behavior.

Acceptance details:

- Opening the inspector does not change active workflow stage.
- Closing the inspector restores focus to the invoking control.
- Inspector content is reachable by keyboard.
- Inspector does not obscure focused controls in the main work area.
- Existing history/options workflows remain reachable.

### Workstream D - Research Cockpit And Workflow Stages

Files likely involved:

- `web/src/App.vue`
- strategy/config components currently mounted inside the main app
- new `web/src/components/research/ResearchCockpit.vue`
- new `web/src/components/research/ResearchStageNav.vue`
- new `web/src/utils/workbenchActionModel.ts`

Research stages:

- Data: source, symbol/universe, date range, cache/provider state, load status.
- Configure: strategy family, parameters, optimization ranges, route/capability summary.
- Optimize: candidate generation, execution route, progress, result ranking.
- Validate: out-of-sample settings, options validation where applicable, validation result summary.
- Analyze: charts, metrics, trades, distribution, comparison, export.

Cockpit tiles:

- Active dataset.
- Selected strategy.
- Optimization readiness.
- Validation readiness.
- Recent result.
- Provider/cache status.
- Next valid action.

Implementation requirements:

- The next valid action is derived from state, not hardcoded text.
- Blocked actions explain the missing prerequisite.
- The first-run path should be:
  - choose/load data
  - configure strategy
  - run optimization
  - validate selected result
  - analyze/export
- Stage navigation should not duplicate global destinations.
- Stage content should expose local actions through a stage toolbar, not the global rail.
- No stage transition should discard user input unless the user confirms or the state is persisted.

Acceptance details:

- Empty app shows a clear first action.
- Loaded-data app shows strategy/configure as the next action.
- Completed optimization shows validation/analyze as the next action.
- Failed/blocked states show a repair action, not only an error message.

### Workstream E - Dense Data, Tables, And Analysis Surfaces

Files likely involved:

- `web/src/components/AppDataTableShell.vue`
- result table/chart components
- `web/src/styles/app.css`
- new table view model utilities if needed

Implementation requirements:

- Table title and description are explicit and tied to data provenance.
- Column headers are short, readable, and sortable where valuable.
- Sorted column state is visible and exposed through `aria-sort` where relevant.
- Column resizing is supported for wide result tables or explicitly deferred with an issue.
- Column presets/saved views are provided for:
  - optimization results
  - validation results
  - trades
  - options follow-up
  - live positions/events when applicable
- Toolbar actions are grouped:
  - search/filter/view settings/export/global table actions
  - row-specific actions stay inline or in row overflow
- Batch actions appear only after selection and include a visible exit/clear-selection control.
- Hidden/applied filters expose count and reset without reopening the filter panel.
- Expensive filters use Apply/Reset batch behavior.
- Interactive grids receive a documented keyboard model:
  - Tab enters/leaves the table region
  - arrows move row/cell focus if grid behavior is implemented
  - Enter opens details or enters cell controls according to table type
  - Escape exits an inline edit/menu/detail state

Acceptance details:

- Tables remain usable at 1366x768.
- Wide data may scroll horizontally inside the table region, but the app shell must not overflow horizontally.
- Hover-only row actions also appear on focus.
- Empty, loading, error, and partial-data states are covered in Storybook or E2E fixtures if Storybook is not present.

### Workstream F - Command Palette, Menus, Shortcuts

Files likely involved:

- existing command palette components/services
- `web/src/services/desktopProductization.ts`
- new `web/src/utils/commandRegistry.ts` if no central registry exists

Implementation requirements:

- Define a central command registry with:
  - id
  - title
  - scope
  - enabled predicate
  - action
  - optional shortcut
  - optional native menu mapping
  - optional selector for E2E
- Every backend capability in the preservation matrix has at least one command entry.
- Disabled commands explain why they are unavailable.
- Command palette search terms include product language and user language.
- Native menu entries, if implemented, call the same command registry actions.

Acceptance details:

- Command palette can reach every major workflow without mouse navigation.
- Shortcut labels do not appear unless the shortcut is actually wired.
- Keyboard conflicts are detected or documented.

### Workstream G - Visual System, Theme, And Density

Files likely involved:

- `web/src/styles/app.css`
- shared UI primitives
- relevant component styles

Implementation requirements:

- Define tokens for:
  - background layers
  - surface layers
  - borders
  - focus ring
  - selected state
  - active navigation state
  - warning/error/success/info states
  - text emphasis levels
  - chart/category accents
- Reduce visual equivalence between chips, tabs, pills, and buttons.
- Reserve primary button treatment for the current next valid action.
- Use icon-only buttons only where the icon is familiar or paired with a tooltip/accessibility label.
- Avoid decorative gradients, blobs, and oversized marketing composition.
- Keep cards for repeated items, modals, or truly framed tools.
- Use compact desktop density but respect pointer/focus targets.

Acceptance details:

- CSS color audit does not reveal a one-note palette.
- Focus, hover, selected, active, disabled, loading, and error states are distinguishable.
- Light/dark or theme work is not started until shell hierarchy is stable, unless current contrast is blocking.

## Migration Strategy

The redesign shipped through staged parity and now has one production shell.

1. Keep existing workflows wired to the current panels while workbench navigation and action state mature.
2. Maintain Research, Library/history/options, Live, Settings, and Advanced Mode reachability through `WorkbenchShell`.
3. Remove redundant chrome only after selector/action coverage proves parity.
4. Update `docs/REPO_MAP.md`, browser E2E docs, and desktop E2E docs in the same implementation slice that changes navigation ownership.

Regression plan:

- Keep current selectors aliased or mapped where practical while new selectors land.
- If a shell/navigation phase fails desktop smoke or broad workflow E2E, fix the production workbench path directly rather than reintroducing a second shell.

## Pre-Implementation Checklist

Complete these before writing production UI code:

- Confirm the global destination list.
- Confirm whether Diagnostics is top-level or Settings/command-only.
- Confirm ML Signal placement.
- Confirm first-slice rollout posture.
- Identify exact current command/store sources for:
  - active dataset
  - active strategy
  - optimization readiness
  - validation readiness
  - latest result
  - running task state
- Snapshot current desktop, responsive, workflow, and action E2E results.
- Capture baseline screenshots at 1720x1080, 1440x900, 1366x768, 1024x768, and 390x844.
- Record known acceptable current layout issues separately so the redesign does not hide new regressions inside old debt.

## Backend Capability Preservation Matrix

| Capability | Current surface | Target surface | Required reachability |
| --- | --- | --- | --- |
| CSV load | Data panel | Data stage and cockpit | Data stage, command palette, drag/drop |
| Polygon stock/FX/crypto data | Data panel | Data stage source drawer | Data stage, Settings market data |
| Strategy selection | Strategy/advanced panels | Configure stage | Configure, command palette |
| Optimization | Optimize panel/titlebar/footer | Workbench action bar | Action bar, command palette, shortcut |
| Validation | Validation panel/dock/footer | Validate stage + inspector | Validate stage, inspector, command palette |
| Analysis views | Results tabs | Analyze stage local views | Analyze, inspector, command palette |
| History/library | History panel/dock | Library workspace + inspector | Library, command palette |
| Options follow-up | History/options panel | Library detail + inspector | Library, inspector, command palette |
| ML Signal | Mode switch + panels | Research mode or workspace section | Research, command palette |
| Live execution | Live workspace | Live workspace + inspector | Live, command palette |
| Settings/diagnostics | Settings and Diagnostics | Settings + command palette | Settings, command palette |

## Verification Strategy

### Automated Browser QA

Required after each shell phase:

```powershell
npm --prefix web run test:e2e:responsive
npm --prefix web run test:e2e:workflow
npm --prefix web run test:e2e:regressions
```

Add or update tests for:

- rail navigation
- stage navigation
- inspector open/close/persistence
- mobile sheet behavior
- no horizontal overflow
- no inspector/content/footer overlap
- primary action state per workflow
- command palette reachability

### Desktop QA

Required after shell/navigation changes:

```powershell
npm --prefix web run test:desktop:geometry
```

Run when broader desktop behavior changes:

```powershell
npm --prefix web run test:desktop:smoke
```

Desktop-specific assertions should cover:

- Tauri window launches to workbench shell
- titlebar controls remain reachable
- deterministic resize keeps navigation and action bar usable
- command palette and task center open/close
- inspector does not overlap the desktop footer

### Unit/Component Tests

Add tests close to the changed modules:

- `workbenchNavigationModel.test.ts`
- `workbenchActionModel.test.ts`
- `inspectorModel.test.ts`
- `WorkbenchShell.test.ts`
- `WorkbenchInspector.test.ts`
- `ResearchCockpit.test.ts`
- `AppDataTableShell.test.ts` additions

### Visual Evidence

Use screenshot capture for review evidence, not as the only test authority:

- 1720x1080 desktop
- 1440x900 laptop
- 1366x768 laptop
- 1024x768 tablet/compact desktop
- 390x844 mobile/narrow

Store local evidence under `output/playwright/`.

### Accessibility And Keyboard QA

Required manual/automated checks for shell phases:

- Tab from first focusable element through titlebar, rail, stage nav, main content, inspector, and action bar.
- Confirm focus is always visible and never fully hidden behind titlebar, footer, sticky toolbar, inspector, sheet, or modal.
- Confirm Escape closes command palette, menus, popovers, modal dialogs, and compact inspector sheets in a predictable order.
- Confirm focus returns to the invoking control after closing dialog/palette/inspector sheet.
- Confirm disabled-but-important actions either remain discoverable with a reason or have a visible blocked-state explanation nearby.
- Confirm rail/stage/tabs distinguish focus from selected/current state.
- Confirm icon-only buttons expose accessible names and tooltips where the icon is not universally obvious.

Automated coverage to add where feasible:

- browser E2E keyboard sweep for the new workbench shell
- focus-not-obscured assertion for representative controls after tabbing
- ARIA role/name checks for rail, stage nav, toolbar, dialog, and data grid/table shells
- command palette keyboard-only open/search/execute flow
- compact sheet focus trap and Escape close behavior

### End-To-End Scenario Matrix

The goal is not only "screens render"; the goal is that major user workflows are executable from a clean app state.

| Scenario | Desktop wide | Laptop 1366x768 | Compact 1024x768 | Narrow 390x844 | Keyboard-only |
| --- | --- | --- | --- | --- | --- |
| First-run empty state to data load prompt | Required | Required | Required | Required | Required |
| Load mocked CSV data and see dataset status | Required | Required | Required | Required | Required |
| Configure stock strategy and see next action | Required | Required | Required | Required | Required |
| Run mocked optimization and inspect result | Required | Required | Required | Required | Required |
| Select result row and open inspector detail | Required | Required | Required | Required as sheet | Required |
| Run validation from selected result | Required | Required | Required | Required | Required |
| Review validation summary and blockers | Required | Required | Required | Required | Required |
| Open history/library and restore a run | Required | Required | Required | Required | Required |
| Open options follow-up for eligible run | Required | Required | Required | Required | Required |
| Use command palette to reach each workspace | Required | Required | Required | Required | Required |
| Open task center during/after run | Required | Required | Optional | Optional | Required |
| Toggle inspector and preserve workflow state | Required | Required | Required | Required | Required |
| Use table search/filter/sort/export controls | Required | Required | Required | Required where exposed | Required |
| Open settings/provider diagnostics | Required | Required | Required | Required | Required |
| Recover from mocked backend error | Required | Required | Required | Required | Required |

### Regression Budgets

Each implementation phase should maintain these budgets:

- App-level horizontal overflow: `0` at all tested viewport widths.
- Main workbench minimum usable height: at least 560 px at 1366x768 after titlebar/action bar.
- Persistent global chrome rows: titlebar plus action/status bar only; stage nav is local to active workbench.
- Inspector overlap: `0` overlap with action bar, titlebar, rail, or active focused element.
- Command reachability: every capability in the preservation matrix has command palette access.
- Keyboard trap failures: `0` for palette, modal, sheet, inspector, and menus.
- Hidden focus failures: `0` for tabbing through representative shell controls.
- Stale E2E selectors: `0` in action coverage manifest.

### Definition Of Done For A Shell Phase

A shell phase is complete only when:

- The changed surface has unit/component tests for its state model.
- Browser E2E covers mouse and keyboard use for the changed surface.
- Desktop geometry or smoke coverage is updated if native shell behavior changes.
- Screenshot evidence exists for the required viewport set.
- Accessibility notes are updated for any deferred APG/WCAG behavior.
- `docs/REPO_MAP.md` is updated if ownership or entry points changed.
- Old behavior is either still feature-flag reachable or intentionally removed with coverage parity.

## Measurement Targets

| Metric | Current direction | Target |
| --- | --- | --- |
| Number of persistent global chrome rows | Too many | Titlebar + rail + action bar, with stage nav local |
| 390px horizontal overflow | Recently fixed | Always 0 in E2E |
| Validation dock overlap | Recently fixed by suppression | Eliminated by inspector architecture |
| First-run path clarity | Moderate | Single visible next action from empty app |
| Main visible height at 1366x768 | Improved but chrome-heavy | At least 560 px after shell migration |
| Command reachability | Improving | Every major command in command palette |
| Dense table consistency | Mixed | Shared toolbar/selection/view model |
| Keyboard operation | Good in some components | APG-style behavior for tabs/dialogs/grids |
| Focus visibility under sticky regions | Not fully measured | 0 hidden-focus failures in shell keyboard sweep |
| Inspector state preservation | Dock-specific | Active mode and pinned/collapsed state preserved per workspace |
| Native desktop fit | Credible but web-like | Titlebar, menu/shortcut, drag, and desktop smoke all verified |
| Applied filter visibility | Mixed by table | Count/reset visible whenever filters are hidden |
| Result table scan speed | Usable | Sort, resize/preset, selection/detail, and export paths standardized |

## Risks And Mitigations

### Risk: Workbench Rewrite Breaks Existing Workflows

Mitigation:

- Keep workbench shell changes covered by browser workflow and desktop smoke before handoff.
- Migrate one workspace surface at a time.
- Preserve `data-qa-action` selectors or update action coverage in the same PR.

### Risk: Inspector Becomes A New Junk Drawer

Mitigation:

- Inspector modes must be explicit.
- Each mode gets a primary question it answers.
- Use section collapse and local actions.
- Do not dump every store field into inspector.

### Risk: Mobile Becomes An Afterthought

Mitigation:

- Keep `390x844` layout regression tests.
- Use sheet/inline behavior instead of shrinking desktop rail/inspector.
- Keep first valid action visible without horizontal scroll.

### Risk: Visual Polish Reduces Density

Mitigation:

- Treat this as an operational workstation, not a marketing app.
- Use density budgets and table-specific QA.
- Avoid oversized hero sections, decorative cards, and broad gradients.

### Risk: Cross-WebView Differences

Mitigation:

- Browser Playwright proves Chromium behavior.
- WDIO desktop smoke proves WebView2 behavior on Windows.
- Keep CSS conservative: avoid fragile features without fallback.
- When possible, validate macOS/Linux manually before release.

## Proposed Initial Work Order

1. Run pre-implementation baseline: current responsive/workflow E2E, desktop geometry if available, and viewport screenshots.
2. Add `workbenchNavigationModel` and `workbenchActionModel` tests before rendering the new shell.
3. Add or revise `WorkbenchShell` skeleton coverage.
4. Add `WorkbenchRail` with top-level workspace routing only.
5. Add `WorkbenchActionBar` that mirrors current footer primary action.
6. Add `WorkbenchInspector` shell with current run-context summary.
7. Add browser layout guards for rail/actionbar/inspector geometry and focus-not-obscured behavior.
8. Port validation/history side-panel behavior into inspector modes.
9. Add `ResearchCockpit` as the Research default.
10. Move Research stage navigation local to workbench.
11. Tighten layout budgets to the target geometry.
12. Upgrade Top/Validation/History table toolbars.
13. Remove obsolete dock/shell paths after parity.

## First Slice Definition

The first implementation slice should be intentionally narrow:

Scope:

- `workbenchNavigationModel`
- `workbenchActionModel`
- `WorkbenchShell`
- `WorkbenchRail`
- `WorkbenchActionBar`
- `WorkbenchInspector` placeholder
- browser layout guards for rail/actionbar/inspector geometry
- keyboard smoke for rail, stage placeholder, inspector toggle, and command palette reachability

Out of scope:

- broad shell replacement work
- redesigning every panel
- changing backend commands
- redesigning charts/tables
- changing options-expression semantics

Acceptance criteria for first slice:

- Mocked-Tauri shell renders with rail, main content, inspector placeholder, and action bar.
- Research, Library, Runs/Aggregate, Live, Settings can be selected.
- Primary action state matches current footer state.
- `test:e2e:responsive` passes for both shell modes or has explicit old/new coverage.
- No new horizontal overflow.
- Tabbing through shell controls keeps focus visible and unobscured.
- Feature-flagged shell has screenshot evidence at 1720, 1366, 1024, and 390 widths.

## Open Decisions

1. Should Diagnostics remain a top-level destination, or move into Settings and command palette?
2. Should ML Signal be a Research mode, a top-level destination, or a specialized workflow under Research?
3. Should the left rail show labels by default on desktop, or compact icons with labels on hover/expanded state?
4. Should the inspector be resizable in the first slice, or fixed width until behavior stabilizes?
5. Which future inspector states should be user-resizable versus fixed until behavior stabilizes?
6. Should the current Aggregate workspace be renamed to Runs in the visible UI while keeping internal ids/aliases during migration?

## Recommendation

Proceed with a staged shell migration, not another isolated CSS polish pass.

The app's backend capability is already strong. The next competitive jump comes from making the frontend feel like a coherent desktop workbench:

- left rail for top-level navigation
- central Research cockpit and workflow stages
- right inspector for selected context
- bottom action/status bar for the next valid action
- command palette as universal reachability
- table/chart surfaces tuned for scan speed and keyboard review

The recent E2E and layout regression work gives us the safety net to make this migration without accepting UI regressions as the cost of redesign.
