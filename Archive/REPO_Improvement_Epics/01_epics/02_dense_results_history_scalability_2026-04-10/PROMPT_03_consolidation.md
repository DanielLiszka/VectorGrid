# Consolidation Prompt

Use this prompt after you have both a first-pass output and a second-pass output for this same packet. Attach the same context files again, and paste the two prior outputs below the prompt where indicated.

## Attached Context Files

- `02_dense_results_history_scalability_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/02_dense_results_history_scalability_2026-04-10.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `ExecutionWorkspacePanel.vue` (original repo path: `web/src/components/ExecutionWorkspacePanel.vue`)
- `HistoryWorkspacePanel.vue` (original repo path: `web/src/components/HistoryWorkspacePanel.vue`)
- `OverviewWorkspacePanel.vue` (original repo path: `web/src/components/OverviewWorkspacePanel.vue`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `TopWorkspacePanel.vue` (original repo path: `web/src/components/TopWorkspacePanel.vue`)
- `useHeadlessMatrixTable.ts` (original repo path: `web/src/composables/useHeadlessMatrixTable.ts`)
- `useVirtualTableRows.ts` (original repo path: `web/src/composables/useVirtualTableRows.ts`)
- `ValidationWorkspacePanel.vue` (original repo path: `web/src/components/ValidationWorkspacePanel.vue`)

## Prompt

```text
You are consolidating two prior critiques of one second-tier non-options epic document from my Rust/Tauri project.

Your inputs are:
1. the original bundle files
2. a first-pass critique
3. a second-pass adversarial critique

Your job is to produce one merged, repo-grounded conclusion.

Important constraints:
- Base the review only on the files I provide.
- Stay inside the current additive non-options scope.
- Do not propose broad UI redesigns or feature expansion unless directly required by the provided files.
- Prefer smaller, additive, low-regression improvements over large redesigns.
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
Dense Results And History Scalability

Files to review:
- `02_dense_results_history_scalability_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/02_dense_results_history_scalability_2026-04-10.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `ExecutionWorkspacePanel.vue` (original repo path: `web/src/components/ExecutionWorkspacePanel.vue`)
- `HistoryWorkspacePanel.vue` (original repo path: `web/src/components/HistoryWorkspacePanel.vue`)
- `OverviewWorkspacePanel.vue` (original repo path: `web/src/components/OverviewWorkspacePanel.vue`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `TopWorkspacePanel.vue` (original repo path: `web/src/components/TopWorkspacePanel.vue`)
- `useHeadlessMatrixTable.ts` (original repo path: `web/src/composables/useHeadlessMatrixTable.ts`)
- `useVirtualTableRows.ts` (original repo path: `web/src/composables/useVirtualTableRows.ts`)
- `ValidationWorkspacePanel.vue` (original repo path: `web/src/components/ValidationWorkspacePanel.vue`)

Bundle-specific focus to preserve:
- verify that viewport ownership, scroll-host semantics, virtualization thresholds, and keyboard/a11y behavior are specified clearly enough
- check whether the plan uses realistic dense fixtures rather than synthetic micro-tests
- look for missing browser-side and accessibility verification for heavy table surfaces

Bundle-specific challenge focus to preserve:
- look for any path where virtualization could bind to the wrong scroll host or rely on unstable shell layout assumptions
- challenge whether the plan protects keyboard navigation, focus restoration, and visible-row truth strongly enough
- look for missing rollback conditions if dense-table performance improves while debuggability or accessibility regresses

First-pass critique to consolidate:
[paste here]

Second-pass critique to consolidate:
[paste here]
```
