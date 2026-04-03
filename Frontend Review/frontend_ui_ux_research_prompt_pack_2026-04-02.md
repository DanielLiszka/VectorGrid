# Frontend UI/UX Research Prompt Pack

Date: 2026-04-02

Purpose: a repo-grounded prompt pack for handing specific frontend UI/UX research questions to a stronger model such as GPT-5.4 Pro.

This document is intentionally narrower than "all frontend ideas."

It is centered on the next-phase UI/UX questions for the Vue/Tauri desktop shell:

- how the product should feel more modern, interactive, and visually intentional
- how dense information should stay readable and keyboard-safe
- how open items in `Frontend Review/Unfinished/` should be resolved without relitigating already-settled work
- how the currently shipped startup animation should be reconsidered even though its first-wave implementation is technically complete
- how every major frontend surface should be covered, not only shell/forms/results

## How To Use This Pack

- Use one prompt per chat/thread. Do not bundle multiple prompts into one request unless you want a shallow answer.
- Always attach the shared baseline files first.
- Then attach the prompt-specific files for the question you want answered.
- Treat the completed docs in `Frontend Review/` as the current settled baseline unless the prompt explicitly asks to challenge part of that baseline.
- Treat files under `Frontend Review/Unfinished/` as active decision surfaces, not blank slates.
- If the prompt touches a currently implemented surface, ask the model to distinguish:
  - what the repo already decided
  - what the repo already implemented
  - what is still genuinely unresolved
  - what should be intentionally rejected
- Keep answers repo-specific and product-specific:
  - this is a Windows-first Tauri desktop workstation
  - dense information and keyboard-safe interaction matter more than decorative novelty
  - backend-backed capability reachability must be preserved
  - the current Vue/Tauri stack should be used deeply before recommending new libraries

Recommended shared framing for every thread:

- VectorGrid should feel like an operational desktop research workstation, not generic SaaS and not a flashy consumer dashboard.
- The UI should become more modern and more interactive without becoming noisier.
- Dense tables, forms, charts, and state summaries must remain easy to scan.
- The app should keep one obvious keyboard-safe path through primary workflows.
- Suggestions should preserve the frontend functional-coverage guardrail instead of hiding backend-backed capability behind "cleaner" surfaces.

## Shared Baseline Files

Attach these to every prompt:

- `AGENTS.md`
- `REPO_MAP.md`
- `web/README.md`
- `Frontend Review/frontend_design_system_spec_2026-03-28.md`
- `Frontend Review/frontend_target_comps_2026-03-28.md`
- `Frontend Review/frontend_functional_coverage_matrix_2026-03-28.md`
- `Frontend Review/workstation_behavior_plan_2026-03-28.md`
- `Frontend Review/existing_stack_utilization_plan_2026-03-28.md`

Recommended additional shared framing docs when context allows:

- `Frontend Review/frontend_audit_2026-03-27.md`
- `Frontend Review/frontend_followup_evaluation_2026-03-28.md`
- `Frontend Review/desktop_capability_inventory_2026-03-28.md`
- `Frontend Review/desktop_plugin_inventory_2026-03-28.md`
- `Frontend Review/frontend_design_system_seed_2026-03-28.md`

## Shared Answer Shape

For any prompt in this pack, ask the model to keep the answer concrete and implementation-oriented.

Minimum answer shape to request when you want a full-quality response:

1. What the repo already decided
2. What the repo already implemented
3. What remains genuinely unresolved
4. The best 1-2 directions still open
5. What should explicitly remain unchanged
6. Concrete code touch points by file and component/store/composable
7. A staged implementation order
8. A verification plan
9. Rejection criteria and failure modes

If you need more structure, also ask for:

- a surface-by-surface UX change map
- a capability-preservation note tied to `frontend_functional_coverage_matrix_2026-03-28.md`
- a "do now / defer / reject" table

## Frontend Surface Coverage Map

This pack is intended to cover the full major frontend surface area.

- Shell, titlebar, palette, task center, navigation:
  - Prompt 1
  - Prompt 2
  - Prompt 8
- Shared visual system, tokens, motion, hierarchy:
  - Prompt 2
- Data, strategy, advanced, validation authoring:
  - Prompt 3
- Results triage, drilldown, compare, provenance, charts:
  - Prompt 4
- Run Library, History, Meta artifact review, destructive flows:
  - Prompt 5
- Execution, Settings, diagnostics, high-stakes safety UX:
  - Prompt 6
- Startup animation / startup experience:
  - Prompt 7
- Desktop presence, titlebar-adjacent native posture, tray/shortcut decisions:
  - Prompt 8
- `MlSignal` shell-mode separation:
  - Prompt 9
- Verification, accessibility, browser smoke, desktop smoke, evidence:
  - Prompt 10

If you skip prompts:

- do not skip Prompt 5 if you care about artifact/history UX
- do not skip Prompt 6 if you care about live-risk/operator UX
- do not skip Prompt 10 if you expect to actually implement any of the earlier research

## Suggested Prompt Order

Use this order when you want answers that build on one another and stay fresh:

1. Prompt 1: Shell hierarchy and workstation posture
2. Prompt 2: Visual system, depth, and motion concretization
3. Prompt 3: Dense form architecture and progressive disclosure
4. Prompt 4: Results triage and analysis workflow modernization
5. Prompt 5: Run Library, History, and artifact workflow modernization
6. Prompt 6: Execution, Settings, and operator-safety UX
7. Prompt 7: Startup animation replacement
8. Prompt 8: Desktop presence and chrome decisions
9. Prompt 9: `MlSignal` mode separation UX
10. Prompt 10: UI/UX verification and evidence strategy

Reason:

- shell and visual hierarchy decisions affect nearly every later surface
- form and results redesign should inherit the shell and visual-system choices
- history/artifact and execution/settings decisions should inherit the shell and visual-system choices too
- startup animation should match the chosen product tone, not define it alone
- desktop presence and `MlSignal` questions are partly downstream of the broader shell model
- verification strategy is easiest to shape once the intended UX direction is clearer

## Prompt 1: Shell Hierarchy And Workstation Posture Refresh

### Research Question

How should the current desktop shell evolve so the app feels more modern, interactive, and intentional while preserving dense workflow clarity and keyboard-first navigation?

### Attach These Files

Required:

- `Frontend Review/frontend_design_system_spec_2026-03-28.md`
- `Frontend Review/frontend_target_comps_2026-03-28.md`
- `Frontend Review/workstation_behavior_plan_2026-03-28.md`
- `Frontend Review/frontend_functional_coverage_matrix_2026-03-28.md`
- `web/src/App.vue`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/WorkflowQuickSwitch.vue`
- `web/src/components/CommandPaletteOverlay.vue`

Optional:

- `web/src/components/TaskCenterOverlay.vue`
- `Frontend Review/Unfinished/desktop_ux_decisions_2026-03-28.md`

### Prompt

```text
You are reviewing the top-level UI/UX structure of the VectorGrid desktop shell in this repo.

Using only the attached files, answer this repo-specific question:

How should the current shell hierarchy evolve so the product feels more modern, more interactive, and more like a serious desktop workstation, while still preserving dense workflow clarity and keyboard-first navigation?

Constraints:
- Keep the backend command surface and capability exposure intact.
- Preserve the keyboard-safe path through the shell.
- Do not propose a generic SaaS sidebar/dashboard redesign.
- Treat the completed design-system and workstation docs as the baseline unless you have a strong repo-specific reason to challenge them.

Deliverables:
1. Summarize the shell decisions the repo has already made.
2. Identify the strongest remaining shell-level UX weaknesses.
3. Propose the best 1-2 shell hierarchy directions still open.
4. Explain how workspace tabs, workflow steps, task center, command palette, and titlebar should relate to each other.
5. Explain what should become more interactive and what should become quieter.
6. Point to concrete code touch points by file and component.
7. Give a phased implementation order and a verification plan.
8. State the signals that would make you reject your own direction.

Be repo-specific. Do not answer with a generic "desktop apps should have a sidebar" essay.
```

## Prompt 2: Visual System, Depth, And Motion Concretization

### Research Question

How should the repo turn its current visual thesis into a more concrete, stronger, and more modern component-level UI language without sacrificing dense readability?

### Attach These Files

Required:

- `Frontend Review/Unfinished/frontend_theme_proposal_2026-03-28.md`
- `Frontend Review/Unfinished/frontend_depth_ux_implementation_brief_2026-03-28.md`
- `Frontend Review/frontend_design_system_spec_2026-03-28.md`
- `web/src/styles/app.css`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/SettingsWorkspacePanel.vue`

Optional:

- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/TaskCenterOverlay.vue`

### Prompt

```text
You are reviewing the next-step visual-system work for the VectorGrid desktop frontend in this repo.

Using the attached files, answer this repo-specific question:

How should the repo concretize its current theme, depth, and motion direction into a sharper component-level UI language that feels more modern and more visually intentional, while still handling dense information cleanly?

Constraints:
- Keep the product in workstation territory, not retail-fintech SaaS or flashy motion-heavy UI.
- Use the current design-system and theme docs as the starting point.
- Prefer semantic tokens, shared primitives, and repeatable patterns over one-off component styling.
- Preserve readability on dense tables, forms, charts, and status surfaces.

Deliverables:
1. Summarize what the repo already decided visually.
2. Identify the strongest remaining visual weaknesses or vagueness.
3. Propose the best next-step visual direction, including where it should be bolder than the current implementation.
4. Define the most important token, pattern, and motion refinements.
5. Explain how shell, tables, forms, dialogs, and chart-adjacent surfaces should differ visually.
6. Point to code touch points by file and styling surface.
7. Give a narrow implementation and validation plan.
8. State what visual ideas should be explicitly rejected for this product.

Do not answer with generic design-system advice. Keep it grounded in the repo's current visual language and component surfaces.
```

## Prompt 3: Dense Form Architecture And Progressive Disclosure

### Research Question

How should the densest authoring and configuration surfaces be reorganized so they remain fast for expert users but become more readable, more interactive, and less visually punishing?

### Attach These Files

Required:

- `Frontend Review/Unfinished/frontend_followup_evaluation_2026-03-28.md`
- `Frontend Review/frontend_target_comps_2026-03-28.md`
- `Frontend Review/frontend_functional_coverage_matrix_2026-03-28.md`
- `web/src/components/StrategyWorkspacePanel.vue`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`

Optional:

- `web/src/composables/useQueuedInputWriter.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/validationStore.ts`

### Prompt

```text
You are reviewing the dense authoring and configuration surfaces in the VectorGrid frontend.

Using the attached files, answer this repo-specific question:

How should the repo reorganize its densest forms and parameter surfaces so they become more modern, more interactive, and easier to scan, while still remaining efficient for expert users and preserving frontend reachability of backend-backed capabilities?

Constraints:
- Do not remove backend-backed capabilities as a cleanliness shortcut.
- Do not assume a broad form-library rewrite is the right first move.
- Respect the repo's current conclusion that schema-driven migration is "later, not now" unless you can justify a narrower slice.
- Favor progressive disclosure, grouping, summary chips, and safer interaction hierarchy over wizardizing everything.

Deliverables:
1. Summarize what the repo already concluded about form architecture.
2. Identify the worst current form-density problems by surface.
3. Propose the best organization model for Strategy, Advanced, Data, and Validation.
4. Explain what should stay always visible versus move behind disclosure, menus, or contextual summaries.
5. Explain how to make the forms feel more interactive without slowing down expert operators.
6. Point to code touch points by file and component.
7. Give a phased implementation and regression plan.
8. State the changes you would reject because they would hurt speed, discoverability, or capability coverage.

Keep the answer grounded in these repo surfaces rather than giving generic form UX advice.
```

## Prompt 4: Results Triage And Analysis Workflow Modernization

### Research Question

How should the analysis and review surfaces evolve so candidate triage feels faster, more interactive, and more legible across tables, charts, compare flows, and provenance cues?

### Attach These Files

Required:

- `Frontend Review/workstation_behavior_plan_2026-03-28.md`
- `Frontend Review/frontend_functional_coverage_matrix_2026-03-28.md`
- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- `web/src/stores/analysisStore.ts`

Optional:

- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/services/analysisService.ts`

### Prompt

```text
You are reviewing the results-triage and analysis workflow in the VectorGrid frontend.

Using the attached files, answer this repo-specific question:

How should the analysis surfaces evolve so result review feels more modern, interactive, and decision-oriented across shortlist tables, charts, compare flows, and provenance cues, while still preserving dense information access?

Constraints:
- Do not collapse the workflow into chart-only exploration.
- Keep a strong keyboard-safe path between shortlist review and deeper inspection.
- Treat compare, provenance, and stale-state cues as real workstation value, not decorative extras.
- Preserve export and backend-backed analysis capabilities.

Deliverables:
1. Summarize the analysis/workstation behavior the repo already implemented or decided.
2. Identify the strongest remaining UX weaknesses in shortlist triage and deeper review.
3. Propose the best next-step interaction model across Overview, Top, Drill, Pareto, and Heatmap.
4. Explain how compare, provenance, and stale cues should appear and where they should live.
5. Explain what should become more immediate versus more contextual.
6. Point to code touch points by file and component/store.
7. Give a phased implementation and verification plan.
8. State the anti-patterns you would reject for this workflow.

Do not answer with generic analytics-dashboard advice. Keep it repo-specific and operator-oriented.
```

## Prompt 5: Run Library, History, And Artifact Workflow Modernization

### Research Question

How should the repo modernize the run-library, history, and artifact-management experience so saved runs feel manageable, trustworthy, and fast to work with rather than like a secondary dump of outputs?

### Attach These Files

Required:

- `Frontend Review/frontend_target_comps_2026-03-28.md`
- `Frontend Review/frontend_functional_coverage_matrix_2026-03-28.md`
- `Frontend Review/workstation_behavior_plan_2026-03-28.md`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/stores/historyStore.ts`
- `web/src/services/historyService.ts`
- `web/src/components/ConfirmationDialog.vue`

Optional:

- `web/src/components/ContextActionMenu.vue`
- `web/src/components/RunFooterPanel.vue`
- `web/src/stores/workflowArtifactsStore.ts`

### Prompt

```text
You are reviewing the saved-run, history, and artifact-management UX in the VectorGrid frontend.

Using the attached files, answer this repo-specific question:

How should the repo modernize the run-library, history, and artifact-review experience so it feels more intentional, more manageable, and more trustworthy for dense operator workflows, while preserving artifact CRUD reachability and destructive-action safety?

Constraints:
- Keep at least one clear frontend path for every backend-backed artifact capability.
- Treat History and Meta overlap as a consolidation question, not as permission to hide functionality.
- Preserve keyboard-safe list and selection behavior.
- Keep destructive actions explicit and safe.

Deliverables:
1. Summarize what the repo already decided or implemented around history/artifact UX.
2. Identify the strongest current UX weaknesses across History, Meta, saved-run review, and destructive actions.
3. Propose the best next-step model for browsing, selecting, inspecting, rerunning, tagging, and deleting artifacts.
4. Explain whether the repo should introduce a clearer shared artifact-browser abstraction and how visible it should be.
5. Explain how detail cards, row actions, destructive confirmations, provenance, and folder/report reveal actions should be organized.
6. Point to concrete code touch points by file and component/store/service.
7. Give a phased implementation and regression plan.
8. State the anti-patterns you would reject for this workflow.

Keep the answer grounded in this repo's current History and Meta surfaces rather than generic file-manager advice.
```

## Prompt 6: Execution, Settings, And Operator-Safety UX

### Research Question

How should the repo improve execution, settings, and diagnostics UX so high-stakes actions are safer, runtime posture is clearer, and the app feels more like a serious operator console?

### Attach These Files

Required:

- `Frontend Review/frontend_functional_coverage_matrix_2026-03-28.md`
- `Frontend Review/frontend_audit_2026-03-27.md`
- `Frontend Review/frontend_target_comps_2026-03-28.md`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/components/AppRuntimeBanner.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/stores/executionStore.ts`
- `web/src/stores/taskStore.ts`
- `web/src/utils/statusPresentation.ts`

Optional:

- `web/src/services/executionService.ts`
- `web/src/services/settingsService.ts`
- `web/src/utils/desktopNotificationPolicy.ts`

### Prompt

```text
You are reviewing the execution, settings, runtime-status, and diagnostics UX in the VectorGrid frontend.

Using the attached files, answer this repo-specific question:

How should the repo improve execution, settings, and diagnostics UX so high-stakes actions are safer, runtime posture is clearer, and the product feels more like a serious operator console without burying advanced controls?

Constraints:
- Preserve reachability for backend-backed execution and runtime utilities.
- Treat live-risk UX and destructive-key actions as high-stakes surfaces.
- Keep the answer grounded in the existing task center, runtime banner, and settings diagnostics posture.
- Prefer clearer grouping, hierarchy, and action weighting over adding more chrome.

Deliverables:
1. Summarize what the repo already fixed versus what still feels weak in execution/settings UX.
2. Identify the highest-value remaining UX improvements for execution safety, runtime status, and settings diagnostics.
3. Propose the best interaction model for paper/live actions, key management, stream control, diagnostics review, and settings utilities.
4. Explain how severity, confirmation, posture summaries, and action grouping should work.
5. Explain what should be immediate in the panel versus moved behind disclosure or secondary menus.
6. Point to concrete code touch points by file and component/store/service.
7. Give a phased implementation and verification plan.
8. State the failure modes that would make you reject your own direction.

Do not answer with generic "make dangerous buttons red" advice. Keep it repo-specific and operator-oriented.
```

## Prompt 7: Startup Animation Replacement

### Research Question

Given that the first-wave startup overlay is implemented but not visually successful, what should replace it so startup feels intentional, honest, and brand-consistent instead of weak or awkward?

### Attach These Files

Required:

- `Frontend Review/startup_animation_plan_2026-03-30.md`
- `Frontend Review/Unfinished/frontend_theme_proposal_2026-03-28.md`
- `web/src/main.ts`
- `web/src/App.vue`
- `web/src/components/StartupAnimationOverlay.vue`
- `web/src/components/VectorAlphaLogo.vue`
- `web/src/components/AppRuntimeBanner.vue`
- `web/src/stores/runtimeStore.ts`

Optional:

- `web/src/assets/branding/vectoralpha-animated.svg`
- `web/src/assets/branding/vectoralpha-static.svg`
- `web/src/styles/app.css`

### Prompt

```text
You are reviewing the startup experience in the VectorGrid desktop frontend.

The current first-wave startup animation is implemented in the repo, but the product owner is dissatisfied with it and wants a better replacement.

Using the attached files, answer this repo-specific question:

What should replace the current startup overlay so startup feels more intentional, more premium, and more aligned with the product's workstation identity, while still staying honest about real readiness, bundle discipline, and reduced-motion behavior?

Constraints:
- Do not propose fake latency theater.
- Do not assume a multi-window cinematic splash is justified.
- Keep startup truthful: boot state and failure state must remain real.
- Keep the answer compatible with the repo's current runtime/store structure.
- The goal is not "more animation." The goal is a better startup experience.

Deliverables:
1. Summarize the startup decisions the repo already made.
2. Explain why the current approach is likely visually weak or mismatched.
3. Propose the best 1-2 replacement concepts for this repo.
4. Explain how the animation, overlay, and runtime banner handoff should work.
5. Explain what brand assets or visual cues should and should not be used.
6. Point to concrete code touch points by file and component.
7. Give a narrow implementation and validation plan.
8. State the exact visual or product failure modes that would make you reject your own concept.

Keep the answer repo-specific. Do not answer with generic "make a nice splash screen" advice.
```

## Prompt 8: Desktop Presence And Chrome Decisions

### Research Question

Which unresolved desktop UX decisions are still worth adopting, and which should remain deferred, if the goal is a stronger workstation presence rather than more feature surface for its own sake?

### Attach These Files

Required:

- `Frontend Review/Unfinished/desktop_productization_plan_2026-03-28.md`
- `Frontend Review/Unfinished/desktop_ux_decisions_2026-03-28.md`
- `Frontend Review/desktop_capability_inventory_2026-03-28.md`
- `Frontend Review/desktop_plugin_inventory_2026-03-28.md`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/components/GlobalShortcutBridge.vue`

Optional:

- `src-tauri/tauri.conf.json`
- `web/src/services/desktopProductization.ts`

### Prompt

```text
You are reviewing the unresolved desktop-presence questions in the VectorGrid Tauri app.

Using the attached files, answer this repo-specific question:

Which unresolved desktop UX decisions are still worth adopting now, and which should remain deferred, if the goal is a stronger workstation presence rather than more feature surface for its own sake?

Focus especially on:
- tray/menu
- global shortcuts outside the focused webview
- updater/relaunch UX posture
- the overall presence model when the app is minimized, backgrounded, or long-running

Constraints:
- Keep the answer product- and workflow-driven, not technology-driven.
- The current custom titlebar, task center, notifications, and settings diagnostics already exist; do not ignore them.
- Do not recommend desktop features just because native apps can have them.
- Keep Windows-first reality in mind.

Deliverables:
1. Summarize what the repo already adopted versus deferred.
2. Identify the unresolved desktop-presence questions that are genuinely still open.
3. Recommend what should be adopted now, what should stay deferred, and why.
4. Explain the desired interaction model across titlebar, task center, notifications, and any new desktop affordances.
5. Point to code and config touch points by file and component.
6. Give a staged implementation and validation plan.
7. State the user/workflow evidence that would justify changing your recommendation later.

Do not answer with generic "desktop apps should have tray icons and shortcuts" advice.
```

## Prompt 9: `MlSignal` Mode Separation UX

### Research Question

Should `MlSignal` become a distinct shell mode from a UI/UX standpoint, and if so, what is the cleanest way to make that separation legible without fragmenting the product?

### Attach These Files

Required:

- `Frontend Review/Unfinished/mlsignal_mode_separation_plan_2026-03-31.md`
- `web/src/utils/core/workflowModel.ts`
- `web/src/stores/workflowStore.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/WorkflowQuickSwitch.vue`
- `web/src/App.vue`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/components/MlSignalWorkspacePanel.vue`

Optional:

- `web/src/stores/optimizeStore.ts`
- `web/src/components/RunFooterPanel.vue`
- `web/src/services/historyService.ts`

### Prompt

```text
You are reviewing the frontend user-model question around `MlSignal` in this repo.

Using the attached files, answer this repo-specific question:

Should `MlSignal` become a distinct shell mode from a UI/UX standpoint, and if so, what is the cleanest way to make that separation obvious to users without fragmenting the product or confusing history/persistence behavior?

Constraints:
- Keep one app shell.
- Preserve artifact/history restoration behavior.
- Treat the current conventional workflow as the default unless you have a strong repo-specific reason to argue otherwise.
- Focus on user model, shell vocabulary, and workflow clarity more than backend implementation mechanics.

Deliverables:
1. Summarize the current `MlSignal` posture and why it creates UX drift.
2. State whether a separate mode is the right product move now.
3. Propose the cleanest shell vocabulary and navigation model if the split happens.
4. Explain the impact on strategy selection, quick-switch, command palette, persistence, and history reopening.
5. Point to code touch points by file and component/store.
6. Give a phased implementation and migration plan.
7. State the failure modes that would make you reject the split or delay it.

Keep the answer grounded in the repo's current shell model rather than generic product-taxonomy advice.
```

## Prompt 10: UI/UX Verification And Evidence Strategy

### Research Question

How should this repo prove that a larger UI/UX modernization pass actually improves the product without regressing dense-workflow speed, keyboard access, or desktop reliability?

### Attach These Files

Required:

- `Frontend Review/Unfinished/tauri_desktop_e2e_plan_2026-03-29.md`
- `Frontend Review/frontend_functional_coverage_matrix_2026-03-28.md`
- `Frontend Review/existing_stack_utilization_plan_2026-03-28.md`
- `web/README.md`
- `web/scripts/check-a11y-smoke.mjs`
- `web/scripts/capture-baseline-ui.mjs`
- `web/e2e/desktop/README.md`

Optional:

- `web/package.json`
- `web/wdio.desktop.conf.ts`
- `web/src/components/VueWorkspaceShell.vue`

### Prompt

```text
You are reviewing how this repo should validate a larger frontend UI/UX modernization pass.

Using the attached files, answer this repo-specific question:

How should the repo prove that a larger UI/UX modernization effort actually improves the product without regressing dense-workflow speed, keyboard access, accessibility, or desktop reliability?

Constraints:
- Do not answer with generic "add more tests" advice.
- Keep browser QA and real desktop-shell QA distinct where that matters.
- The repo already has meaningful browser checks and a Windows-first desktop smoke harness; build on that.
- The verification plan should be practical for ongoing iteration, not only for one big redesign branch.

Deliverables:
1. Summarize the verification layers the repo already has.
2. Identify the biggest current evidence gaps for UI/UX-heavy work.
3. Propose the best validation stack for a modernized shell/forms/results redesign.
4. Explain what should be covered by component tests, browser smoke, desktop smoke, and visual baseline capture.
5. Define a small set of task-level UX scenarios the repo should treat as canonical evidence.
6. Point to code, test, and script touch points by file.
7. State what validation work is overkill versus high-value.

Keep the answer tied to this repo's actual test surfaces and desktop constraints.
```

## Notes On Reuse And Freshness

- If a prompt leads to a real design decision or implementation change, rerun downstream prompts against the updated repo snapshot rather than trusting earlier answers.
- The most reusable shared answers are likely:
  - Prompt 1 shell hierarchy
  - Prompt 2 visual system
- The most likely prompts to go stale after design changes are:
  - Prompt 5 run-library/history/artifact workflow
  - Prompt 6 execution/settings/operator safety
  - Prompt 7 startup animation replacement
  - Prompt 8 desktop presence decisions
  - Prompt 9 `MlSignal` mode separation
  - Prompt 10 verification strategy

## Bottom Line

The repo does not need another generic frontend audit.

What it needs now is a focused UI/UX research sequence that:

- respects the completed March frontend work
- resolves the remaining unfinished decision surfaces
- covers shell, forms, results, artifacts, execution, settings, desktop posture, startup, and verification rather than only the most visible panels
- sharpens the product's workstation identity
- improves dense workflow readability and interaction quality
- and reopens the startup experience as a deliberate replacement problem rather than treating the current overlay as the final answer
