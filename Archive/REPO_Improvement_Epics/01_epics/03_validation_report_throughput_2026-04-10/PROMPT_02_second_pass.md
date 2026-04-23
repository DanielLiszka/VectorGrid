# Second-Pass Prompt

Use this prompt in a separate fresh GPT-5.4 Pro thread after attaching the same context files from this packet.

## Attached Context Files

- `03_validation_report_throughput_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/03_validation_report_throughput_2026-04-10.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `commands_reports_validation_common.rs` (original repo path: `src-tauri/src/commands_reports_validation_common.rs`)
- `commands_reports_validation_xlsx.rs` (original repo path: `src-tauri/src/commands_reports_validation_xlsx.rs`)
- `commands_validation.rs` (original repo path: `src-tauri/src/commands_validation.rs`)
- `double_ma_validate.rs` (original repo path: `crates/app_core/src/double_ma_validate.rs`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `run_artifacts.rs` (original repo path: `src-tauri/src/run_artifacts.rs`)
- `validationService.ts` (original repo path: `web/src/services/validationService.ts`)

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
- Do not propose changing validation verdict semantics, workbook meaning, or report schema unless directly required by the provided files.
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
Validation And Report Throughput

Files to review:
- `03_validation_report_throughput_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/03_validation_report_throughput_2026-04-10.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `commands_reports_validation_common.rs` (original repo path: `src-tauri/src/commands_reports_validation_common.rs`)
- `commands_reports_validation_xlsx.rs` (original repo path: `src-tauri/src/commands_reports_validation_xlsx.rs`)
- `commands_validation.rs` (original repo path: `src-tauri/src/commands_validation.rs`)
- `double_ma_validate.rs` (original repo path: `crates/app_core/src/double_ma_validate.rs`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `run_artifacts.rs` (original repo path: `src-tauri/src/run_artifacts.rs`)
- `validationService.ts` (original repo path: `web/src/services/validationService.ts`)

Bundle-specific challenge focus:
- look for any hidden path where throughput gains could actually change verdict semantics or workbook fidelity
- challenge whether the plan identifies the dominant bottleneck before proposing implementation changes
- look for missing rollback conditions if faster output becomes harder to trust or maintain
```
