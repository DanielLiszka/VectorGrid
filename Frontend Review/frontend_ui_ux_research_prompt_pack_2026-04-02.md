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

## First-Pass Prompt Discipline

For every primary prompt in this pack, prefer answers that do all of the following:

- cite concrete repo files, components, stores, services, and UI surfaces
- compare at least:
  - one conservative direction
  - one recommended direction
  - one more aggressive direction
- state which option should actually win now
- separate:
  - adopt now
  - defer
  - reject
- explain what should remain unchanged because it is already working
- identify dependencies on other prompt sections when relevant
- translate any web-researched reference back into repo-specific implications rather than leaving it as generic inspiration

What to avoid:

- moodboard-style answers with no implementation path
- broad "best practices" lists detached from this repo
- redesign advice that silently removes backend-backed capability reachability
- answers that treat every option as equally good

## Second-Pass Prompt Discipline

For every second-pass prompt in this pack:

- do not merely restate the first answer
- identify contradictions, weak assumptions, and over-designed ideas
- explicitly trim advice that is low-confidence or low-ROI
- collapse the answer toward one winning direction
- use targeted current web research only when it materially sharpens a repo decision
- say what new evidence changed the recommendation, if any

## Final Consolidation Prompt Discipline

For every final consolidation prompt in this pack:

- write as if the result will be checked into `Frontend Review/`
- present one coherent repo direction, not an equal-weight summary of alternatives
- include explicit:
  - adopted direction
  - deferred ideas
  - rejected ideas
- include file-level or component-level implementation touch points
- include a staged implementation order
- include a verification plan tied to the repo's real test surfaces
- keep open questions narrow and specific

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

- `web/src/App.vue`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/WorkflowQuickSwitch.vue`
- `web/src/components/CommandPaletteOverlay.vue`

Optional:

- `web/src/components/TaskCenterOverlay.vue`
- `Frontend Review/Unfinished/desktop_ux_decisions_2026-03-28.md`

### Specific Tensions To Resolve

- visible navigation versus command-palette-driven navigation
- dense shell context versus visual overload
- titlebar status/context versus duplicated shell chrome
- stable workflow ownership versus overly dynamic shell behavior
- discoverability for new users versus speed for returning operators

### Optional Research Targets

- workstation-style navigation in professional desktop tools
- coexistence of visible navigation, quick-switch, and command palette
- dense application shell hierarchy patterns that avoid generic SaaS sidebars

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

### Second-Pass Prompt

```text
Review your prior answer to Prompt 1 as a critical second pass.

I want you to pressure-test the shell/workstation recommendation you just made for this repo.

Tasks:
1. Identify weak assumptions, vague claims, and places where your recommendation sounds plausible but is not yet well defended from the attached repo evidence.
2. Identify any ways your proposal could accidentally reduce capability discoverability, overcomplicate navigation, or make the shell feel more like generic SaaS than a workstation.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - dense desktop/workstation navigation patterns
   - command-palette plus visible-navigation coexistence
   - information-dense application shell hierarchy
4. Compare your proposal against at least one more conservative alternative and one more aggressive alternative.
5. Decide which shell direction should actually win for this repo now.

Deliverables:
1. "Keep / revise / reject" verdict on your first-pass recommendation
2. The refined shell direction
3. The specific compromises or tradeoffs now chosen
4. The top implementation risks
5. The exact conditions that would make the repo fall back to the more conservative path

Stay repo-specific. Do not broaden this into a whole-app redesign.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 1, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`Frontend Review/shell_hierarchy_workstation_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Recommended shell direction
3. Explicitly retained current strengths
4. Explicitly rejected shell ideas
5. Surface-by-surface shell behavior map:
   - titlebar
   - workspace tabs
   - workflow steps
   - quick switch
   - command palette
   - task center
   - footer
6. Capability-preservation note
7. Implementation order by component/file
8. Verification plan
9. Open questions requiring product-owner choice

Do not restate all brainstormed options equally. Make a decision and present one coherent direction for this repo.
```

## Prompt 2: Visual System, Depth, And Motion Concretization

### Research Question

How should the repo turn its current visual thesis into a more concrete, stronger, and more modern component-level UI language without sacrificing dense readability?

### Attach These Files

Required:

- `Frontend Review/Unfinished/frontend_theme_proposal_2026-03-28.md`
- `Frontend Review/Unfinished/frontend_depth_ux_implementation_brief_2026-03-28.md`
- `Frontend Review/Prompt1/shell_hierarchy_workstation_direction_FINAL.md`
- `Frontend Review/Prompt1/shell_hierarchy_workstation_implementation_tracking_2026-04-03.md`
- `web/src/styles/app.css`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/SettingsWorkspacePanel.vue`

Optional:

- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/TaskCenterOverlay.vue`

### Specific Tensions To Resolve

- stronger depth and hierarchy versus ornamental noise
- modernity versus dense readability
- motion quality versus reduced-motion and long-session fatigue
- dark/light parity versus over-tuned single-theme polish
- token discipline versus component-local styling drift

### Optional Research Targets

- dense professional UI visual systems
- restrained motion in data-dense desktop products
- hierarchy systems for technical tools rather than marketing-led dashboards

### Prompt

```text
You are reviewing the next-step visual-system work for the VectorGrid desktop frontend in this repo.

Using the attached files, answer this repo-specific question:

How should the repo concretize its current theme, depth, and motion direction into a sharper component-level UI language that feels more modern and more visually intentional, while still handling dense information cleanly?

Constraints:
- Keep the product in workstation territory, not retail-fintech SaaS or flashy motion-heavy UI.
- Use the current design-system and theme docs as the starting point.
- Treat the Prompt 1 shell hierarchy direction as already implemented baseline context. Do not reopen shell information architecture unless you identify a narrow visual inconsistency that must be corrected.
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

### Second-Pass Prompt

```text
Review your prior answer to Prompt 2 as a critical second pass.

I want you to challenge the visual-system recommendation you just made for this repo.

Tasks:
1. Identify where your first answer may still be too abstract, too aesthetic, or not concrete enough for implementation in `app.css` and the current component set.
2. Identify any ways your proposal could hurt dense readability, overuse motion, or drift away from the repo's workstation posture.
3. Treat the Prompt 1 shell hierarchy implementation as fixed baseline unless you identify a narrow visual inconsistency that truly requires adjustment.
4. If targeted current web research would materially improve the answer, use it selectively to study:
   - modern dense-data desktop UI patterns
   - motion/accessibility guidance for data-dense interfaces
   - component-level hierarchy systems for professional tools
5. Refine the recommendation down to the minimum token, pattern, and motion changes that would create a visible step up.
6. Decide which ideas are core, which are optional, and which should be rejected.

Deliverables:
1. "Keep / revise / reject" verdict on your first-pass visual direction
2. Refined visual system direction
3. Must-have token/pattern changes
4. Optional refinements
5. Explicit rejects
6. Failure modes that would show the direction is becoming too decorative or too soft

Stay grounded in the repo's current component and token system.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 2, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`Frontend Review/visual_system_depth_motion_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Final visual thesis
3. Required token updates
4. Required shared component/state patterns
5. Motion rules:
   - adopt
   - defer
   - reject
6. Surface translation guidance for:
   - shell
   - forms
   - tables/lists
   - dialogs/overlays
   - chart-adjacent panels
   - settings/execution surfaces
7. File-level implementation order
8. Validation plan
9. Explicit non-goals

Make one concrete recommendation set. Do not leave this as an open-ended moodboard.
```

## Prompt 3: Dense Form Architecture And Progressive Disclosure

### Research Question

How should the densest authoring and configuration surfaces be reorganized so they remain fast for expert users but become more readable, more interactive, and less visually punishing?

### Attach These Files

Required:

- `Frontend Review/Unfinished/frontend_followup_evaluation_2026-03-28.md`
- `web/src/components/StrategyWorkspacePanel.vue`
- `web/src/components/AdvancedWorkspacePanel.vue`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`

Optional:

- `web/src/composables/useQueuedInputWriter.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/validationStore.ts`

### Specific Tensions To Resolve

- expert speed versus readability for first-pass review
- always-visible controls versus progressive disclosure
- store-driven immediacy versus clearer validation and grouping
- guidance versus clutter
- safer defaults versus keeping advanced control nearby

### Optional Research Targets

- dense expert-form patterns
- progressive disclosure in technical configuration UIs
- professional-tool patterns for grouping advanced parameters without wizardizing workflows

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

### Second-Pass Prompt

```text
Review your prior answer to Prompt 3 as a critical second pass.

I want you to pressure-test the form-architecture recommendation you just made for this repo.

Tasks:
1. Identify where your first answer may slow down expert operators, hide capabilities too deeply, or assume more refactor budget than the repo should spend now.
2. Identify any tensions between readability, speed, and backend-capability reachability.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - dense expert-form patterns
   - progressive disclosure in professional tools
   - safety and scanability patterns in complex configuration UIs
4. Refine the recommendation to a realistic next-step architecture, not an idealized greenfield form system.
5. Decide what should be changed now, what should wait, and what should be rejected.

Deliverables:
1. "Keep / revise / reject" verdict on the first-pass recommendation
2. Refined form-architecture direction
3. Surface-by-surface priority:
   - Strategy
   - Advanced
   - Data
   - Validation
4. Immediate wins versus later refactors
5. Failure modes that would indicate the redesign is hurting power-user speed

Keep the answer tied to the repo's existing panel and store model.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 3, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`Frontend Review/dense_form_progressive_disclosure_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Final recommendation for dense-form architecture
3. Surface-by-surface guidance for:
   - Data
   - Strategy
   - Advanced
   - Validation
4. Always-visible versus progressive-disclosure rules
5. Capability-preservation rules
6. Interaction-speed guardrails for expert users
7. Implementation order by file/component/composable
8. Regression and UX validation plan
9. Explicit defers and rejects

Present one coherent near-term form direction, not a broad future-state rewrite.
```

## Prompt 4: Results Triage And Analysis Workflow Modernization

### Research Question

How should the analysis and review surfaces evolve so candidate triage feels faster, more interactive, and more legible across tables, charts, compare flows, and provenance cues?

### Attach These Files

Required:

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

### Specific Tensions To Resolve

- shortlist speed versus deeper chart analysis
- compare richness versus cognitive load
- provenance/stale cues versus summary clutter
- analysis power versus a clear next-step path
- export capability versus declutter efforts

### Optional Research Targets

- table/chart coordination patterns in expert tools
- compare workflows for technical analysis or research products
- dense results-review patterns that stay keyboard-safe

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

### Second-Pass Prompt

```text
Review your prior answer to Prompt 4 as a critical second pass.

I want you to stress-test the analysis and triage recommendation you just made for this repo.

Tasks:
1. Identify where the first answer may overcomplicate triage, over-prioritize charts, or create too many simultaneous compare/provenance surfaces.
2. Identify tensions between shortlist speed, chart depth, provenance clarity, and cognitive load.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - dense analysis workbenches
   - compare workflows in professional tools
   - chart/table coordination patterns for expert users
4. Refine the recommendation down to the highest-ROI interaction changes.
5. Decide which analysis improvements should happen now versus later.

Deliverables:
1. "Keep / revise / reject" verdict on the first-pass direction
2. Refined triage and analysis workflow recommendation
3. Immediate wins versus later expansions
4. Concrete tradeoffs chosen
5. Failure modes that would show the redesign is becoming too busy or too chart-centric

Keep the answer repo-specific and centered on operator decision-making.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 4, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`Frontend Review/results_triage_analysis_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Final triage/workflow direction
3. Surface-by-surface guidance for:
   - Overview
   - Top
   - Drill
   - Pareto
   - Heatmap
4. Compare/provenance/stale-state guidance
5. Keyboard-safe path definition
6. Export and capability-preservation note
7. Implementation order by file/store/service
8. Verification plan
9. Explicit rejects

Make a decision and present one coherent results-workflow direction for this repo.
```

## Prompt 5: Run Library, History, And Artifact Workflow Modernization

### Research Question

How should the repo modernize the run-library, history, and artifact-management experience so saved runs feel manageable, trustworthy, and fast to work with rather than like a secondary dump of outputs?

### Attach These Files

Required:

- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/stores/historyStore.ts`
- `web/src/services/historyService.ts`
- `web/src/components/ConfirmationDialog.vue`

Optional:

- `web/src/components/ContextActionMenu.vue`
- `web/src/components/RunFooterPanel.vue`
- `web/src/stores/workflowArtifactsStore.ts`

### Specific Tensions To Resolve

- library feel versus activity-log feel
- History versus Meta overlap
- browse speed versus rich detail
- destructive safety versus workflow flow
- artifact action discoverability versus row clutter

### Optional Research Targets

- artifact-library and list/detail patterns in technical tools
- action-menu and secondary-action patterns for dense lists
- safe destructive flows in professional desktop interfaces

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

### Second-Pass Prompt

```text
Review your prior answer to Prompt 5 as a critical second pass.

I want you to pressure-test the artifact/history recommendation you just made for this repo.

Tasks:
1. Identify where the first answer may hide artifact capabilities, oversimplify History/Meta overlap, or make destructive actions less explicit.
2. Identify tensions between browse speed, artifact detail richness, and action discoverability.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - library-style review patterns for technical artifacts
   - list/detail and action-menu patterns for expert tools
   - safe destructive-action flows in dense desktop interfaces
4. Refine the recommendation to the highest-ROI next-step model for this repo.
5. Decide what should be consolidated now, what should stay duplicated as a shortcut, and what should remain separate.

Deliverables:
1. "Keep / revise / reject" verdict on the first-pass direction
2. Refined run-library/history/artifact direction
3. Consolidate-now versus preserve-now decisions
4. Key workflow tradeoffs
5. Failure modes that would indicate the redesign is hurting manageability or trust

Keep the answer tied to the repo's current artifact surfaces.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 5, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`Frontend Review/history_artifact_workflow_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Final artifact/history UX direction
3. Recommended relationship between:
   - Run Library
   - History
   - Meta artifact review
4. Artifact-browser abstraction recommendation, if any
5. Row actions, detail cards, tags, rerun, reveal/open, and delete-flow guidance
6. Capability-preservation note
7. Implementation order by file/store/service
8. Regression and UX validation plan
9. Explicit defers and rejects

Do not produce a broad content-management theory. Produce one workable repo direction.
```

## Prompt 6: Execution, Settings, And Operator-Safety UX

### Research Question

How should the repo improve execution, settings, and diagnostics UX so high-stakes actions are safer, runtime posture is clearer, and the app feels more like a serious operator console?

### Attach These Files

Required:

- `Frontend Review/frontend_audit_2026-03-27.md`
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

### Specific Tensions To Resolve

- routine controls versus high-stakes controls
- faster operator flow versus safer confirmations
- settings as canonical utility home versus in-context control needs
- diagnostics richness versus operator overload
- visible risk posture versus alarm fatigue

### Optional Research Targets

- operator-console patterns
- high-stakes action confirmation UX
- runtime/diagnostics presentation in professional applications

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

### Second-Pass Prompt

```text
Review your prior answer to Prompt 6 as a critical second pass.

I want you to challenge the execution/settings/operator-safety recommendation you just made for this repo.

Tasks:
1. Identify where the first answer may still underspecify high-stakes safeguards, overcomplicate routine actions, or push too much into Settings at the expense of in-context clarity.
2. Identify tensions between safety, speed, diagnostics visibility, and operator confidence.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - operator console patterns
   - high-stakes action confirmation UX
   - diagnostics and runtime-status presentation in professional tools
4. Refine the recommendation to the highest-value near-term safety and clarity improvements.
5. Decide what must change now, what can remain as-is, and what should be explicitly rejected.

Deliverables:
1. "Keep / revise / reject" verdict on the first-pass direction
2. Refined execution/settings/operator-safety direction
3. Must-fix now items
4. Nice-to-have later items
5. Failure modes that would indicate the redesign is making operators slower or less confident

Keep the answer grounded in the repo's actual execution, task, and settings surfaces.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 6, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`Frontend Review/execution_settings_operator_safety_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Final execution/settings UX direction
3. High-stakes safety posture
4. Runtime-status and diagnostics presentation rules
5. Surface-by-surface guidance for:
   - Execution panel
   - Settings panel
   - Runtime banner
   - Task center
6. Immediate versus deferred changes
7. File-level implementation order
8. Verification plan
9. Explicit rejects

Make one decisive operator-UX recommendation set for this repo.
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

### Specific Tensions To Resolve

- premium feel versus fake latency theater
- stronger identity versus startup honesty
- distinct startup experience versus consistency with the shell
- reduced-motion safety versus visual flatness
- better experience versus implementation/bundle cost

### Optional Research Targets

- launch/startup transitions in professional desktop software
- honest readiness signaling patterns
- reduced-motion handling for branded startup sequences

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

### Second-Pass Prompt

```text
Review your prior answer to Prompt 7 as a critical second pass.

I want you to stress-test the startup replacement concept you just proposed for this repo.

Tasks:
1. Identify where the first answer may still be too theatrical, too weak, too similar to the current overlay, or too costly relative to the product value.
2. Identify tensions between brand expression, honesty, startup speed, reduced-motion behavior, and implementation simplicity.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - startup/launch transitions in professional desktop tools
   - reduced-motion handling for branded startup sequences
   - honest readiness signaling patterns
4. Refine the concept to the strongest near-term replacement for this repo.
5. Decide what visual ideas should be retained, revised, or explicitly rejected.

Deliverables:
1. "Keep / revise / reject" verdict on the first-pass concept
2. Refined startup experience direction
3. Required versus optional visual elements
4. Handoff and timing rules
5. Failure modes that would make the concept not worth implementing

Keep the answer specific to this repo's current runtime and startup path.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 7, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`Frontend Review/startup_experience_replacement_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Final startup experience recommendation
3. Visual concept description
4. Timing and handoff rules
5. Reduced-motion and error-state behavior
6. Asset usage rules
7. Implementation order by file/component/store
8. Validation plan
9. Explicitly rejected startup ideas

Do not leave this as multiple equally weighted concepts. Pick one direction for the repo.
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

### Specific Tensions To Resolve

- native value versus feature creep
- Windows-first needs versus cross-platform safety
- background presence versus product simplicity
- titlebar/task center/notification interplay versus duplicate native surfaces
- future productization versus current workflow evidence

### Optional Research Targets

- tray/menu posture in long-running desktop tools
- global shortcut use in professional apps
- minimized/background presence models for workstation software

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

### Second-Pass Prompt

```text
Review your prior answer to Prompt 8 as a critical second pass.

I want you to pressure-test the desktop-presence recommendation you just made for this repo.

Tasks:
1. Identify where the first answer may recommend native features with weak workflow justification or may defer too much and miss a real workstation need.
2. Identify tensions between native presence, simplicity, Windows-first reality, and product clarity.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - tray/menu patterns for long-running desktop tools
   - shortcut posture in professional desktop apps
   - minimized/background presence models
4. Refine the recommendation down to what this repo should actually adopt now versus later.
5. State what evidence would be needed to reverse any defer decision.

Deliverables:
1. "Keep / revise / reject" verdict on the first-pass direction
2. Refined desktop-presence recommendation
3. Adopt now / defer / reject breakdown
4. Key workflow tradeoffs
5. Failure modes that would indicate the native-surface plan is overbuilt or underpowered

Keep the answer product- and workflow-driven rather than feature-driven.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 8, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`Frontend Review/desktop_presence_chrome_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Final desktop-presence posture
3. Adopt now / defer / reject decisions for:
   - tray/menu
   - global shortcuts
   - updater/relaunch posture
   - background/minimized presence model
4. Relationship between titlebar, task center, notifications, and any new native affordances
5. Config/code touch points
6. Implementation order
7. Validation plan
8. Evidence that would justify revisiting deferred items

Make one coherent desktop-presence recommendation set for this repo.
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

### Specific Tensions To Resolve

- product clarity versus shell complexity
- separate mode versus special-case strategy handling
- mode clarity versus persistence/history migration risk
- shared app shell versus mental-model fragmentation
- conventional default posture versus ML-specific workflow needs

### Optional Research Targets

- mode separation in complex technical applications
- workflow restoration across multiple product modes
- shell vocabulary patterns when one branch behaves qualitatively differently

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

### Second-Pass Prompt

```text
Review your prior answer to Prompt 9 as a critical second pass.

I want you to challenge the `MlSignal` mode-separation recommendation you just made for this repo.

Tasks:
1. Identify where the first answer may create too much shell complexity, persistence risk, or conceptual fragmentation.
2. Identify the strongest arguments for leaving `MlSignal` as-is versus separating it now.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - mode separation in complex technical applications
   - workflow and artifact restoration across multiple modes
   - shell vocabulary patterns when one branch is qualitatively different
4. Refine the recommendation to the clearest product choice for this repo now.
5. Decide what conditions would justify delaying the split.

Deliverables:
1. "Keep / revise / reject" verdict on the first-pass direction
2. Refined `MlSignal` UX recommendation
3. Strongest arguments for and against separating now
4. Migration/persistence risk summary
5. Failure modes that would make the split not worth doing yet

Keep the answer grounded in the repo's current shell and artifact behavior.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 9, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`Frontend Review/mlsignal_mode_separation_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Final product decision on `MlSignal`
3. Recommended shell vocabulary
4. Navigation and persistence implications
5. Impact on strategy selection, quick switch, command palette, and history restoration
6. Implementation order by file/component/store
7. Validation plan
8. Explicit defers, risks, and rejection conditions

Choose one direction for the repo. Do not leave this as an unresolved taxonomy debate.
```

## Prompt 10: UI/UX Verification And Evidence Strategy

### Research Question

How should this repo prove that a larger UI/UX modernization pass actually improves the product without regressing dense-workflow speed, keyboard access, or desktop reliability?

### Attach These Files

Required:

- `Frontend Review/Unfinished/tauri_desktop_e2e_plan_2026-03-29.md`
- `web/scripts/check-a11y-smoke.mjs`
- `web/scripts/capture-baseline-ui.mjs`
- `web/e2e/desktop/README.md`

Optional:

- `web/package.json`
- `web/wdio.desktop.conf.ts`
- `web/src/components/VueWorkspaceShell.vue`

### Specific Tensions To Resolve

- evidence quality versus maintenance cost
- browser smoke versus desktop smoke responsibilities
- visual baselines versus brittle screenshot gating
- accessibility confidence versus QA sprawl
- canonical user journeys versus exhaustive coverage

### Optional Research Targets

- lightweight but credible UX regression strategies
- hybrid desktop-web QA layering
- accessibility and interaction validation for data-dense applications

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

### Second-Pass Prompt

```text
Review your prior answer to Prompt 10 as a critical second pass.

I want you to pressure-test the verification strategy you just proposed for this repo.

Tasks:
1. Identify where the first answer may be too heavy, too vague, too expensive to sustain, or too weak to catch real UX regressions.
2. Identify tensions between evidence quality and iteration speed.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - lightweight but credible UX regression strategies
   - desktop-web hybrid QA patterns
   - accessibility plus interaction validation for data-dense apps
4. Refine the verification strategy down to the smallest high-value set of evidence the repo should actually maintain.
5. Decide what is essential, optional, and overkill.

Deliverables:
1. "Keep / revise / reject" verdict on the first-pass verification strategy
2. Refined validation stack
3. Essential versus optional checks
4. Canonical UX scenarios to preserve
5. Failure modes that would show the strategy is too weak or too expensive

Keep the answer grounded in this repo's existing test harnesses and scripts.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 10, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`Frontend Review/ui_ux_validation_strategy_FINAL.md`

The document must contain:
1. Executive summary
2. Final UX validation strategy
3. Test-layer responsibilities:
   - component/unit
   - browser smoke
   - desktop smoke
   - visual baseline capture
4. Canonical user workflows to protect
5. Required scripts and file touch points
6. Essential versus optional evidence
7. Maintenance-cost guidance
8. Explicit overkill to avoid

Make one practical sustained validation strategy for the repo, not a maximalist wish list.
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
