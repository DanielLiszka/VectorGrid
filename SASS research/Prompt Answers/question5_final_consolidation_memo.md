# Question 5 Final Consolidation Memo
## Exact pruning / certified elimination

This memo follows the Question 5 first-pass, second-pass, and final-consolidation bundle instructions.

## Adopted direction

Do **not** reopen implementation for broad exact pruning in this repo.

The only version of this direction worth keeping alive is a **design-only, falsification-oriented study of certified local elimination on a stripped-down `DoubleMa` lattice**. The broader VectorGrid workload is too thresholded, path-dependent, and route-conditional to support a convincing repo-wide exact-pruning thesis, while the nearest literature already covers generic deterministic partition search, optimistic tree search, combinatorial BO, and MILP-backed black-box search.

As a repo-level research target, this is **weaker than the multi-fidelity and portfolio directions** and survives only because the bare `fast_len < slow_len` `DoubleMa` lattice gives one clean place to test whether any admissible bound exists at all. The prompt pack’s own post-literature priority order already demotes exact pruning on realism and publishability grounds.

## Exact problem statement

The live problem is:

Given a fixed candle series, fixed MA kinds, fixed execution semantics, and an exact score produced by the current backtest semantics, determine whether there exists an admissible bound \(U(R)\) over a region \(R \subseteq \{(f,s): f < s\}\) of the `DoubleMa` lattice such that pruning any region with \(U(R)\) below the incumbent cannot remove the true exact winner or exact top-k.

That is narrower than “optimize strategies on GPU.” The current optimizer support crate still defines plain exhaustive `grid_search`, and the product framing remains exact-grid optimization with a mature exact `DoubleMa` GPU path.

The broader application is not the target of this question. `StrategyConfig` includes stop-loss / take-profit, Kaufman-stop exits, regime-hybrid behavior, breakout momentum, volume-flow confirmation, SET, MTS, ORC, PIT, cooldown, and signal persistence, and `double_ma.rs` implements those through thresholded regime logic plus stateful execution overlays. That is exactly the kind of surface that makes tight admissible bounds difficult.

## Why this is still barely plausible after prior work

The closest prior-work collisions are already strong:

- **DIRECT** covers deterministic partition-based global optimization without a known Lipschitz constant. ([Jones, Perttunen, and Stuckman, 1993](https://link.springer.com/content/pdf/10.1007/BF00941892.pdf))
- **HOO / X-armed optimistic tree search** covers black-box search under local smoothness assumptions. ([Bubeck et al., 2011](https://www.jmlr.org/papers/volume12/bubeck11a/bubeck11a.pdf))
- **BOCS** and **COMBO** cover expensive combinatorial black-box optimization on discrete structures. ([Baptista and Poloczek, 2018](https://proceedings.mlr.press/v80/baptista18a/baptista18a.pdf))
- **NN+MILP** covers constrained discrete black-box optimization with MILP-optimized surrogate inner loops and explicit optimality guarantees for that inner discrete solve. ([Papalexopoulos et al., 2022](https://proceedings.mlr.press/v162/papalexopoulos22a.html))

So the novelty bar is high. This direction is only publishable if it produces a **new admissible bound specific to exact MA-crossover backtests**. If it falls back to “partition the lattice optimistically,” it collides with DIRECT/HOO-style work. If it falls back to “smart structured black-box search,” it collides with BOCS/COMBO and related mixed/discrete BO methods. If it falls back to surrogate-plus-MILP screening, it collides with NN+MILP-style work.

The one reason not to close it entirely is that the repo provides a rare clean falsification surface: the ordered `DoubleMa` lattice already exists, exhaustive evaluation already exists, and exact GPU/CPU baselines already exist. Even the repo’s earlier search review treated dominance/pruning as a question worth formal analysis, not as an implementation-ready claim.

## Deferred directions

These remain stronger repo-level directions and should stay in the broader research queue ahead of exact pruning:

- multi-fidelity optimization using real fidelity ladders,
- optimizer portfolios / online algorithm selection,
- search-space compilation / structure extraction as a systems direction.

## Rejected directions

Reject these as the adopted outcome for Question 5:

- **Broad repo-wide exact global pruning** across `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid`. Those profiles currently route through the `double_ma` family, but their ADX- and regime-dependent behavior makes them poor candidates for global bounds.
- **Hybrid exact/heuristic screening** sold as “certified elimination.” Once the admissibility theorem disappears, this becomes another structured black-box optimizer, not an exact-pruning contribution. ([MiVaBo](https://arxiv.org/pdf/1806.08838))
- **Search-stage engineering reframed as pruning novelty.** The current truth surfaces are still the exact row-first `GpuKernel` route, with score-first blocked or losing in its current form; that is systems/runtime work, not certified search.
- **Any CUDA-carried thesis.** If the claim stops making sense when CUDA is replaced by another fast evaluator, it is implementation novelty only.

A further warning sign is that the code already contains local-stability and neighborhood-robustness tooling. If this lane degrades into “good points have smooth neighborhoods,” it risks looking like a theoremized version of existing diagnostics rather than a new admissible search algorithm.

## Role of the custom CUDA implementation

**Substrate only.**

In this lane, CUDA would mainly accelerate bound evaluation, branch scoring, and candidate elimination. That is useful, but it is not the research claim. The claim must survive with a different exact evaluator. Otherwise the result is only a faster backend around a search idea the literature already covers.

## Minimum prototype needed

1. **Restricted surface only.**  
   Bare `DoubleMa`, one MA-kind pair, fixed cost model, `fast_len < slow_len`, no regime filters, no cooldown / persistence, no Kaufman-stop, no extra exits, one objective.

2. **Theory first.**  
   A proof sketch for a nontrivial admissible bound over lattice regions must exist before implementation work starts.

3. **Exactness requirement.**  
   The prototype must return the same exact winner and exact top-k as exhaustive CPU/GPU search.

4. **One brittleness test.**  
   Add exactly one mild real complication, such as cooldown or Kaufman-stop, to see whether the proof or prune power collapses.

## Mandatory baseline matrix

If this direction is ever promoted beyond design, the comparison set must be:

- **Exact baselines:** current exact GPU row-first `DoubleMa` route and exact CPU exhaustive grid.
- **Deterministic partition / optimistic-search baseline:** DIRECT or an HOO/X-armed style method on the restricted lattice. ([Jones, Perttunen, and Stuckman, 1993](https://link.springer.com/content/pdf/10.1007/BF00941892.pdf))
- **Classical heuristic baseline:** one strong evolutionary or cross-entropy style method.
- **Modern structured black-box baseline:** BOCS or COMBO on the discrete lattice; MiVaBo or CoCaBO if any mixed-variable extension is claimed; NN+MILP if constraint-heavy claims are made. ([Baptista and Poloczek, 2018](https://proceedings.mlr.press/v80/baptista18a/baptista18a.pdf))
- **Simple pragmatic baseline:** random or uniform batched sampling.
- **Ablation:** the same search procedure with the admissible bound removed.

Broader evaluation is not optional. Recent MCBO benchmark work has already raised expectations for mixed/combinatorial evaluation breadth. ([MCBO Benchmarks, NeurIPS 2023 Datasets and Benchmarks](https://proceedings.neurips.cc/paper_files/paper/2023/file/dbc4b67c6430c22460623186c3d3fdc2-Paper-Datasets_and_Benchmarks.pdf))

## Mandatory benchmark slate

The internal benchmark slate must include:

- **Structured exact lattice:** bare `DoubleMa`.
- **Mild complication:** `DoubleMa` plus cooldown or Kaufman-stop.
- **Broader mixed-family failure/generalization test:** at least one of `CompositeTrend`, `BreakoutMomentum`, or `RegimeHybrid`.
- **Route-conditional case:** ADX-off versus ADX-on where route policy changes between `GpuKernel` and `GpuHybridKernel`.
- **Validation-coupled case:** one shortlist/validation workflow where exact follow-up semantics matter, because current shortlist confidence / union / slice behavior is tightly tied to exact `GpuKernel` routing and single-slice execution.

Measure at least wall-clock to exact winner, simple regret under fixed budget, anytime incumbent quality, prune fraction, exactness parity against exhaustive search, and robustness across seeds for stochastic baselines.

## Direction-specific kill test

Close the lane immediately if any of these happen:

1. No nontrivial admissible bound can be written down for bare `DoubleMa` before coding starts.
2. A proved bound exists, but it still evaluates most of the lattice and does not beat the current exact GPU route to the exact winner/top-k.
3. Adding one mild real feature such as cooldown, Kaufman-stop, or a single regime filter destroys either the proof or nearly all prune power.

A fourth practical stop signal is drift into staged or heuristic screening. The codebase already has non-exact staged search (`GpuVdps`), so another approximate screening mode is not a certified-elimination contribution.

## Final verdict

**Design-only follow-up.**

Do **not** reopen implementation for Question 5 as a broad research direction. Close the repo-wide exact-pruning thesis. Keep only a short theorem/design memo on **certified local elimination for stripped-down `DoubleMa`**, with explicit permission for the result to be negative. Reopen code only if a nontrivial admissible bound exists on paper first.

## Repo context used

The memo above was grounded in the Question 5 upload bundle and the associated repo context, including:

- `01_first_pass_prompt.md`
- `02_second_pass_prompt.md`
- `03_final_consolidation_prompt.md`
- `AGENTS.md`
- `README.md`
- `REPO_MAP.md`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/lib.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `double_crossover_phase2_external_review_findings_2026-04-03.md`
- `double_crossover_phase2_research_artifact_plan_2026-04-03.md`
- `double_crossover_phase2_research_findings_2026-04-03.md`
- `tm_native_vwma_pilot_findings_2026-04-04.md`
- `xlarge_cache_reuse_probe_findings_2026-04-04.md`
- `cupti_sass_metric_followup_findings_2026-04-05.md`
