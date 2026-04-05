# `VectorGrid` Broader CS Research Question Pack

Date: 2026-04-05

Purpose: a broader prompt pack for ChatGPT Research Mode / GPT-5.4 Pro that is not anchored to SASS, kernel microtuning, or one specific CUDA pass.

This file is for finding a viable research direction in the context of this application that could still be defended as systems / algorithms / AutoML-style CS, rather than as narrow computer-engineering optimization work.

## Core Conclusion Before You Spend Research Mode

The broad literature now makes four things clear:

1. A generic “strategy-agnostic CUDA optimizer” is not enough as a paper thesis.
2. “Beat the exact `double_ma` brute-force GPU solver” is the wrong universal win condition.
3. A custom CUDA implementation may absolutely be part of the final system, but it is usually the **execution substrate**, not the novelty claim.
4. The strongest remaining directions are the ones that use this repo as:
   - an expensive batched black-box optimization environment,
   - a heterogeneous multi-strategy optimization workload,
   - a structured search-space compilation problem,
   - or a benchmark / platform candidate,
   rather than as a single-kernel optimization story.

## 2026 Literature Snapshot

Recent and still-relevant anchors:

- Mixed-space and high-dimensional black-box optimization are already crowded:
  - `Hyperband` (JMLR 2017): https://jmlr.org/papers/v18/16-558.html
  - `BOHB` (ICML 2018 / arXiv): https://arxiv.org/abs/1807.01774
  - `TuRBO` (NeurIPS 2019): https://papers.nips.cc/paper_files/paper/2019/hash/6c990b7aca7bc7058f5e98ea909e924b-Abstract.html
  - `CoCaBO` (ICML 2020): https://proceedings.mlr.press/v119/ru20a.html
  - `Additive Tree-Structured Covariance` for conditional spaces (AISTATS 2020): https://proceedings.mlr.press/v108/ma20a.html
  - `Think Global and Act Local` (ICML 2021): https://proceedings.mlr.press/v139/wan21b.html
  - `BARK` (ICML 2025): https://proceedings.mlr.press/v267/boyne25a.html
  - `MORBO` (UAI 2022): https://proceedings.mlr.press/v180/daulton22a.html
  - `VGT` (UAI 2024): https://proceedings.mlr.press/v244/zhao24a.html

- Multi-fidelity optimization is mature, but still fertile when the fidelity structure is real and domain-grounded:
  - `Hyperband` (JMLR 2017): https://jmlr.org/papers/v18/16-558.html
  - `BOHB` (ICML 2018 / arXiv): https://arxiv.org/abs/1807.01774
  - `Multi-Fidelity Black-Box Optimization with Hierarchical Partitions` (ICML 2018): https://proceedings.mlr.press/v80/sen18a.html
  - `HPOBench` (NeurIPS Datasets and Benchmarks 2021): https://openreview.net/forum?id=1k4rJYEwda-
  - `YAHPO Gym` (AutoML 2022): https://openreview.net/forum?id=H2zlL3EXBxq

- Algorithm selection and optimizer portfolios are active and plausible, not closed:
  - `Opening the Black Box: Automated Software Analysis for Algorithm Selection` (AutoML Conf 2022): https://proceedings.mlr.press/v188/pulatov22a.html
  - `PS-AAS` (AutoML Conf 2023): https://proceedings.mlr.press/v224/kostovska23a.html
  - `Is Algorithm Selection Worth It?` (PMLR 2021): https://proceedings.mlr.press/v140/kashgarani21a.html
  - `Open Source Vizier` (AutoML Conf 2022): https://proceedings.mlr.press/v188/song22a.html

- Exact or certified discrete optimization is lower-level, harder, and more “pure CS,” but not obviously a fit yet:
  - `Constrained Discrete Black-Box Optimization using Mixed-Integer Programming` (ICML 2022): https://proceedings.mlr.press/v162/papalexopoulos22a.html
  - `Optimistic Tree Searches for Combinatorial Black-Box Optimization` (NeurIPS 2022): https://papers.nips.cc/paper_files/paper/2022/hash/d6099a36f6c1720438de00c366aa1737-Abstract-Conference.html
  - `Every Call is Precious` (AISTATS 2025): https://proceedings.mlr.press/v258/fourati25a.html

- Benchmark and platform papers are publishable when the benchmark exposes real structural gaps:
  - `HPOBench` (2021): https://openreview.net/forum?id=1k4rJYEwda-
  - `MetaBox` (NeurIPS Datasets and Benchmarks 2023): https://openreview.net/forum?id=j2wasUypqN&noteId=xnSCKeUXBs
  - `MetaBox-v2` (NeurIPS Datasets and Benchmarks 2025): https://openreview.net/forum?id=415T06x0vG
  - `CATBench` (AutoML 2025): https://proceedings.mlr.press/v293/torring25a.html
  - `AMLB` (JMLR 2024): https://jmlr.org/papers/v25/22-0493.html

- Generic automated optimizer-design work is accelerating quickly:
  - `DesignX` (NeurIPS 2025): https://openreview.net/forum?id=FAiIRMvIwy

## What This Literature Implies For This Repo

1. A paper cannot rely on “we built another mixed-space optimizer.”
2. CUDA is execution substrate, not novelty.
3. The strongest broader directions are:
   - principled multi-fidelity optimization,
   - optimizer portfolios / online selection,
   - a benchmark or platform contribution,
   - a structure-extraction or search-space compiler direction,
   - or an exact-pruning angle if the domain truly supports it.
4. The weakest direction is:
   - “a general optimizer that beats exact brute-force `double_ma`.”

## Novelty Bar

For a direction in this file to count as a serious CS candidate, it should satisfy at least one of these:

1. It introduces a genuinely new problem formulation that existing mixed-space / budgeted optimization papers do not already cover well.
2. It exploits application structure that is both real and generalizable, not just “this GPU evaluator is fast.”
3. It yields stronger guarantees, evaluation methodology, or interoperability than a standard engineering implementation.
4. It produces a benchmark or platform that exposes structural challenges missing from current benchmark suites.

If a direction only offers:

- a fast CUDA implementation,
- better constants on one in-house benchmark,
- or a generic optimizer with no sharper claim,

then it is probably not the right research target.

## How To Treat A Custom CUDA Implementation

Your likely end state is reasonable:

- the final system may well require a custom CUDA implementation
- the implementation may itself be nontrivial and novel at the engineering level
- the paper can still be CS if the contribution is not merely “we wrote custom CUDA”

The safest rule is:

1. **The algorithmic claim must make sense without CUDA.**
2. **CUDA should be the execution substrate, scaling argument, or systems realization, not the whole novelty claim.**
3. **If removing CUDA destroys the conceptual contribution, the work is drifting toward CE.**

Research Mode should therefore always be asked to distinguish between:

- **algorithmic novelty**
  - new search policy
  - new fidelity policy
  - new portfolio or selector policy
  - new exact-pruning logic
  - new structure-extraction / compilation layer
- **systems novelty**
  - batched execution model
  - search-space compilation
  - orchestration/interface design
  - benchmark/platform contribution
- **implementation novelty only**
  - a clever custom kernel
  - a fast GPU runtime
  - a high-throughput batched engine

If the answer says the work is only in the third category, that is a warning sign.

## What Research Mode Should Always Be Forced To Answer

For every question in this pack, require the model to answer all of the following:

1. **What is the exact problem statement?**
   - not just “optimization,” but the formal setting, objective shape, search-space type, and budget model
2. **What is the closest prior work?**
   - ideally 3 to 5 strongest neighbors, not vague literature categories
3. **What is the novelty delta?**
   - what your direction adds beyond the closest prior work
4. **Would the claim survive if the CUDA implementation were replaced with another fast backend?**
5. **What exact baselines are mandatory?**
   - classical methods
   - strong modern black-box methods
   - exact baselines where available
   - ablations where needed
6. **What would reviewers most likely say is missing?**
7. **What result would falsify the direction?**

If a research thread cannot answer those well, it should probably be dropped.

## Shared Baseline Files

Attach these to every prompt:

- `AGENTS.md`
- `REPO_MAP.md`
- `README.md`
- `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_research_artifact_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_external_review_findings_2026-04-03.md`
- `CUDA kernel review/CUPTI/cupti_sass_metric_followup_findings_2026-04-05.md`
- `CUDA kernel review/Prompt1/xlarge_cache_reuse_probe_findings_2026-04-04.md`
- `CUDA kernel review/Prompt2/tm_native_vwma_pilot_findings_2026-04-04.md`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Recommended framing for every thread:

- the repo already has a very fast exact GPU evaluator
- the next worthwhile research direction does not need to be SASS-related
- the evaluator can be treated as a batched expensive black-box objective engine
- the question is what generalizable algorithmic or systems contribution still remains

## Shared Answer Shape

Ask for this structure:

1. What the actual research question is.
2. What class of existing literature is closest.
3. Why this repo makes the question interesting.
4. Whether the contribution would be CS, CE, AutoML, systems, or only engineering.
5. The minimum prototype needed.
6. The minimum benchmark/evaluation suite needed.
7. The strongest likely reviewer objection.
8. A keep / revise / abandon verdict.

## First-Pass Prompt Discipline

For every first-pass prompt in this pack:

- force the model to identify the exact problem statement, not just a topic label
- force comparison against the closest 3 to 5 prior works
- make it distinguish:
  - algorithmic novelty
  - systems novelty
  - implementation novelty only
- require an explicit answer on whether custom CUDA is:
  - necessary for execution
  - useful but not novel
  - or incorrectly carrying the whole claim
- require a baseline matrix and a falsification condition

What to avoid:

- generic optimizer brainstorming
- “another BO variant could help” with no literature delta
- any answer that treats speed on `double_ma` alone as a paper claim
- any answer that silently equates custom CUDA with research novelty

## Second-Pass Prompt Discipline

For every second-pass prompt in this pack:

- do not restate the first answer
- identify the weakest assumptions, nearest prior-work collisions, and likely reviewer attacks
- force the model to cut or demote directions that are too saturated
- require it to identify where the direction would likely fail
- require it to re-check whether the contribution survives if CUDA is treated only as substrate

The second pass should collapse the space toward:

- one leading direction
- one serious fallback
- and one or more explicitly rejected directions

## Final Consolidation Prompt Discipline

For every final consolidation prompt in this pack:

- write as if the result will be checked into `SASS research/`
- present one coherent direction, not a balanced list of options
- include explicit:
  - adopted direction
  - deferred directions
  - rejected directions
- include the minimum prototype, baseline matrix, benchmark slate, and kill test
- state clearly whether the outcome is:
  - a research direction worth reopening implementation for
  - a direction worth only a measurement/design memo
  - or a direction to close

## Reusable Second-Pass Prompt Template

Use this after any first-pass answer in this file:

```text
Review your prior answer as a critical second pass.

Do not restate the first answer. Pressure-test it.

Tasks:
1. Identify the weakest assumptions, nearest prior-work collisions, and places where the answer overstates novelty.
2. Identify whether the proposed contribution is really:
   - algorithmic
   - systems/tooling
   - benchmark/platform
   - or only implementation-level
3. Re-evaluate whether the contribution still exists if the custom CUDA implementation is treated purely as execution substrate.
4. Identify the strongest likely reviewer objection and the most likely failure mode.
5. Compare the leading direction against:
   - one more conservative fallback
   - one more ambitious but riskier alternative
6. Cut any direction that is too saturated or too dependent on in-repo specifics.

Deliverables:
1. `Keep / revise / reject` verdict on the first-pass leading direction
2. The revised leading direction
3. The strongest fallback direction
4. Explicitly rejected directions
5. The minimum baseline matrix and benchmark slate still required
6. The kill test that would end the direction early
```

## Reusable Final Consolidation Prompt Template

Use this after the second pass for any question in this file:

```text
Write a final consolidation memo for this research direction as if it will be checked into the repo.

Do not provide an equal-weight summary. Choose one outcome.

Deliverables:
1. A one-paragraph statement of the adopted direction
2. The exact problem statement
3. Why it is still plausible after comparing against prior work
4. Why the rejected alternatives were rejected
5. The role of the custom CUDA implementation:
   - substrate only
   - systems enabler
   - or warning sign
6. The minimum prototype needed
7. The mandatory baseline matrix
8. The mandatory benchmark slate
9. The direction-specific kill test
10. A final verdict:
   - reopen implementation
   - design-only follow-up
   - or close the direction

Write it in a repo-ready style, with crisp sections and no generic CUDA commentary.
```

## Revised Priority Order After Literature Search

### Best balance of realism and publishability

1. **Multi-fidelity optimization using real fidelity ladders from the application**
2. **Optimizer portfolios / online algorithm selection**
3. **Benchmark or platform contribution**
4. **Batch-synchronous strategy-agnostic optimization**, but only with a sharper target than “beat brute force”
5. **Exact pruning / certified elimination**

### Highest upside if it works, but highest risk

1. **Exact pruning / certified elimination**
2. **A strategy-search-space compiler with conditional, fidelity, and constraint structure**
3. **Batch-synchronous optimizer with a genuinely new problem formulation**

## Strongest Questions To Ask Research Mode

## Question 1: What Should The Actual Research Target Be If “Beat Brute Force” Is Wrong?

Why this matters:

- This is the most important question.
- If the win condition is wrong, every optimizer idea built on top of it is likely wrong too.

Attach in addition:

- `README.md`
- `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

Prompt:

```text
I want you to pressure-test a broader CS research direction for this application.

Using the attached repo context and broad literature search, answer this question:

If an exact brute-force GPU baseline already exists for simple structured cases like double MA, what should the real research target be for a broader strategy-agnostic optimizer?

Constraints:
- Do not answer with generic optimizer advice.
- Compare against modern mixed-space, budgeted, and expensive black-box optimization literature.
- Treat “beat brute force on double MA” as a thesis candidate that may need to be rejected.

Deliverables:
1. Explain why an exact brute-force baseline is or is not the wrong universal target.
2. Rank better targets such as:
   - simple regret under wall-clock budget
   - anytime performance
   - mixed-space scalability
   - robustness across strategy families
   - constrained or multi-objective search quality
3. State which target would look most like CS research rather than engineering.
4. State which target would look weakest to reviewers.
5. Give a keep / revise / abandon verdict on the original “beat brute force” framing.
```

Expanded sub-questions to force:

- Is the correct target:
  - budgeted simple regret,
  - anytime incumbent quality,
  - constrained feasibility rate,
  - cross-strategy transfer,
  - or something else?
- Which of those targets is least saturated in the current literature?
- Which targets would force a custom CUDA implementation to matter as systems support without making it the whole claim?
- For each plausible target, what exact baselines should be considered mandatory?
- Does the application naturally induce:
  - mixed discrete/continuous spaces,
  - conditional spaces,
  - constrained spaces,
  - multi-objective spaces,
  - or multi-fidelity spaces?
- Which of those is strongest as the paper’s primary formal setting?

## Question 2: Is Multi-Fidelity Optimization The Best Broad Direction?

Why this matters:

- The application appears to have real candidate fidelities:
  - shorter horizons
  - smaller datasets
  - reduced metrics
  - partial validation stages
- This is more differentiated than inventing another general optimizer.

Attach in addition:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`

Prompt:

```text
I want you to evaluate whether multi-fidelity optimization is the strongest broader CS direction for this repo.

Using the attached repo context and broad literature search, answer this question:

Could this application support a meaningful multi-fidelity optimization research project, where cheaper partial evaluations are used to guide expensive exact batched evaluations?

Constraints:
- Distinguish real low-fidelity signals from fake shortcuts that would break application semantics.
- Compare against Hyperband, BOHB, multi-fidelity BO, and modern benchmark literature.

Deliverables:
1. Identify the fidelity sources that this application actually exposes.
2. State whether they are reliable enough to form a serious research basis.
3. Define the minimal prototype and benchmark plan.
4. State the strongest possible research claim.
5. Explain the biggest failure mode.
6. Give a keep / revise / abandon verdict.
```

Expanded sub-questions to force:

- Which fidelity ladders are actually principled here?
  - fewer bars
  - reduced metric contracts
  - partial shortlist reruns
  - cheaper validation slices
  - approximate filters before exact evaluation
- Which of those preserve ranking information well enough to be research-grade?
- Is there a fidelity notion that generalizes beyond `double_ma`?
- Would the project be stronger as:
  - a fidelity scheduler,
  - a multi-fidelity optimizer,
  - or a benchmark on real fidelity hierarchies?
- What theoretical or empirical evidence would be required to show that the fidelities are not arbitrary shortcuts?
- What would a custom CUDA implementation actually be doing here:
  - accelerating all fidelities uniformly,
  - enabling cheap screening stages,
  - or supporting batch scheduling?

## Question 3: Would Optimizer Portfolios Or Online Algorithm Selection Be Stronger Than One Universal Optimizer?

Why this matters:

- The repo already spans multiple strategy families, route shapes, and workload patterns.
- That makes algorithm selection a natural application-level research question.

Attach in addition:

- `web/src/utils/strategyOptimizationCapabilities.ts`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/strategy_presets.rs`

Prompt:

```text
I want you to evaluate whether optimizer portfolios or online algorithm selection are a stronger research direction than one universal optimizer.

Using the attached repo context and broad literature search, answer this question:

Would a portfolio-selection or online algorithm-selection layer for expensive batched strategy optimization be a better research direction than designing a single new optimizer?

Constraints:
- Compare against automated algorithm selection, portfolio selection, and platform literature.
- Tie the answer to the repo’s actual multi-strategy setting.

Deliverables:
1. Explain the best algorithm-selection question this repo could ask.
2. Define the cheapest features or signals that could drive selection.
3. State whether this looks more publishable than a single universal optimizer.
4. Define a minimum benchmark suite.
5. State whether this should be promoted over the batch-synchronous optimizer idea.
```

Expanded sub-questions to force:

- What per-instance features could be extracted cheaply before expensive search begins?
- Could selection happen:
  - once per strategy family,
  - once per dataset,
  - once per workload size,
  - or adaptively during optimization?
- Is the contribution better framed as:
  - algorithm selection,
  - portfolio scheduling,
  - meta-learning,
  - or optimization systems orchestration?
- Does this repo have enough heterogeneity to make the question real rather than artificial?
- Would a custom CUDA implementation matter here mainly because:
  - different optimizers exploit batched evaluation differently,
  - or because selection itself can be GPU-aware?

## Question 4: Can A Batch-Synchronous, Strategy-Agnostic Optimizer Still Be A Real Contribution?

Why this matters:

- This is your original idea, revised into a harder but more defensible form.
- The novelty must come from the batch-synchronous expensive-evaluation regime, not from the use of CUDA.

Attach in addition:

- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `SASS research/double_crossover_phase2_experiment_table_2026-04-03.md`

Prompt:

```text
I want you to evaluate a broader research direction for this application that is not about SASS or low-level CUDA tuning.

Using the attached repo context and broad literature search, answer this question:

Can a batch-synchronous, strategy-agnostic optimizer for expensive mixed-space objectives be a viable CS contribution here?

The target setting is:
- the search space may be mixed discrete/continuous/conditional
- one GPU batch acts like one optimization iteration
- objective feedback arrives at batch boundaries
- exact evaluations are expensive but massively parallel

Constraints:
- Do not frame the contribution as merely “faster on one benchmark.”
- Evaluate whether this problem formulation is genuinely different from standard mixed-space BO or evolutionary search.
- Explain whether CUDA is just execution substrate or part of the scientific contribution.

Deliverables:
1. State whether the idea is viable, too saturated, or viable only with a sharper angle.
2. Propose the strongest precise formulation of the problem.
3. Define the minimum algorithmic novelty needed.
4. Define the minimum benchmark suite needed.
5. State the strongest claim that could be defended honestly.
6. Give a keep / revise / abandon verdict.
```

Expanded sub-questions to force:

- Is “batch-synchronous expensive optimization” genuinely underexplored, or is it already covered implicitly by existing large-batch BO and evolutionary work?
- What exact property of this application makes the batch-synchronous regime distinctive?
  - huge synchronous evaluation sets
  - delayed feedback only at batch boundaries
  - heterogeneous mixed conditional spaces
  - exact expensive evaluation with warm state
- Would the strongest contribution be:
  - a new optimizer,
  - a new acquisition / proposal policy,
  - a new archive / elite maintenance policy,
  - a new asynchronous-to-synchronous adaptation,
  - or a systems co-design result?
- What would the strongest ablations be?
- What would make the resulting custom CUDA implementation scientifically necessary rather than merely helpful?

## Question 5: Is There A Real Exact-Pruning Or Certified-Elimination Angle?

Why this matters:

- This is the most “pure CS” option still on the table.
- It only works if the strategy families expose enough structure for meaningful bounds or certified elimination.

Attach in addition:

- `crates/strategies/src/double_ma/mod.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/lib.rs`

Prompt:

```text
I want you to evaluate whether this application can support an exact-pruning or certified-elimination research direction.

Using the attached repo context and broad literature search, answer this question:

Is there a viable research direction around exact pruning, certified elimination, or bound-based search for strategy optimization in this application, or is the domain too weakly structured for that?

Constraints:
- Focus on methods with clear guarantees or exactness properties.
- Compare against combinatorial black-box optimization, tree search, MILP-backed black-box optimization, and Lipschitz or certified search work.

Deliverables:
1. State whether the application exposes enough structure for exact pruning to be realistic.
2. Identify which strategy families are plausible or implausible candidates.
3. Define the minimum theoretical and empirical contribution required.
4. State whether this is stronger or weaker than the multi-fidelity and algorithm-selection directions.
5. Give a keep / revise / abandon verdict.
```

Expanded sub-questions to force:

- Which strategy families have enough monotonicity, decomposability, or bounding structure to make pruning realistic?
- Can any useful upper or lower bounds be derived without destroying product semantics?
- Would the likely result be:
  - exact global pruning,
  - certified local elimination,
  - optimistic tree search,
  - or hybrid exact/heuristic screening?
- Is this really stronger than the multi-fidelity and portfolio directions, or just more mathematically attractive?
- Would a custom CUDA implementation here mainly accelerate:
  - bound evaluation,
  - branch scoring,
  - or candidate elimination?

## Question 6: Could A Search-Space Compiler Or Structure Extractor Be A Better Systems Direction?

Why this matters:

- The application already has conditional strategy definitions, route policies, validation stages, and strategy-local capability logic.
- A compiler-like contribution might be more defensible than “yet another optimizer” if it can turn strategy definitions into optimizer-friendly structure such as:
  - conditional variables
  - constraints
  - fidelity ladders
  - objective bundles

Attach in addition:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `crates/app_core/src/strategy_presets.rs`
- `crates/app_core/src/composite_profile_route.rs`

Prompt:

```text
I want you to evaluate a systems-style alternative to designing a new optimizer.

Using the attached repo context and broad literature search, answer this question:

Could a search-space compiler or structure extractor that turns strategy definitions into optimizer-friendly conditional spaces, constraints, and fidelity ladders be a better CS direction than inventing a new optimizer?

Constraints:
- Compare against work on conditional search spaces, structured Bayesian optimization, and optimization infrastructure.
- Be explicit about whether this would be a systems/tooling paper, an AutoML paper, or only engineering.

Deliverables:
1. Define the clearest possible contribution.
2. Explain the closest existing work.
3. State whether the attached repo already contains enough structure to justify the idea.
4. Define the minimum prototype and evaluation.
5. Give a keep / revise / abandon verdict.
```

Expanded sub-questions to force:

- What exactly would be compiled or extracted?
  - conditional parameters
  - admissible parameter ranges
  - constraints
  - fidelity ladders
  - metric bundles
  - route-dependent costs
- Is there enough structure already present in the repo to justify calling this a compiler or structure extractor?
- Which adjacent literatures are most relevant:
  - structured BO
  - optimization infrastructure
  - AutoML search-space DSLs
  - systems for experiment orchestration
- Would the strongest output be:
  - a compiler
  - an IR
  - a search-space normalizer
  - or a benchmark interface?

## Question 7: Could A Benchmark Or Platform Direction Be Better Than A New Algorithm?

Why this matters:

- The literature now clearly shows that benchmark and platform papers are real contributions when they expose underrepresented structure.
- This repo may have useful ingredients:
  - exact batched evaluators
  - mixed discrete/continuous/conditional spaces
  - possible constraints
  - possible fidelity ladders
  - multiple strategy families

Attach in addition:

- `SASS research/double_crossover_phase2_experiment_table_2026-04-03.md`
- `SASS research/double_crossover_phase2_research_artifact_index_2026-04-03.md`
- `SASS research/double_crossover_phase2_paper_outline_2026-04-03.md`

Prompt:

```text
I want you to evaluate whether this repo is better positioned for a benchmark or platform contribution than for a novel optimization-method paper.

Using the attached repo context and broad literature search, answer this question:

Could this application become a useful benchmark or platform for batched expensive black-box optimization in mixed, conditional, constrained, or multi-fidelity search spaces?

Constraints:
- Compare against HPOBench, YAHPO Gym, MetaBox, CATBench, AMLB, and optimization infrastructure such as OSS Vizier.
- Be explicit about what structural gaps the benchmark or platform would need to expose.

Deliverables:
1. State the strongest benchmark/platform framing.
2. Define the minimum packaging needed.
3. Explain what is still missing.
4. Compare this path against the strongest algorithmic path.
5. Give a keep / revise / abandon verdict.
```

Expanded sub-questions to force:

- What structural gaps in current benchmarks could this application expose?
  - exact expensive evaluators
  - conditional/mixed strategy spaces
  - realistic fidelity ladders
  - exact-vs-heuristic tradeoff surfaces
  - warm-state and batch-synchronous execution effects
- Would researchers outside trading care about the benchmark if the domain semantics were abstracted cleanly?
- Is the right contribution:
  - a benchmark suite,
  - a reusable platform,
  - a benchmark generation pipeline,
  - or a challenge track proposal?
- Would a custom CUDA implementation be part of the benchmark infrastructure, or an optional accelerator backend?

## Question 8: Which Venue Family Best Fits The Strongest Remaining Direction?

Why this matters:

- A direction can sound interesting but still be mismatched to the standards of the likely venues.
- This question helps decide whether any path is worth reopening at all.

Attach in addition:

- `SASS research/double_crossover_phase2_paper_outline_2026-04-03.md`
- `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`

Prompt:

```text
I want a venue-aware evaluation of the strongest remaining broader research directions for this application.

Using the attached repo context and broad literature search, answer this question:

Which venue families would best fit the strongest remaining research direction for this application, and what contribution standards would matter there?

Deliverables:
1. Rank the most plausible venue families.
2. For each, state the expected contribution style.
3. State which broader direction best matches each venue family.
4. State whether any direction is strong enough to justify reopening technical work.
```

Expanded sub-questions to force:

- Which venue families would reward:
  - algorithmic novelty,
  - systems/platform novelty,
  - benchmark novelty,
  - or exact-pruning theory?
- Which directions are more naturally CE or practitioner-engineering even if technically impressive?
- Which directions are strong enough that a custom CUDA implementation helps rather than hurts the venue fit?

## Questions I Would Not Spend Research Mode On

These look too weak or too saturated as stated:

- “Can I invent a generic optimizer that simply beats brute-force `double_ma`?”
- “Can I make a new evolutionary algorithm and run it on CUDA?”
- “Can I make a new Bayesian optimizer without a sharper problem statement?”
- “Can I turn more CUDA tuning into a pure CS paper?”
- “Can I use GPU implementation alone as the research novelty?”

## If You Expect To Build A Custom CUDA Algorithm Anyway

That is fine, but the custom CUDA work should be paired with one of these research stories:

1. **Multi-fidelity scheduler + custom CUDA evaluation engine**
   - CUDA makes the evaluation ladder practical, but the contribution is the fidelity policy.
2. **Optimizer portfolio / selector + GPU-aware execution substrate**
   - CUDA makes diverse optimizers economically comparable under one batch engine, but the contribution is the selector policy.
3. **Batch-synchronous optimizer with a clearly novel budgeted objective model**
   - CUDA makes the batch regime realistic, but the contribution is the optimization formulation and policy.
4. **Exact pruning / certified elimination with GPU-parallel bound evaluation**
   - CUDA accelerates bound computation, but the contribution is the guarantee-bearing elimination logic.

The weakest pairing is:

- **custom CUDA implementation + generic optimizer idea**

That pairing is the most likely to be read as strong engineering and weak research.

## Mandatory Baseline Matrix

For any direction that survives the first research pass, require comparison against a baseline matrix, not just one favorite baseline.

Minimum baseline categories:

1. **Exact baseline where available**
   - current exact GPU brute-force route on `double_ma`
   - exact CPU route where relevant for correctness or scaling reference
2. **Classical heuristic baseline**
   - at least one strong evolutionary or bandit-style method
3. **Modern mixed-space black-box baseline**
   - at least one BO-style or structured search baseline from the current literature
4. **Simple pragmatic baseline**
   - random search, Sobol, or uniform batched sampling
5. **Ablation baseline**
   - your proposed method with one key idea removed

What should be measured:

- wall-clock to target quality
- simple regret under fixed budget
- anytime incumbent quality
- robustness across seeds
- performance across multiple strategy families, not only `double_ma`

If a direction cannot name this baseline matrix cleanly, it is probably not mature enough.

## Candidate Benchmark Slate Inside This Repo

Any serious broader direction should be evaluated on more than one search-space shape.

Minimum internal slate:

1. **Structured exact lattice**
   - `DoubleMa`
   - use this as the “exact baseline exists” anchor
2. **Broader mixed-family profile**
   - one or more of `CompositeTrend`, `BreakoutMomentum`, `RegimeHybrid`
   - use this to test cross-strategy generalization
3. **Constraint-bearing or route-conditional case**
   - a case where capability rules, route policies, or indicator availability matter
4. **Validation-coupled case**
   - a case where optimization quality interacts with validation-stage cost or fidelity choice

If a direction only looks good on `double_ma`, it is more likely to be a product optimization than a broader CS contribution.

## Go / No-Go Rubric

Before reopening implementation work, the direction should clear at least four of these six checks:

1. **Problem clarity**
   - the formal problem statement is precise and not interchangeable with generic HPO language
2. **Novelty delta**
   - the closest prior work still leaves a meaningful gap
3. **Generalization**
   - the idea applies beyond one highly structured strategy family
4. **Evaluation clarity**
   - the baseline matrix and benchmark slate are obvious enough to name now
5. **CUDA independence**
   - the conceptual contribution still exists if CUDA is replaced by another fast backend
6. **Failure visibility**
   - it is obvious what negative result would falsify the idea

If a direction clears fewer than four, it should stay in the prompt pack and out of implementation.

## Publishable Negative Results

Not every surviving direction needs to end in a “new algorithm wins” paper.

The following negative outcomes could still be worth keeping if the evaluation is strong:

1. **Multi-fidelity is not trustworthy in this domain**
   - because cheap fidelities fail to preserve ranking or regret order
2. **Portfolio selection beats universal optimizers**
   - because workload heterogeneity is too large for one method to dominate
3. **Exact baselines dominate more often than expected**
   - showing where sophisticated black-box methods are the wrong tool
4. **Search-space compilation matters more than search policy**
   - because structure extraction improves multiple optimizers similarly

These are worth calling out because they help prevent overcommitting to “invent a new optimizer” as the only acceptable outcome.

## Bottom Line

If you only run three long Research Mode threads, use this order:

1. `Question 1: What Should The Actual Research Target Be If “Beat Brute Force” Is Wrong?`
2. `Question 2: Is Multi-Fidelity Optimization The Best Broad Direction?`
3. `Question 3: Would Optimizer Portfolios Or Online Algorithm Selection Be Stronger Than One Universal Optimizer?`

If those three all come back weak, the next most credible fallback is:

4. `Question 7: Could A Benchmark Or Platform Direction Be Better Than A New Algorithm?`
