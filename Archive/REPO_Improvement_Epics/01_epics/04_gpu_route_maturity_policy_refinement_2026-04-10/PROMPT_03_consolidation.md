# Consolidation Prompt

Use this prompt after you have both a first-pass output and a second-pass output for this same packet. Attach the same context files again, and paste the two prior outputs below the prompt where indicated.

## Attached Context Files

- `04_gpu_route_maturity_policy_refinement_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/04_gpu_route_maturity_policy_refinement_2026-04-10.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `composite_profile_route.rs` (original repo path: `crates/app_core/src/composite_profile_route.rs`)
- `double_ma_validate.rs` (original repo path: `crates/app_core/src/double_ma_validate.rs`)
- `double_ma.rs` (original repo path: `crates/app_core/src/double_ma.rs`)
- `optimizeService.ts` (original repo path: `web/src/services/optimizeService.ts`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `run_gpu_alma_contract_rows_perf_gate.ps1` (original repo path: `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`)
- `run_gpu_perf_gate.ps1` (original repo path: `scripts/run_gpu_perf_gate.ps1`)
- `strategyOptimizationCapabilities.ts` (original repo path: `web/src/utils/strategyOptimizationCapabilities.ts`)

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
- Do not propose broad new strategy-family expansion or speculative CUDA reopenings unless directly required by the provided files.
- Prefer smaller, additive, evidence-backed route-policy improvements over large redesigns.
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
GPU Route Maturity And Policy Refinement

Files to review:
- `04_gpu_route_maturity_policy_refinement_2026-04-10.md` (original repo path: `REPO_Improvement_Epics/01_epics/04_gpu_route_maturity_policy_refinement_2026-04-10.md`)
- `AGENTS.md` (original repo path: `AGENTS.md`)
- `composite_profile_route.rs` (original repo path: `crates/app_core/src/composite_profile_route.rs`)
- `double_ma_validate.rs` (original repo path: `crates/app_core/src/double_ma_validate.rs`)
- `double_ma.rs` (original repo path: `crates/app_core/src/double_ma.rs`)
- `optimizeService.ts` (original repo path: `web/src/services/optimizeService.ts`)
- `REPO_MAP.md` (original repo path: `REPO_MAP.md`)
- `run_gpu_alma_contract_rows_perf_gate.ps1` (original repo path: `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`)
- `run_gpu_perf_gate.ps1` (original repo path: `scripts/run_gpu_perf_gate.ps1`)
- `strategyOptimizationCapabilities.ts` (original repo path: `web/src/utils/strategyOptimizationCapabilities.ts`)

Bundle-specific focus to preserve:
- verify that the plan is really about route truth, fallback clarity, and measurement evidence rather than generic GPU enthusiasm
- check whether optimize-route and validation-route differences are protected clearly enough
- look for missing route-matrix refresh steps or frontend/backend copy-drift safeguards

Bundle-specific challenge focus to preserve:
- look for any path where user-facing route copy could overclaim measured GPU maturity
- challenge whether the plan defines product guarantees versus best-effort previews clearly enough
- look for hidden drift between backend route behavior, frontend capability summaries, and perf evidence

First-pass critique to consolidate:
[paste here]

Second-pass critique to consolidate:
[paste here]
```
