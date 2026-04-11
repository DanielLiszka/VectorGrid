# First-Pass Prompt

Use this prompt in a fresh GPT-5.4 Pro thread after attaching the context files from this packet. Do not attach the prompt files themselves unless you want the model to inspect the wording.

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
- Do not propose changing validation verdict semantics, workbook meaning, or report schema unless directly required by the provided files.
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

Bundle-specific focus:
- verify that the plan separates validation compute time, workbook/report build time, and persistence time cleanly
- check whether GPU fallback and backend-used truth remain explicit if throughput work changes route behavior
- look for missing parity gates or workbook-contract safeguards
```
