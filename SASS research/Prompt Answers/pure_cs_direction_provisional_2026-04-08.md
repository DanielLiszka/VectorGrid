# Provisional Pure-CS Research Direction

Date: 2026-04-08

Status: provisional recommendation based on the completed answers for Questions 1 through 8. The two math-focused prompt tracks are still in progress and may still replace this recommendation with a stronger theorem or negative-result direction.

## Adopted direction

The strongest **pure CS** direction currently available in this repo is:

**an optimization-contract IR / search-space normalizer with semantic conformance checking**

This should be framed as a systems / PL / optimization-infrastructure problem, not as:

- a new optimizer paper,
- a benchmark-only paper,
- a multi-fidelity methods paper,
- or a CUDA/kernel paper.

## Exact problem statement

For a strategy family `s` and evaluation context `d`, automatically extract a canonical optimizer-facing IR

`I(s, d) = (Theta, A, C, O, R, X, H)`

where:

- `Theta`: parameters and admissible domains
- `A`: conditional activation structure
- `C`: hard constraints and forbidden configurations
- `O`: objective and metric bundles
- `R`: backend / route capability classes
- `X`: output-contract legality
- `H`: optional stage or cost hints carried as metadata

Then:

1. lower that IR into multiple existing optimizer backends,
2. check semantic agreement against the repo’s current executable behavior,
3. and measure whether the extracted IR improves legality preservation, invalid-trial rate, portability, and optimization efficiency relative to hand-authored spaces.

## Why this is the strongest pure-CS direction

This direction survives the stricter “pure CS, not quant-finance-specific” filter better than the others because:

1. The core object is **not trading**.
   The domain only supplies a rich executable contract surface.

2. The contribution can be stated without CUDA.
   CPU-only, hybrid, and GPU-backed routes all still matter.

3. The research questions are recognizably CS:
   - IR design
   - extraction from executable policy surfaces
   - semantic conformance checking
   - lowering to multiple solver backends
   - portability across optimization runtimes

4. The repo already contains the raw contract structure in authoritative code paths rather than only in prose.

## Why the other directions are weaker under a pure-CS bar

## Not the lead: multi-fidelity trustworthiness

This remains the strongest broader AutoML/evaluation direction, but under a strict pure-CS bar it still reads as more application/evaluation-specific than the IR/conformance direction.

## Not the lead: optimizer portfolios / solver selection

This is still plausible, but it depends too much on workload-specific complementarity and benchmark design. It is more credible as a phase-2 consumer of the extracted IR and normalized contract surface.

## Not the lead: standalone batch-synchronous optimizer

Too crowded and too dependent on the repo’s execution substrate to be the cleanest pure-CS story.

## Not the lead: broad exact pruning

Only a stripped-down theorem-first or negative-result version survives, and that is still less robust than the IR/conformance direction unless the in-progress math prompts uncover a much stronger formal angle.

## Repo grounding

This direction is grounded in code that already encodes optimizer-facing structure:

- `web/src/services/optimizeRequestBuilder.ts`
  - request legality
  - candle-field requirements
  - shortlist / metric-contract / export gating
  - route-dependent restrictions
- `web/src/utils/strategyOptimizationCapabilities.ts`
  - capability summaries
  - route previews
  - `DynamicDoubleMa` preview behavior and fallback messaging
- `crates/app_core/src/composite_profile_route.rs`
  - route-conditional GPU / hybrid / CPU targeting
- `crates/app_core/src/double_ma.rs`
  - request structure
  - constraints
  - score modes
  - metric contracts
  - staged-search metadata such as `VdpsConfig`

These surfaces already behave like pieces of an optimization contract, but they are scattered and not yet normalized into one canonical IR.

## Minimum prototype

The minimum serious prototype is:

1. Define a canonical IR schema covering:
   - parameters
   - conditional activation
   - hard constraints
   - route/backend capability annotations
   - output legality
   - objective/metric bundles
   - optional stage/cost hints

2. Extract from authoritative surfaces only:
   - `web/src/services/optimizeRequestBuilder.ts`
   - `web/src/utils/strategyOptimizationCapabilities.ts`
   - `crates/app_core/src/composite_profile_route.rs`
   - `crates/app_core/src/double_ma.rs`

3. Support a narrow first slice:
   - `DoubleMa`
   - `DynamicDoubleMa`
   - `CompositeTrend`
   - `BreakoutMomentum`
   - `RegimeHybrid`

4. Lower to at least two existing optimizer ecosystems.

5. Add a semantic checker that measures:
   - legality agreement
   - route-preview agreement
   - output-contract agreement
   - invalid-trial reduction
   - optimizer portability

## What would make this research-grade

It is not enough to merely serialize existing rules.

The direction becomes research-grade only if it shows at least one of these:

1. automatic extraction can replace substantial hand-authored search-space work,
2. semantic conformance can be checked systematically and exposed as a first-class artifact,
3. the extracted IR materially improves optimizer portability across backends,
4. or the extracted structure improves multiple optimizers similarly, showing that structure extraction matters more than any one search policy.

## Kill test

Close or demote this direction if any of the following happens:

1. The extractor cannot reach near-perfect agreement with current legality and route behavior without heavy manual per-strategy patching.
2. The extracted IR only produces trivial cleanup wins and no meaningful portability or optimization benefits.
3. The gains appear only on one exact GPU route and collapse on CPU-fallback or hybrid cases.
4. The work still reads primarily as internal engineering cleanup rather than a reusable optimization-contract extraction problem.

If that happens, demote the result to:

- a useful internal architecture/tooling memo,
- not a paper lead.

## Relationship to the in-progress math prompts

This recommendation is provisional.

If the still-running math prompts produce a stronger result in one of these forms, that result should be reconsidered ahead of this direction:

- a real theorem-worthy restricted certified-elimination result,
- a rigorous impossibility / negative-result formulation,
- or another mathematically explicit claim that is cleaner and more general than the IR/conformance path.

Until then, the IR/conformance direction is the strongest surviving **pure CS** lead.

## Current verdict

**Reopen only a narrow design/prototype phase for optimization-contract IR extraction and semantic conformance checking.**

Do **not** reopen:

- a universal optimizer lane,
- a standalone benchmark lane,
- a broad multi-fidelity methods lane,
- or more CUDA/SASS work as the pure-CS lead.
