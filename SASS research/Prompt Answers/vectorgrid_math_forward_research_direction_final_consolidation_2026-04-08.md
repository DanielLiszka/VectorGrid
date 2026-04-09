# VectorGrid Math-Forward Research Direction: Final Consolidation Memo

Date: 2026-04-08

## Adopted Direction

Adopt **admitted-fragment semantics and differential conformance for the Double-MA-family optimization language**. The repo already exposes a real conditional search language: finite period/type/parameter grids, explicit objective and score modes, route-dependent legality for export and shortlist controls, deterministic composite routing, and a constrained `DynamicDoubleMa` path that is only exact on previewed `GpuKernel(dynamic_strategy_v1)` shapes. The strongest math-forward direction is therefore not a new optimizer theorem. It is a formal extractor for that admitted fragment, plus a soundness and route-preservation claim checked against the frontend builder, capability preview, backend runtime, and validation fallback behavior. This uses structure the repo actually has, survives backend substitution, and is stronger than exact-pruning or generic regret framing for the current codebase.

## Exact Formal Problem Statement

Let `r` be a normalized request in an admitted fragment `R_adm` of the repo request language. Write

`r = (P_f, P_s, T_f, T_s, Θ_f, Θ_s, s, b, o, u)`

where:

- `P_f, P_s` are discrete fast and slow period ranges.
- `T_f, T_s` are fast and slow MA-type sets.
- `Θ_f, Θ_s` are optional MA-parameter grids.
- `s` is the strategy or profile configuration.
- `b` is the requested backend.
- `o` is the objective / score-mode / metric-contract bundle.
- `u` is the execution and control bundle, including `mode`, `top_k`, `include_all`, `export_all_csv_path`, extra shortlist heads, confidence / union / slice controls, eval-range controls, trade-metric requests, and validation-facing options.

Define the feasible candidate space

`X_r = { x = (p_f, p_s, t_f, t_s, θ_f, θ_s, s) : p_f < p_s, t_f ∈ T_f, t_s ∈ T_s, θ_f ∈ Θ_f(t_f), θ_s ∈ Θ_s(t_s), A(r, x) = 1 }`

where `A(r, x)` is the legality predicate induced by normalization, frontend guards, backend route guards, family-specific routing rules, and exact-control restrictions.

Define the request-level objective

`f_r(x) = S_o(M_r(x))`

where `M_r(x)` is the authoritative metric vector returned for `x` on the resolved route and requested metric surface, and `S_o` is the selected objective or score mode.

Define the extracted semantics

`E(r) = (X_r, ρ(r), K(r), μ(r), ν(r), c(r))`

where:

- `ρ(r) ∈ {CpuOnly, GpuKernel, GpuHybridKernel}` is the resolved route class.
- `K(r)` is the allowed advanced-control set.
- `μ(r)` is the exactness or audited-surface tag.
- `ν(r)` is the requested or emitted metric-surface contract tag when applicable.
- `c(r)` is a symbolic cost class, not a wall-clock predictor:

`c(r) = (ρ(r), mode(r), variantPairCount(r), validationFallbackClass(r))`

The research problem is to construct `E` for `R_adm` so that it soundly predicts feasible search space, route class, and control legality across the actual repo surfaces.

### Admitted Fragment

The admitted fragment should include:

- static `DoubleMa` and audited Double-MA-family routes,
- composite profiles with explicit route rules,
- and only those `DynamicDoubleMa` cases that preview as exact-compatible `dynamic_strategy_v1` GPU shapes.

The admitted fragment should exclude cases that require a full-language completeness claim across dynamic fallbacks, validation-only exceptions, or undocumented capability drift.

## Assumptions

### Repo-supported

The repo already provides the structure needed for the direction:

- a finite discrete request language with explicit objective and mode enums,
- route-dependent control guards,
- deterministic composite route rules,
- backend-side legality checks for exact-kernel controls,
- explicit validation/runtime fallback behavior,
- and a clear distinction between exact kernel routes, hybrid routes, and CPU routes.

Concrete supporting surfaces include:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

### Plausible but unverified

These assumptions are reasonable, but not yet proved by the repo:

- the builder, capability layer, backend runtime, and validation runtime can be reconciled into one compact shared semantics rather than only a long list of special cases,
- a narrow completeness result may hold on a static exact-route subfragment,
- the extracted semantics will have practical value for request generation, route preview, benchmark coverage, or mismatch detection,
- and the admitted fragment will generalize across the intended Double-MA family without collapsing into many one-off exceptions.

### Unrealistic but intentionally rejected

The direction explicitly rejects the following assumptions:

- full-language completeness across all dynamic and fallback cases,
- smoothness, convexity, Lipschitz continuity, or monotonicity of the objective surface,
- admissible pruning bounds derived only from parameter order,
- a globally valid distortion model for `GpuVdps`,
- and a quantitative cost algebra strong enough for theorem-level runtime claims.

These are intentionally rejected because recent repo-side results show that local systems behavior does not support clean global runtime abstractions and because the objective family is too discontinuous for strong optimizer-side structure claims.

## Minimum Claim Worth Pursuing

Pursue the following correctness claim and stop there unless more falls out naturally.

For every request `r` in `R_adm`:

1. **Normalization:** semantically equivalent requests map to the same extracted object `E(r)`.
2. **Soundness:** every `x ∈ X_r` is executable on the predicted route class without violating repo legality guards.
3. **Route preservation:** the extracted route `ρ(r)` matches preview/runtime behavior up to documented fallback classes such as `Auto->GpuKernel`, `Auto->CpuOnly`, `GpuHybridKernel->GpuKernel`, and exact-compatible `GpuKernel(dynamic_strategy_v1)`.
4. **Guard preservation:** legality of `exportAllCsvPath`, extra shortlist heads, shortlist confidence / union / slice controls, metric-contract requests, trade-metric requests, and single-slice restrictions matches repo behavior on the admitted fragment.

Full completeness is **not** part of the minimum claim. Attempt it only on a narrow static exact-route subset if it becomes easy.

## Why Rejected Alternatives Were Rejected

### Exact pruning / certified elimination

Rejected because the repo exposes syntactic feasibility, not objective order. The objective family is shaped by crossover logic, exits, regime filters, breakout/reversion logic, cooldown and persistence overlays, and family-specific toggles. That is not a repo-supported basis for monotonicity, admissible bounds, or sound order-only elimination.

### Distortion-aware multi-fidelity optimization

Rejected as the lead because `GpuVdps` supplies rung ladders and overlap/regret telemetry, but not a repo-supported distortion or rank-preservation relation between rung scores and final exact scores. The current theorem burden would sit mostly on an unproved assumption.

### Batch-delayed regret or cost-aware simple regret

Rejected because it would use too little repo structure. The repo does have batched GPU execution surfaces, but a theorem that only says “this system evaluates in delayed batches” would read as generic black-box optimization theory, not as a contribution induced by this codebase.

### Pure systems benchmark / platform direction

Not adopted as the primary direction because the repo already has enough explicit conditional semantics to justify one stronger step: a sound extractor with conformance evidence. This remains the better target unless the proof collapses.

## Minimum Empirical Benchmark Matrix

| Matrix | Axes | Required result |
|---|---|---|
| Route and capability conformance | strategy family × requested backend × ADX toggle × dynamic exact-compatible toggle × data-field availability × advanced-control bundle | Extracted route and allowed-control set match builder output, capability preview, backend runtime, and documented fallbacks. |
| Guard matrix | `{extra_shortlist, confidence, union, slice, slice_benchmark, neighborhood, exportAll, metric_contract, include_trade_metrics}` × `{topK, includeAll, variantPairCount, mode}` | No undocumented accept/reject mismatches on the admitted fragment. |
| Validation boundary matrix | optimization backend × validation backend × fixed-candidate GPU eligibility | Predicted validation route and fallback notes match runtime behavior. |
| Differential fuzzing corpus | generated requests covering the admitted fragment, including composite profiles and exact-compatible dynamic shapes | Counterexamples are either eliminated or turned into explicit out-of-fragment cases. |
| Utility ablation | request generation / UI preview / test synthesis with extractor vs without extractor | The extracted semantics must either reduce invalid requests, detect real drift, or increase meaningful coverage. |
| Route smoke-test harness | strategy preset × workload × candle shape × metric contract × shortlist slice mode | Route labels and diagnostics stay stable where semantics predict stability. Use as execution sanity check, not as theorem evidence. |

## Direction-Specific Kill Test

Kill the direction if, after narrowing to the admitted fragment, any of the following is true:

- the best formal statement is only a restatement of existing `if` branches with no smaller canonical model,
- cross-layer conformance exposes too many exceptions to state a compact soundness theorem,
- the extractor yields no practical benefit over ordinary tests,
- or making the result look mathematical requires importing rejected assumptions about smoothness, pruning bounds, distortion, or runtime cost.

If the kill test fires, fall back to the benchmark/platform version and do not reopen optimizer implementation.

## Final Verdict

**Design-only follow-up.**

Do **not** reopen implementation yet. The next justified work is:

1. formalize the admitted fragment,
2. prove the minimum correctness claim,
3. run the conformance matrix,
4. and only then decide whether there is enough substance to justify new implementation work.

This is consistent with the repo’s recent branch posture: retain bounded wins, preserve negative results, and stop when the next step is no longer clearly justified by the evidence.
