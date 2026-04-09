# Question 1 — final consolidation memo

## Adopted direction

Adopt **cost-aware fidelity and solver selection under fixed wall-clock budget** as the research target, and treat **VectorGrid as the benchmark/problem environment for that question**, not as the basis for a new “universal optimizer beats brute force” thesis. In this framing, exact GPU grid search on `DoubleMa` is one arm in the decision problem, `GpuVdps`-style approximate search is another, and standard mixed-space black-box optimizers are additional arms; the publishable unit is a measurement-led benchmark/problem memo showing **when exact, approximate, or generic search is the right tool under authoritative full-fidelity validation**. Final verdict: **design-only follow-up**, not a fresh optimizer implementation campaign. fileciteturn7file7 fileciteturn7file2 fileciteturn7file1 ([datasets-benchmarks-proceedings.neurips.cc](https://datasets-benchmarks-proceedings.neurips.cc/paper/2021/file/93db85ed909c13838ff95ccfa94cebd9-Paper-round2.pdf))

## Exact problem statement

For each task \(t\) — defined by a strategy family, dataset, objective/score mode, constraints, and route policy — let \(X_t\) be the configuration space, \(A_t\) the available solver set, and \(P_t\) the set of **validated** cheap proxies/fidelities. Let \(f_t(x)\) be the **authoritative full-fidelity score**, obtained only after the repo’s full product semantics are honored. A policy chooses a sequence of queries \(q_i=(x_i,a_i,p_i)\) with wall-clock cost \(c_t(q_i)\), subject to \(\sum_i c_t(q_i)\le B\), and must return an incumbent \(\hat x_B\) chosen by full-fidelity score. The primary metric is **full-fidelity simple regret** under budget; the secondary metric is **anytime incumbent quality**. In this repo, the natural candidate solver arms already exist: exact `GpuKernel` on structured `DoubleMa`, approximate `GpuVdps` with rung telemetry and quality checks, and route-dependent CPU/GPU alternatives; the profiling harness also already exposes evaluation-window controls and multiple strategy/workload surfaces. fileciteturn7file2 fileciteturn7file1 fileciteturn7file0 ([datasets-benchmarks-proceedings.neurips.cc](https://datasets-benchmarks-proceedings.neurips.cc/paper/2021/file/93db85ed909c13838ff95ccfa94cebd9-Paper-round2.pdf))

## Why this is still plausible after prior work

A **new mixed-space optimizer** is not the right claim. Conditional and hierarchical spaces are already well-covered by SMAC/CASH-style work, while hybrid mixed-variable BO is already represented by methods such as HyBO and MiVaBO. Likewise, multi-fidelity HPO is already a mature lane with BOHB, DEHB, DyHPO, cost-sensitive MFBO, and recent reviews. A paper that says only “we optimize mixed, conditional, budgeted black-box spaces with a fast backend” would collide directly with existing literature. ([jmlr.org](https://www.jmlr.org/papers/volume23/21-0888/21-0888.pdf))

What survives that literature pass is narrower: this repo combines **an exact structured lattice**, **an in-repo approximate ladder**, **heterogeneous strategy families with route-conditional execution**, and **full-fidelity validation authority** in one environment. That makes the interesting question less “which optimizer is best in general?” and more “when should a system spend budget on exact search, when is a validated cheap proxy trustworthy, and when is a generic optimizer the wrong tool?” That is closer to an AutoML/systems/benchmark contribution than to a new algorithm paper, and it is where VectorGrid has a real gap relative to current benchmark suites. fileciteturn7file7 fileciteturn7file0 fileciteturn7file8 fileciteturn7file2 ([datasets-benchmarks-proceedings.neurips.cc](https://datasets-benchmarks-proceedings.neurips.cc/paper/2021/file/93db85ed909c13838ff95ccfa94cebd9-Paper-round2.pdf))

## Deferred directions

Two directions stay alive, but only as follow-ons.

First, a **learned selector or portfolio** over exact, approximate, and generic optimizers is deferred until the cheap-fidelity audit passes. Algorithm selection and algorithm configuration are already rich literatures, so there is no reason to jump there before the benchmark/problem itself is credible. ([arxiv.org](https://arxiv.org/abs/2406.06629?utm_source=chatgpt.com))

Second, **cross-strategy transfer on heterogeneous search spaces** is deferred. That lane is no longer empty; MPHD and related transfer work already target BO across heterogeneous domains. Unless VectorGrid can expose stronger task descriptors or route-aware context than current benchmark suites, transfer should not be the lead claim. ([openreview.net](https://openreview.net/notes/edits/attachment?id=EXiokwjgSL&name=pdf))

## Rejected directions

Reject **“beat brute force on `DoubleMa`”** as the thesis. On the repo’s own terms, `DoubleMa` already has a mature exact GPU route, and recent retained research work around that route has already shifted toward **measurement and attribution** rather than “one more broad optimization story.” fileciteturn7file7 fileciteturn7file10 fileciteturn7file11

Reject **mixed-space scalability alone** as the primary claim. SMAC3, HyBO, and MiVaBO already occupy that space too well for a repo-specific workload to reopen it without a sharper theoretical or structural gap. ([jmlr.org](https://www.jmlr.org/papers/volume23/21-0888/21-0888.pdf))

Reject **constrained or multi-objective quality as the main paper**. HyperMapper 2.0 already covers multi-objective, mixed categorical/ordinal variables, and unknown feasibility in systems-style black-box optimization, while this repo’s own README still says `CompositeMo` is CPU-bound. ([arxiv.org](https://arxiv.org/pdf/1810.05236)) fileciteturn7file7

Reject **exact pruning / certified elimination now**. The retained repo artifact is currently about static-plus-dynamic attribution for irregular exact CUDA kernels, not about proving elimination structure. Reopening pruning would be a different research program. fileciteturn7file10 fileciteturn7file11 fileciteturn7file9

## Role of the custom CUDA implementation

The custom CUDA implementation is a **systems enabler and execution substrate**, not the main novelty claim. It matters because it makes the exact baseline cheap enough, and the repeated full-fidelity reruns numerous enough, to support a serious benchmark and solver-selection study. But the conceptual contribution survives if CUDA is replaced by another sufficiently fast backend. If removing CUDA would erase the claim entirely, that is the warning sign that the work has drifted back to implementation novelty only. fileciteturn7file7 fileciteturn7file1 fileciteturn7file2

## Minimum prototype

1. Define a **task manifest** over repo-native surfaces: `DoubleMa` as the exact anchor; `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid` as heterogeneous family cases; one ADX-driven route-conditional case; and one optimization-plus-validation case. The repo already exposes those families and route policies. fileciteturn7file0 fileciteturn7file7 fileciteturn7file8

2. Define a **query catalog** rather than a new optimizer: exact `GpuKernel` where available; `GpuVdps` runged approximate search; at least one generic mixed-space optimizer wrapper; and optional evaluation-window proxies. By contrast, `metric_contract`, `score_mode`, shortlist confidence/union/slice, and similar switches should **not** be promoted to fidelities until proven rank-preserving, because they alter metric surfaces or diagnostics rather than offering clean cheaper approximations. That last point is an inference from the request surface and exact-route semantics, not a claim the repo has already proven. fileciteturn7file2 fileciteturn7file3 fileciteturn7file9

3. Implement a **fidelity-audit harness** before any learned selector: measure rank correlation, top-k overlap, and best-score regret of each cheap route against full fidelity. The repo already exposes `GpuVdps` telemetry for overlap/regret checks, which is the right starting point. fileciteturn7file2

4. Only if the audit passes, add a **small selector baseline**: first rule-based, then optionally contextual. Do not start with a new BO or RL optimizer.

## Mandatory baseline matrix

The minimum credible matrix is:

- **Exact baseline where available:** current exact GPU `DoubleMa`, plus exact CPU as a correctness/scaling reference. fileciteturn7file7
- **Simple pragmatic baseline:** random and Sobol. ([jmlr.org](https://www.jmlr.org/papers/volume23/21-0888/21-0888.pdf))
- **Classical heuristic baseline:** Differential Evolution or an equivalent evolutionary search baseline. ([ijcai.org](https://www.ijcai.org/proceedings/2021/296?utm_source=chatgpt.com))
- **Modern structured mixed-space baseline:** SMAC3, plus one of HyBO or MiVaBO if the final task encoding preserves meaningful continuous variables. ([jmlr.org](https://www.jmlr.org/papers/volume23/21-0888/21-0888.pdf))
- **Modern multi-fidelity baseline:** BOHB and one of DEHB or DyHPO. ([proceedings.mlr.press](https://proceedings.mlr.press/v80/falkner18a/falkner18a.pdf))
- **Ablation baseline:** exact-only, approximate-only, and selector-without-fidelity-audit.

That matrix is the minimum because current HPO/BBO practice already treats mixed-space, multi-fidelity, and benchmarked comparison as standard, not optional. ([datasets-benchmarks-proceedings.neurips.cc](https://datasets-benchmarks-proceedings.neurips.cc/paper/2021/file/93db85ed909c13838ff95ccfa94cebd9-Paper-round2.pdf))

## Mandatory benchmark slate

The internal slate should be:

- **Structured exact lattice:** `DoubleMa`, including at least one exact-friendly surface where the brute-force GPU route is authoritative. fileciteturn7file7
- **Broader mixed-family profile:** at least two of `CompositeTrend`, `BreakoutMomentum`, `RegimeHybrid`. fileciteturn7file0 fileciteturn7file7
- **Constraint/route-conditional case:** ADX on/off cases that change route policy across `GpuKernel`, `GpuHybridKernel`, and fallback behavior. fileciteturn7file0
- **Validation-coupled case:** one case where the optimization winner must survive a distinct validation-stage rerun and cost model, because the repo explicitly separates optimization and validation execution. fileciteturn7file0 fileciteturn7file8

`CompositeMo` should stay out of the minimum slate for Question 1 because it would drag the study into a weaker multi-objective-first framing while the repo still treats it as CPU-bound. fileciteturn7file7

## Kill test

End the direction early if either condition holds.

First, **no candidate cheap route is trustworthy**: if `GpuVdps` rungs, evaluation-window proxies, or other low-cost queries fail to preserve ranking/top-k structure against full-fidelity outcomes on `DoubleMa` plus at least two non-`DoubleMa` families, the solver-selection story collapses. In that case, the publishable result is the negative finding that “multi-fidelity is not trustworthy enough on this workload.” fileciteturn7file2

Second, **selection adds nothing over fixed baselines**: if exact-only, SMAC3, BOHB, or a fixed mixed-space baseline matches or beats any selector on full-fidelity simple regret and anytime incumbent quality, then there is no algorithmic contribution to reopen. The right landing spot then is a benchmark/problem memo showing that exact baselines dominate more often than expected, not a new optimizer paper. ([jmlr.org](https://www.jmlr.org/papers/volume23/21-0888/21-0888.pdf))

## Final verdict

**Design-only follow-up.** Do not reopen broad implementation for a new optimizer. Reopen only a **bounded measurement harness** that audits candidate fidelities and compares exact, approximate, and generic solvers under one full-fidelity budgeted protocol. If that harness passes, the next implementation branch can be a small selector/portfolio study. If it fails, close Question 1 with a benchmark/negative-result memo. That is also consistent with the repo’s current research posture: the retained `double_crossover` work was explicitly reframed as a research artifact, the xlarge cache probe was a clean no-go despite better cache metrics, the TM-native VWMA work was retained only as a bounded pilot, and the CUPTI SASS branch was classified as useful for research/tooling rather than product optimization. fileciteturn7file10 fileciteturn7file11 fileciteturn7file5 fileciteturn7file6 fileciteturn7file4
