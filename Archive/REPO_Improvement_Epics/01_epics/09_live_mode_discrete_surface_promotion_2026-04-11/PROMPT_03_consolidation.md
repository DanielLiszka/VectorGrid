# Consolidation Prompt

Use this prompt after you have both a first-pass output and a second-pass output for this same packet. Attach the same context files again, and paste the two prior outputs below the prompt where indicated.

## Attached Context Files

- `05_live_execution_productization_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`)
- `09_live_mode_discrete_surface_promotion_2026-04-11.md` (original repo path: `REPO_Improvement_Epics/01_epics/09_live_mode_discrete_surface_promotion_2026-04-11.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `App.vue` (original repo path: `web/src/App.vue`)
- `commands_execution.rs` (original repo path: `src-tauri/src/commands_execution.rs`)
- `executionService.ts` (original repo path: `web/src/services/executionService.ts`)
- `executionStore.ts` (original repo path: `web/src/stores/executionStore.ts`)
- `ExecutionWorkspacePanel.vue` (original repo path: `web/src/components/ExecutionWorkspacePanel.vue`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `SCREENSHOT_01_settings_workspace_execution_shell.jpg` (original repo path: `current UI screenshot captured via Playwright browser harness`)
- `SCREENSHOT_02_settings_workspace_fullpage.jpg` (original repo path: `current UI screenshot captured via Playwright browser harness`)
- `VueWorkspaceShell.vue` (original repo path: `web/src/components/VueWorkspaceShell.vue`)
- `workflowStore.ts` (original repo path: `web/src/stores/workflowStore.ts`)

## Prompt

```text
You are consolidating two prior critiques of one live-mode implementation plan document from my Rust/Tauri project.

Your inputs are:
1. the original bundle files
2. a first-pass critique
3. a second-pass adversarial critique

Your job is to produce one merged, repo-grounded conclusion.

Important constraints:
- Base the review only on the files I provide.
- Treat the current live scope as narrow: current broker path, current feed path, single-stream, single-runner, non-options only.
- Do not propose broad new product families, portfolio dashboards, multi-broker expansion, options live trading, compliance platforms, or cloud sync unless directly required by the provided files.
- Prefer smaller, additive changes over redesigns.
- Do not give generic praise.
- Inspect the included screenshot files as current UI evidence. Use them to evaluate hierarchy, clarity, placement, and visible trust signals in addition to the code and plan.
- Base the consolidation on the provided files first, and use the two prior critiques only as candidate claims to validate.
- Reject unsupported suggestions even if they sound attractive.
- Separate must-fix items from optional improvements.
- Cite the file and section or module for every material claim.
- Flag stale or nonexistent file references explicitly.

Return:
1. Surface understanding
2. Consolidated must-fix findings
3. Consolidated should-fix findings
4. Optional improvements worth considering
5. Suggestions to reject as out of scope or unsupported
6. Recommended doc edits
7. Recommended implementation order

Bundle under review:
Live Mode Discrete Surface Promotion

Files to review:
- `05_live_execution_productization_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`)
- `09_live_mode_discrete_surface_promotion_2026-04-11.md` (original repo path: `REPO_Improvement_Epics/01_epics/09_live_mode_discrete_surface_promotion_2026-04-11.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `App.vue` (original repo path: `web/src/App.vue`)
- `commands_execution.rs` (original repo path: `src-tauri/src/commands_execution.rs`)
- `executionService.ts` (original repo path: `web/src/services/executionService.ts`)
- `executionStore.ts` (original repo path: `web/src/stores/executionStore.ts`)
- `ExecutionWorkspacePanel.vue` (original repo path: `web/src/components/ExecutionWorkspacePanel.vue`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `SCREENSHOT_01_settings_workspace_execution_shell.jpg` (original repo path: `current UI screenshot captured via Playwright browser harness`)
- `SCREENSHOT_02_settings_workspace_fullpage.jpg` (original repo path: `current UI screenshot captured via Playwright browser harness`)
- `VueWorkspaceShell.vue` (original repo path: `web/src/components/VueWorkspaceShell.vue`)
- `workflowStore.ts` (original repo path: `web/src/stores/workflowStore.ts`)

Bundle-specific focus to preserve:
- determine whether the plan defines a credible discrete live-mode surface without broad app-shell redesign
- verify which current shell owners actually control placement and mode entry
- use the included screenshots to judge how the current execution surface reads inside the settings workspace

Bundle-specific challenge focus to preserve:
- look for stale shell-owner assumptions
- note that this repo does not currently have web/src/components/AppShell.vue or web/src/router; challenge the doc if it assumes those owners
- look for any mismatch between the plan and the current visual evidence around shell placement and prominence

First-pass critique to consolidate:
[paste here]

Second-pass critique to consolidate:
[paste here]
```
