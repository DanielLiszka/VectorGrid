# Second-Pass Prompt

Use this prompt in a separate fresh GPT-5.4 Pro thread after attaching the same context files from this packet.

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
- Do not propose broad new strategy-family expansion or speculative CUDA reopenings unless directly required by the provided files.
- Prefer smaller, additive, evidence-backed route-policy improvements over large redesigns.
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

Bundle-specific challenge focus:
- look for any path where user-facing route copy could overclaim measured GPU maturity
- challenge whether the plan defines product guarantees versus best-effort previews clearly enough
- look for hidden drift between backend route behavior, frontend capability summaries, and perf evidence
```
