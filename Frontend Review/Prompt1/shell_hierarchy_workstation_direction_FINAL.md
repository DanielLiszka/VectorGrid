# Shell Hierarchy and Workstation Direction — Final

Date: 2026-04-03

Scope: final repo-grounded shell/workstation direction for the current VectorGrid Vue/Tauri frontend, based on Prompt 1 from `frontend_ui_ux_research_prompt_pack_2026-04-02.md` plus direct review of the current shell component files.

Status: implementation-ready recommendation for the next shell-focused frontend pass.

---

## 1. Executive summary

VectorGrid should **keep its tab-first workstation shell** and refine it into a **quieter, stratified top shell**, not pivot to a generic left-sidebar dashboard, not demote visible navigation behind the command palette, and not promote the current dev quick-switch into a user-facing navigation model.

The correct near-term direction is:

- keep **workspace tabs** and **workflow-step tabs** as the primary visible orientation system
- keep the **controls/results split** as the core working posture for workflow work
- add a **first-class visible task-center launcher** to the persistent shell
- reduce the titlebar to **desktop chrome plus one compact operational affordance**, instead of letting it act like a second context dashboard
- tighten the shell **context strip** into a smaller, more decision-relevant summary instead of turning it into a broader adaptive rail right now
- keep the **command palette** as the global go/do accelerator, not the main orientation system
- keep **saved perspectives** in Settings and palette flows for now, rather than giving them permanent shell real estate yet
- preserve existing keyboard semantics, active-only/lazy posture, and canonical capability ownership in the main panels

This direction is narrower than a full shell redesign. The repo already has the right primitives; the biggest problem is **hierarchy duplication and weighting**, not lack of shell features.

---

## 2. Source basis and current-state evidence

This direction is grounded in the following repo files and current implementation surfaces:

### Repo-level direction and invariants

- `AGENTS.md`
- `REPO_MAP.md`
- `README.md`
- `frontend_ui_ux_research_prompt_pack_2026-04-02.md`
- `frontend_design_system_spec_2026-03-28.md`
- `frontend_target_comps_2026-03-28.md`
- `frontend_functional_coverage_matrix_2026-03-28.md`
- `workstation_behavior_plan_2026-03-28.md`
- `existing_stack_utilization_plan_2026-03-28.md`

### Current shell implementation files reviewed directly

- `App.vue`
- `VueWorkspaceShell.vue`
- `DesktopTitlebar.vue`
- `WorkflowQuickSwitch.vue`
- `CommandPaletteOverlay.vue`

### Most important current code facts confirmed in the component files

`App.vue` already centralizes the global shell surfaces and lazy panel ownership. It mounts `DesktopTitlebar`, `StartupAnimationOverlay`, `VueWorkspaceShell`, `CommandPaletteOverlay`, `AppRuntimeBanner`, `TaskCenterOverlay`, `RunFooterPanel`, context docks, and panel surfaces in one place. It also keeps `WorkflowQuickSwitch` explicitly **DEV-only** through a local gate. It uses active-only/lazy mounting for heavyweight panels such as Data, Optimize, and Overview.

`VueWorkspaceShell.vue` already owns the visible shell structure: workspace tabs, workflow-step tabs, context strip, controls/results tab rows, split-versus-stacked layout behavior, live selection announcement, and keyboard navigation across tab rows. It also already derives run context from stores and exposes the current context strip fields as `Dataset`, `Range`, `Rows`, `Strategy`, `Backend`, `Status`, and `Loaded ID`.

`DesktopTitlebar.vue` already shows product identity, workspace/workflow summary, subtitle, dataset/task/status chips, Search, Settings, a primary Run/Stop action, and desktop window controls. This confirms the titlebar is already carrying both desktop chrome and a second layer of shell context.

`WorkflowQuickSwitch.vue` is a developer-oriented popover that exposes raw workspace, workflow, and workflow-step toggles and even shows raw state like `ws=... · wf=... · step=...`. It is useful for development and testing, but it is not the right end-user shell abstraction.

`CommandPaletteOverlay.vue` already behaves like a real command palette with dialog + combobox/listbox semantics and grouped commands across `actions`, `data`, `exports`, `layout`, and `navigation`. It already navigates among workflow steps and workspaces, toggles controls visibility, and triggers run/export/data actions.

---

## 3. What the repo already decided

These shell-level decisions are already settled enough that the shell direction should treat them as baseline, not reopen them casually.

### 3.1 Product posture is workstation-first, not dashboard-first

The design system explicitly says VectorGrid should read as an **operational desktop tool**: workflow first, data-dense but readable, strong state visibility, progressive disclosure over clutter, and one obvious keyboard-safe path through every primary task.

### 3.2 The shell owns the main workflow skeleton

The design system says the shell owns:

- workspace selection
- workflow selection
- the controls/results split

It also explicitly says docks and overlays are contextual aids only, not primary structure.

### 3.3 The target shell composition is already defined

The target comps document already defines the Workflow Shell composition as:

- app title and workflow subtitle
- workspace tab row
- workflow-step tab row
- shell context strip
- controls/results tab rows
- task center launcher
- workflow-only footer

That means the repo already has a clear intended top-level shell model.

### 3.4 Saved perspectives, compare flows, provenance, stale cues, lifecycle orchestration, and density are real product value

The workstation plan already establishes these behaviors as core workstation value, not optional polish. That matters because it means shell changes should serve those behaviors rather than compete with them.

### 3.5 Capability reachability is a hard guardrail

The functional coverage matrix is explicit: UX cleanup is allowed, but every backend-backed capability must retain at least one clear frontend path. Canonical panel ownership must remain visible, and shortcuts such as footer, docks, and palette must not quietly replace primary surfaces.

### 3.6 Existing stack/productization work already favors shared, keyboard-safe primitives over custom one-off shell invention

The existing-stack plan already established shared command/search, local search scoped to the right surfaces, shared action menus, task vocabulary, and additive scanability improvements. That argues for **refining** the current shell model rather than swapping it out.

---

## 4. What the repo already implemented

This matters because the right next step depends on what is already real, not what is only described in planning docs.

### 4.1 The current shell already has the right navigation layers

`VueWorkspaceShell.vue` already contains:

- stable workspace tab row
- stable workflow-step tab row
- controls/results tab rows when appropriate
- store-driven visibility rules for different workspaces and steps
- keyboard movement across those rows
- live announcement of the current shell selection

This is not a shell that lacks structure. It is a shell that currently needs **better hierarchy discipline**.

### 4.2 The shell already behaves like a workstation more than a dashboard

`VueWorkspaceShell.vue` still centers the work in a controls/results plane, with split and stacked layout behavior depending on viewport width and current layout mode. That is consistent with a workstation posture and should be preserved.

### 4.3 The command palette is already strong enough to remain secondary

`CommandPaletteOverlay.vue` already has:

- real dialog semantics
- combobox/listbox search behavior
- grouped command categories
- workflow-step and workspace navigation commands
- layout toggles
- action and export commands

That means the palette is already a strong global accelerator. The shell does **not** need to make it more primary.

### 4.4 The titlebar is already too context-heavy to ignore

`DesktopTitlebar.vue` currently duplicates:

- workspace/workflow identification
- subtitle
- dataset state
- task count/state
- run/stop action
- settings access

That is useful evidence: the main shell problem is not “we need more top-shell affordances.” It is “we already have too many parallel context surfaces near the top.”

### 4.5 The task center already exists as a product surface

The target comps, workstation plan, and current global mounting in `App.vue` all confirm that the task center already exists and matters. What is missing is a clearer **visible launcher role** in the shell.

### 4.6 Quick switch is already correctly treated as non-canonical

`WorkflowQuickSwitch.vue` is DEV-only in `App.vue`, which is the right current posture. The shell should not collapse into that quick-switch model.

### 4.7 Performance posture is already good enough that shell work should not destabilize it

`App.vue` already preserves active-only/lazy behavior for several heavy surfaces. The workstation and existing-stack docs also already define bundle-discipline expectations. Shell work should keep that posture intact.

---

## 5. What remains genuinely unresolved

The current shell is not missing its core model. The unresolved questions are mostly about **relative prominence, duplication, and surface roles**.

### 5.1 The top shell is carrying duplicated context

There are too many context-bearing layers across:

- titlebar
- shell header/subtitle area
- context strip
- footer
- task center / task chip surfaces

The user can currently see state, but the weighting is noisier than it needs to be.

### 5.2 The target shell calls for a task-center launcher, but the visible shell does not yet give it enough prominence

The task center exists, but it is not yet clearly treated as the main async lifecycle inbox in the visible navigation model.

### 5.3 Saved perspectives exist, but their shell-level visibility is unresolved

The workstation plan made perspectives important, and the existing-stack work already added local search for them in Settings. The remaining question is **whether they should become permanent shell chrome now**. The answer in this document is **not yet**.

### 5.4 Settings continuity is still awkward

The repo map notes that when Settings is active, `App.vue` mounts both `SettingsWorkspacePanel.vue` and `ExecutionWorkspacePanel.vue`. That is workable, but it is still an odd shell condition that deserves later cleanup.

### 5.5 The shell still lacks one crisp role model for titlebar, palette, task center, context strip, footer, and docks

These surfaces are all individually reasonable, but the hierarchy among them is not yet sharp enough.

---

## 6. Strongest remaining shell-level UX weaknesses

### 6.1 Titlebar duplication

The titlebar currently repeats too much of what the main shell already says. It repeats workspace/workflow identity and subtitle, then adds dataset/tasks/status chips, and then adds a prominent run/stop action. This makes the shell feel heavier and more dashboard-like than the repo’s workstation thesis intends.

### 6.2 Missing visible task-center launcher

The task center should be a clear visible entry point because the repo already treats task lifecycle orchestration as a major workstation behavior. Right now, the shell has the overlay but not a strong visible launcher role.

### 6.3 Context strip overload risk

`VueWorkspaceShell.vue` currently exposes seven context fields by default. That is a reasonable data inventory, but it is more than the shell should need at all times. The strip is becoming an always-on telemetry shelf rather than a decision-focused summary.

### 6.4 Too many top-level status carriers

Status is currently expressed across titlebar chips, shell context strip, footer/task surfaces, and possibly runtime banner/task center. This risks repetitive rather than clarifying status language.

### 6.5 Unclear future home for perspectives

The repo clearly supports perspectives, but a shell-level perspective switcher is not yet justified strongly enough to add permanent chrome. The shell direction needs to say that explicitly so the repo does not quietly accrete another band.

### 6.6 Settings exception posture

Settings still feels more exceptional than the other workspaces. That should be improved eventually, but not by adding more shell complexity first.

---

## 7. Recommended shell direction

### 7.1 Adopted direction

Adopt a **quieter, stratified top shell** with three persistent responsibilities:

1. **Desktop chrome layer** — `DesktopTitlebar.vue`
2. **Visible navigation layer** — workspace tabs, workflow-step tabs, explicit task-center launcher in `VueWorkspaceShell.vue`
3. **Compact shell context strip** — still present, but smaller and more decision-relevant

Under those layers, keep the current **controls/results working plane** and the **workflow-only footer**.

This is the right direction because it improves the shell by clarifying **role boundaries**, not by adding a new navigation model.

### 7.2 Shell structure after refinement

#### Band 1: Desktop chrome

Owned by `DesktopTitlebar.vue`.

Purpose:

- product identity
- current workspace/workflow summary
- command-palette entry
- settings entry
- one context-aware primary action
- window controls
- optionally one compact task/status launcher chip

Not its purpose:

- a full run context dashboard
- a second shell context strip
- the main place where users read dataset + range + backend + status simultaneously

#### Band 2: Visible navigation band

Owned by `VueWorkspaceShell.vue`.

Purpose:

- workspace tabs
- workflow-step tabs when in Workflow workspace
- explicit task-center launcher
- visible orientation and mode change

This becomes the unambiguous answer to:

- where am I?
- what area am I in?
- what workflow stage am I in?
- where do I go for queued/running/completed work?

#### Band 3: Compact context strip

Also owned by `VueWorkspaceShell.vue`.

Purpose:

- concise current decision context
- small number of state reminders that inform the current work plane
- actionable jumps only where they lead to canonical panels

Not its purpose:

- serving as a second nav bar
- carrying every available piece of current run metadata
- becoming a large adaptive right-rail system yet

#### Main plane

Keep:

- controls/results split
- step-specific controls tab row
- results tab row
- split/stacked responsive behavior
- keyboard-safe path between tab rows and working panels

#### Footer

Keep it workflow-only and compact.

### 7.3 Why this wins now

This direction wins because it does the most important things without overreaching:

- it preserves the repo’s already-correct tab-first workstation identity
- it directly solves the most obvious current weakness: duplicated top-shell context
- it promotes the one surface the target comps explicitly say should be visible: the task-center launcher
- it does not spend shell complexity budget on a permanent perspective switcher yet
- it does not destabilize current keyboard semantics or lazy-loading posture

---

## 8. Explicitly retained current strengths

The following current strengths should remain unchanged in principle.

### 8.1 Tab-first workstation vocabulary

Keep the current visible shell vocabulary of workspace tabs, workflow-step tabs, controls tabs, results tabs, and aggregate subworkspace routing. This is one of the repo’s strongest shell assets.

### 8.2 Controls/results working posture

Do not replace the controls/results split with a generic dashboard or a chart-first canvas. The shell and target comps already assume that the main work alternates between authoring/setup and review/analysis.

### 8.3 Keyboard-safe tab semantics and overlay behavior

The shell already has good accessible tab behavior, live selection summaries, and palette semantics. Preserve and extend those patterns rather than replacing them.

### 8.4 Command palette as accelerator, not primary shell

The command palette already does enough. It should stay powerful and global, but still secondary to visible shell orientation.

### 8.5 DEV quick switch gating

Keep `WorkflowQuickSwitch.vue` gated for development/testing. Do not quietly normalize it into the production navigation model.

### 8.6 Lazy/active-only panel posture

Keep the current active-only mounting discipline and bundle-awareness. Shell cleanup should not make panel switching heavier or cause hidden heavy surfaces to stay mounted unnecessarily.

### 8.7 Workflow-only footer

Keep the footer compact and scoped to workflow status rather than letting it expand into another global dashboard band.

---

## 9. Explicitly rejected shell ideas

### 9.1 Generic left sidebar / dashboard-shell redesign

Reject. The repo already has a better-fit shell model for this product.

### 9.2 Command-palette-first navigation model

Reject. The palette should stay an accelerator layered on top of visible workbench landmarks.

### 9.3 Promoting `WorkflowQuickSwitch.vue` into end-user shell navigation

Reject. It is a developer/testing aid and exposes raw state toggles rather than a polished user mental model.

### 9.4 Permanent shell-level perspective switcher now

Reject for now. Perspectives are real product value, but current repo evidence supports them as Settings-managed workstation posture with palette support, not yet as always-visible shell chrome.

### 9.5 Large adaptive context rail right now

Reject for now. The current shell needs a **tighter strip**, not a larger dynamic shell subsystem.

### 9.6 Duplicating task state across titlebar, context strip, footer, and task center

Reject. The task center should be the detailed async lifecycle home; titlebar and footer should summarize and deep-link.

### 9.7 Hiding canonical panel actions behind shell polish

Reject. Capability reachability must remain anchored in the right panels.

---

## 10. Adopt now / defer / reject breakdown

| Category | Adopt now | Defer | Reject |
| --- | --- | --- | --- |
| Shell model | Stratified top shell with clearer role separation | Larger shell topology changes | Sidebar/dashboard model |
| Titlebar | Quieter chrome + one compact operational affordance | Fully stripping it to near-bare chrome | Titlebar as second dashboard row |
| Task center | Add visible launcher to shell nav band | Broader task-center redesign | Leaving task center visually secondary |
| Context strip | Reduce to smaller default set, add overflow/actionability selectively | Larger adaptive rail | Expanding it into another full surface |
| Perspectives | Keep in Settings + palette flows | Small visible Views/Perspectives menu later if usage proves it | Permanent perspective band now |
| Quick switch | Keep DEV-only | None | Promote to production navigation |
| Palette | Keep as global go/do surface | Additional commands only when they reinforce visible structure | Using it as the main navigation paradigm |
| Settings continuity | Revisit after shell cleanup | Broader structural rework later | Trying to solve it by adding more shell chrome |

---

## 11. Surface-by-surface shell behavior map

### 11.1 Titlebar

**Current evidence**

`DesktopTitlebar.vue` currently includes:

- VectorGrid identity via `VectorAlphaLogo`
- workspace and workflow-step summary
- subtitle derived from workflow/workspace
- dataset chip
- tasks chip
- status chip
- Search button opening the command palette
- Settings button switching workspace
- primary Run/Stop action
- native-style window controls

**Final direction**

The titlebar should be treated as **desktop chrome plus one compact operational affordance**.

Keep:

- identity
- workspace/workflow summary
- subtitle
- Search / command palette entry
- Settings entry
- one context-aware primary action
- window controls

Reduce:

- dataset/tasks/status chip burden

Recommended change:

- collapse the three current chips into either:
  - one compact task/status launcher chip, or
  - one small chip cluster where only one item is always visible and the rest are condensed
- move dataset detail emphasis to the shell context strip rather than the titlebar
- keep the primary action context-aware, but do not let the titlebar imply that optimization is the universal primary action across all workspaces

**Important rule**

The titlebar should never become the canonical place to inspect detailed state.

### 11.2 Workspace tabs

**Current evidence**

`VueWorkspaceShell.vue` already renders `workflow`, `library`, `aggregate`, `pro`, and `settings` as a tablist.

**Final direction**

Workspace tabs remain the **first visible orientation system**.

Rules:

- always visible
- stable order
- explicit current state
- keyboard-safe
- visually calmer than secondary status chrome
- no replacement by hidden command-only navigation

The workspace tab row answers “what operating area am I in?”

### 11.3 Workflow steps

**Current evidence**

`VueWorkspaceShell.vue` already renders workflow steps `1 Data Source`, `2 Strategy Setup`, `3 Validation`, `4 Summary`, `5 Finalizer` when relevant.

**Final direction**

Workflow steps remain the **second visible orientation system**, only within the Workflow workspace.

Rules:

- keep numbered step vocabulary
- keep accessible tab semantics
- keep them visually subordinate to workspace tabs but still clearly active
- do not duplicate them elsewhere in the shell

The workflow-step row answers “where am I in the default research flow?”

### 11.4 Quick switch

**Current evidence**

`WorkflowQuickSwitch.vue` exposes raw workspace/workflow/workflow-step state and is mounted only when DEV gate conditions pass.

**Final direction**

Keep it DEV-only.

Rules:

- do not ship it as a production shell primitive
- do not use it as a shortcut for avoiding better visible shell hierarchy
- if a future user-facing quick-switch is ever needed, it should be framed around **Views/Perspectives**, not raw internal workflow state

### 11.5 Command palette

**Current evidence**

`CommandPaletteOverlay.vue` already groups commands across actions, data, exports, layout, and navigation, and exposes workspace/workflow-step navigation commands plus run/export/data actions.

**Final direction**

The command palette remains the global **go/do** surface.

Keep:

- command search
- navigation commands
- run/export/data actions
- layout toggles
- keyboard-first access

Add later only if justified:

- perspective restore commands
- shell-local layout commands that reinforce visible structure

Refine:

- keep terminology aligned with visible shell language
- for example, if the product language is “controls panel” or “setup panel,” avoid leftover terms like “sidebar” in command descriptions

**Important rule**

The palette should never become the main way a user understands the app’s structure.

### 11.6 Task center

**Current evidence**

The task center already exists as a global overlay and the workstation plan already treats task lifecycle orchestration as first-class workstation value.

**Final direction**

The task center should become the visible shell’s **first-class async work launcher**.

What it should own:

- queued / running / completed / failed / cancelled grouping
- related-result navigation
- high-signal lifecycle state
- detailed task inspection

What other surfaces should do instead:

- titlebar: summarize and link to task center
- footer: summarize current workflow task state and link to task center
- context strip: only echo a compact status summary when truly helpful

**Implementation implication**

Add an explicit launcher near the workspace/workflow navigation region in `VueWorkspaceShell.vue`, rather than relying only on footer or hidden commands.

### 11.7 Footer

**Current evidence**

The target comps already require a workflow-only footer, and the workstation plan already places in-app task routing primarily in task center + footer + diagnostics.

**Final direction**

Keep the footer workflow-only and compact.

Rules:

- summarize current workflow status
- expose a next-action or related-result posture when useful
- do not duplicate the full task-center information model
- do not let the footer become another navigation bar

### 11.8 Context strip

Although not separately requested in the prompt-pack surface list, this is a major shell element and needs explicit direction.

**Current evidence**

`VueWorkspaceShell.vue` currently shows:

- Dataset
- Range
- Rows
- Strategy
- Backend
- Status
- Loaded ID

**Final direction**

Keep the context strip, but tighten it.

Default visible set recommendation:

- Dataset
- Range
- Strategy
- Backend
- Status

Move to overflow/disclosure by default:

- Rows
- Loaded ID

Interaction rule:

- any context item that becomes clickable must jump to the canonical owning surface
- example: Dataset jumps to Data workspace/panel, Status jumps to Task Center, Strategy jumps to Strategy/Optimize surface

Do not let the context strip become another dense telemetry ribbon.

### 11.9 Docks and overlays

Again, not one of the mandatory prompt-pack surface rows, but important for shell direction.

Final rule:

- keep docks and overlays as contextual aids only
- do not promote them into co-equal primary navigation surfaces
- preserve their shortcut value while keeping canonical actions in the main panels

---

## 12. Capability-preservation note

This shell direction preserves the functional-coverage guardrail.

### 12.1 Canonical primary paths remain in the main panels

Examples:

- optimization launch remains canonical in `OptimizeWorkspacePanel.vue`
- validation launch/report/export remains canonical in `ValidationWorkspacePanel.vue`
- data loading and import posture remain canonical in `DataWorkspacePanel.vue`
- analysis capabilities remain canonical in analysis panels such as Overview, Top, Pareto, Drill, and Heatmap
- runtime and environment utilities remain canonical in `SettingsWorkspacePanel.vue`
- execution actions remain canonical in `ExecutionWorkspacePanel.vue`
- history/artifact operations remain canonical in `HistoryWorkspacePanel.vue` and related artifact surfaces

### 12.2 Shell surfaces stay secondary where they should

- titlebar Run/Stop is a convenience, not a replacement for workflow panels
- command palette commands are accelerators, not canonical ownership
- docks are contextual shortcuts, not primary homes for capability sets
- footer is summary, not primary action inventory

### 12.3 Shell cleanup must never remove the only obvious path to a backend-backed capability

Any future consolidation that removes a duplicate entry point must explicitly document the surviving canonical path.

---

## 13. Concrete file and component touch points

### 13.1 `App.vue`

Keep responsibilities:

- global surface mounting
- lazy async panel registry
- dev gating of `WorkflowQuickSwitch`
- global shell ownership composition

Most relevant near-term changes:

- preserve current lazy/active-only posture while shell surfaces are refined
- keep `WorkflowQuickSwitch` dev-only
- later, revisit the Settings/Execution combined mounting condition without broad shell churn

### 13.2 `VueWorkspaceShell.vue`

This is the main implementation center for Prompt 1.

High-value changes:

- reduce header duplication relative to the titlebar
- add explicit **task-center launcher** in the persistent visible navigation area
- tighten context strip defaults from 7 items to a smaller visible set
- add overflow/disclosure for lower-value context metadata
- optionally make selected context cards actionable when they jump to canonical panels
- preserve keyboard tab semantics and live shell-selection announcement
- preserve split/stacked responsive behavior and current working-plane ownership

Specific current code areas affected:

- `contextItems` computed block
- shell header/template around workspace tabs and workflow-step tabs
- context strip template
- responsive shell layout near split/stacked logic

### 13.3 `DesktopTitlebar.vue`

High-value changes:

- collapse current dataset/task/status chip burden
- preserve Search and Settings entries
- keep one context-aware primary action, but make it more clearly secondary to canonical panel ownership
- consider replacing the current three chips with one compact task/status launcher chip
- preserve desktop window controls and drag/maximize behavior

Specific current code areas affected:

- `datasetChip`, `activityChip`, `activityDetail` computed values
- titlebar chips template block
- primary action wording and visibility logic

### 13.4 `CommandPaletteOverlay.vue`

High-value changes:

- keep it as the global go/do surface
- align command wording to visible shell vocabulary
- add perspective restore commands later only if justified
- avoid adding commands that compensate for weak visible structure

Specific current code areas affected:

- command definitions for navigation/layout
- search summary and command grouping remain as-is in principle

### 13.5 `WorkflowQuickSwitch.vue`

High-value change:

- none for product UX beyond preserving its dev-only status

Possible small cleanup:

- keep its styling and behavior as internal tooling, not user-facing product direction

### 13.6 Future adjacent files likely touched after Prompt 1 implementation begins

These are not part of the core Prompt 1 shell files but are likely to be touched by follow-on shell alignment work:

- `TaskCenterOverlay.vue`
- `RunFooterPanel.vue`
- `useTaskLifecycleSummary.ts`
- `statusPresentation.ts`
- `workflowStore.ts`
- `workflowBootstrap.ts`
- `settingsStore.ts` if titlebar/controls-collapsed/task-launcher behavior needs small coordination

---

## 14. Implementation order by component/file

### Phase 1 — clarify the visible shell hierarchy in `VueWorkspaceShell.vue`

Goal: make the shell’s main visible navigation and context layers clearer without changing the workflow model.

Steps:

1. add a visible **task-center launcher** beside or within the top nav region
2. reduce contextual noise in the shell header
3. tighten the context strip default items
4. add overflow/disclosure for secondary context metadata
5. keep current tab semantics, selection announcements, and split layout behavior intact

Primary files:

- `VueWorkspaceShell.vue`
- possibly `workflowStore.ts` if a launcher state hook is needed

### Phase 2 — quiet `DesktopTitlebar.vue`

Goal: turn the titlebar back into desktop chrome plus one compact operational affordance.

Steps:

1. collapse the current chip cluster
2. decide whether one compact task/status chip remains visible
3. keep Search and Settings actions
4. refine primary action wording/weighting so it reads as contextual, not universally primary

Primary files:

- `DesktopTitlebar.vue`
- possibly `useTaskLifecycleSummary.ts`

### Phase 3 — align task vocabulary across titlebar, footer, and task center

Goal: remove redundant task language and make task center the obvious detailed destination.

Steps:

1. align summary wording
2. ensure launcher/deep-link behavior is consistent
3. keep footer compact
4. avoid task duplication in context strip unless it serves a strong purpose

Primary files:

- `TaskCenterOverlay.vue`
- `RunFooterPanel.vue`
- `useTaskLifecycleSummary.ts`
- `statusPresentation.ts`

### Phase 4 — preserve palette as secondary and perspectives as deferred shell chrome

Goal: prevent shell accretion while still supporting fast expert retrieval.

Steps:

1. keep perspective management in Settings
2. optionally add palette commands for perspective restore/switch
3. do **not** add a permanent perspective band
4. keep `WorkflowQuickSwitch` dev-only

Primary files:

- `CommandPaletteOverlay.vue`
- `SettingsWorkspacePanel.vue`
- `perspectiveSearch.ts` only if needed

### Phase 5 — address Settings continuity after the shell is quieter

Goal: revisit the Settings + Execution relationship only after top-shell duplication is reduced.

Steps:

1. evaluate whether Settings should remain a combined operator workspace
2. clarify how Settings and Execution should relate in the workspace model
3. avoid solving this by adding more top-shell controls

Primary files:

- `App.vue`
- `SettingsWorkspacePanel.vue`
- `ExecutionWorkspacePanel.vue`
- `workflowModel.ts`

---

## 15. Verification plan

The shell direction should be validated against the repo’s real verification surfaces, not only by visual preference.

### 15.1 Minimum required repo checks for any non-trivial shell slice

Run:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run parity`
- `npm --prefix web run a11y:smoke`
- `npm --prefix web run baseline:ui`
- `npm --prefix web run check:bundle-budget`

When the slice affects real desktop-shell behavior such as titlebar/window-controls integration, also run:

- `npm --prefix web run typecheck:desktop`
- `npm --prefix web run test:desktop:smoke`

### 15.2 Shell-specific evidence that must be collected

#### Keyboard path proof

Verify:

- workspace tab row traversal
- workflow-step tab row traversal
- controls tab row traversal
- results tab row traversal
- command palette open/close and focus return
- task-center launcher open/close and focus return
- no trap or dead-end introduced by new launcher elements

#### Capability-preservation proof

Verify that these flows remain obvious:

- go to Data and load/change data context
- go to Optimize and start/cancel optimization
- go to Validate and run validation/export report
- navigate to analysis results
- open Run Library
- open Aggregate Analysis
- open Settings
- reach task center

#### Responsive shell proof

Verify at minimum:

- wide desktop
- common laptop width
- narrower supported desktop width
- split-to-stacked transition still works cleanly
- top shell does not become taller or noisier at smaller widths

#### Desktop-shell proof

If titlebar code changes, verify:

- window drag behavior
- maximize/restore behavior
- focus styling
- titlebar button usability
- overlay stacking with command palette/task center

#### Visual baseline proof

Capture at minimum:

- Workflow / Data
- Workflow / Optimize
- Workflow / Analyze
- Run Library
- Aggregate Analysis
- Settings

### 15.3 UX pass/fail criteria

This shell direction passes only if it produces all of the following:

- less duplicated top-shell context than today
- clearer visible task-center entry
- preserved keyboard-safe path
- preserved canonical capability reachability
- no noticeable increase in shell height or visual noise
- no regression to lazy-loading/bundle posture

---

## 16. Top implementation risks

### 16.1 Competing primary surfaces

If the titlebar, task center, footer, and context strip all continue to carry equally prominent status, the shell will still feel redundant even after cleanup.

### 16.2 Overpromoting perspectives too early

Because perspectives already exist in Settings, promoting them into permanent shell chrome before usage evidence is strong risks adding weight without enough product value.

### 16.3 Context-strip churn

If the context strip becomes too dynamic, too clickable, or too dense, it could create layout instability and make the shell harder to scan.

### 16.4 Accidental capability hiding

If titlebar cleanup or shell decluttering removes visible action paths without preserving the canonical panel path clearly, the functional coverage guardrail is violated.

### 16.5 Keyboard regression

Adding a task-center launcher or new overflow/disclosure elements could easily add focus-management regressions if implemented casually.

### 16.6 Settings continuity being papered over instead of solved

If the repo tries to hide the Settings/Execution awkwardness with more shell chrome instead of addressing the workspace relationship later, the shell will get heavier without becoming clearer.

---

## 17. Conservative fallback path

If the full recommended direction proves too risky during implementation, the exact fallback path should be:

1. keep the shell structure almost exactly as-is
2. add a visible task-center launcher only
3. reduce titlebar chips slightly
4. trim the context strip to a smaller default set
5. leave perspectives fully in Settings/palette

This fallback still improves the shell materially while minimizing layout change.

---

## 18. Conditions that would require falling back to the more conservative path

Fall back immediately if any of the following occur:

- the shell becomes taller or steals meaningful vertical space from the work plane
- new launcher/disclosure elements make keyboard traversal noticeably worse
- users can no longer tell the canonical home for optimization, validation, or settings/runtime utilities
- perspective-related shell additions feel underused or confusing
- status language remains duplicated across too many top-level surfaces even after the refactor
- the shell begins to read more like a general-purpose dashboard than a focused workstation

---

## 19. Signals that would make this direction a failure

Reject this direction if, after implementation, the result shows any of the following:

- titlebar still behaves like a second dashboard row
- task center is still not the obvious async-work destination
- the context strip still feels like a telemetry wall instead of concise context
- users need the command palette **more often** because visible structure became weaker
- the shell becomes more visually modern at the cost of scanability
- Settings/Execution feels more confusing, not more coherent
- the repo starts adding more shell bands to compensate for unresolved hierarchy problems

---

## 20. Open questions requiring product-owner choice

These should remain narrow and explicit.

### 20.1 Should the titlebar keep one compact task/status chip?

Recommendation: yes, at most one compact launcher chip. But product should choose whether that chip is worth the permanent titlebar space.

### 20.2 Which context-strip fields are always visible by default?

Recommendation:

- keep: Dataset, Range, Strategy, Backend, Status
- move to overflow: Rows, Loaded ID

### 20.3 Is the current Settings + Execution combined workspace intentional long term?

Recommendation: treat this as a later Prompt 6 / Prompt 8 follow-up rather than solving it now through shell accretion.

### 20.4 At what usage threshold should saved perspectives become visibly shell-level?

Recommendation: do not promote them yet. Revisit only if strong operator evidence shows that perspective switching is a frequent first-class daily action.

### 20.5 Where exactly should the task-center launcher live in the nav band?

Recommendation: place it within the persistent shell navigation area, visually aligned with tabs but clearly secondary to workspace/workflow selection.

---

## 21. Final decision statement

For this repo, the shell direction is:

**Keep the tab-first workstation shell. Promote the task center visibly. Quiet the titlebar. Tighten the context strip. Keep the command palette secondary but powerful. Keep perspectives in Settings and palette flows for now. Do not add a sidebar. Do not promote the dev quick-switch.**

That is the cleanest near-term shell move that fits the repo’s current design-system thesis, target comps, workstation behaviors, implemented code, and capability-preservation guardrails.
