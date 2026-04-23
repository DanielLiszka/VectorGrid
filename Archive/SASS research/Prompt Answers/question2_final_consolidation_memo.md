# Final consolidation memo — Question 2: Is Multi-Fidelity Optimization The Best Broad Direction?

Source bundle: Question 2 upload bundle (`Question2 Upload Bundle`) and the final-consolidation prompt.

## Adopted direction

Adopt **a benchmark/systems direction on trustworthy fidelity ladders for exact, validation-coupled optimization**, with only a **small reliability-gated scheduler** as the algorithmic layer. The claim is not “we built a new generic multi-fidelity optimizer.” The claim is that VectorGrid exposes a distinctive problem class: mixed and route-conditional strategy search spaces, exact batched evaluation, and downstream validation costs that themselves admit cheaper but potentially unreliable budgets. The publishable question is therefore which repo-native fidelities preserve exact ranking and validation decisions well enough to justify promotion, not whether one more MFBO variant beats prior schedulers. That keeps CUDA in the role the prompt pack demands—execution substrate and systems enabler, not the novelty claim.

## Exact problem statement

Let \(x \in X\) be a candidate in a mixed, conditional strategy/configuration space. Let \(F(x)\) be the **authoritative exact endpoint**, meaning exact batched backtest metrics plus downstream validation evidence and verdicts. Let \(F_\lambda(x)\) be a cheaper evaluation at fidelity \(\lambda\), where \(\lambda\) changes **evaluation budget** but is not allowed to change the candidate semantics or replace exact finalist authority. Under wall-clock budget \(B\), the task is to schedule candidate-fidelity pairs \((x,\lambda)\) so as to maximize anytime incumbent quality and reduce simple regret, while promoting only candidates whose low-fidelity evidence is reliable enough and always finishing finalists with exact evaluation.

In this repo, that problem is real because the optimization path already exposes `GpuVdps` rungs and the validation path already exposes `walk_forward`, `reopt_walk_forward`, `spa`, and `cscv_pbo`, all on top of an exact GPU/CPU optimization and validation stack.

## Why it is still plausible after prior work

The repo already has the right raw ingredients. `double_ma.rs` contains explicit VDPS rung budgets and telemetry, including rung fractions, survivor/refill controls, and quality-check overlap/regret-style statistics. `double_ma_validate.rs` exposes validation-stage resource knobs such as walk-forward window/step size, reoptimization shortlist width, CSCV slices, and SPA slices. The product path also extends beyond a single exact lattice: `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid` already route through the current `double_ma` family, with route-conditional GPU/Hybrid/CPU behavior. That means the fidelity question is not confined to one toy surface.

The repo also enforces the right semantic guardrail: exact finalist authority is preserved, and the current exact product route keeps full-row semantics rather than replacing them with approximate shortlist logic. That matters because the only defensible multi-fidelity story here is one where cheap stages guide promotion and screening, but never become the final truth surface.

The algorithmic side, however, is already crowded. Hyperband framed budgeted HPO as pure-exploration with resource levels; BOHB combined Hyperband with model-based search; FABOLAS modeled loss and cost as a function of dataset size; BOCA modeled continuous fidelity spaces; DyHPO pushed dynamic multi-budget racing; and rMFBO directly addressed unreliable low-fidelity information sources. A paper whose headline is merely “a reliability-aware MF optimizer” would therefore collide with existing algorithmic territory almost immediately.

Benchmark literature strengthens the same conclusion. HPOBench, YAHPO Gym, and JAHS-Bench-201 already provide substantial multi-fidelity benchmarking ecosystems. The gap here is narrower and more specific: this repo offers **exact batched black-box evaluations coupled to downstream validation semantics**, not just training-curve or surrogate/tabular multi-fidelity HPO. That gap is plausible, but it is a **problem-class and benchmark** gap first, not a clean new-MFBO-algorithm gap.

## Deferred directions

### Deferred 1: pure benchmark/platform only

This is the immediate fallback if the scheduler story is weak. It stays credible because a careful audit of trustworthy versus untrustworthy fidelities would already be useful. It is deferred, not adopted, because the benchmark becomes stronger if it includes at least one simple promotion policy rather than stopping at descriptive measurement.

### Deferred 2: optimizer portfolios / online algorithm selection

This remains the strongest non-MF fallback if the fidelity audit fails. It is deferred here because Question 2 should not be diluted into a different research program before the fidelity question is actually tested.

### Deferred 3: exact pruning / certified elimination

This has higher upside, but it needs much stronger elimination logic than the repo currently exposes. The current evidence is materially stronger on real fidelity ladders than on certified pruning structure.

## Rejected directions

### Rejected 1: a new generic multi-fidelity optimizer as the headline contribution

That is too saturated relative to Hyperband, BOHB, FABOLAS, BOCA, DEHB, DyHPO, and rMFBO. The surviving novelty is in the repo’s fidelity structure and evaluation semantics, not in claiming one more universally better scheduler.

### Rejected 2: kernel-first CUDA performance work as the broader-CS target

The repo’s own retained findings classify the CUPTI/SASS branch as useful for research/tooling, not as a new product optimization priority; the cache-reuse probe was a no-go despite improved cache hit rates; and the TM-native VWMA pilot was retained only as a narrow backend-local improvement, not as a broader thesis. That is exactly why CUDA should stay substrate-only here.

### Rejected 3: fake fidelities as the main ladder

`Full17` versus `Trade22` is a useful ablation, but not the primary fidelity story, because the current exact route already decodes current product metrics without replay by default. Likewise, replay descriptor reuse is not the next ladder: after batching, descriptor prep/upload round to `0 ms`, so they are not the dominant cost. Cache-fit heuristics are also rejected because better cache residency did not translate into better wall-clock. These are pseudo-fidelities or engineering sub-buckets, not the main research ladder.

## Role of the custom CUDA implementation

The custom CUDA implementation is a **systems enabler**. It accelerates the exact endpoint, makes large batched fidelity audits affordable, and supports uniform evaluation of both exact and cheaper stages. It may also make heterogeneous batch scheduling practical. But the conceptual contribution must survive if CUDA is replaced by another fast backend. If the idea stops mattering once CUDA is removed, the work has collapsed into implementation novelty only.

This direction avoids that failure mode because the core question—*which fidelities are trustworthy enough to guide exact, validation-coupled search?*—still makes sense without CUDA. CUDA simply makes the prototype and benchmark scale credible.

## Minimum prototype needed

1. **Fidelity audit first.**  
   Implement a harness that evaluates a fixed candidate pool at every candidate fidelity and at the exact endpoint. Start with only the principled ladders already in the repo:
   - optimization-stage VDPS rungs
   - validation-stage walk-forward budgets
   - reopt shortlist width
   - CSCV slices
   - SPA slices

   Treat `metric_contract` only as a secondary ablation, not as a core ladder. Measure rank correlation to the exact endpoint, top-k recall, false-elimination rate, and wall-clock savings.

2. **A small reliability-gated scheduler.**  
   After the audit, add one lightweight scheduler that uses audited fidelities and promotes candidates only when a ladder is locally trustworthy. Do not start with a new GP model or a custom MFBO theory paper. Start with a strong existing scheduler family—Successive Halving/Hyperband, BOHB, or DEHB—and add the repo-specific reliability gate as the only new mechanism.

3. **Exact finalization and validation verdicts remain authoritative.**  
   All promoted finalists must still go through the current exact endpoint and current validation logic. That invariant is part of the research claim, not an implementation afterthought.

## Mandatory baseline matrix

The minimum baseline matrix is:

- **Exact baseline:** current exact GPU `DoubleMa` route, plus exact CPU reference.
- **Simple pragmatic baselines:** random search and Sobol.
- **Classical budgeted baseline:** Successive Halving / Hyperband.
- **Modern mixed-space multi-fidelity baselines:** BOHB and DEHB.
- **Model-based multi-fidelity baseline:** BOCA or FABOLAS, plus a practical mixed-space tool baseline such as SMAC3 multi-fidelity.
- **Ablations:** no reliability gate; optimization-stage fidelities only; validation-stage fidelities only; metric-contract pseudo-fidelity ablation.

Measure, at minimum:
- wall-clock to target quality
- simple regret under fixed budget
- anytime incumbent quality
- rank preservation to the exact endpoint
- false-elimination rate
- robustness across seeds

The prompt pack is correct that “beat exact `double_ma` brute force” cannot be the universal win condition; exact `DoubleMa` is an anchor baseline, not the entire thesis.

## Mandatory benchmark slate

The benchmark slate must include:

- **Structured exact lattice:** `DoubleMa`, because exact GPU and CPU anchors exist.
- **Broader mixed-family cases:** at least two of `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid`, because the product path already routes them through the current family.
- **Route-conditional case:** an ADX-on/off or similar case that changes route policy across `GpuKernel`, `GpuHybridKernel`, and CPU fallback.
- **Validation-coupled cases:** one walk-forward or reopt-walk-forward case, and one CSCV or SPA case.

If the direction only looks good on `DoubleMa`, it should be treated as product optimization, not broader CS work.

## Direction-specific kill test

Pre-register a kill rule and stop early if it fails.

A reasonable kill test is:

- **Reliability failure:** after the fidelity audit, no candidate ladder achieves stable rank preservation to the exact endpoint across the slate. In practice, at least one ladder should show both strong rank agreement and strong top-k retention on most tasks, not just on `DoubleMa`.
- **Competitiveness failure:** once exact finalist reevaluation is included, the best audited-fidelity method does not beat random/Sobol and does not provide a meaningful wall-clock-to-target improvement over strong budgeted baselines such as Hyperband/BOHB/DEHB.
- **Generalization failure:** the ladders only work on one exact lattice and collapse on broader mixed-family or validation-coupled cases.

If any of those hold, end algorithm work immediately and downshift the effort to a **negative-result benchmark/methodology memo**: “multi-fidelity is not trustworthy enough in this domain.” That is still publishable if the audit is clean.

## Final verdict

**Reopen implementation, but only for a bounded fidelity-audit prototype and a small reliability-gated scheduler.**

So the consolidated answer to Question 2 is:

**Yes—multi-fidelity is the best remaining broad direction only after being revised into a benchmark/systems project on trustworthy fidelity ladders for exact, validation-coupled optimization.**  
Do **not** reopen implementation for a new generic MFBO algorithm. Do **not** reopen implementation for kernel-first CUDA research. Reopen it only to answer the one question that still looks publishable: *which repo-native fidelities are real, trustworthy, and general enough to guide exact search without breaking semantics?* If the audit is negative, close the algorithm direction and keep the benchmark/negative-result artifact instead.

## Reference notes

### Repo files used in the argument

- `Question2 Upload Bundle`
- `AGENTS.md`
- `README.md`
- `REPO_MAP.md`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `SASS research/double_crossover_phase2_external_review_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`
- `SASS research/double_crossover_phase2_research_artifact_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`
- `CUDA kernel review/CUPTI/cupti_sass_metric_followup_findings_2026-04-05.md`
- `CUDA kernel review/Prompt1/xlarge_cache_reuse_probe_findings_2026-04-04.md`
- `CUDA kernel review/Prompt2/tm_native_vwma_pilot_findings_2026-04-04.md`

### Literature anchor points cited in the chat-version analysis

- Li et al., **Hyperband: A Novel Bandit-Based Approach to Hyperparameter Optimization** (JMLR, 2018)
- Falkner et al., **BOHB: Robust and Efficient Hyperparameter Optimization at Scale** (ICML/PMLR, 2018)
- Klein et al., **Fast Bayesian Optimization of Machine Learning Hyperparameters on Large Datasets (FABOLAS)** (AISTATS/PMLR, 2017)
- Kandasamy et al., **BOCA / Multi-fidelity Bayesian Optimization with Continuous Approximations** (representative continuous-fidelity MFBO work)
- DyHPO / CAMO literature on dynamic multi-fidelity scheduling
- Mikkola et al., representative **reliable / robust multi-fidelity BO** cautionary literature
- HPOBench, YAHPO Gym, and JAHS-Bench-201 as benchmark/platform comparison points
