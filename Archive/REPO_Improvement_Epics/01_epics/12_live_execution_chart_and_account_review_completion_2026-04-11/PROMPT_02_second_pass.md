# Second-Pass Prompt

Use this prompt in a separate fresh GPT-5.4 Pro thread after attaching the same context files from this packet.

## Attached Context Files

- `05_live_execution_productization_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`)
- `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (original repo path: `REPO_Improvement_Epics/01_epics/12_live_execution_chart_and_account_review_completion_2026-04-11.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `analysisService.ts` (original repo path: `web/src/services/analysisService.ts`)
- `commands_execution.rs` (original repo path: `src-tauri/src/commands_execution.rs`)
- `executionService.ts` (original repo path: `web/src/services/executionService.ts`)
- `executionStore.ts` (original repo path: `web/src/stores/executionStore.ts`)
- `ExecutionWorkspacePanel.vue` (original repo path: `web/src/components/ExecutionWorkspacePanel.vue`)
- `lib.rs` (original repo path: `src-tauri/src/lib.rs`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `SCREENSHOT_01_execution_review_surface.jpg` (original repo path: `current UI screenshot captured via Playwright browser harness`)
- `SCREENSHOT_02_execution_review_section.jpg` (original repo path: `current UI screenshot captured via Playwright browser harness`)

## Prompt

```text
You are doing an adversarial second-pass critique of one live-mode implementation plan document from my Rust/Tauri project.

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
- Treat the current live scope as narrow: current broker path, current feed path, single-stream, single-runner, non-options only.
- Do not propose broad new product families, portfolio dashboards, multi-broker expansion, options live trading, compliance platforms, or cloud sync unless directly required by the provided files.
- Prefer smaller, additive changes over redesigns.
- Do not give generic praise.
- Inspect the included screenshot files as current UI evidence. Use them to evaluate hierarchy, clarity, placement, and visible trust signals in addition to the code and plan.
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
Live Execution Chart And Account Review Completion

Files to review:
- `05_live_execution_productization_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`)
- `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (original repo path: `REPO_Improvement_Epics/01_epics/12_live_execution_chart_and_account_review_completion_2026-04-11.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `analysisService.ts` (original repo path: `web/src/services/analysisService.ts`)
- `commands_execution.rs` (original repo path: `src-tauri/src/commands_execution.rs`)
- `executionService.ts` (original repo path: `web/src/services/executionService.ts`)
- `executionStore.ts` (original repo path: `web/src/stores/executionStore.ts`)
- `ExecutionWorkspacePanel.vue` (original repo path: `web/src/components/ExecutionWorkspacePanel.vue`)
- `lib.rs` (original repo path: `src-tauri/src/lib.rs`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `SCREENSHOT_01_execution_review_surface.jpg` (original repo path: `current UI screenshot captured via Playwright browser harness`)
- `SCREENSHOT_02_execution_review_section.jpg` (original repo path: `current UI screenshot captured via Playwright browser harness`)

Bundle-specific challenge focus:
- look for any path where the chart becomes prettier but less truthful
- look for assumptions that the execution review surface already has a completed charting contract
- look for any mismatch between the plan and the current visual evidence around chart completeness and account review density
```
