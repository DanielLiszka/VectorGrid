# First-Pass Prompt

Use this prompt in a fresh GPT-5.4 Pro thread after attaching the context files from this packet. Do not attach the prompt files themselves unless you want the model to inspect the wording.

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
- Do not propose broad new strategy-family expansion or speculative CUDA reopenings unless directly required by the provided files.
- Prefer smaller, additive, evidence-backed route-policy improvements over large redesigns.
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

Bundle-specific focus:
- verify that the plan is really about route truth, fallback clarity, and measurement evidence rather than generic GPU enthusiasm
- check whether optimize-route and validation-route differences are protected clearly enough
- look for missing route-matrix refresh steps or frontend/backend copy-drift safeguards
```
