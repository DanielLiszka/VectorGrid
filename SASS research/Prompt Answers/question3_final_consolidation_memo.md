## Final consolidation memo — Question 3

This closes Question 3 in the broader VectorGrid CS prompt bundle and follows the first-pass, second-pass, and final-consolidation instructions to choose one outcome, name the baseline matrix, and specify a kill test. fileciteturn5file3 fileciteturn5file18 fileciteturn5file0 fileciteturn5file1 fileciteturn5file2

### Adopted direction

Adopt **exact-anchor-aware budgeted portfolio scheduling** for VectorGrid. Do **not** pursue “one universal optimizer” as the thesis. The surviving research question is whether a small, cost-aware scheduler can decide when exact search should run, when an approximate optimizer is justified, and how to allocate a fixed wall-clock budget across a tiny optimizer portfolio with at most one switch. This is best framed as **systems-aware portfolio scheduling / algorithm selection**, not as a new optimizer family and not as a generic meta-learning paper. It is more publishable than a universal-optimizer story because generic optimizer-wizard framing already collides with ASlib, AutoFolio, ABBO, and newer sequential-portfolio work, while VectorGrid contributes a rarer setting with exact anchors, route-conditional GPU behavior, and mixed strategy families. ([arxiv.org](https://arxiv.org/abs/1506.02465?utm_source=chatgpt.com))

### Exact problem statement

Let a workload be  
\[
\tau = (s, d, X, r, o, B)
\]
where \(s\) is strategy/profile kind, \(d\) is dataset, \(X\) is the mixed/conditional search space, \(r\) is the route/backend preview, \(o\) is the requested output/metric contract, and \(B\) is a wall-clock budget. Let portfolio \(P\) contain exact and approximate search members. The policy \(\pi\) receives free static features \(\phi_0(\tau)\) before search, may optionally consume a tiny probe trace \(\phi_p(\tau)\), and outputs either: (a) exact `Grid`, or (b) a short schedule \(\sigma=[(A_i,b_i)]\) with at most one switch, maximizing anytime incumbent quality and minimizing simple regret under total wall-clock \(B\). Exact finalist semantics and current metric-contract behavior remain authoritative. fileciteturn5file12 fileciteturn5file6

This formulation is real in this repo because VectorGrid is explicitly positioned around exact-grid optimization, already exposes `Grid`, `CoarseToFine`, and `GpuVdps`, and supports multiple resolved backends (`CpuOnly`, `GpuOnly`, `GpuHybridKernel`, `GpuKernel`) rather than one homogeneous search surface. fileciteturn5file12 fileciteturn5file6

### Why this still survives prior-work collision

A generic “learn features and pick an optimizer” paper would be too saturated. ASlib standardized per-instance algorithm selection; AutoFolio already uses cheap instance features, probing, and pre-solving schedules; ELA-based selector work already covers continuous black-box optimization; ABBO is already a broad algorithm-selection wizard; PS-AAS studies portfolio construction for automated selection; recent work on computationally expensive fixed-budget BBO studies portfolio construction directly; and 2026 sequential-portfolio work again shows portfolios beating single-algorithm baselines. ([arxiv.org](https://arxiv.org/abs/1506.02465?utm_source=chatgpt.com))

The novelty gap that remains is narrower: **exact-vs-approx gating plus wall-clock-aware scheduling on mixed, route-conditional, batched strategy optimization**. VectorGrid is unusual because one family has a strong exact GPU anchor while neighboring workloads share infrastructure but differ in backend eligibility, hybrid behavior, metric contracts, and exact-route-only controls. That makes “should I use a heuristic at all?” a first-class decision, not a minor implementation choice. fileciteturn5file17 fileciteturn5file5 fileciteturn5file4 fileciteturn5file6

Recent literature also raises the bar for how this must be evaluated. A 2025 generalization study reports that, under out-of-distribution evaluation, feature-based AS models did not beat the Single Best Solver, and a 2025 benchmarking paper shows that weak evaluation methodology can create overly optimistic conclusions. So the claim has to be narrow and hard-split: leave-family-out, leave-dataset-out, and wall-clock-based. ([arxiv.org](https://arxiv.org/abs/2501.17663))

### Why this repo makes the question real

The capability surface exposes **24 concrete optimization kinds** plus `Unknown`, and **11** of those concrete kinds are already grouped into the DoubleMa-family surface. That is enough heterogeneity to justify selection at the workload level rather than pretending one optimizer should dominate all cases. fileciteturn5file17

That heterogeneity is structural. `CompositeTrend` routes to `GpuKernel` with ADX off and `GpuHybridKernel` with ADX on; `BreakoutMomentum` follows the same split; `RegimeHybrid` currently targets `GpuHybridKernel`; and `AGENTS.md` explicitly says these profiles use the `double_ma` product path, not a separate `CompositeCpu` optimizer. fileciteturn5file5 fileciteturn5file4

`DynamicDoubleMa` makes the setting even richer: it previews constrained `strategy_ir` shapes, enables extra shortlist objectives and metric-contract overrides only on exact-compatible `dynamic_strategy_v1` `GpuKernel` shapes, and can fall back to CPU for constrained shapes that miss that exact surface. That is exactly the kind of free route/capability signal an execution-aware scheduler should use. fileciteturn5file17 fileciteturn5file6

### Cheapest signals and when selection should happen

The selector should start with **free static features** only: strategy/profile kind, requested and resolved backend, route preview, exact-kernel availability, ADX/hybrid flags, bars, total pair count/variant count, candle-field requirements, objective/score mode, export/metric-contract/shortlist controls, and whether the task has an exact lattice anchor. These signals are already exposed by the capability and routing layers. fileciteturn5file17 fileciteturn5file5 fileciteturn5file13

Only if that static gate says “approximation might matter” should the system spend a tiny probe budget on **cheap online signals**: first-incumbent improvement slope, early invalid/constraint-failure rate, realized candidates-per-second, and early score dispersion. That matches the cheap-feature / probing pattern in AutoFolio and the newer probing-trajectory line in black-box optimization. ([ml.informatik.uni-freiburg.de](https://ml.informatik.uni-freiburg.de/wp-content/uploads/papers/15-JAIR-Autofolio.pdf))

So selection should happen **once per run**, with family and dataset only providing priors. The v1 controller should be: static gate, optional short probe, then commit with at most one switch. Full online RL switching is deferred. Existing online portfolio and dynamic-selection work makes that too saturated and too overhead-sensitive for the first implementation. ([pasky.or.cz](https://pasky.or.cz/sci/cocopf-opt13/optpf-extended.pdf))

### Why the rejected alternatives were rejected

**Rejected:** one universal optimizer. ABBO and newer sequential portfolios already make that story crowded, and VectorGrid’s own exact, hybrid, and CPU-fallback regimes make universal dominance less plausible than usual. ([arxiv.org](https://arxiv.org/abs/2010.04542)) fileciteturn5file12 fileciteturn5file5

**Rejected:** a plain offline feature-based selector. Without the exact-vs-approx gate and scheduling angle, the work collapses into AutoFolio/ASlib/ELA transfer. ([arxiv.org](https://arxiv.org/abs/1506.02465?utm_source=chatgpt.com))

**Deferred:** full online switching / RL control. It is a possible follow-on, but not a defensible v1 claim. ([pasky.or.cz](https://pasky.or.cz/sci/cocopf-opt13/optpf-extended.pdf))

**Deferred fallback:** benchmark/platform paper. If the scheduler fails the kill test, the remaining artifact is still useful as a benchmark showing where exact baselines dominate and where simple route policies are already sufficient. fileciteturn5file2

### Role of the custom CUDA implementation

CUDA is **substrate only**. The claim must still make sense if the evaluator is replaced by another fast backend. CUDA matters mainly because different portfolio members exploit batched execution, residency, and route previews differently; it is not the conceptual contribution. The repo’s own retained research artifacts already push in that direction: the SASS/CUPTI line is classified as useful mainly for research/tooling, the retained Phase 2 findings are framed as an attribution artifact rather than a new optimizer thesis, and the xlarge cache-reuse probe was a NO-GO despite better cache metrics because wall time worsened. fileciteturn5file9 fileciteturn5file10 fileciteturn5file16

### Minimum prototype needed

Implement only a **two-stage scheduler**:

1. Portfolio members: exact `Grid`, `CoarseToFine`, and `GpuVdps`, plus one classical heuristic baseline, one modern mixed-space BO/HPO-style baseline, and random/Sobol. Exact `Grid` stays mandatory wherever available. fileciteturn5file6 fileciteturn5file2

2. Policy: a small cost-sensitive tree/GBDT or ranking model that first predicts exact-vs-approx, then chooses among a tiny menu of schedules. No RL in v1.

3. Probe: one short pre-solve/probe window and at most one switch.

4. Logging: static features, route preview, probe traces, chosen schedule, anytime incumbent curve, wall-clock, final regret, and selector overhead.

5. Controls: hand-written family/route policy, Single Best Solver, Virtual Best Solver, no-probe ablation, no-route-feature ablation, no-switch ablation. Benchmarking literature makes these controls non-optional. ([arxiv.org](https://arxiv.org/pdf/2109.08377))

### Mandatory baseline matrix

Use the prompt-pack baseline categories directly: exact GPU `double_ma` brute-force route and CPU exact reference where feasible; one classical heuristic or bandit-style baseline; one modern mixed-space black-box baseline; random or Sobol; and an ablation with one key idea removed. Add three selector-specific controls: hand-written family/route policy, Single Best Solver, and Virtual Best Solver. Measure wall-clock to target, simple regret, anytime incumbent quality, robustness across seeds, and selector overhead. fileciteturn5file2

For the modern baseline family, include at least one BO/TPE/SMAC-like method. Low-budget comparisons still show BO-style methods can be strong at small budgets, even though they often lose ground as dimension and budget increase, which is exactly why a portfolio comparison is preferable to a single-family story. ([arxiv.org](https://arxiv.org/html/2310.00077v3))

### Mandatory benchmark slate

Use this minimum internal slate:

1. `DoubleMa` as the exact anchor. fileciteturn5file12 fileciteturn5file2  
2. `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid`, with both ADX-off and ADX-on shapes where relevant. fileciteturn5file5 fileciteturn5file4  
3. `Vfct` plus at least one of `Pit`, `Set`, `Mts`, or `Orc` as route-conditional / constraint-bearing cases. The preset helpers already make these cases easy to instantiate. fileciteturn5file8 fileciteturn5file17  
4. `DynamicDoubleMa` on both exact-compatible `GpuKernel(dynamic_strategy_v1)` surfaces and CPU-fallback surfaces. fileciteturn5file17 fileciteturn5file6  
5. One validation-coupled case passed through the current validation pipeline so total workflow cost, not just optimization cost, is measured. The repo architecture already treats validation as a separate GPU-capable rerun stage. fileciteturn5file4 fileciteturn5file13  
6. Small, medium, and xlarge workload tiers, including at least one shortlist-heavy exact surface. Repo measurement history already shows workload size can change route behavior materially. fileciteturn5file10 fileciteturn5file16

### Direction-specific kill test

Stop the direction early if, under leave-family-out and leave-dataset-out evaluation, a hand-written family/route policy recovers nearly all gains; the Single Best Solver is within about 5% of the learned scheduler on wall-clock regret; exact `Grid` dominates almost everywhere important; the probe overhead erases the benefit; or removing route/backend features collapses performance. If that happens, close the algorithm-selection claim and keep only the benchmark/platform memo. Recent work is explicit that weak evaluation protocol can inflate selector performance. ([arxiv.org](https://arxiv.org/pdf/2505.07750))

### Final verdict

**Reopen implementation**, but only for the narrow **exact-anchor-aware budgeted portfolio scheduler** above. This direction is stronger than a universal optimizer for VectorGrid, and it should be promoted over the batch-synchronous universal-optimizer idea because it uses the repo’s actual heterogeneity instead of averaging it away. Do **not** reopen a generic selector, a full online RL controller, or a CUDA-first story. If the minimal gate-plus-one-switch prototype fails the kill test, immediately downgrade the line to a benchmark/platform memo and stop there. fileciteturn5file0 fileciteturn5file1 fileciteturn5file2 ([arxiv.org](https://arxiv.org/abs/2601.16896))
