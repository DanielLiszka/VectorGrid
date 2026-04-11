# First-Pass Prompt

Use this prompt in a fresh GPT-5.4 Pro thread after attaching the context files from this packet. Do not attach the prompt files themselves unless you want the model to inspect the wording.

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
You are reviewing one live-mode implementation plan document from my Rust/Tauri project.

Your job is to critique the plan for:
1. missing scope boundaries
2. hidden assumptions about the current repo behavior
3. correctness, safety, or truthfulness gaps
4. migration, rollout, and rollback gaps
5. verification and test gaps
6. stale repo references or missing owner files
7. additive, low-regression improvements to the plan

Important constraints:
- Base the review only on the files I provide.
- Treat the current live scope as narrow: current broker path, current feed path, single-stream, single-runner, non-options only.
- Do not propose broad new product families, portfolio dashboards, multi-broker expansion, options live trading, compliance platforms, or cloud sync unless directly required by the provided files.
- Prefer smaller, additive changes over redesigns.
- Do not give generic praise.
- Inspect the included screenshot files as current UI evidence. Use them to evaluate hierarchy, clarity, placement, and visible trust signals in addition to the code and plan.
- Distinguish clearly between:
  - confirmed doc gap
  - likely issue
  - open question
  - optional improvement
- Cite the file and section or module for every material claim.
- If a plan references a stale or missing file, call that out explicitly.

For each finding, use:
- Severity: critical / high / medium / low
- Type: scope / safety / correctness / operator-truth / migration / verification / testing / maintainability
- Location: file + section or file + module
- Why it matters
- Recommended change

Then provide:
1. Surface understanding
2. Findings
3. Missing details or assumptions
4. Verification and rollout gaps
5. Suggested doc changes
6. Recommended implementation order adjustments

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

Bundle-specific focus:
- determine whether the plan defines a credible discrete live-mode surface without broad app-shell redesign
- verify which current shell owners actually control placement and mode entry
- use the included screenshots to judge how the current execution surface reads inside the settings workspace
```
