# Consolidation Prompt

Use this prompt after you have both a first-pass output and a second-pass output for this same packet. Attach the same context files again, and paste the two prior outputs below the prompt where indicated.

## Attached Context Files

- `05_live_execution_productization_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`)
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` (original repo path: `REPO_Improvement_Epics/01_epics/08_live_execution_recovery_and_reconnect_2026-04-11.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `commands_data.rs` (original repo path: `src-tauri/src/commands_data.rs`)
- `commands_execution.rs` (original repo path: `src-tauri/src/commands_execution.rs`)
- `executionService.ts` (original repo path: `web/src/services/executionService.ts`)
- `executionStore.ts` (original repo path: `web/src/stores/executionStore.ts`)
- `ExecutionWorkspacePanel.vue` (original repo path: `web/src/components/ExecutionWorkspacePanel.vue`)
- `lib.rs` (original repo path: `src-tauri/src/lib.rs`)
- `polygon.rs` (original repo path: `src-tauri/src/polygon.rs`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `state.rs` (original repo path: `src-tauri/src/state.rs`)

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
Live Execution Recovery And Reconnect

Files to review:
- `05_live_execution_productization_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`)
- `08_live_execution_recovery_and_reconnect_2026-04-11.md` (original repo path: `REPO_Improvement_Epics/01_epics/08_live_execution_recovery_and_reconnect_2026-04-11.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `commands_data.rs` (original repo path: `src-tauri/src/commands_data.rs`)
- `commands_execution.rs` (original repo path: `src-tauri/src/commands_execution.rs`)
- `executionService.ts` (original repo path: `web/src/services/executionService.ts`)
- `executionStore.ts` (original repo path: `web/src/stores/executionStore.ts`)
- `ExecutionWorkspacePanel.vue` (original repo path: `web/src/components/ExecutionWorkspacePanel.vue`)
- `lib.rs` (original repo path: `src-tauri/src/lib.rs`)
- `polygon.rs` (original repo path: `src-tauri/src/polygon.rs`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `state.rs` (original repo path: `src-tauri/src/state.rs`)

Bundle-specific focus to preserve:
- confirm restart, reconnect, and stale-state behavior is conservative and explicit
- verify separation of feed state, runner state, and broker snapshot state

Bundle-specific challenge focus to preserve:
- look for any path that could silently re-arm live behavior after interruption
- look for missing stale-symbol or stale-broker-state detection

First-pass critique to consolidate:
[paste here]

Second-pass critique to consolidate:
[paste here]
```
