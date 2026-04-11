# Consolidation Prompt

Use this prompt after you have both a first-pass output and a second-pass output for this same packet. Attach the same context files again, and paste the two prior outputs below the prompt where indicated.

## Attached Context Files

- `01_artifact_history_scale_bootstrap_latency_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/01_artifact_history_scale_bootstrap_latency_2026-04-10.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `commands_artifacts.rs` (original repo path: `src-tauri/src/commands_artifacts.rs`)
- `HistoryContextDock.vue` (original repo path: `web/src/components/HistoryContextDock.vue`)
- `HistoryWorkspacePanel.vue` (original repo path: `web/src/components/HistoryWorkspacePanel.vue`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `run_artifacts.rs` (original repo path: `src-tauri/src/run_artifacts.rs`)
- `workflowArtifactsStore.ts` (original repo path: `web/src/stores/workflowArtifactsStore.ts`)
- `workflowArtifactsSync.ts` (original repo path: `web/src/services/workflowArtifactsSync.ts`)
- `workflowBootstrap.ts` (original repo path: `web/src/services/workflowBootstrap.ts`)

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
- Do not propose broad new product families, options scope, or architecture rewrites unless directly required by the provided files.
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
Artifact History Scale And Bootstrap Latency

Files to review:
- `01_artifact_history_scale_bootstrap_latency_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/01_artifact_history_scale_bootstrap_latency_2026-04-10.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `commands_artifacts.rs` (original repo path: `src-tauri/src/commands_artifacts.rs`)
- `HistoryContextDock.vue` (original repo path: `web/src/components/HistoryContextDock.vue`)
- `HistoryWorkspacePanel.vue` (original repo path: `web/src/components/HistoryWorkspacePanel.vue`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `run_artifacts.rs` (original repo path: `src-tauri/src/run_artifacts.rs`)
- `workflowArtifactsStore.ts` (original repo path: `web/src/stores/workflowArtifactsStore.ts`)
- `workflowArtifactsSync.ts` (original repo path: `web/src/services/workflowArtifactsSync.ts`)
- `workflowBootstrap.ts` (original repo path: `web/src/services/workflowBootstrap.ts`)

Bundle-specific focus to preserve:
- verify that the plan separates latest-run fast path, full-library scanning, and bootstrap critical-path work clearly
- check whether stale index risk, fallback-to-full-scan rules, and saved-run compatibility are specified strongly enough
- look for missing baseline or acceptance evidence for startup, reopen, and history-library scale scenarios

Bundle-specific challenge focus to preserve:
- look for hidden artifact-truth regressions, compatibility-alias drift, or stale-index traps
- challenge whether the plan could accidentally optimize perceived speed by weakening hydration truth
- look for missing rollback triggers if a fast path becomes harder to reason about than the current full scan

First-pass critique to consolidate:
[paste here]

Second-pass critique to consolidate:
[paste here]
```
