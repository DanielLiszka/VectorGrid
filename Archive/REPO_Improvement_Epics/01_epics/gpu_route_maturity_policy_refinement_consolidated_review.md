# GPU Route Maturity And Policy Refinement — Consolidated Review

This document packages the consolidated third-pass review produced from the attached packet for **GPU Route Maturity And Policy Refinement**.

## Surface understanding

This epic is pointed at the right problem. Its own purpose and problem framing say the work is about route maturity, fallback clarity, route-policy explainability, and measured support boundaries, not generic GPU expansion. That matches the repo surface: `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` already have explicit target route policy; optimization already emits route and fallback metadata; validation has its own GPU fixed-candidate rules; and the frontend already has capability-summary surfaces. The real risk is drift between request legality, backend runtime behavior, capability summaries, validation wording, perf evidence, and persisted artifact provenance.

**Source anchors:** `04_gpu_route_maturity_policy_refinement_2026-04-10.md`; `REPO_MAP.md`; `AGENTS.md`; `composite_profile_route.rs`; `double_ma.rs`; `double_ma_validate.rs`; `strategyOptimizationCapabilities.ts`; `optimizeService.ts`.

## Consolidated must-fix findings

### A. The owner-file list is incomplete

The epic says route-policy logic already exists in explicit backend/frontend owners, but its `Current Code Surface` only names `composite_profile_route.rs`, `double_ma.rs`, `double_ma_validate.rs`, `strategyOptimizationCapabilities.ts`, `optimizeService.ts`, and the two GPU perf gates. The repo map says request builders are current request-rule authorities, capability summaries also include `compositeProfileGpuPolicy.ts`, and GPU route-policy work should start in `ma_policy.rs`, `double_ma_backend.rs`, and `double_ma_mode.rs` before the large `double_ma.rs` hub. `AGENTS.md` also points contributors for `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` to `optimizeRequestBuilder.ts`, `optimizeService.ts`, and `strategyOptimizationCapabilities.ts` first. Without those owners in scope, the epic can clean up copy while leaving legality and route-preview truth drifting.

**Recommended change:** add a `Route truth owners` subsection naming at least:
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`

Add `validationService.ts` if validation summaries or report-facing wording are touched.

**Source anchors:** `04_gpu_route_maturity_policy_refinement_2026-04-10.md` (`Current Code Surface`); `REPO_MAP.md` (`Task Routers`, `Strategy-surface matrix truth anchors`, `GPU route policy or kernel eligibility`); `AGENTS.md` (`Profile file map`).

### B. The matrix scope is too loose

The epic says it covers “supported current optimization and validation route behavior for the existing strategy families,” but the repo does not have one uniform family surface. `REPO_MAP.md` separates the Double-MA family, the composite/advanced family, `CompositeMo`, and `MlSignal`; it also notes that `DynamicDoubleMa` is a preview-plus-normal-runtime flow, while validation policy is more nuanced than optimize-route UI text. Given the attached files and the AGENTS profile map, this bundle is centered on the Double-MA family plus the routed composite-profile members `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum`. If the doc does not freeze that scope, it invites a flattened route matrix that mixes incompatible contracts.

**Recommended change:** explicitly state that this epic defaults to the Double-MA family plus routed `CompositeTrend` / `RegimeHybrid` / `BreakoutMomentum`, or else require separate sub-matrices if standalone composite/advanced routes or `CompositeMo` are intentionally included.

**Source anchors:** `04_gpu_route_maturity_policy_refinement_2026-04-10.md` (`Scope Boundaries`, `Checklist`); `REPO_MAP.md` (`Core Mental Model`, `Task Routers`, `Dynamic strategy IR or builder preview`); `AGENTS.md` (`For CompositeTrend, RegimeHybrid, and BreakoutMomentum, the current product optimization route is the double_ma family`).

### C. Truth precedence and guarantee tiers are still underspecified

The repo already defines a precedence rule: backend/request-builder legality first, capability-summary and route-preview truth second, component copy third. The epic says it will define product guarantees versus best-effort previews, but it does not extend that hierarchy to runtime result fields or persisted artifacts. Meanwhile `double_ma.rs` already emits route and fallback metadata, and `optimizeService.ts` both previews those fields and persists `route_provenance` into saved artifacts. Without an explicit hierarchy, the same route claim can drift across builder legality, runtime truth, preview text, and reopened history.

**Recommended change:** add a `Truth precedence` subsection that makes the order explicit:
1. backend/request-builder legality
2. backend runtime result fields
3. capability-summary / route-preview text
4. component-level presentation text
5. historical artifact provenance

**Source anchors:** `REPO_MAP.md` (`Strategy-surface matrix truth anchors`); `double_ma.rs` (`BackendOptimizationResult`, route/fallback fields); `optimizeService.ts` (route provenance persistence and run artifact shaping).

### D. Optimize and validation cannot share one flattened route matrix

The epic correctly says validation reality can differ from optimization copy, but that is still too abstract. The repo map explicitly warns not to infer validation support only from optimize-route UI text. `double_ma_validate.rs` makes the differences concrete: `GpuOnly` is rejected for GPU fixed-candidate validation, `cross_ticker` blocks that GPU path, non-vectorized validation blocks it, and CPU fallback can still emit the generic string “GPU validation runtime not implemented yet; using CPU validation.” That is a scenario matrix, not a small footnote.

**Recommended change:** require explicit optimize and validation route columns, or separate optimize/validation matrices, with at least:
- requested backend
- resolved backend
- GPU fixed-candidate eligibility
- blocker class
- fallback reason family
- export-all eligibility

**Source anchors:** `04_gpu_route_maturity_policy_refinement_2026-04-10.md` (`Current User-Facing Behavior`, `Regressions To Avoid`, `Proposed Improvement Approach`); `REPO_MAP.md` (`If the task is validation`, validation-routing notes); `double_ma_validate.rs` (`validation_supports_gpu_fixed_candidates`, `gpu_validation_feature_blocker`, `resolve_validation_backend_runtime`).

### E. Artifact compatibility is underdefined

The rollout section says route-policy copy is a compatibility surface, but that is not enough. `optimizeService.ts` persists `route_provenance` into run artifacts, so vocabulary or guarantee-tier changes affect reopened history, not just current copy. Right now the doc does not say whether historical labels are preserved, normalized, or versioned.

**Recommended change:** add an explicit artifact-compatibility rule: preserve historical labels as historical provenance, or introduce a versioned normalization rule for reopen/compare surfaces. Require at least one reopen-history verification slice when route vocabulary changes.

**Source anchors:** `04_gpu_route_maturity_policy_refinement_2026-04-10.md` (`Rollout / Fallback / Compatibility Notes`); `optimizeService.ts` (artifact persistence and route provenance shaping).

### F. The verification plan is too narrow for a cross-layer truth epic

The epic’s verification plan covers route-policy unit tests, the GPU perf gate, the ALMA gate when relevant, and basic Rust/web quality checks. But `REPO_MAP.md` says verification is layered: frontend tests, parity, output parity, workflow regression, and perf gates all matter; it also says `run_workflow_gate.ps1` should run before perf gates when staged workflow behavior can change, and that `run_gpu_alma_contract_rows_perf_gate.ps1` is specialized/manual rather than default PR perf authority. Since this epic touches route wording, legality, validation semantics, and artifacts, those additional gates are part of the real contract.

**Recommended change:** promote these to default candidates when route behavior, wording, or artifact shape changes:
- `npm --prefix web run test`
- `npm --prefix web run parity`
- `npm --prefix web run parity:outputs`
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`

Keep `run_gpu_perf_gate.ps1` as the main GPU perf authority and the ALMA gate only for ALMA-specific claims.

**Source anchors:** `04_gpu_route_maturity_policy_refinement_2026-04-10.md` (`Verification Plan`); `REPO_MAP.md` (`Build / Dev / Release Workflow`, `Current verification order to know`).

## Consolidated should-fix findings

### A. The doc should explicitly call out the current overclaim path in frontend capability logic

`strategyOptimizationCapabilities.ts` uses `requestedGlobalBackendIsGpuKernel()`, and that helper returns `true` for `GpuKernel`, `GpuHybridKernel`, and `GpuOnly`. That helper then drives DynamicDoubleMa and standalone-profile capability toggles. But the repo treats those backends distinctly, and validation explicitly rejects `GpuOnly` for fixed-candidate GPU validation. That makes this helper family a concrete example of how user-facing route copy can overclaim measured GPU maturity if it is not audited.

**Recommended change:** add one explicit checklist item to audit helpers and copy paths that collapse `GpuKernel`, `GpuHybridKernel`, and `GpuOnly` into a single “GPU kernel preview” bucket. Include a test or acceptance slice for that.

**Source anchors:** `strategyOptimizationCapabilities.ts` (`requestedGlobalBackendIsGpuKernel`, `DynamicDoubleMa`, standalone-profile capability branches); `double_ma_validate.rs` (`validation_supports_gpu_fixed_candidates`).

### B. The doc should clarify which composite-profile labels are product-facing

`composite_profile_route.rs` exposes both route-target labels (`CpuOnly`, `GpuKernel`, `GpuHybridKernel`) and runtime labels like `CompositeTrendCpu`, `RegimeHybridCpu`, and `BreakoutMomentumCpu`. The packet does not prove the runtime labels are wrong, but it does prove the distinction exists. The epic should make that distinction explicit so route matrices, copy, and artifacts do not accidentally mix target labels with internal/runtime compatibility labels.

**Recommended change:** say which label family is product-facing truth and keep the other family internal unless a surface explicitly needs it.

**Source anchors:** `composite_profile_route.rs` (`composite_profile_runtime_label`, `composite_profile_gpu_target_label`, associated tests).

### C. The evidence owners should be named directly

The repo map already points to maintained evidence bins and diagnostic owners, including `real_data_fidelity_audit.rs` for exact-vs-cheaper optimization/validation comparisons and the headless route/workflow helpers such as `ta_profile_route_matrix.rs` and `ta_workflow_regression.rs`. Since the epic is explicitly about measurement-backed route truth, naming those owners would make the route-matrix refresh reproducible instead of ad hoc.

**Recommended change:** add those maintained evidence lanes directly to the doc’s measurement/evidence language.

**Source anchors:** `REPO_MAP.md` (`If the task is headless tooling, route matrices, regression gates, or diagnostic binaries`).

## Optional improvements worth considering

Anchor the refreshed matrix to representative cases from `strategy_presets.rs` and `double_ma_test_support.rs`, especially:
- `CompositeTrend` ADX off/on
- `BreakoutMomentum` ADX off/on
- `RegimeHybrid`
- `GpuOnly` validation requests
- pure `GpuKernel` export-all
- DynamicDoubleMa preview/runtime resolution

`AGENTS.md` and `REPO_MAP.md` already point contributors to those preset/test helpers for representative route cases.

Reword the improvement approach to emphasize normalizing existing route/fallback surfaces before inventing new ones. Optimization already exposes `backend_used`, `backend_route`, `backend_fallback_reason`, `indicator_route_plan`, and artifact provenance, so the highest-value work is alignment, not extra reporting fields.

If validation wording is touched, narrow the generic “GPU validation runtime not implemented yet” string so supported GPU fixed-candidate slices do not inherit it when a more specific blocker is available.

**Source anchors:** `AGENTS.md` (`Profile file map`); `REPO_MAP.md` (strategy/test and diagnostic routing); `double_ma.rs`; `double_ma_validate.rs`.

## Suggestions to reject as out of scope or unsupported

Reject broad strategy-family expansion or a single repo-wide unified route matrix by default. The epic is scoped as additive non-options route-policy refinement, and the repo distinguishes Double-MA family routes from standalone composite/advanced routes, `CompositeMo`, and `MlSignal`, each with different contracts.

Reject speculative generic CUDA/SASS reopening inside this epic. The epic explicitly excludes generic CUDA or SASS micro-optimization without route-level evidence, and the repo map says GPU route work should start in route-policy files before low-level kernel files.

Reject any suggestion that `run_gpu_alma_contract_rows_perf_gate.ps1` is the default PR perf authority. The repo map labels it specialized/manual.

Reject any confident claim that the `Related Docs` paths in the epic are stale or nonexistent. That is not provable from the provided packet. The packet supports missing owner coverage inside the epic, but it does not prove those related-doc references are invalid.

Reject automatic renaming/removal of `composite_profile_runtime_label()` outputs. The packet proves label-family divergence, not breakage. Clarify usage first.

**Source anchors:** `04_gpu_route_maturity_policy_refinement_2026-04-10.md` (`Scope Boundaries`, `Related Docs`); `REPO_MAP.md` (`GPU route policy or kernel eligibility`, verification notes); `composite_profile_route.rs`.

## Recommended doc edits

Add a `Route truth owners` subsection under `Current Code Surface` and include:
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`

Add `validationService.ts` if route wording can affect validation/report summaries.

Add a `Matrix scope` subsection stating that this epic defaults to the Double-MA family plus routed `CompositeTrend` / `RegimeHybrid` / `BreakoutMomentum`, and that widening scope requires separate sub-matrices for standalone composite/advanced routes or `CompositeMo`.

Add a `Truth precedence` subsection with:
1. backend/request-builder legality
2. backend runtime result fields
3. capability-summary / route-preview truth
4. component copy
5. historical artifact provenance

Expand the checklist so it requires:
- representative scenario rows
- separate optimize and validation route columns
- validation blocker classes and fallback-reason families
- artifact-compatibility handling for route vocabulary changes
- cross-layer sync tests for backend labels versus frontend summaries/warnings

Expand the verification plan so it names:
- repo quality gate first
- `run_workflow_gate.ps1`
- `npm --prefix web run test`
- `npm --prefix web run parity`
- `npm --prefix web run parity:outputs`
- `run_gpu_perf_gate.ps1` when supported optimization routes change
- `run_gpu_alma_contract_rows_perf_gate.ps1` only when ALMA-specific claims are touched

Add one explicit evidence note that refreshed route claims should cite maintained evidence owners such as `real_data_fidelity_audit.rs`, `ta_profile_route_matrix.rs`, and representative preset/test-backed cases.

**Source anchors:** `04_gpu_route_maturity_policy_refinement_2026-04-10.md`; `REPO_MAP.md`; `AGENTS.md`; `double_ma_validate.rs`; `double_ma.rs`; `optimizeService.ts`.

## Recommended implementation order

1. Freeze scope and add the truth-precedence section. This prevents the rest of the work from drifting into a repo-wide rewrite or from mixing guarantees with previews.

2. Expand the owner-file list so request legality, route policy, capability summaries, validation wording, and artifact provenance all have named owners before copy changes begin.

3. Define the representative route matrix with separate optimize and validation columns and explicit scenario rows.

4. Capture baseline evidence from the current repo-owned truth and evidence surfaces before changing wording.

5. Update backend/frontend/artifact wording in one wave: request builders, capability summaries, optimize preview, validation wording, and artifact compatibility handling.

6. Add route-sync, validation-blocker, and reopen-history coverage. Then run the repo quality gate, workflow regression, web tests/parity/output parity, and only then the relevant GPU perf gate.

7. Only after the truth/evidence refresh lands should the team consider any narrower follow-up on route-specific optimization gaps. That keeps this epic aligned with its own purpose and scope.

**Source anchors:** `04_gpu_route_maturity_policy_refinement_2026-04-10.md`; `REPO_MAP.md`; `AGENTS.md`; `double_ma_validate.rs`; `double_ma.rs`; `optimizeService.ts`.

## Reviewed packet files

- `04_gpu_route_maturity_policy_refinement_2026-04-10.md`
- `AGENTS.md`
- `REPO_MAP.md`
- `composite_profile_route.rs`
- `double_ma.rs`
- `double_ma_validate.rs`
- `optimizeService.ts`
- `strategyOptimizationCapabilities.ts`
- `run_gpu_perf_gate.ps1`
- `run_gpu_alma_contract_rows_perf_gate.ps1`
- `PROMPT_01_first_pass.md`
- `PROMPT_02_second_pass.md`
- `PROMPT_03_consolidation.md`
