# Question 7 Final Consolidation Memo

## Adopted direction

Do **not** reopen implementation for a standalone benchmark or platform paper. The adopted outcome is a **design-only benchmark-generation pipeline** that remains subordinate to a stronger future research thread, most likely multi-fidelity or optimizer-portfolio work. The benchmark/platform space is already crowded: HPOBench is a reproducible containerized multi-fidelity suite with 100+ problems; YAHPO Gym is a surrogate-based library with 14 scenarios and 700+ multi-fidelity instances and argues surrogate benchmarks are more faithful than tabular ones; MetaBox is a benchmark platform with 300+ problem instances, 19 baselines, and standardized metrics; CATBench already targets discrete, conditional, constrained, multi-fidelity, multi-objective real tasks in a unified containerized suite; AMLB is an open extensible benchmark; and OSS Vizier already occupies the distributed infrastructure/API position. Against that field, VectorGrid only has a plausible benchmark-side delta if it exposes **exact-anchor tasks, protocolized warm/cold execution, and validation-coupled or fidelity-aware costs** as first-class benchmark objects. That is worth preserving as a design, but it is not yet strong enough to justify a standalone reopen.  
Primary prior-work references: [HPOBench](https://openreview.net/forum?id=1k4rJYEwda-), [YAHPO Gym](https://openreview.net/forum?id=SWcg-TrUgc), [MetaBox](https://proceedings.neurips.cc/paper_files/paper/2023/hash/232eee8ef411a0a316efa298d7be3c2b-Abstract-Datasets_and_Benchmarks.html), [CATBench](https://proceedings.mlr.press/v293/torring25a.html), [AMLB](https://www.jmlr.org/papers/v25/22-0493.html), [OSS Vizier](https://proceedings.mlr.press/v188/song22a.html).  
Repo context: `README.md`, `REPO_MAP.md`, `AGENTS.md`, `double_crossover_phase2_research_artifact_plan_2026-04-03.md`, `double_crossover_phase2_research_findings_2026-04-03.md`.

## Exact problem statement

The only benchmark-side problem worth retaining is:

Given a family of application evaluators, build a generator that emits benchmark tasks

\[
t = (X, A, g, Z, P, f, c, o)
\]

where:
- \(X\) is a mixed search space,
- \(A\) is a conditional activation graph,
- \(g\) is a feasibility predicate,
- \(Z\) is a fidelity graph or rung schedule,
- \(P\) is execution protocol metadata such as cold vs warm and search-only vs validation-coupled,
- \(f\) is objective quality,
- \(c\) is realized evaluation cost,
- \(o\) is optional exact-optimum or exact-gap metadata for finite-lattice tasks.

This is a **systems/tooling** problem, not an algorithm paper. VectorGrid can plausibly instantiate it because the repo already has an exact-grid `DoubleMa` anchor, request-level constraints and VDPS rung controls, GPU-execution policy surfaces, warm resident execution, and multiple execution families beyond the core `DoubleMa` lane. But the repo is still product-shaped: it is a Tauri desktop app for exact-grid optimization and validation, and several broader profiles such as `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid` still route through the `DoubleMa` product path.  
Repo context: `README.md`, `REPO_MAP.md`, `AGENTS.md`, `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`.

## Why it is still plausible after prior work

The direction is not empty. VectorGrid does have benchmark ingredients that are unusual in combination: an exact-lattice anchor, explicit constraint and fidelity controls in `DoubleMaRequest`, warm resident-buffer execution, and validation workflows that can make cost semantics depend on more than a single objective call. Those are real assets.  
Repo context: `crates/app_core/src/double_ma.rs`, `README.md`.

But the closest collision is too strong for a standalone benchmark paper **right now**. CATBench already claims the hardest generic benchmark features that would otherwise carry this direction: discrete and conditional variables, constraints, multi-fidelity, multi-objective evaluation, and a unified reproducible suite. HPOBench and YAHPO Gym already cover much of the reproducible multi-fidelity benchmark story, while MetaBox, AMLB, and OSS Vizier cover benchmark-platform standardization and optimization infrastructure from different angles. By inference from those scopes, the only remaining gap VectorGrid could test is whether **exact-anchor expensive evaluators plus execution-protocol metadata** materially change optimizer conclusions. That is a sharper claim than “another benchmark,” but it is still unproven.  
Primary prior-work references: [CATBench](https://proceedings.mlr.press/v293/torring25a.html), [HPOBench](https://openreview.net/forum?id=1k4rJYEwda-), [YAHPO Gym](https://openreview.net/forum?id=SWcg-TrUgc), [MetaBox](https://proceedings.neurips.cc/paper_files/paper/2023/hash/232eee8ef411a0a316efa298d7be3c2b-Abstract-Datasets_and_Benchmarks.html), [AMLB](https://www.jmlr.org/papers/v25/22-0493.html), [OSS Vizier](https://proceedings.mlr.press/v188/song22a.html).

There is also a repo-state reason not to promote this direction. The current research artifact plan explicitly shifts the branch away from open-ended optimization work and says further technical effort is justified only when it materially improves the research artifact. The strongest live artifact is the static-plus-dynamic attribution case study around `double_crossover`, not a benchmark platform, and the paper outline is already written around that thesis.  
Repo context: `double_crossover_phase2_research_artifact_plan_2026-04-03.md`, `double_crossover_phase2_paper_outline_2026-04-03.md`, `double_crossover_phase2_research_findings_2026-04-03.md`.

Recent adjacent branches point the same way. The cache-reuse probe was a no-go despite better cache hit rates, the CUPTI follow-up is explicitly classified as useful only for research/tooling, and the TM-native VWMA work was retained only as a bounded backend-local pilot rather than a broad generalization. That is evidence of strong systems measurement and selective backend engineering, not yet evidence of a community-facing benchmark program.  
Repo context: `xlarge_cache_reuse_probe_findings_2026-04-04.md`, `cupti_sass_metric_followup_findings_2026-04-05.md`, `tm_native_vwma_pilot_findings_2026-04-04.md`.

## Why the rejected alternatives were rejected

### Rejected now: standalone benchmark suite

Rejected because the novelty delta is too small against CATBench, HPOBench, and YAHPO Gym unless VectorGrid can show that exact-anchor tasks and execution protocols materially alter conclusions. The repo also still leans too heavily on `DoubleMa` and `DoubleMa`-routed profiles for its most visible GPU story.  
References: [CATBench](https://proceedings.mlr.press/v293/torring25a.html), [HPOBench](https://openreview.net/forum?id=1k4rJYEwda-), [YAHPO Gym](https://openreview.net/forum?id=SWcg-TrUgc)); repo context: `README.md`, `AGENTS.md`.

### Rejected now: reusable platform paper

Rejected because OSS Vizier, MetaBox, and AMLB already occupy most of the credible platform/infrastructure territory, while VectorGrid is still primarily presented as a Tauri desktop product with internal request routing and validation workflows, not as a public optimization service.  
References: [OSS Vizier](https://proceedings.mlr.press/v188/song22a.html), [MetaBox](https://proceedings.neurips.cc/paper_files/paper/2023/hash/232eee8ef411a0a316efa298d7be3c2b-Abstract-Datasets_and_Benchmarks.html), [AMLB](https://www.jmlr.org/papers/v25/22-0493.html)); repo context: `README.md`, `REPO_MAP.md`.

### Rejected now: challenge track or leaderboard

Rejected because the task taxonomy, protocol contract, and public baseline harness are not stable enough, and the current repo stop rule favors artifact consolidation over new outward-facing productization.  
Repo context: `double_crossover_phase2_research_artifact_plan_2026-04-03.md`.

### Deferred, not rejected: fuller platform after another direction lands

Deferred because a future multi-fidelity or optimizer-portfolio paper could generate the exact task families and evaluation disagreements that would finally justify benchmark packaging. Without that precursor, the benchmark story is speculative.

## Role of the custom CUDA implementation

For this direction, custom CUDA is **substrate only**. It is a useful executor for expensive tasks and a systems enabler for realistic wall-clock studies, especially because the repo already distinguishes warm GPU-resident execution from other paths and maintains resident/staged market caches. But if removing CUDA destroys the conceptual contribution, the benchmark claim has failed and has collapsed into implementation novelty only.  
Repo context: `README.md`, `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`.

## Minimum prototype needed

If this direction is ever reopened, the minimum prototype is:

- A headless evaluator interface, separate from the Tauri UI, with ask/tell or RPC semantics. Today the repo is still organized primarily as a desktop application and command surface.  
  Repo context: `README.md`, `REPO_MAP.md`.

- A declarative task schema covering mixed variables, conditional activation, constraints, fidelity metadata, protocol metadata, and exact-anchor availability. The raw ingredients exist now in `DoubleMaRequest`, `DoubleMaConstraints`, `VdpsConfig`, and `GpuExecutionPolicy`, but not as a benchmark-neutral contract.  
  Repo context: `crates/app_core/src/double_ma.rs`.

- A minimal public task bundle containing one exact `DoubleMa` anchor, one route-conditional profile task, one genuinely non-`DoubleMa` family, and one validation- or fidelity-coupled task. The repo has the raw family structure, but not yet a public benchmark taxonomy.  
  Repo context: `REPO_MAP.md`, `README.md`, `AGENTS.md`.

- A reproducibility harness that can run the same task in cold and warm protocol modes and record both wall-clock and evaluation-budget views. Warm resident execution is currently part of the product benchmark story, not a formal benchmark protocol.  
  Repo context: `README.md`, `crates/app_core/src/double_ma.rs`.

- A baseline runner and results schema that make optimizer comparisons portable outside the repo.

## Mandatory baseline matrix

Any reopened benchmark effort would need, at minimum:

- **Exact baseline**: current exact GPU `DoubleMa` brute force, plus exact CPU on smaller controls.
- **Simple pragmatic baseline**: random search and Sobol.
- **Classical heuristic baseline**: one evolutionary mixed-variable searcher.
- **Modern mixed-space baseline**: one structured model-based optimizer for mixed/conditional spaces.
- **Fidelity-aware baseline**: one ASHA/Hyperband/BOHB-style method on tasks with real fidelity ladders.
- **Ablation baseline**: the same benchmark stripped of one claimed structural advantage, preferably exact-anchor metadata or warm/cold protocol metadata.

If that matrix cannot be run cleanly, the benchmark should not be published.

## Mandatory benchmark slate

A credible slate would need all of the following:

- **Exact lattice anchor**: `DoubleMa`, because that is where exact baseline authority already exists.  
  Repo context: `README.md`.

- **Route-conditional mixed profile**: at least one of `CompositeTrend`, `BreakoutMomentum`, or `RegimeHybrid`, precisely because they currently exercise route policy and are not just another flat lattice.  
  Repo context: `AGENTS.md`, `README.md`.

- **Genuinely non-`DoubleMa` family**: one task from the composite/advanced, `CompositeMo`, or `MlSignal` families, otherwise the benchmark is still too dominated by one product lane.  
  Repo context: `REPO_MAP.md`.

- **Constraint/fidelity case**: a task that uses `DoubleMaConstraints` and a real VDPS rung schedule or equivalent fidelity control.  
  Repo context: `crates/app_core/src/double_ma.rs`.

- **Validation-coupled case**: one task where search cost and downstream validation cost are both part of the protocol. The repo already treats optimization and validation as coupled workflows.  
  Repo context: `README.md`, `REPO_MAP.md`.

If the public slate stops at `DoubleMa` plus `DoubleMa`-routed cousins, the direction fails.

## Direction-specific kill test

Close the direction immediately if both of these fail:

1. After building one exact-anchor family and one non-`DoubleMa` family, the added benchmark structure — exact-gap metadata, warm/cold protocol, and validation/fidelity metadata — does **not** materially change optimizer rankings or measurement conclusions relative to a stripped HPOBench/CATBench-style task view.
2. The task abstraction still reads to outsiders as “a trading backtest app benchmark” rather than as a benchmark for mixed, conditional, constrained, fidelity-aware expensive black-box optimization.

A benchmark paper without those two properties is not differentiated enough.

## Final verdict

**Design-only follow-up.**

Keep a **benchmark-generation pipeline specification** as a deferred support direction. Do **not** reopen implementation for a standalone benchmark/platform paper now. Revisit only if another direction — most plausibly multi-fidelity or optimizer portfolios — produces compelling task families and empirical disagreements that actually require benchmark packaging. That outcome is consistent with the repo’s current stop rule and with the fact that the strongest live artifact remains the static-plus-dynamic attribution research bundle, not a benchmark platform.  
Repo context: `double_crossover_phase2_research_artifact_plan_2026-04-03.md`, `double_crossover_phase2_research_findings_2026-04-03.md`.

## Repo references used

- `AGENTS.md`
- `README.md`
- `REPO_MAP.md`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `cupti_sass_metric_followup_findings_2026-04-05.md`
- `xlarge_cache_reuse_probe_findings_2026-04-04.md`
- `tm_native_vwma_pilot_findings_2026-04-04.md`
- `double_crossover_phase2_experiment_table_2026-04-03.md`
- `double_crossover_phase2_external_review_findings_2026-04-03.md`
- `double_crossover_phase2_paper_outline_2026-04-03.md`
- `double_crossover_phase2_research_artifact_index_2026-04-03.md`
- `double_crossover_phase2_research_artifact_plan_2026-04-03.md`
- `double_crossover_phase2_research_findings_2026-04-03.md`
- `01_first_pass_prompt.md`
- `02_second_pass_prompt.md`
- `03_final_consolidation_prompt.md`
