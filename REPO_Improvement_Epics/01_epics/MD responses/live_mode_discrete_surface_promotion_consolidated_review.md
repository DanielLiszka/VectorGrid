# Consolidated critique — Live Mode Discrete Surface Promotion

**Bundle under review:** `09_live_mode_discrete_surface_promotion_2026-04-11.md`

**Method:** This consolidation revalidated candidate claims from the earlier first-pass and second-pass critiques against the provided bundle files, owner files, and screenshots.

## 1. Surface understanding

The plan is aimed at a real product-truth problem. The umbrella live-execution doc says the current live surface is real but intentionally narrow, and that the current repo should productize and harden that narrow path before widening scope. It also explicitly calls out the settings-mounted execution surface as an open posture decision: execution may remain settings-mounted or move to a more deliberate live surface. **Sources:** `05_live_execution_productization_2026-04-10.md` (Purpose, Scope Boundaries, Current User-Facing Behavior, Proposed Improvement Approach).

The current shell is not page-routed. The repo’s own routing map says the frontend is a Vue + Pinia workspace shell, not a router-style SPA, and that visible shell behavior is owned by `App.vue`, `VueWorkspaceShell.vue`, `workflowStore.ts`, and restoration/model owners listed in `REPO_MAP.md`, especially `workflowModel.ts` and `workflowBootstrap.ts`. In code, `App.vue` only mounts `ExecutionWorkspacePanel` when `settingsWorkspaceActive`, and it mounts `SettingsWorkspacePanel` at the same time. `VueWorkspaceShell.vue` then renders both `#settingsVueShellMount` and `#executionVueShellMount` inside `#settingsShellGrid`. `ExecutionWorkspacePanel.vue` also computes its own visibility from `workspace === 'settings'`. That means live execution is currently a special settings-workspace co-mount, not a first-class shell primitive. **Sources:** `REPO_MAP.md` (Frontend Runtime Map; Frontend workflow model; Shell and panel ownership; Settings, bootstrapping, and persistence); `App.vue` (`settingsWorkspaceActive`, template mounts of `ExecutionWorkspacePanel` and `SettingsWorkspacePanel`); `VueWorkspaceShell.vue` (`settingsWorkspaceActive`, `#settingsShellGrid`, `#settingsVueShellMount`, `#executionVueShellMount`); `ExecutionWorkspacePanel.vue` (`visible`, `useShellMount`).

The screenshots confirm that the current issue is not merely low prominence. The outer shell still reads as a settings/preferences surface — including settings workspace selection and settings-oriented subtitle/copy — while the inner execution pane can simultaneously present broker-facing language like **Paper/Live Execution** and a **LIVE BROKER** badge. In other words, the current problem is both **secondary placement** and **outer-shell/inner-surface contradiction**. **Sources:** provided screenshots `SCREENSHOT_01_settings_workspace_execution_shell.jpg` and `SCREENSHOT_02_settings_workspace_fullpage.jpg`; `workflowStore.ts` (`deriveSubtitle` for settings workspace); `VueWorkspaceShell.vue` (workspace labels, subtitle, context chips); `ExecutionWorkspacePanel.vue` (panel header, mode badge, posture items).

The live surface is also still narrower and more context-coupled than the phrase “discrete live mode” can imply. The live backend only accepts `DoubleMa` and `CompositeCpu` execution specs. The runner requires an active same-symbol Polygon live stream. The frontend builds execution requests from the latest workflow run snapshot or from current UI/run-context settings in manual mode. The store’s initial state also says a loaded dataset is required and that chart/account review is not yet complete. **Sources:** `commands_execution.rs` (`LiveStrategySpec`, `execute_strategy_signal_once`, `start_live_execution_runner`, stream preconditions); `executionService.ts` (`buildExecutionInvokePayload`, `resolveManualExecutionStrategy`, `startRunner`, `runOnce`, `chartMeta`, `syncPersistedExecutionKeys`); `executionStore.ts` (initial `statusHint`, initial `chartMeta`, blocked reasons, key-state behavior); `REPO_MAP.md` (Data, Polygon, And Live Execution > Live execution).

**Bottom line:** the plan is directionally correct, but it is not yet implementation-ready. It still needs to define what “discrete live mode” means inside the current shell model, which files own that move, how persistence/restore behaves, and how the promoted surface stays truthful about its narrow operating envelope and current dependencies.

## 2. Consolidated must-fix findings

### Must-fix 1 — Resolve the current `mode` collision and choose the actual shell primitive
- **Classification:** confirmed doc gap
- **Severity:** critical
- **Type:** correctness / migration / operator-truth
- **Location:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Purpose, Scope Boundaries, Proposed Improvement Approach, Checklist); `workflowStore.ts` (`WorkflowState`, `DEFAULT_MODE`, `selectMode`, `selectWorkspace`, `selectWorkflowStep`); `VueWorkspaceShell.vue` (`modeItems`); `REPO_MAP.md` (Frontend workflow model)
- **Why it matters:** The plan talks about “live mode as a distinct operating mode and navigation surface,” but this repo already uses **mode** as a concrete shell primitive (`conventional` vs `ml-signal`). `VueWorkspaceShell.vue` renders those mode tabs explicitly, and `workflowStore.ts` persists `mode`, `workspace`, `workflow`, `workflowStep`, `controlsTab`, `resultsTab`, `aggregateTab`, and `splitFraction`. `selectMode()` and `selectWorkspace()` also coerce specific combinations. Without deciding whether “discrete live mode” means a true third shell mode, a new workspace, a new workflow-step entry, or only a promoted settings-adjacent surface, the core implementation target stays ambiguous.
- **Recommended change:** Add a short decision record that chooses **one** first-slice promotion shape and rejects the others. The doc should explicitly say whether this epic:
  1. adds a new shell `mode`,
  2. adds a new `workspace`,
  3. adds a new workflow-step entry inside the current shell, or
  4. keeps live as a promoted settings-adjacent sub-surface.
  If the answer is **not** “new shell mode,” stop using “mode” as the primary noun for this epic.
- **Sources:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Purpose, Scope Boundaries, Proposed Improvement Approach, Checklist); `workflowStore.ts` (`WorkflowState`, `DEFAULT_MODE`, `selectMode`, `selectWorkspace`); `VueWorkspaceShell.vue` (`modeItems`); `REPO_MAP.md` (Frontend Runtime Map > Frontend workflow model).

### Must-fix 2 — Acknowledge the current special-cased settings topology and expand the owner map accordingly
- **Classification:** confirmed doc gap
- **Severity:** high
- **Type:** migration / maintainability
- **Location:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Current Code Surface, Checklist); `App.vue`; `VueWorkspaceShell.vue`; `ExecutionWorkspacePanel.vue`; `workflowStore.ts`; `REPO_MAP.md`
- **Why it matters:** The plan names some relevant files, but it does not fully describe the actual topology it is trying to change. Today, live execution is not routed as a normal workflow result or standalone workspace panel; it is a **settings-workspace special case**. `App.vue` mounts both settings and execution when `settingsWorkspaceActive`. `VueWorkspaceShell.vue` reserves two separate settings-grid mount points. `ExecutionWorkspacePanel.vue` teleports into `#executionVueShellMount`. `workflowStore.ts` also special-cases the settings workspace in ways that keep it aligned with settings/data/overview semantics rather than a real live workflow. Missing that topology makes it too easy to move only the panel markup while leaving shell-state, mount behavior, and restore behavior inconsistent.
- **Recommended change:** Add a “current shell owner map” section that names:
  - `App.vue` as the high-level mount owner,
  - `VueWorkspaceShell.vue` as the shell-slot/layout owner,
  - `workflowStore.ts` as the shell-state and persistence owner,
  - `workflowModel.ts` as the allowed-vocabulary/allowed-combination owner,
  - `workflowBootstrap.ts` as the restore owner,
  - `SettingsWorkspacePanel.vue` and `ExecutionWorkspacePanel.vue` as the current co-mounted settings/live surfaces.
  The doc should also say whether the first slice preserves or removes the current settings-workspace special case.
- **Sources:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Current Code Surface, Checklist); `App.vue` (`settingsWorkspaceActive`, template mounts); `VueWorkspaceShell.vue` (`#settingsShellGrid`, `#settingsVueShellMount`, `#executionVueShellMount`); `ExecutionWorkspacePanel.vue` (`visible`, `useShellMount`); `workflowStore.ts` (`selectWorkspace`, `selectMode`, `deriveSubtitle`); `REPO_MAP.md` (Frontend Runtime Map; Shell and panel ownership; Settings, bootstrapping, and persistence).

### Must-fix 3 — Add a shell-truth contract for the outer chrome, not just the inner panel
- **Classification:** confirmed doc gap
- **Severity:** high
- **Type:** operator-truth
- **Location:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Current User-Facing Behavior, Proposed Improvement Approach, Checklist); `workflowStore.ts` (`deriveSubtitle`); `VueWorkspaceShell.vue` (workspace labels, subtitle, context chips); provided screenshots
- **Why it matters:** The screenshots show the core trust problem very clearly: the app header/subtitle/context still reads like a settings surface while the execution panel presents broker-facing controls and even a live-broker state. `workflowStore.ts` literally derives the settings subtitle as “Appearance + storage,” and `VueWorkspaceShell.vue` surfaces that subtitle and the settings workspace label in the outer chrome. If the plan only reorganizes inner live-panel sections, it can still fail the actual user-facing problem.
- **Recommended change:** Add a “shell truth contract” section that defines which outer-shell elements must change in the first slice. At minimum, the doc should state whether the first slice changes:
  - workspace label,
  - shell subtitle,
  - selection-summary/context chips,
  - settings-facing copy still visible around the promoted live surface,
  - acceptance evidence requirements for full-shell screenshots.
  The acceptance bar should be full-shell truth, not inner-panel polish only.
- **Sources:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Current User-Facing Behavior, Proposed Improvement Approach, Checklist); `workflowStore.ts` (`deriveSubtitle`); `VueWorkspaceShell.vue` (`currentWorkspaceLabel`, `shellSubtitle`, context items, shell header); provided screenshots `SCREENSHOT_01_settings_workspace_execution_shell.jpg` and `SCREENSHOT_02_settings_workspace_fullpage.jpg`.

### Must-fix 4 — State the current dependency contract so the promoted surface does not overclaim independence
- **Classification:** confirmed doc gap
- **Severity:** high
- **Type:** correctness / operator-truth
- **Location:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Purpose, Scope Boundaries, Proposed Improvement Approach); `executionStore.ts`; `executionService.ts`; `commands_execution.rs`; `05_live_execution_productization_2026-04-10.md`
- **Why it matters:** The plan’s language about a “clearer discrete live mode with its own operating surface” can read like a more standalone trading surface than the current repo actually implements. The store initializes with “Requires a loaded dataset. Optimization run is optional when using Manual parameter source.” The service builds execution requests from the latest workflow run snapshot or from current run/UI context. The Rust command path requires strategy `data_id` / loaded candles, and the runner requires an active same-symbol Polygon live stream. The umbrella doc also keeps the live scope intentionally narrow. If the promoted shell looks more autonomous than these dependencies allow, it becomes less truthful, not more.
- **Recommended change:** Add an explicit “current dependency contract” section that says the first promoted surface still depends on:
  - a loaded dataset / strategy candle context,
  - current parameter-source rules (`selected`, `best`, `manual`),
  - current run/context shaping when not using purely manual parameters,
  - an active same-symbol Polygon live stream for runner use,
  - the current narrow strategy/broker/feed envelope.
  The doc should make clear that this slice clarifies posture and placement; it does **not** yet create a fully standalone live-trading workspace.
- **Sources:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Purpose, Scope Boundaries, Proposed Improvement Approach); `executionStore.ts` (initial `statusHint`, blocked reasons, live/runner readiness logic); `executionService.ts` (`buildExecutionInvokePayload`, `resolveManualExecutionStrategy`, `startRunner`, `runOnce`); `commands_execution.rs` (`LiveStrategySpec`, `execute_strategy_signal_once`, `start_live_execution_runner`, stream preconditions); `05_live_execution_productization_2026-04-10.md` (Scope Boundaries, Current User-Facing Behavior).

### Must-fix 5 — Add persisted-state migration, restore, fallback, and coexistence rules
- **Classification:** confirmed doc gap
- **Severity:** high
- **Type:** migration / safety
- **Location:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Checklist, Verification Plan, Rollout / Fallback / Compatibility Notes); `workflowStore.ts` (`persistRuntimeState`, `applyLegacySnapshot`, state fields); `REPO_MAP.md` (Settings, bootstrapping, and persistence; workflow restoration)
- **Why it matters:** The current plan says to preserve the existing execution surface as fallback, but it does not say what that means in a persisted-shell app. `workflowStore.ts` writes shell state to storage and reconstructs it from stored values. `REPO_MAP.md` says `workflowBootstrap.ts` restores persisted workspace, workflow-step, controls/results tabs, and aggregate state. If the live entry point changes without a storage/restore plan, users can land in contradictory shell states after restart. If the old settings-mounted surface remains fully interactive at the same time as the promoted surface, the app can temporarily have two competing “authoritative” live surfaces.
- **Recommended change:** Add a migration and rollout subsection that covers:
  - storage compatibility / restore behavior for pre-promotion state,
  - how rollback maps promoted state back to the prior settings-mounted surface,
  - whether fallback is feature-flagged, hidden, read-only, or fully interactive,
  - which surface is authoritative during rollout,
  - what happens when persisted state points to a surface that is no longer the primary entry point.
- **Sources:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Checklist, Verification Plan, Rollout / Fallback / Compatibility Notes); `workflowStore.ts` (`WorkflowState`, `persistRuntimeState`, `applyLegacySnapshot`, root attrs); `REPO_MAP.md` (Frontend Runtime Map > Settings, bootstrapping, and persistence; workflow restoration).

## 3. Consolidated should-fix findings

### Should-fix 1 — Strengthen verification to match the real shell blast radius
- **Classification:** confirmed doc gap
- **Severity:** high
- **Type:** verification / testing
- **Location:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Current Verification Surface, Verification Plan); `App.vue`; `VueWorkspaceShell.vue`; `workflowStore.ts`; `REPO_MAP.md`; `AGENTS.md`
- **Why it matters:** The current verification list is still mostly execution-panel-centric. But this change’s blast radius is the shell: mount ownership, workspace entry, persisted state, restoration, subtitle/context truth, and keyboard/a11y behavior. The repo map also points to `npm --prefix web run a11y:smoke`, the workflow gate, and frontend parity/quality tooling as relevant surfaces. `AGENTS.md` also pushes small, surgical, evidence-backed changes. The plan should make that shell coverage explicit.
- **Recommended change:** Expand the verification plan to require:
  - targeted shell-state coverage for `workflowStore.ts`,
  - mount/visibility checks for `App.vue` and `VueWorkspaceShell.vue`,
  - restore checks via `workflowBootstrap.ts`,
  - `npm --prefix web run a11y:smoke` when tab/selection semantics change,
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` when shell-state behavior changes,
  - full-shell before/after screenshots in addition to execution-panel shots.
- **Sources:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Current Verification Surface, Verification Plan); `App.vue`; `VueWorkspaceShell.vue`; `workflowStore.ts`; `REPO_MAP.md` (Verification Map; Frontend Runtime Map); `AGENTS.md` (Coding conventions; Frontend work policy; Execution quality).

### Should-fix 2 — Replace router-like wording with repo-native shell-state language
- **Classification:** likely issue
- **Severity:** medium
- **Type:** maintainability / correctness
- **Location:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Scope Boundaries, Checklist, Verification Plan)
- **Why it matters:** The doc uses phrases like “route/workspace placement” and “route-flow acceptance evidence.” This repo does **not** use a page router for shell ownership. `REPO_MAP.md` explicitly says the frontend is a workspace shell, not a page-routed SPA. There is no actual stale file reference like `web/src/components/AppShell.vue` or `web/src/router` in the current doc, so the problem is not a fake filename; the problem is slightly misleading language that can pull implementation toward the wrong mental model.
- **Recommended change:** Replace “route” language with `shell-state`, `workspace entry`, `workflow step`, `mount placement`, or `shell placement` language tied to the real owners: `App.vue`, `VueWorkspaceShell.vue`, `workflowStore.ts`, `workflowModel.ts`, and `workflowBootstrap.ts`.
- **Sources:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Scope Boundaries, Checklist, Verification Plan); `REPO_MAP.md` (Frontend Runtime Map; Frontend workflow model).

### Should-fix 3 — Keep the current support envelope explicit in both the doc and the promoted surface
- **Classification:** confirmed doc gap
- **Severity:** medium
- **Type:** scope / operator-truth
- **Location:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Scope Boundaries, Proposed Improvement Approach, Checklist); `05_live_execution_productization_2026-04-10.md`; `commands_execution.rs`; `REPO_MAP.md`
- **Why it matters:** Promotion increases perceived seriousness. If the promoted live surface becomes more prominent without equally prominent support-envelope statements, it will look more mature than it is. The current live backend is still limited to `DoubleMa` and `CompositeCpu`, Alpaca REST, Polygon stocks live feed, one active stream slot, and one active runner slot.
- **Recommended change:** Add an explicit support-envelope statement to the plan and require the promoted surface to keep that envelope visible at the point of use.
- **Sources:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Scope Boundaries, Proposed Improvement Approach, Checklist); `05_live_execution_productization_2026-04-10.md` (Current User-Facing Behavior); `commands_execution.rs` (`LiveStrategySpec`); `REPO_MAP.md` (Data, Polygon, And Live Execution > Live execution).

### Should-fix 4 — Make current hardening dependencies explicit, or constrain this slice to posture/hierarchy only
- **Classification:** likely issue
- **Severity:** high
- **Type:** safety / operator-truth
- **Location:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Regressions To Avoid, Rollout / Fallback / Compatibility Notes, Related Docs); `executionService.ts`; `executionStore.ts`; `05_live_execution_productization_2026-04-10.md`
- **Why it matters:** The plan correctly says to start with shell and posture clarification before broader action expansion, but it does not state clearly enough what the promoted shell must **not** imply yet. The current service still persists remembered API keys in frontend storage when enabled. The current review surface still says “Execution chart sync pending,” and broker-confirmed order truth still depends on explicit refresh. If the shell becomes markedly more first-class before those sibling hardening lanes are documented or shipped, the UI can look more production-ready than the repo’s actual guarantees.
- **Recommended change:** Add a sequencing note: if sibling docs for safety controls, secure credential storage, and chart/account review truth are not yet implemented, this epic’s first slice is limited to shell labeling, posture, entry/exit boundaries, and hierarchy clarification.
- **Sources:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Regressions To Avoid, Rollout / Fallback / Compatibility Notes, Related Docs); `executionService.ts` (`syncPersistedExecutionKeys`, `runOnce`, `chartMeta`); `executionStore.ts` (initial `chartMeta`, initial `statusHint`, key-state handling); `05_live_execution_productization_2026-04-10.md` (Weaknesses / Failure Modes, Related Docs).

### Should-fix 5 — Clarify the relationship to existing runtime-status surfaces
- **Classification:** likely issue
- **Severity:** medium
- **Type:** operator-truth / maintainability
- **Location:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Proposed Improvement Approach); `REPO_MAP.md`; `executionService.ts`
- **Why it matters:** The promoted live shell will not be the only place runtime truth already appears. The repo already has task-center, footer, and settings diagnostics surfaces, and the execution service is already consuming `polygon_live_status` and `live_execution_runner_event`. If the plan does not define how the promoted surface relates to those existing truth surfaces, it risks duplicating or contradicting status.
- **Recommended change:** Add one short section that says whether the promoted surface is the primary operator surface, a curated summary of existing event/status truth, or a richer in-shell view that still defers to shared task/status surfaces for timeline/history semantics.
- **Sources:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Proposed Improvement Approach); `REPO_MAP.md` (Global UI surfaces; Runtime events and task tracking; Settings diagnostics); `executionService.ts` (`subscribeExecutionRuntimeEvents`, status parsing helpers).

## 4. Optional improvements worth considering

1. **Add a one-page decision table comparing the four promotion shapes.**
   That would make it easier to document why the chosen first slice is preferable to the rejected alternatives without turning the epic into a broad redesign. **Sources:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Purpose, Proposed Improvement Approach); `workflowStore.ts`; `REPO_MAP.md` (Frontend workflow model).

2. **Add a mini glossary for `mode`, `workspace`, `workflow`, `workflow step`, and `live surface`.**
   This would reduce language drift across the live docs and prevent future confusion about whether “mode” is literal or descriptive. **Sources:** `workflowStore.ts` (`WorkflowState`); `VueWorkspaceShell.vue` (mode/workspace/workflow-step labels); `REPO_MAP.md` (Frontend workflow model).

3. **Use the existing `ExecutionWorkspacePanel.vue` section structure as the additive foundation for the first slice.**
   The current panel already has posture cards, credential state, account/review actions, manual execution, live automation, preflight notes, and confirmation prompts. A first slice can promote and reframe that structure without inventing a dashboard-style redesign. **Sources:** `ExecutionWorkspacePanel.vue` (header, `executionPostureItems`, safety notice, credential state, account/review, manual execution, live automation, confirmation dialog); `AGENTS.md` (Coding conventions; Frontend work policy).

4. **Add a small acceptance appendix that mirrors the current screenshot pair.**
   One “current settings reading” screenshot and one “promoted live reading” screenshot would make shell-truth improvement measurable rather than subjective. **Sources:** provided screenshots; `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Verification Plan).

5. **Make the first-slice implementation posture explicitly frontend-first.**
   The current ownership evidence suggests the first slice should mainly touch `App.vue`, `VueWorkspaceShell.vue`, `workflowStore.ts`, and possibly `workflowModel.ts` / `workflowBootstrap.ts`, not the Rust command surface. **Sources:** `App.vue`; `VueWorkspaceShell.vue`; `workflowStore.ts`; `REPO_MAP.md` (Frontend Runtime Map; workflow restoration); `AGENTS.md` (Frontend work policy).

## 5. Suggestions to reject as out of scope or unsupported

1. **Reject any suggestion that assumes page-router or `AppShell.vue` ownership.**
   There is no current `web/src/components/AppShell.vue` or `web/src/router` owner in this bundle, and `REPO_MAP.md` explicitly describes a workspace shell instead. The right owners are `App.vue`, `VueWorkspaceShell.vue`, `workflowStore.ts`, plus `workflowModel.ts` and `workflowBootstrap.ts` named in the repo map. **Sources:** `REPO_MAP.md` (Frontend Runtime Map; Frontend workflow model); `App.vue`; `VueWorkspaceShell.vue`; `workflowStore.ts`.

2. **Reject broad app-shell redesign as a prerequisite for this epic.**
   The plan’s own non-goal is to avoid redesigning unrelated workflow surfaces, and the current code shows a narrow, shell-owned settings/live topology that can be clarified additively. **Sources:** `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Out of scope, Proposed Improvement Approach); `App.vue`; `VueWorkspaceShell.vue`; `AGENTS.md` (Coding conventions).

3. **Reject any scope expansion into multi-broker, options live trading, portfolio dashboards, OMS behavior, or cloud sync.**
   None of that is required by the provided files, and it directly violates the narrow-scope guard in both live docs. **Sources:** `05_live_execution_productization_2026-04-10.md` (Scope Boundaries); `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Scope Boundaries); `REPO_MAP.md` (Live execution scope).

4. **Reject any first-slice proposal that pretends live execution is already a standalone trading cockpit detached from research/run context.**
   Current execution still depends on loaded datasets, strategy candle context, parameter-source rules, and same-symbol live-feed state for the runner. **Sources:** `executionService.ts` (`buildExecutionInvokePayload`, `resolveManualExecutionStrategy`, `startRunner`); `executionStore.ts` (initial `statusHint`); `commands_execution.rs` (`execute_strategy_signal_once`, `start_live_execution_runner`).

5. **Reject backend/Tauri redesign as the primary response to this plan gap.**
   The core contradiction identified here is shell placement and outer-shell truth, which are currently owned by the frontend shell files and workflow state. **Sources:** `App.vue`; `VueWorkspaceShell.vue`; `workflowStore.ts`; `ExecutionWorkspacePanel.vue`; `REPO_MAP.md` (Shell and panel ownership; Frontend workflow model).

## 6. Recommended doc edits

1. **Add a new section after Scope Boundaries: `Promotion shape decision`.**
   It should answer, in one place, whether this first slice creates a new shell mode, a new workspace, a workflow-step entry, or a promoted settings-adjacent surface.

2. **Add a new section after Current Code Surface: `Current shell owner map`.**
   Include `App.vue`, `VueWorkspaceShell.vue`, `workflowStore.ts`, `workflowModel.ts`, `workflowBootstrap.ts`, `SettingsWorkspacePanel.vue`, and `ExecutionWorkspacePanel.vue`, with a one-line description of each file’s responsibility.

3. **Add a new section: `Current dependency and support contract`.**
   This should state the current support envelope and dependency chain: supported strategies, broker/feed limits, dataset/run-context dependence, same-symbol live-feed requirement for runner, and single-stream/single-runner constraints.

4. **Add a new section: `Shell truth contract for the first slice`.**
   Define exactly which outer-shell cues must change together with the promoted live surface.

5. **Add a new section: `Persistence, restore, fallback, and rollback`.**
   Cover storage compatibility, restoration defaults, coexistence posture during rollout, and rollback mapping.

6. **Revise the Verification Plan to be shell-aware.**
   Keep the existing execution-panel/store/service checks, but add shell-state, restore-state, a11y, workflow-gate, and full-shell screenshot requirements.

7. **Revise Rollout / Fallback / Compatibility Notes to identify dependency boundaries.**
   Say explicitly whether this doc can ship before the related safety / credential / chart-review docs, or whether it is limited to posture/hierarchy work until those sibling docs land.

8. **Replace route wording with shell wording throughout.**
   Specifically replace phrases like “route/workspace placement” and “route-flow acceptance evidence” with “shell placement,” “workspace entry,” or “shell-state acceptance evidence.”

## 7. Recommended implementation order

1. **Update the doc first to choose the promotion shape and name the real shell owners.**
   Until that exists, implementation remains under-specified.

2. **Document the current support/dependency contract and the shell-truth contract second.**
   This keeps the first implementation slice honest about what it is and is not improving.

3. **Define persistence/restore/fallback rules before any shell move lands.**
   That is the minimum migration discipline for a persisted-shell app.

4. **Implement the first slice as a narrow frontend shell change.**
   The highest-signal files are:
   - `App.vue`
   - `VueWorkspaceShell.vue`
   - `workflowStore.ts`
   - and, if the chosen shape requires it, `workflowModel.ts` / `workflowBootstrap.ts`

   Only after that should the work refine the promoted presentation inside `ExecutionWorkspacePanel.vue`. This matches the repo’s small, surgical, frontend-first guidance. **Sources:** `App.vue`; `VueWorkspaceShell.vue`; `workflowStore.ts`; `REPO_MAP.md` (Frontend Runtime Map; workflow restoration); `AGENTS.md` (Coding conventions; Frontend work policy).

5. **Keep backend/Tauri execution behavior unchanged in the first slice unless a specific shell decision forces a small wiring adjustment.**
   The current evidence does not support treating backend change as the main work for this epic. **Sources:** `commands_execution.rs`; `executionService.ts`; `REPO_MAP.md` (Live execution; Shell and panel ownership).

6. **Only after shell truth is validated should broader live-surface hardening proceed.**
   That next layer belongs with the sibling safety, credentials, reconciliation, recovery, and chart/account review docs, not with the initial shell-promotion slice. **Sources:** `05_live_execution_productization_2026-04-10.md` (Related Docs, Proposed Improvement Approach); `09_live_mode_discrete_surface_promotion_2026-04-11.md` (Related Docs, Rollout / Fallback / Compatibility Notes).

