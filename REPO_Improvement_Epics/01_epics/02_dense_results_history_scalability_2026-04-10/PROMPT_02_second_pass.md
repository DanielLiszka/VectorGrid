# Second-Pass Prompt

Use this prompt in a separate fresh GPT-5.4 Pro thread after attaching the same context files from this packet.

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
You are doing an adversarial second-pass critique of one second-tier non-options epic document from my Rust/Tauri project.

Assume the plan may be overconfident, context-incomplete, or missing dangerous edge cases.

Your job is to stress-test the plan for:
1. hidden safety, correctness, or truthfulness problems
2. migration traps and ambiguous ownership
3. stale assumptions about the current repo
4. rollout paths that could silently weaken trust or clarity
5. verification plans that are too weak for the proposed change
6. contradictions between the plan and the provided owner files

Important constraints:
- Base the review only on the files I provide.
- Stay inside the current additive non-options scope.
- Do not propose broad UI redesigns or feature expansion unless directly required by the provided files.
- Prefer smaller, additive, low-regression improvements over large redesigns.
- Do not give generic praise.
- Prefer identifying dangerous omissions and contradictions over suggesting lots of new features.
- Distinguish clearly between:
  - confirmed doc gap
  - likely issue
  - open question
  - optional improvement
- Cite the file and section or module for every material claim.
- Flag any stale or nonexistent file references explicitly.

For each finding, use:
- Severity: critical / high / medium / low
- Type: scope / safety / correctness / operator-truth / migration / verification / testing / maintainability
- Location: file + section or file + module
- Why it matters
- Recommended change

Then provide:
1. Surface understanding
2. Findings
3. Dangerous assumptions or contradictions
4. Verification and rollback weaknesses
5. Must-fix doc changes before implementation
6. Nice-to-have doc improvements

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

Bundle-specific challenge focus:
- look for any path where virtualization could bind to the wrong scroll host or rely on unstable shell layout assumptions
- challenge whether the plan protects keyboard navigation, focus restoration, and visible-row truth strongly enough
- look for missing rollback conditions if dense-table performance improves while debuggability or accessibility regresses
```
