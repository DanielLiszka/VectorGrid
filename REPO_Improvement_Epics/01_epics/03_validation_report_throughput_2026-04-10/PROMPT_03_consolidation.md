# Consolidation Prompt

Use this prompt after you have both a first-pass output and a second-pass output for this same packet. Attach the same context files again, and paste the two prior outputs below the prompt where indicated.

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
You are consolidating two prior critiques of one second-tier non-options epic document from my Rust/Tauri project.

Your inputs are:
1. the original bundle files
2. a first-pass critique
3. a second-pass adversarial critique

Your job is to produce one merged, repo-grounded conclusion.

Important constraints:
- Base the review only on the files I provide.
- Stay inside the current additive non-options scope.
- Do not propose changing validation verdict semantics, workbook meaning, or report schema unless directly required by the provided files.
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

Bundle-specific focus to preserve:
- verify that the plan separates validation compute time, workbook/report build time, and persistence time cleanly
- check whether GPU fallback and backend-used truth remain explicit if throughput work changes route behavior
- look for missing parity gates or workbook-contract safeguards

Bundle-specific challenge focus to preserve:
- look for any hidden path where throughput gains could actually change verdict semantics or workbook fidelity
- challenge whether the plan identifies the dominant bottleneck before proposing implementation changes
- look for missing rollback conditions if faster output becomes harder to trust or maintain

First-pass critique to consolidate:
[paste here]

Second-pass critique to consolidate:
[paste here]
```
