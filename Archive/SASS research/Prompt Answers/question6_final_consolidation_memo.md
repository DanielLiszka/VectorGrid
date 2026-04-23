# Question 6 — Final Consolidation Memo

This memo follows the uploaded Question 6 prompt bundle and final-consolidation rubric.

## Adopted direction

Adopt a **conformance-checked optimization-contract IR / search-space normalizer** for the `DoubleMa` family and its route-conditional overlays. Do **not** reopen work on “a new optimizer,” and do **not** lead with “compiler” yet. The contribution worth testing is an extractor that turns the repo’s existing executable request rules, route policies, backend legality checks, and output-contract gates into a canonical optimizer-facing IR, then lowers that IR into existing optimizers and checks semantic agreement against current behavior. This is the narrowest version that is still plausibly research-grade and still survives if CUDA is treated as substrate rather than novelty.

## Exact problem statement

For a strategy family `s` and evaluation context `d`, build an automatically extracted IR

`I(s,d) = (Θ, A, C, O, R, X, H)`

where:

- `Θ`: parameters and admissible domains
- `A`: activation graph for conditional parameters
- `C`: forbidden configurations and hard constraints
- `O`: objective and metric bundles
- `R`: backend and route capability classes
- `X`: output-contract legality
- `H`: optional stage or cost hints that are carried as metadata, not yet a main claim

Then evaluate whether using `I(s,d)` instead of hand-authored spaces improves legality preservation, invalid-trial rate, optimizer portability, and time-to-target under a fixed evaluation budget.

This repo already contains that structure, but it is scattered. `optimizeRequestBuilder.ts` enforces candle-field requirements, metric-contract conflicts, route preview, and exact-kernel-only shortlist/export controls; `double_ma.rs` exposes hard constraints, score modes, metric contracts, and `VdpsConfig`; `strategyOptimizationCapabilities.ts` describes `DynamicDoubleMa` as a constrained `strategy_ir` path with previewed GPU eligibility and explicit CPU fallback reasons; and `composite_profile_route.rs` plus `AGENTS.md` encode ADX-dependent routing for `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid`.

## Why it is still plausible after prior work

The prior-work collision is real. ConfigSpace and SMAC3 already cover conditions, forbidden clauses, and conditional/hierarchical spaces; Optuna and Hyperopt already support define-by-run or nested stochastic conditional spaces; OSS Vizier already supports conditional search spaces and infeasibility; BOHB already covers multi-fidelity optimization; and HyperMapper 2.0 already targets systems design-space exploration with mixed variables, constraints, and multiple objectives. That means the novelty is **not** “conditional spaces,” **not** “constraints,” and **not** “fidelity ladders” by themselves.

What remains plausible is narrower: **automatic extraction plus conformance checking from executable application surfaces**. VectorGrid is not merely exposing tunable parameters; it is exposing a distributed optimization API contract. The repo ties legality, backend class, output admissibility, and metric surface to strategy context. For example, the kernel route requires `exit_mode = cross` and rejects cooldown/persistence overlays; shortlist confidence, union, and slice are kernel-only; `exportAllCsvPath` is kernel-only and incompatible with MA-param sweeps; `metricContract=full_17` conflicts with `includeTradeMetrics=true`; and `DynamicDoubleMa` only admits extra shortlist and metric-contract controls on exact-compatible `GpuKernel(dynamic_strategy_v1)` shapes. That is closer to compiling an optimization contract than to hand-writing a static ConfigSpace.

## Why the rejected alternatives were rejected

A **new optimizer** is rejected because the literature is already crowded, and the repo’s strongest distinctive asset is not a new search rule but the structure already encoded in request shaping and runtime policy.

A **compiler-first** label is rejected because the repo does not yet have a first-class canonical IR, multi-target lowering, or semantic-preservation checks. Until those exist, “compiler” overstates the contribution.

A **fidelity-first** thesis is rejected because `VdpsConfig` is real stage structure, but the current repo evidence only justifies treating it as optional hint metadata. It does not yet justify a general claim that the extractor discovers trustworthy fidelity ladders. BOHB already occupies the standard multi-fidelity story, so any fidelity claim here needs stronger evidence than the repo currently provides.

A **CUDA-centered** thesis is rejected because the uploaded prompts explicitly treat CUDA as substrate, and the repo’s own route model already spans CPU fallback, hybrid GPU routing, and exact GPU routing. If the idea stops making sense when CUDA is replaced, it is the wrong research target.

A **repo-wide universal extractor** is rejected as a first implementation slice. The right initial scope is the `DoubleMa` family, `DynamicDoubleMa`, and the route-conditional overlays already funneled through the `double_ma` path.

## Deferred directions

Deferred, not adopted:

- A **true compiler** with inferred cost/fidelity semantics and optimizer-specific lowering rules
- A **benchmark/platform** fallback, if the extracted IR improves conformance and portability but does not materially improve optimization outcomes beyond reducing invalid trials
- A broader **stage-hint / fidelity** extension using `vdps` or validation-coupled costs, but only after ranking preservation is demonstrated

## Role of the custom CUDA implementation

**Substrate only.**

The IR should still matter on CPU-only or hybrid routes because it is extracting parameter legality, activation structure, route capability class, and output admissibility. The repo already models CPU fallback and ADX-driven hybrid routing, so the conceptual contribution is not tied to one CUDA kernel.

## Minimum prototype

1. Build a canonical IR with:
   - parameter nodes and domains
   - conditional activation edges
   - forbidden combinations and hard constraints
   - backend/route capability annotations
   - objective and metric-bundle annotations
   - output-contract legality
   - optional stage-hint metadata

2. Extract only from authoritative code paths:
   - `web/src/services/optimizeRequestBuilder.ts`
   - `web/src/utils/strategyOptimizationCapabilities.ts`
   - `crates/app_core/src/composite_profile_route.rs`
   - `crates/app_core/src/double_ma.rs`

3. First supported scope:
   - `DoubleMa`
   - `DynamicDoubleMa` exact-compatible and CPU-fallback shapes
   - `CompositeTrend`
   - `BreakoutMomentum`
   - `RegimeHybrid`
   - one or more of `PIT`, `VFCT`, `SET`, `MTS`, `ORC`

4. Lower to at least two existing optimizer backends:
   - `ConfigSpace` + `SMAC3`
   - `Optuna` or `Hyperopt`
   - OSS Vizier as optional third target

5. Add a semantic checker that measures:
   - legality agreement with current request-builder/backend behavior
   - route-preview agreement
   - output-contract agreement
   - metric-surface agreement where applicable

6. Keep stage hints non-committal in v1:
   - carry `vdps` metadata
   - do not claim discovered fidelity ladders unless a later experiment validates them

## Mandatory baseline matrix

Per the final-consolidation rubric, the baseline matrix must include exact, classical, modern, simple, and ablation baselines, and must report wall-clock to target, simple regret, anytime incumbent quality, robustness across seeds, and performance across multiple strategy families.

Concretely, use:

1. **Exact baseline where available**
   - current exact GPU brute-force route on `DoubleMa`
   - exact CPU route on smaller tractable subsets for correctness/reference

2. **Classical heuristic baseline**
   - one typed evolutionary baseline over the same exported IR, such as regularized evolution or a simple mutation/crossover search that respects conditional activation

3. **Modern mixed-space black-box baselines**
   - `SMAC3` with `ConfigSpace`
   - `Optuna` TPE
   - optionally `Hyperopt` TPE or OSS Vizier

4. **Simple pragmatic baselines**
   - random search
   - Sobol or uniform batched sampling

5. **Ablation baselines**
   - extracted IR **without** constraints
   - extracted IR **without** route/output-contract annotations
   - extracted IR **without** dynamic preview/fallback information
   - expert hand-authored space vs extracted IR for the same optimizer

In addition to the prompt’s required metrics, add:

- invalid-trial rate
- time to first feasible incumbent
- legality disagreement rate
- route-prediction disagreement rate
- optimizer portability from one lowered backend to another

## Mandatory benchmark slate

Per the uploaded rubric, the benchmark slate must extend beyond `double_ma` and include structured exact, broader mixed-family, constraint-bearing/route-conditional, and validation- or stage-coupled cases.

Use this internal slate:

1. **Structured exact lattice**
   - `DoubleMa` exact grid anchor

2. **Dynamic extracted case**
   - one exact-compatible `DynamicDoubleMa GpuKernel(dynamic_strategy_v1)` shape
   - one previewed CPU-fallback `DynamicDoubleMa` shape

3. **Broader mixed-family route cases**
   - `CompositeTrend` with ADX off and on
   - `BreakoutMomentum` with ADX off and on
   - `RegimeHybrid`

4. **Constraint-bearing / field-gated cases**
   - `PIT` or `VFCT`
   - plus one of `SET`, `MTS`, or `ORC`

   The request builder already makes these materially different through candle-field requirements and route legality.

5. **Output-contract case**
   - one exact-kernel run exercising extra shortlist heads
   - one shortlist confidence/union/slice case
   - one explicit `metric_contract` override case

6. **Stage-hint case**
   - one `GpuVdps` or related staged case, but only as an auxiliary representation test, not a headline fidelity claim

## Direction-specific kill test

End this direction early if any of the following happens:

1. The extractor cannot reach near-perfect agreement with current legality, route preview, and output-contract behavior without heavy manual per-strategy patching.

2. Across at least two optimizers and multiple strategy families, the extracted IR does not outperform expert hand-authored spaces on anything beyond trivial invalid-trial reduction.

3. The gains appear only on exact `GpuKernel` cases and collapse on CPU-fallback, hybrid, or `DynamicDoubleMa` previewed shapes, showing that the idea is really just a wrapper around one in-house route.

4. The optional stage hints fail to preserve useful ordering or regret behavior, in which case the fidelity/cost extension stays deferred and is removed from the claim.

If that happens, demote the work to a benchmark/interface memo. The prompt explicitly allows “search-space compilation matters more than search policy” as a publishable negative result, but not an overclaimed compiler thesis.

## Final verdict

**Reopen implementation**, but only for a **narrow IR prototype**.

This direction clears the go/no-go bar on problem clarity, generalization, evaluation clarity, CUDA independence, and failure visibility. The remaining question is novelty delta, and that is exactly why the prototype must be conformance-checked and compared against expert hand-authored spaces plus existing optimizers. If that prototype wins only on legality cleanup, the right end state is a benchmark/interface artifact. If it improves multiple optimizers similarly, that is still a useful result and a better research outcome than inventing another optimizer.

Operationally, reopen work only as:

**Phase 0**
- IR schema
- extraction from the four authoritative surfaces
- semantic checker
- two lowering targets

Do **not** reopen:
- a new optimizer lane
- a fidelity-first lane
- or a full compiler lane

The adopted direction is therefore:

**Optimization-contract IR / search-space normalizer with semantic conformance checking for the `DoubleMa` family and route-aware overlays.**

## Source anchors

### Uploaded prompt bundle
- `01_first_pass_prompt.md`
- `02_second_pass_prompt.md`
- `03_final_consolidation_prompt.md`
- `vectorgrid_broader_cs_prompt_bundle_index_2026-04-08.md`

### Repo files referenced
- `AGENTS.md`
- `REPO_MAP.md`
- `README.md`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/strategy_presets.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

### External reference anchors
- ConfigSpace guide: https://automl.github.io/ConfigSpace/latest/guide/
- SMAC3 package overview: https://automl.github.io/SMAC3/v2.0.1/2_package_overview.html
- Optuna configuration tutorial: https://optuna.readthedocs.io/en/stable/tutorial/10_key_features/002_configurations.html
- BOHB paper page: https://proceedings.mlr.press/v80/falkner18a.html
