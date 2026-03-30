# Existing Stack Utilization Plan

Date: 2026-03-28

Scope: repo-specific implementation plan for getting more objective product value out of technologies that are already installed and already partly integrated in the current app.

Status: complete for the planned scope.

Current implementation status:

- Phase 1 is complete.
- Phase 2 is complete.
- Phase 3 is complete.
- Phase 4 is complete.
- Phase 5 is complete.
- The shipped Phase 1 slices are persisted column visibility for:
  - `Top Results`
  - `Validation`
  - `Meta`
- Shared infrastructure now lives in:
  - [usePersistedMatrixColumnState.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\usePersistedMatrixColumnState.ts)
  - [useHeadlessMatrixTable.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useHeadlessMatrixTable.ts)
  - [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
  - [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)
  - [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue)
- The shipped Phase 2 slice is a shared contextual action menu for secondary saved-artifact actions:
  - [ContextActionMenu.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ContextActionMenu.vue)
  - first rollout in [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue)
  - browser-verified against the built UI after fixing teleported menu layering so the menu stays above panel chrome and remains pointer-safe
- The shipped Phase 3 slices are local fuzzy-search improvements that stay subordinate to the global command palette:
  - primary local-search surface:
    - [HistoryWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\HistoryWorkspacePanel.vue)
    - [historyStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\historyStore.ts)
    - current repo note: short queries stay deterministic while `fuse.js` adds typo tolerance and better ranking for longer operator queries
  - justified second local-search surface:
    - [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
    - [perspectiveSearch.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\perspectiveSearch.ts)
    - current repo note: perspective search stays component-local, keeps the current selection visible, and searches by name plus saved workstation metadata
  - local-vs-global search distinction is now explicit in the UI:
    - Saved Runs search filters the library only
    - saved perspective search stays scoped to the workstation block in Settings
    - `Ctrl+K` remains the app-wide command/navigation surface
- Saved perspectives also already use local fuzzy search:
  - [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
  - [perspectiveSearch.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\perspectiveSearch.ts)
- The shipped Phase 4 slices are scanability and interaction-polish improvements that stay additive to existing labels and surfaces:
  - minimal shared icon grammar in [iconography.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\iconography.ts)
  - compare/pin actions now rendered with icon + label in [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
  - saved-artifact secondary actions now render through the shared menu with icon + label in [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue) and [ContextActionMenu.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ContextActionMenu.vue)
  - task-state and diagnostic severity badges now use icon + label in [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue) and [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
  - the animated brand mark now respects reduced-motion posture in [VectorAlphaLogo.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VectorAlphaLogo.vue) by disabling hover animation when reduced motion is preferred
  - desktop notifications now route only for backgrounded windows via [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts) and [desktopNotificationPolicy.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\desktopNotificationPolicy.ts)
- The shipped Phase 5 slices are desktop-plugin usefulness improvements that stay subordinate to the existing task center and settings surfaces:
  - desktop diagnostics now support level filtering, copyable snapshots, and in-app log clearing in [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
  - desktop notification routing was re-evaluated and kept high-signal only:
    - important completions and failures remain opt-in
    - OS notifications are suppressed while the desktop window is visible
    - policy remains centralized in [desktopNotificationPolicy.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\desktopNotificationPolicy.ts)
  - window-state recovery was explicitly evaluated and intentionally not shipped:
    - the current repo exposes persistence readiness and the backing file path in Settings
    - no credible “user gets trapped by persisted geometry” failure mode has been observed yet
    - adding a reset affordance now would add product surface area without a demonstrated user need
- Phase 4 verification is green:
  - `lint`
  - `typecheck`
  - `test`
  - `build`
  - `a11y:smoke`
  - `baseline:ui`
  - `check:bundle-budget`
- Phase 5 verification is green:
  - `lint`
  - `typecheck`
  - `test`
  - `build`
  - `a11y:smoke`
  - `baseline:ui`
  - `check:bundle-budget`
- Execution-order review remains intentionally deferred because it is the lowest-value table for this feature set.
- Arbitrary pinned columns remain deferred; Phase 1 proves persisted visibility, restore-defaults, and row-action stability first.

This plan is intentionally separate from:

- `frontend_audit_2026-03-27.md`, which is the completed remediation ledger
- `frontend_followup_evaluation_2026-03-28.md`, which records the completed frontend follow-up pass and prior package decisions
- `workstation_behavior_plan_2026-03-28.md`, which records the completed workstation UX phase
- `desktop_productization_plan_2026-03-28.md`, which owns broader Tauri-native topology and productization work
- `svg_generation_prompts_2026-03-28.md`, which contains the paste-ready GPT-5.4 Pro prompts for generating the custom SVG set proposed here

This document is for the narrower question:

- where are we already paying for useful technologies
- where are we only using the shallowest slice of them
- what additional use cases would objectively improve the app without adding new libraries

## Why This Exists

The repo already ships a stronger frontend/runtime stack than the UI currently exploits.

Current examples:

- `@tanstack/vue-table` is present and already routed through [useHeadlessMatrixTable.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useHeadlessMatrixTable.ts)
- `reka-ui` already powers [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue), [ContextHelpTip.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ContextHelpTip.vue), and [WorkflowQuickSwitch.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\WorkflowQuickSwitch.vue)
- `@vueuse/core` is already used in multiple composables and shell surfaces
- `@floating-ui/vue` already exists behind [useAnchoredOverlay.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useAnchoredOverlay.ts)
- `fuse.js` already powers the command palette
- `lucide-vue-next` is already installed and used sparingly
- `tauri-plugin-log`, `tauri-plugin-notification`, and `tauri-plugin-window-state` are already installed and bridged through [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts)

The missed opportunity is not "buy more technology." The missed opportunity is using the current stack more deeply where it clearly helps operators.

## Non-Goals

This plan does not include:

- adding new libraries
- forcing `vue-router` into the shell just because it is installed
- replacing `lightweight-charts`
- speculative desktop-native features such as tray, updater, or global shortcuts
- decorative UI changes that do not improve workflow speed, trust, or scanability

## Product Principles

1. Favor deeper use of current primitives over adding new ones.

2. Prefer improvements that help operators make decisions faster:
   - better dense-table control
   - better search/discovery
   - better contextual actions
   - better notification/log routing
   - better state persistence and recovery

3. Keep backend-backed capability exposure intact.
   If a control is regrouped behind a row menu, popover, or compare strip, the capability still needs to remain clearly reachable.

4. Treat consistency as part of technology adoption.
   It is not enough to use a package once. The value comes from standardizing repeated behavior.

5. Keep each phase independently shippable.
   None of these improvements should require a broad rewrite.

## Current Repo-Grounded Opportunity Areas

### 1. `@tanstack/vue-table`

Already used for headless row/header modeling in:

- [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
- [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)
- [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue)
- [ExecutionWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ExecutionWorkspacePanel.vue)

Underused behaviors:

- column visibility
- pinned columns
- persisted sort/filter state
- column sizing
- unified selection state semantics across dense review surfaces

### 2. `reka-ui`

Already used for:

- command palette dialog
- help tooltip / hover-card behavior
- workflow quick-switch popover / toolbar behavior

Underused behaviors:

- row action menus
- compact perspective-management menus
- advanced-settings disclosures
- small action clusters and menu-button patterns in dense panels

### 3. `@vueuse/core`

Already used for:

- storage
- resize observers
- event listeners
- debouncing

Underused behaviors:

- reduced-motion-aware interaction fallbacks in JS-driven flows
- media-query / breakpoint-driven shell adjustments
- stronger keyboard-chord helpers
- page visibility / idle cues for long-running workflows

### 4. `@floating-ui/vue`

Already used generically for anchored overlays.

Underused behaviors:

- row-level quick-action popovers
- compare/provenance callouts
- lightweight chart-point explanations
- consistent collision-safe contextual actions in dense tables

### 5. `fuse.js`

Already used for the command palette.

Underused behaviors:

- Run Library search
- saved perspective search
- strategy/indicator lookup
- artifact/search surfaces that already have large label/tag vocabularies

### 6. `lucide-vue-next`

Already used in the titlebar and help affordances.

Underused behaviors:

- consistent icons for run states
- pin/compare actions
- reports/artifacts actions
- task outcomes and diagnostic levels

### 7. Existing Tauri plugins

Already installed and bridged:

- log
- notification
- window-state

Underused behaviors:

- more useful diagnostics filtering/export on top of current log plumbing
- notification click-through / task-related follow-up behavior
- window-state recovery/reset affordances

## Regression Policy For This Plan

Every phase in this plan must preserve the current frontend quality baseline.

Minimum guardrails for any implementation slice:

- functional-coverage guardrail:
  - no backend-backed user capability may disappear from the frontend without explicit approval
- keyboard guardrail:
  - any new contextual action or table behavior must remain keyboard reachable
  - focus return must remain deterministic after dialogs, popovers, and menus close
- scanability guardrail:
  - added search boxes, menus, icons, or callouts must reduce clutter rather than add competing surfaces
- persistence guardrail:
  - persisted state must fail safe when stale, malformed, or partially unavailable
- performance guardrail:
  - dense-table and search improvements must not degrade first paint or make panel switching noticeably heavier

Required verification for any non-trivial slice:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run a11y:smoke`
- `npm --prefix web run baseline:ui`
- `npm --prefix web run check:bundle-budget`

When a slice changes desktop plugin behavior, also run:

- `cargo check -p ta_desktop_demo_app`

## Technology Deep Dives

The sections below turn each existing technology into an implementation-oriented plan:

- what it is already doing in this repo
- what objective improvements are worth pursuing
- what should not be forced
- what regressions to guard against
- how to verify the change

### `@tanstack/vue-table`

Current repo role:

- shared headless table model in [useHeadlessMatrixTable.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useHeadlessMatrixTable.ts)
- current dense consumers:
  - [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
  - [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)
  - [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue)
  - [ExecutionWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ExecutionWorkspacePanel.vue)

What is objectively worth improving:

- move from "header/row modeling only" toward a shared table-state model
- make column visibility a first-class operator tool on the surfaces with the widest column sets
- add pinned-column support only where row identity or key outcome metrics are hard to retain while horizontally scrolling
- persist useful table posture where users repeatedly return to the same surface
- keep selected-row semantics and selected-summary semantics aligned

What should not be forced:

- spreadsheet-grade controls on every list-like surface
- column pinning where there is little or no horizontal scroll
- table-state persistence for ephemeral or low-value review contexts

Implementation notes:

- evolve [useHeadlessMatrixTable.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useHeadlessMatrixTable.ts) into a richer shared wrapper instead of adding panel-specific table engines
- use the current workstation-state/persistence model as the place to store table posture where it is worth remembering
- define a narrow interface for:
  - visible columns
  - pinned columns
  - sort descriptor
  - filter descriptor if later justified
- add one surface at a time, starting with the highest-value dense panel rather than changing all tables together

Best initial candidates:

- Top Results:
  - likely value from column visibility and pinned leading metrics
- Validation:
  - likely value from column visibility and persisted review posture
- Meta:
  - likely value from column visibility before anything else
- Execution orders:
  - likely lowest priority for richer table-state controls

Regression risks:

- table controls becoming more visually prominent than the actual data review
- persisted column posture hiding important fields with no obvious recovery path
- selection and compare behavior drifting when table posture changes

Required acceptance:

- users can always restore a sane default table posture
- hidden columns never remove core row identity without an obvious recover path
- selected-row summary cards still reflect the same selected row after table-state changes
- virtualization and current keyboard row behavior must remain intact

Suggested verification focus:

- component tests around column visibility state and selection stability
- baseline screenshots for Top and Validation after any visible table-control additions

### `reka-ui`

Current repo role:

- dialog semantics in [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
- help tooltips/hover cards in [ContextHelpTip.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ContextHelpTip.vue)
- popover/toolbar behavior in [WorkflowQuickSwitch.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\WorkflowQuickSwitch.vue)

What is objectively worth improving:

- standardize row/item action menus on a shared primitive
- replace duplicated inline micro-button clusters with menu-button or popover patterns where actions are secondary
- use shared disclosure patterns for advanced settings blocks instead of one-off custom expanders

What should not be forced:

- converting every existing custom surface to `reka-ui` for its own sake
- replacing shell tabs that are already working and tightly integrated with the layout

Implementation notes:

- start by defining one shared "row actions" pattern that can be reused in:
  - Top Results
  - Run Library / saved runs
  - Meta review surfaces where multi-action rows exist
- standardize one disclosure pattern for "secondary controls" and reuse it in settings-heavy panels
- prefer `reka-ui` where semantics and focus behavior are otherwise easy to get wrong by hand

Regression risks:

- action menus that hide primary actions users currently rely on inline
- focus return regressions after closing menus or disclosures
- inconsistent open/close behavior between similar surfaces

Required acceptance:

- any action moved behind a menu must remain clearly discoverable
- keyboard users can open the menu, move through items, execute, and return focus cleanly
- the new pattern is reused at least twice before being considered "adopted"
- this must not create a second overlay/focus-management model beside the current shared one

Suggested verification focus:

- focused component tests for keyboard menu behavior and focus restoration
- browser verification of at least one row action flow

### `@vueuse/core`

Current repo role:

- storage and persistence in multiple panels
- event listeners and debounced updates in the shell and dense forms
- resize observation in chart/table helpers

What is objectively worth improving:

- use media-query-aware composables to simplify shell behavior on tighter widths
- use visibility-aware logic where background work affects how aggressively the UI updates
- use reduced-motion-aware branching in JS-managed interactions, not just CSS
- use stronger key or focus helpers only where they reduce hand-rolled event logic

What should not be forced:

- replacing plain Vue APIs with VueUse wrappers when there is no clear benefit
- creating a new composable abstraction for every trivial event or persistence need

Implementation notes:

- prioritize `useMediaQuery` / width-aware behavior where the shell or docks still feel crowded at specific widths
- prioritize visibility/idle handling in long-running task or diagnostics surfaces if it reduces unnecessary churn
- audit JS-managed open/close and animated interactions for reduced-motion sensitivity before adding any new motion behavior

High-value candidates:

- task center update posture when the app is backgrounded
- shell adaptations around dense result controls on narrower widths
- any JS-managed animated expansion/collapse that currently ignores reduced-motion posture

Regression risks:

- shell behavior changing too aggressively across breakpoints
- hidden or collapsed controls becoming harder to discover
- stale visibility state causing missed updates or stale UI

Required acceptance:

- any responsive behavior change must improve legibility at a specific width, not just be "different"
- visibility-aware behavior must never hide important task outcomes from the user
- reduced-motion-aware changes must preserve functional clarity
- new listeners, observers, and watchers must stay bounded and easy to tear down on panel unmount/deactivation

Suggested verification focus:

- targeted viewport screenshot comparisons
- manual or scripted confirmation that backgrounded/foregrounded state does not break task status

### `@floating-ui/vue`

Current repo role:

- shared anchored-overlay helper in [useAnchoredOverlay.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useAnchoredOverlay.ts)
- current consumers already rely on it for collision-safe anchored behavior

What is objectively worth improving:

- use anchored contextual actions instead of permanently visible low-priority button clusters
- support provenance or compare callouts that need to stay close to the current row/item
- standardize collision-safe positioning for compact dense-surface actions

What should not be forced:

- turning every explanation into a floating popover
- using anchored overlays where inline disclosure is clearer

Implementation notes:

- pair `@floating-ui/vue` with `reka-ui` patterns instead of building bespoke floating DOM each time
- reserve anchored overlays for context-sensitive actions or detail that benefits from spatial proximity
- keep action-popover content narrow and task-focused

Regression risks:

- overlays obscuring the same row or chart point they are trying to explain
- collision handling that pushes menus too far from their trigger
- hover-only behavior that fails keyboard access

Required acceptance:

- every anchored action surface must have a keyboard path
- every anchored explanation surface must remain visually tied to its trigger
- any row-level popover must degrade to a simpler pattern if it becomes visually noisy
- anchored overlays must not interfere with sibling toasts, toggles, or other already-mounted shared surfaces

Suggested verification focus:

- browser-side screenshot checks at multiple viewport widths
- targeted keyboard and focus tests for anchored menus/popovers

### `fuse.js`

Current repo role:

- command palette ranking and typo tolerance in [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)

What is objectively worth improving:

- local search in surfaces that already have large and growing vocabularies
- run-history retrieval where exact names are not always remembered
- saved perspective retrieval once perspectives accumulate
- artifact/search surfaces where tags and partial names matter

What should not be forced:

- a search field on every screen
- overlapping global and local search controls that compete for attention

Implementation notes:

- apply `fuse.js` only where:
  - item count is large enough
  - labels/tags are meaningful
  - exact-match search would be frustrating
- keep command palette as the global action/navigation search
- treat local `fuse.js` search as surface-specific retrieval
- keep ranking rules explainable and stable for short queries

Best initial candidates:

- Run Library / saved runs in [HistoryWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\HistoryWorkspacePanel.vue)
- saved perspectives in [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)

Regression risks:

- fuzzy search returning unstable or surprising top hits
- duplicated search state between store-backed filters and local query state
- local search controls crowding already dense panels

Required acceptance:

- each new local fuzzy-search surface must clearly outperform plain substring matching
- query ranking must be covered for typos, short queries, and exact-label matches
- local search must not override the command palette’s role as the global search/action system
- empty-query behavior must remain deterministic rather than fuzzy for its own sake

Suggested verification focus:

- search-specific tests similar to the existing command palette tests
- browser pass against the chosen local-search surface

### `lucide-vue-next`

Current repo role:

- titlebar controls in [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- help affordances in [ContextHelpTip.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ContextHelpTip.vue)

What is objectively worth improving:

- use icons to accelerate scanning where repeated state/action labels already exist
- define a small icon grammar so the same meaning is not represented differently across the app

What should not be forced:

- replacing readable text labels with icon-only controls
- adding icons to every button just because the package exists

Implementation notes:

- define one consistent mapping for:
  - task success / warning / failure
  - pin / compare
  - open report / reveal path / artifact actions
  - diagnostics levels
- apply icons first to repeated utility actions and status chips, not to form controls

Regression risks:

- icon noise making dense panels harder to scan
- inconsistent icon metaphors across nearby surfaces

Required acceptance:

- icons must reinforce labels rather than replace them in primary workflows
- the same action family should use the same icon everywhere it appears
- icon-only controls must always have accessible names

Suggested verification focus:

- screenshot review of the surfaces with the highest icon density
- accessibility checks for icon buttons and labelled controls

### Existing Tauri plugins (`log`, `notification`, `window-state`)

Current repo role:

- initialization and routing in [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts)
- runtime storage in [runtimeStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\runtimeStore.ts)
- operator-facing visibility in [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)

What is objectively worth improving:

- make diagnostics more useful than a raw recent-log list
- keep notifications high-signal and tied to real task outcomes
- make window-state persistence recoverable if it ever becomes awkward for a user

What should not be forced:

- desktop notifications for every minor event
- an overbuilt diagnostics console when the current operator needs are still lightweight

Implementation notes:

- for log plugin usage:
  - add filtering by level if current log volume justifies it
  - add copy/export only if it improves supportability or operator troubleshooting
- for notification plugin usage:
  - keep routing tied to the existing task-intent policy
  - only add click-through or follow-up behavior if it can be made deterministic
- for window-state usage:
  - add a reset/recover affordance only if users can actually get stuck with awkward persisted geometry

Regression risks:

- notification spam reducing trust instead of improving awareness
- diagnostics surfacing too much low-value internal noise
- state-recovery controls that reset more of the app than intended

Required acceptance:

- notifications remain opt-in and high-signal
- diagnostics additions make operator troubleshooting faster, not merely more detailed
- any reset/recover action is narrowly scoped and clearly labeled
- this must not create a second user-status system that competes with the task center and existing in-app status surfaces

Suggested verification focus:

- settings diagnostics tests
- desktop-productization service tests
- a quick manual task-completion notification sanity pass after any routing change

## Success Criteria

This plan is successful when:

- dense results/review tables expose more of the useful table behaviors already available in the stack
- contextual actions are standardized on shared primitives instead of ad hoc buttons and duplicated strips
- search/discovery works across more real user surfaces than just the command palette
- icons improve scanability without replacing text labels
- diagnostics, notifications, and persisted desktop state become more actionable, not just more present
- no existing backend-backed capability loses frontend reachability

## Implementation Order

1. Dense-table power behaviors
2. Shared contextual action primitives
3. Search/discovery expansion
4. Scanability and interaction polish
5. Desktop plugin usefulness pass

## Phase 1: Dense-Table Power Behaviors

Goal: use more of the existing TanStack table foundation where the app is already clearly table-shaped.

Primary files:

- [useHeadlessMatrixTable.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useHeadlessMatrixTable.ts)
- [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
- [ValidationWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ValidationWorkspacePanel.vue)
- [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue)
- [ExecutionWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ExecutionWorkspacePanel.vue)

Deliverables:

- define a shared column-state model for:
  - visible columns
  - pinned columns where warranted
  - persisted sort/filter posture where useful
- identify which dense surfaces should support column visibility immediately
- identify which surfaces genuinely benefit from pinned columns
- standardize row selection metadata and selected-summary coupling

Acceptance:

- dense table behavior feels more consistent across Top, Validation, Meta, and execution orders
- persisted table state improves repeat workflows instead of surprising users
- no panel gets spreadsheet-grade complexity unless the use case justifies it

## Phase 2: Shared Contextual Action Primitives

Goal: use `reka-ui` and `@floating-ui/vue` to reduce ad hoc dense-surface action clutter.

Primary files:

- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
- [WorkflowQuickSwitch.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\WorkflowQuickSwitch.vue)
- [ContextHelpTip.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ContextHelpTip.vue)
- [useAnchoredOverlay.ts](C:\Rust Projects\ta_desktop_demo\web\src\composables\useAnchoredOverlay.ts)
- likely result/review panels under [web/src/components](C:\Rust Projects\ta_desktop_demo\web\src\components)

Deliverables:

- define a shared row-action or item-action menu pattern
- identify duplicated inline button groups that should become contextual popovers or menus
- standardize disclosure behavior for advanced or secondary actions
- use shared collision-safe anchored positioning rather than per-surface custom placement logic

Acceptance:

- dense surfaces lose low-value button clutter without losing capability reachability
- contextual actions remain keyboard reachable
- the new primitives are shared, not re-invented per panel
- teleported menu content remains visually above local panel chrome in the built UI, not just in component tests

## Phase 3: Search / Discovery Expansion

Goal: use `fuse.js` beyond the command palette in places where the app already contains large operator-facing vocabularies.

Primary files:

- [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue)
- [HistoryWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\HistoryWorkspacePanel.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- stores/services behind saved perspectives and artifact history

Deliverables:

- evaluate and implement fuzzy search for:
  - Run Library / history
  - saved perspectives
  - strategy or indicator lookup where the vocabulary is large enough
- keep command search and local surface search behavior consistent
- define when a search should be scoped local vs global

Acceptance:

- search improves retrieval speed on real data-rich surfaces
- the app does not create overlapping search boxes that compete with the command palette
- results remain understandable and stable for short queries and typos
- the local-search surface explicitly tells the user it is scoped local, not a second global action surface

## Phase 4: Scanability And Interaction Polish

Goal: use the existing icon, interaction, and VueUse tooling more deliberately.

Primary files:

- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue)
- [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
- [RunFooterPanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\RunFooterPanel.vue)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- selected dense review surfaces under [web/src/components](C:\Rust Projects\ta_desktop_demo\web\src\components)

Deliverables:

- define a minimal icon grammar for:
  - task state
  - compare/pin actions
  - reports/artifacts
  - diagnostics levels
- use `@vueuse/core` more deliberately where it objectively helps:
  - reduced-motion-aware JS interactions
  - breakpoint/media-query-driven shell adjustments
  - stronger page visibility or idle state handling where background work matters

Acceptance:

- icons improve scan speed without replacing labels
- motion/responsiveness behavior respects the current reduced-motion and dense-workflow posture
- shell behavior becomes more adaptive without adding complexity for its own sake

## Phase 5: Desktop Plugin Usefulness Pass

Goal: get more operator value out of plugins that are already installed and already initialized.

Primary files:

- [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts)
- [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [runtimeStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\runtimeStore.ts)
- [taskStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\taskStore.ts)

Deliverables:

- improve diagnostics usefulness on top of existing log plumbing:
  - filtering by level/source if the current data supports it
  - operator-friendly copy/export actions
- improve notification usefulness:
  - verify current routing remains high-signal only
  - evaluate task-linked follow-up behavior where feasible
- improve window-state usefulness:
  - add an explicit reset/recover affordance if window persistence ever becomes awkward

Acceptance:

- logs, notifications, and state persistence do more than merely exist
- the operator can act on them more easily
- the desktop plugin surfaces remain optional, safe, and non-intrusive

## Open Checklist

### Phase 1: Dense-Table Power Behaviors

- [x] Audit current column-state needs across Top, Validation, Meta, and execution orders.
- [x] Decide where column visibility controls are objectively useful.
- [x] Decide where pinned columns are objectively useful.
- [x] Add shared persisted table-state support where it improves repeat review work.
  - shipped slices:
    - Top Results persisted column visibility + restore-defaults
    - Validation persisted column visibility + restore-defaults
    - Meta persisted column visibility + restore-defaults
  - intentionally deferred:
    - execution-order review
- [x] Standardize selected-row summary coupling across dense review surfaces.
  - current repo note: Top Results remains the only dense review surface with a selected-row summary rail, and the new table posture controls preserve that coupling
- [x] Add targeted regression coverage for any new table-state behavior.

### Phase 2: Shared Contextual Action Primitives

- [x] Inventory duplicated inline action clusters in dense panels.
  - first shipped target: repeated saved-artifact action strips in [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue)
- [x] Define a shared row/item action menu pattern.
  - shared primitive: [ContextActionMenu.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\ContextActionMenu.vue)
- [x] Replace at least one high-clutter button cluster with a shared contextual action primitive.
  - shipped slices:
    - saved meta artifact secondary actions
    - saved Stage 5 artifact secondary actions
    - saved policy-eval artifact secondary actions
- [x] Standardize advanced/disclosure behavior for secondary actions.
  - current repo note: `Load` remains inline as the primary action; refresh/open/reveal/delete now live behind the shared contextual menu
- [x] Add regression coverage for keyboard access and focus return on the new primitives.
  - focused regression coverage:
    - [ContextActionMenu.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\components\ContextActionMenu.test.ts)
    - [MetaWorkspacePanel.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.test.ts)
  - built-UI browser verification:
    - [meta-context-action-menu-2026-03-29.png](C:\Rust Projects\ta_desktop_demo\output\playwright\ui-review\meta-context-action-menu-2026-03-29.png)

### Phase 3: Search / Discovery Expansion

- [x] Audit which surfaces have enough vocabulary/history to justify local fuzzy search.
  - chosen first target: Run Library / Saved Runs in [HistoryWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\HistoryWorkspacePanel.vue)
  - already-shipped secondary target: saved perspectives in [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [x] Add `fuse.js`-backed search to the highest-value local surface after the command palette.
  - shipped slices:
    - Saved Runs search in [historyStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\historyStore.ts)
    - saved perspective search in [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue) backed by [perspectiveSearch.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\perspectiveSearch.ts)
- [x] Define and document the distinction between global command search and local surface search.
  - shipped slices:
    - [HistoryWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\HistoryWorkspacePanel.vue) labels Saved Runs search as a local library retrieval surface
    - [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue) labels saved perspective search as a local workstation retrieval surface
    - the command palette remains the only global action/navigation search
- [x] Add regression coverage for short-query, typo, and ranking behavior.
  - focused regression coverage:
    - [historyStore.search.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\historyStore.search.test.ts)
    - [HistoryWorkspacePanel.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\components\HistoryWorkspacePanel.test.ts)
    - [perspectiveSearch.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\perspectiveSearch.test.ts)
    - [SettingsWorkspacePanel.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.test.ts)
  - built-UI browser verification:
    - [history-fuzzy-search-2026-03-29.png](C:\Rust Projects\ta_desktop_demo\output\playwright\ui-review\history-fuzzy-search-2026-03-29.png)
    - [settings-perspective-search-2026-03-29.png](C:\Rust Projects\ta_desktop_demo\output\playwright\ui-review\settings-perspective-search-2026-03-29.png)

### Phase 4: Scanability And Interaction Polish

- [x] Define a minimal icon grammar for task state, compare/pin, artifacts, and diagnostics.
  - shared utility: [iconography.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\iconography.ts)
- [x] Apply that grammar to the highest-value scan surfaces without replacing labels.
  - shipped surfaces:
    - [TopWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TopWorkspacePanel.vue)
    - [MetaWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\MetaWorkspacePanel.vue)
    - [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue)
    - [SettingsWorkspacePanel.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.vue)
- [x] Audit current JS-driven interactions for reduced-motion-sensitive behavior.
  - shipped slice:
    - [VectorAlphaLogo.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VectorAlphaLogo.vue) now disables hover-driven animation when reduced motion is preferred
- [x] Add at least one breakpoint/media-query or visibility-aware shell improvement using `@vueuse/core`.
  - shipped slices:
    - [VectorAlphaLogo.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VectorAlphaLogo.vue) uses `useMediaQuery('(prefers-reduced-motion: reduce)')`
    - [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts) uses `useDocumentVisibility()` so OS notifications route only while the desktop window is not visible
- [x] Verify that added polish does not reintroduce visual clutter.
  - focused regression coverage:
    - [TaskCenterOverlay.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.test.ts)
    - [SettingsWorkspacePanel.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.test.ts)
    - [desktopProductization.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.test.ts)
    - [desktopNotificationPolicy.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\desktopNotificationPolicy.test.ts)
  - full verification stack remained green after the slice
  - browser-side screenshots:
    - [task-center-phase4-2026-03-29.png](C:\Rust Projects\ta_desktop_demo\output\playwright\ui-review\task-center-phase4-2026-03-29.png)
    - [settings-notification-copy-2026-03-29.png](C:\Rust Projects\ta_desktop_demo\output\playwright\ui-review\settings-notification-copy-2026-03-29.png)

### Phase 5: Desktop Plugin Usefulness Pass

- [x] Audit the current diagnostics surface for missing filtering/export affordances.
- [x] Add the highest-value diagnostics affordance on top of the existing log bridge.
  - shipped slices:
    - desktop log filter (`all`, `warnings and errors`, `errors only`)
    - copyable diagnostics snapshot
    - clear-desktop-logs action
- [x] Re-evaluate notification routing for current task outcomes and remove any low-signal cases.
  - shipped slices:
    - [desktopProductization.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.ts) now routes OS notifications only while the desktop window is not visible
    - [desktopNotificationPolicy.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\desktopNotificationPolicy.ts) remains the single intent-policy gate
- [x] Decide whether a window-state reset/recover affordance is warranted.
  - current repo decision:
    - not warranted yet
    - keep the current readiness/file-path diagnostics surface
    - revisit only if real awkward-geometry reports appear
- [x] Add focused regression coverage for any desktop-surface behavior changed in this pass.
  - focused regression coverage:
    - [SettingsWorkspacePanel.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\components\SettingsWorkspacePanel.test.ts)
    - [desktopProductization.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\services\desktopProductization.test.ts)
    - [desktopNotificationPolicy.test.ts](C:\Rust Projects\ta_desktop_demo\web\src\utils\desktopNotificationPolicy.test.ts)
  - browser-side screenshot:
    - [settings-notification-copy-2026-03-29.png](C:\Rust Projects\ta_desktop_demo\output\playwright\ui-review\settings-notification-copy-2026-03-29.png)

## Optional Visual Goals

These are explicitly optional later-phase goals. They are not required to complete the core existing-stack utilization plan, but they are worth tracking because they could add practical value if implemented carefully.

The standard for any item in this section is:

- it must improve interpretation, trust, orientation, or workflow speed
- it must not add decorative motion or visual noise for its own sake
- it must respect reduced-motion posture
- it must not compete with dense data surfaces, controls, or current analytical charts

### Practical Use Criteria

Any visual or animation work beyond the current SVG plan should meet at least one of these tests:

- helps the user understand state faster
- helps the user compare or rank candidates faster
- helps the user interpret robustness, provenance, or risk faster
- improves confidence in runtime/task status without adding clutter

If it does not clearly pass one of those tests, it should not be prioritized.

### Optional Goal A: Startup Animation

Intent:

- use the animated VectorAlpha logo as part of a brief startup/ready-state moment

Detailed implementation planning now lives in:

- [startup_animation_plan_2026-03-30.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\startup_animation_plan_2026-03-30.md)

Guardrails:

- only while real initialization is occurring
- should disappear as soon as the app is actually ready
- target duration should be brief, ideally `1` to `3` seconds
- must have a reduced-motion-safe behavior
- must not become a blocking splash screen that delays work

Potential value:

- makes startup feel intentional and polished
- gives a clear “desktop runtime is booting / ready” signal

### Optional Goal B: Practical Information Graphics

Intent:

- improve how analytical information is presented in compare, review, provenance, and artifact surfaces

High-value candidates:

- mini comparison visuals inside compare cards
- provenance / confidence graphics attached to selected-result summaries
- compact report or artifact previews
- richer task-state and runtime-status visuals where they reduce interpretation time

Guardrails:

- default to compact 2D information graphics first
- prioritize clarity over novelty
- avoid replacing tables or current `lightweight-charts` usage where they already work well

### Optional Goal C: Strictly Justified 3D Analytical Views

Intent:

- allow 3D only where the third dimension genuinely explains something that 2D cannot communicate as effectively

Potential justified cases:

- parameter response surfaces
- robustness landscapes
- slippage / turnover / objective tradeoff surfaces
- local stability surfaces over two parameters plus a result metric

Guardrails:

- 3D must be an optional alternate analytical view, not the default
- every 3D view must have a 2D fallback such as heatmap, contour, or scatter projection
- exact values must remain inspectable through hover, crosshair, slices, or linked tabular readouts
- avoid cinematic camera motion, decorative perspective, or unnecessary scene chrome

### Optional Goal D: Quality-First Visual Review Discipline

Intent:

- ensure any future visual work is held to the same evidence-based quality bar as the rest of the frontend

Requirements if these optional goals are pursued:

- add focused visual acceptance criteria before implementation
- verify reduced-motion handling
- verify scanability on dense screens
- verify no regression to bundle discipline or first-paint behavior
- prefer shipping one narrowly useful visual at a time rather than a large visual redesign

### Optional Checklist

- [x] Decide whether a startup animation is worth shipping or should remain deferred.
  Current outcome: ship the first-wave in-window overlay implemented in [startup_animation_plan_2026-03-30.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\startup_animation_plan_2026-03-30.md).
- [ ] Identify the highest-ROI practical information graphic that is not already covered by the current chart/table stack.
- [ ] Define strict acceptance criteria before adding any non-SVG motion or visual flourish.
- [ ] If a 3D analytical view is considered, document the practical justification and the required 2D fallback first.
- [ ] Verify any future visual enhancement against reduced-motion, bundle impact, and dense-workflow scanability.

## Out Of Scope For This Plan

These may still be valid later, but they are not part of this implementation plan:

- `vue-router` adoption
- new charting libraries
- tray/menu/updater/global-shortcut work
- strategy-editor or diff tooling
- new notification or animation libraries

## Suggested Starting Point

The planned implementation phases are now complete.

If work continues next, choose from:

- the optional visual goals below
- the separate desktop-productization plan
- the SVG asset integration backlog after external asset generation is complete

## SVG Asset Briefs For External Creation

The app can use SVGs safely and effectively inside the current Tauri/Vue stack, but only if they are designed to reinforce workflow clarity rather than act as decoration.

If you want paste-ready generation prompts instead of design briefs, use:

- [svg_generation_prompts_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\svg_generation_prompts_2026-03-28.md)

The highest-value SVG work for this repo is:

- compact domain-specific iconography that `lucide-vue-next` does not already cover well
- provenance/confidence/status glyphs
- compare/pin/review affordances
- one or two restrained empty-state illustrations

The goal is not to replace the existing Lucide set. The goal is to add a small family of custom quant-workstation vectors that feel native to this product.

### Global SVG Production Rules

Please follow these rules for every SVG you prepare:

- deliver raw `.svg` files only
- no embedded JavaScript
- no raster images embedded inside the SVG
- prefer pure stroke-based or restrained duotone vector work
- keep backgrounds transparent
- avoid gradients unless a specific illustration brief below asks for a very subtle one
- design for dark and light themes
- use `currentColor` where possible for single-tone icons, or keep fills/strokes easy to remap later
- use clean `viewBox` values such as `0 0 24 24`, `0 0 32 32`, `0 0 48 48`, or `0 0 96 96`
- prefer rounded linecaps/linejoins
- avoid tiny visual detail that collapses at small sizes
- do not bake text labels into the SVG
- do not make them look glossy, glassy, neon, or crypto-fintech branded

Animation guidance:

- animation is optional, not required
- if you animate, keep it subtle and purpose-driven
- use transform / opacity / stroke-dashoffset style motion only
- no continuous hyperactive motion
- any animated version should also have a static counterpart
- animated SVGs must still look good when motion is disabled

### Visual Direction

Target feel:

- operator bench
- quant workstation
- restrained
- technical
- trustworthy
- quiet confidence rather than flashy energy

Avoid:

- 3D chrome
- glowing outlines
- cartoon finance metaphors
- trophy / winner imagery
- rockets / moons / lightning-bolt hype visuals

### Asset Group 1: Provenance And Confidence Glyph Set

Priority: very high

Purpose:

- support provenance ribbons, confidence chips, stale-state indicators, and result summaries

These should be small, static SVG icons intended for 14px to 18px rendered size.

Please create one icon for each of these concepts:

1. `oos_stable`
   Description:
   - a calm, balanced mark suggesting stable out-of-sample behavior
   - suggested motif: two aligned bands or two parallel trend traces with a small check-like stabilizer
   - should feel reliable, not celebratory

2. `low_sample`
   Description:
   - a clear “thin evidence” mark
   - suggested motif: sparse histogram bars or a tiny cluster of only two or three points
   - should communicate insufficient depth, not total failure

3. `slippage_sensitive`
   Description:
   - a mark indicating fragility under execution friction
   - suggested motif: a price path disturbed by a sideways drag or offset channel
   - should read as “performance sensitive to friction”

4. `regime_concentrated`
   Description:
   - a mark indicating behavior concentrated in a narrow market regime
   - suggested motif: one emphasized band among several muted segments
   - should feel like “narrow context dependence”

5. `validation_stale`
   Description:
   - a stale-validation marker
   - suggested motif: a checkpoint or badge with a subtle time drift / lag mark
   - should read as outdated, not broken

6. `dataset_changed`
   Description:
   - a dataset lineage shift marker
   - suggested motif: stacked records or bars with one shifted/reloaded layer
   - should communicate “input context changed since review”

7. `review_attention`
   Description:
   - a compact attention flag for “worth a closer look”
   - suggested motif: a focus frame or bracket around one central signal point
   - should not look like a generic warning triangle

Preferred style:

- 1.75px to 2px stroke feel at a 24x24 source size
- mostly monochrome with optional single accent region

Animation:

- not needed

### Asset Group 2: Compare / Pin Workflow Glyphs

Priority: very high

Purpose:

- support compare tray actions and Top Results review actions

These should be clean UI icons intended for 16px to 18px rendered size.

Please create:

1. `pin_candidate`
   Description:
   - not a generic office pushpin
   - should feel like “hold this result in the review set”
   - suggested motif: a compact locator pin merged with a tiny result card or metric tile

2. `compare_candidates`
   Description:
   - should clearly communicate side-by-side comparison
   - suggested motif: two aligned result cards or two mirrored metric columns with a shared axis

3. `replace_pinned`
   Description:
   - indicates swapping the current selected candidate into an existing compare slot
   - suggested motif: directional exchange between two compact cards, but with low visual noise

4. `remove_from_compare`
   Description:
   - remove one pinned result from the compare set
   - suggested motif: a compare tile with a quiet subtraction mark

5. `candidate_focus`
   Description:
   - “make this the primary review candidate”
   - suggested motif: a highlighted center tile with surrounding muted tiles

Animation:

- optional only for `compare_candidates`
- if animated, use a very short alternating emphasis between two aligned tiles
- no perpetual bouncing or rotation

### Asset Group 3: Artifact / Report / Reveal Glyphs

Priority: high

Purpose:

- improve scanability for report/reveal/export actions that are currently mostly text-driven

These should be small static icons intended for 16px size.

Please create:

1. `report_workbook`
   Description:
   - a structured workbook/report icon
   - should imply tabular analysis output, not a generic document
   - suggested motif: a sheet with grid bands and one highlighted summary section

2. `artifact_bundle`
   Description:
   - indicates a saved run artifact package
   - suggested motif: a compact stack or archive block with one visible result card layer

3. `reveal_path`
   Description:
   - reveal in file system / open containing folder
   - suggested motif: folder or path tray with one highlighted outward direction

4. `export_snapshot`
   Description:
   - export current reviewed state
   - suggested motif: a result card leaving a bounded frame

5. `reopen_report`
   Description:
   - open last generated report
   - suggested motif: report workbook with a directional return/open cue

Animation:

- not needed

### Asset Group 4: Task Lifecycle And Runtime State Glyphs

Priority: high

Purpose:

- support task center, diagnostics, footer summaries, and optional notification visuals

These should work at 14px to 18px rendered size.

Please create a cohesive family for:

1. `task_queued`
   Description:
   - a waiting/queued state
   - suggested motif: stacked items with the front item paused in line

2. `task_running`
   Description:
   - active processing
   - suggested motif: segmented ring or staged progression mark
   - should feel like computation, not a generic media spinner

3. `task_cancelling`
   Description:
   - in-between state, neither active nor done
   - suggested motif: running mark interrupted by a soft stop/barrier

4. `task_completed`
   Description:
   - successful completion
   - suggested motif: resolved progression ending in a calm confirmation mark

5. `task_failed`
   Description:
   - failure requiring attention
   - suggested motif: broken progression path, not an alarm siren

6. `task_cancelled`
   Description:
   - intentionally stopped by the user
   - suggested motif: halted progression with a neutral stop mark

7. `runtime_ready`
   Description:
   - application/runtime ready state
   - suggested motif: compact workstation/status panel with a stable indicator

Animation:

- only `task_running` is worth animating
- suggested animation:
  - a slow segmented sweep
  - or a subtle pulse along a circular / staged path
- target feel:
  - measured
  - calm
  - computational
- avoid:
  - spinner cliché
  - sharp flashing
  - constant attention theft

### Asset Group 5: Empty-State Illustration Set

Priority: medium

Purpose:

- upgrade a few empty or first-use states without adding clutter

These should be larger illustrations, designed at `96x96` or `128x128`, then scaled down in UI.

Please create:

1. `empty_results_workspace`
   Description:
   - a sparse analysis bench waiting for results
   - suggested motif: a muted table/grid area, one empty chart frame, and a small focal placeholder
   - should imply “run something, then review here”

2. `empty_compare_workspace`
   Description:
   - a compare tray or dual-review surface waiting for pinned candidates
   - suggested motif: two empty comparison cards with a central shared axis

3. `empty_artifact_library`
   Description:
   - saved runs / reports area with no artifacts yet
   - suggested motif: organized archive trays or shelves with one highlighted empty slot

Visual constraints:

- extremely restrained
- low-detail
- should work in monochrome plus one soft accent
- no human characters
- no mascots

Animation:

- optional
- if animated, use a very slow ambient reveal only:
  - soft opacity fade on one focal segment
  - tiny axis sweep
  - gentle pulse on one placeholder tile

### Asset Group 6: Optional Shell Mode Glyphs

Priority: medium-low

Purpose:

- future support for stronger mode framing such as build / review / operate without relying on generic icons

Please create, only if you have time:

1. `mode_build`
   Description:
   - strategy/data authoring posture
   - motif: structured construction grid with one active parameter tile

2. `mode_review`
   Description:
   - results and comparison posture
   - motif: reviewed result card plus chart summary

3. `mode_operate`
   Description:
   - execution / runtime posture
   - motif: compact runtime console with one active status trace

Animation:

- not needed

## Delivery Format I Want From You

For each SVG you provide, please include:

- the raw `.svg`
- the intended name
- the intended rendered size range
- whether it is:
  - static only
  - static + optional animated version
- a short note if any part is meant to be color-mapped later with CSS

Preferred file naming:

- `provenance-oos-stable.svg`
- `provenance-low-sample.svg`
- `compare-pin-candidate.svg`
- `task-running.svg`
- `empty-results-workspace.svg`

If you provide animated variants, use separate files such as:

- `task-running.animated.svg`
- `empty-compare-workspace.animated.svg`

## Animation Rules If You Choose To Animate

If you animate any SVGs, please keep to these rules:

- animation must survive reduced-motion fallback cleanly
- no embedded JS
- no rapid looping under 1.2s
- no flashing
- no scale-jump bounce
- no decorative perpetual motion on static informational icons
- animated SVGs should still read clearly when rendered as a frozen frame

## Highest-Priority Deliverables

If you only make a small first batch, make these first:

1. provenance/confidence glyph set
2. compare/pin workflow glyphs
3. task lifecycle glyphs

That batch would give the app the most immediate product value.
