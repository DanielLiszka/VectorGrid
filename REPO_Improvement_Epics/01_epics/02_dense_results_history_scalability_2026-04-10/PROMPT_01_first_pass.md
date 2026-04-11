# First-Pass Prompt

Use this prompt in a fresh GPT-5.4 Pro thread after attaching the context files from this packet. Do not attach the prompt files themselves unless you want the model to inspect the wording.

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
You are reviewing one second-tier non-options epic document from my Rust/Tauri project.

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
- Stay inside the current additive non-options scope.
- Do not propose broad UI redesigns or feature expansion unless directly required by the provided files.
- Prefer smaller, additive, low-regression improvements over large redesigns.
- Do not give generic praise.
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

Bundle-specific focus:
- verify that viewport ownership, scroll-host semantics, virtualization thresholds, and keyboard/a11y behavior are specified clearly enough
- check whether the plan uses realistic dense fixtures rather than synthetic micro-tests
- look for missing browser-side and accessibility verification for heavy table surfaces
```
