# GPU Route Maturity And Policy Refinement Plan

Last updated: 2026-04-12

## Purpose

Define a measured epic to improve GPU route maturity, fallback clarity, and route-policy explainability across supported non-options strategy surfaces without reopening broad speculative kernel work.

## Scope Boundaries

In scope:

- route-selection clarity for supported current Double-MA-family strategies and the routed `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` profiles unless this epic is intentionally widened
- backend/fallback reason truthfulness in frontend and backend surfaces
- route-policy measurement and route-matrix evidence
- supported current optimization and validation route behavior for the existing strategy families

Out of scope:

- broad new strategy-family expansion
- reopening generic CUDA or SASS micro-optimization work without route-level evidence
- options-related GPU route work
- live execution expansion except where route vocabulary overlaps with current strategy labels

## Current User-Facing Behavior

The repo already has explicit route-policy code and user-facing capability summaries for supported strategy families. That is better than a typical implicit-route product, but the maturity gap is now less about “is there some GPU path?” and more about:

- which route is selected
- why a fallback happened
- which scenarios are actually measured and supported
- how validation-route reality differs from optimization-route copy

That makes this epic a policy-and-evidence refinement lane rather than a generic performance lane.

## Current Code Surface

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/optimizeService.ts`
- `web/src/services/validationService.ts`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`

## Current Verification Surface

- `crates/app_core/src/composite_profile_route.rs`
  - route-label and precedence tests
- `crates/app_core/src/double_ma_validate.rs`
  - validation backend/fallback tests
- `web/src/services/optimizeRequestBuilder.test.ts`
- `web/src/services/validationRequestBuilder.test.ts`
- `web/src/utils/strategyOptimizationCapabilities.test.ts`
- `crates/app_core/src/strategy_presets.rs`
- `crates/app_core/src/double_ma_test_support.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`
- route-matrix and profiling artifacts created during the first-tier tests/perf wave

## Route Truth Precedence

1. backend and request-builder legality
2. backend runtime result fields and fallback reasons
3. capability-summary and route-preview truth
4. component-level presentation text
5. historical artifact provenance such as persisted `route_provenance`

## Strengths Already Present

- route-policy logic already exists in explicit backend/frontend owners
- first-tier perf/profile work already created real route evidence surfaces
- frontend capability text is already more explicit than a generic “GPU supported” label

## Weaknesses / Failure Modes

- fallback reasons can remain technically correct but still insufficiently measurable for product decisions
- optimization-route and validation-route reality can diverge in ways the UI must describe honestly
- route-policy claims can drift if measurement lanes and user-facing copy evolve separately
- it is easy to reopen broad CUDA tuning when the real missing work is route observability and decision clarity

## Regressions To Avoid

- changing route copy without confirming backend truth
- claiming wider GPU maturity than the measured supported routes actually provide
- broadening scope into unrelated kernel work before route-policy evidence is refreshed
- flattening optimization and validation policy into one simplification when the repo already knows they differ
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html)
  - relevant because route-policy maturity should prefer measured bottlenecks and workload fit over generic GPU enthusiasm
- [Nsight Compute Documentation](https://docs.nvidia.com/nsight-compute/NsightCompute/index.html)
  - relevant because route decisions should still be evidence-backed when kernel-level attribution is needed
- [Nsight Systems Documentation](https://docs.nvidia.com/nsight-systems/UserGuide/index.html)
  - relevant because some route wins or failures are end-to-end pipeline issues rather than hot-kernel issues

## Proposed Improvement Approach

1. Refresh the supported-route matrix with explicit strategy/config scenarios and separate optimize versus validation route/fallback outcomes.
2. Make fallback reasons and backend-used reporting easier to compare across:
   - optimize request shaping
   - optimize execution
   - validation execution
   - user-facing capability summaries
3. Define which route decisions are product-level guarantees versus best-effort previews, and which label families are product-facing versus internal/runtime-only.
4. Keep performance work subordinate to policy truth:
   - only pursue further route-specific optimization where the refreshed matrix shows a real supported-route gap
5. Keep artifact compatibility explicit whenever route vocabulary or fallback wording changes.
6. Keep success criteria explicit:
   - clearer route/fallback truth across backend and frontend
   - refreshed measurement evidence for supported routes
   - no regression in current CPU fallback safety

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Define the supported-route matrix that should be refreshed for this epic, including whether the scope stays within the Double-MA family plus routed composite profiles or is intentionally widened.
- [x] Record the current optimize and validation route/fallback outcomes for those scenarios as separate columns, not one flattened route table.
- [x] Freeze the owner files that define route truth across request legality, runtime behavior, frontend summaries, and persisted artifacts.
- [x] Identify where user-facing route copy currently understates or overstates backend reality.
- [x] Preserve CPU fallback safety and current supported-route boundaries unless a narrower follow-up doc widens them.
- [x] Add or strengthen route-policy and fallback-reason tests where the current contract is too implicit.
- [x] Audit helper and copy paths that collapse `GpuKernel`, `GpuHybridKernel`, and `GpuOnly` into one generic “GPU kernel” preview bucket, and either justify or narrow that grouping.
- [x] Refresh the measurement evidence for any route claim that will change.
- [x] Define artifact-compatibility handling for route vocabulary changes, including how historical `route_provenance` should be reopened or compared.
- [x] Define rollback conditions if a route-policy refinement makes current behavior harder to reason about or less truthful.
- [x] Record any required cross-doc sync in the first-tier strategy/optimization/validation/perf docs when route wording or evidence changes.

## Verification Plan

- rerun the relevant route-policy unit tests and add targeted coverage near touched backend/frontend route owners
- rerun `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1` when the change crosses backend, frontend, and artifact surfaces
- rerun `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` when staged workflow behavior or reopened artifact wording can change
- rerun `npm.cmd --prefix web run test`
- rerun `npm.cmd --prefix web run parity`
- rerun `npm.cmd --prefix web run parity:outputs` when route wording or artifact/output summaries change
- rerun the appropriate GPU measurement lanes for any changed supported-route claims
- rerun `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` when current supported optimization routes are affected
- rerun `powershell -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` only when ALMA contract-row claims or helper paths are affected
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm.cmd --prefix web run lint` and `npm.cmd --prefix web run typecheck`
- capture concrete acceptance evidence for closure, including the refreshed route matrix and the exact measured scenarios used to justify any copy or policy change

## Execution Notes

- This epic closed on two bounded implementation waves:
  - route-truth cleanup across frontend validation previews, backend validation/report summaries, and saved-run route provenance review surfaces
  - refreshed GPU route and workflow evidence after the route-truth changes landed
- Route-truth implementation surfaces updated during this epic:
  - `web/src/services/validationRequestBuilder.ts`
  - `web/src/services/validationRequestBuilder.test.ts`
  - `src-tauri/src/commands_reports_validation_common.rs`
  - `web/src/components/HistoryWorkspacePanel.vue`
  - `web/src/components/HistoryWorkspacePanel.test.ts`
- The route-truth cleanup explicitly preserved the current validation-runtime reality:
  - `Auto` validation previews and report summaries stay on kernel-route wording for the routed composite profiles because backend auto validation still resolves through the kernel route today
  - explicit `GpuHybridKernel` and ADX-bearing routed profile requests now surface the hybrid route honestly
  - saved-run route review rows now show compact requested/resolved/actual/fallback provenance instead of a reduced actual-only pill
- Fresh acceptance evidence used for closure:
  - `npm.cmd --prefix web run test -- src/services/validationRequestBuilder.test.ts src/utils/strategyOptimizationCapabilities.test.ts`
  - `npm.cmd --prefix web run test -- src/components/HistoryWorkspacePanel.test.ts src/services/validationRequestBuilder.test.ts src/utils/strategyOptimizationCapabilities.test.ts`
  - `cargo test -p ta_desktop_demo_app composite_trend_auto_expected_summary_stays_on_kernel_wording_even_with_adx --lib`
  - `cargo test -p ta_desktop_demo_app breakout_momentum_expected_summary_reroutes_explicit_kernel_request_when_adx_is_on --lib`
  - `cargo test -p ta_desktop_demo_app composite_double_like_cpu_and_gpuonly_expected_summaries_keep_strategy_labels --lib`
  - `npm.cmd --prefix web run typecheck`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run build`
  - `npm.cmd --prefix web run a11y:smoke`
  - `npm.cmd --prefix web run test:browser:ux`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests`
  - `cargo run -p ta_desktop_demo_app --release --bin ta_profile_route_matrix --features cuda,cuda-backtest-kernel -- --csv data/benchmarks/AAPL.csv --strict --warmup-runs 1 --measured-runs 3 --out target/profile_route_matrix_epic04_aapl_15m.json`
  - `cargo test -p ta_desktop_demo_app --bin ta_profile_route_matrix`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
  - `npm.cmd --prefix web run parity`
  - `npm.cmd --prefix web run parity:outputs`
  - `powershell -ExecutionPolicy Bypass -File scripts/run_workflow_gate.ps1 -Mode candidate`
- Verified route-matrix artifact:
  - `target/profile_route_matrix_epic04_aapl_15m.json`
  - `schema_version`: `1`
  - `build_profile`: `release`
  - `csv_path`: `data/benchmarks/AAPL.csv`
  - `timeframe`: `15m`
  - `total_bars`: `205886`
  - audited case count: `10`
  - strict failures: `0`
  - all canonical measured runs stayed on the expected route with no fallback reason
- Representative canonical measured-run route timings from the refreshed route matrix:
  - `CompositeTrend/ADX off/GpuKernel`: `90 ms`
  - `CompositeTrend/ADX on/GpuHybridKernel`: `92 ms`
  - `BreakoutMomentum/ADX off/GpuKernel`: `103 ms`
  - `BreakoutMomentum/ADX on/GpuHybridKernel`: `101 ms`
  - `RegimeHybrid/GpuHybridKernel`: `98 ms`
- Verified GPU perf gate artifact:
  - `target/gpu_perf_summary.json`
  - GPU: `NVIDIA GeForce RTX 4090`
  - driver: `595.71`
  - passing checks:
    - `gpu_criterion_case_gpu_sma_small_not_regressed_10pct`: `65.224588 ms` vs baseline `70.640588 ms`
    - `gpu_criterion_case_gpu_alma_large_not_regressed_10pct`: `69.144527 ms` vs baseline `85.373831 ms`
    - `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `0.988372` vs baseline `0.966667`
    - `gpu_dynamic_strategy_static_off_latency_not_regressed_20pct`: `64.5 ms` vs baseline `67.5 ms`
- Verified workflow/output parity artifacts:
  - `target/workflow_regression_summary.json`
  - `runtime_ms`: `241` vs baseline `229`
  - `optimize_ms_total`: `21` vs baseline `22`
  - `validate_ms_total`: `126` vs baseline `119`
  - `meta_select_ms`: `14` vs baseline `14`
  - `holdout_ms`: `57` vs baseline `55`
  - `npm.cmd --prefix web run parity:outputs` passed for `2` pair(s)
- No `README.md` or `REPO_MAP.md` update was required for closure because this epic refined route truth and refreshed evidence without changing the published route-policy scope or benchmark-authority boundaries.

## Rollout / Fallback / Compatibility Notes

- start with route-matrix refresh and truth cleanup before any new optimization branch
- treat route-policy copy as a compatibility surface; change it only when the supporting evidence is updated in the same wave
- keep optimize and validation route truth separate where the repo already knows they differ
- keep CPU fallback behavior explicit and preserved if a GPU route remains unsupported or only partially supported
- do not widen this epic into live execution or broad strategy expansion unless a separate epic explicitly links them

## Related Docs

- `REPO_Improvement/03_opt/24_eng_composite_profiles_backend_policy_2026-04-09.md`
- `REPO_Improvement/05_validation/06_validate_backend_route_semantics_2026-04-09.md`
- `REPO_Improvement/07_strats/13_eng_composite_route_policy_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/36_gpu_route_matrix_profile_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/41_gpu_alma_contract_gate_2026-04-09.md`
