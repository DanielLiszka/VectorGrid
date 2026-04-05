# `VectorGrid` Broader Research Question Pack For ChatGPT Research Mode / GPT-5.4 Pro

Date: 2026-04-05

Purpose: a broader research-question pack for this application that is not limited to SASS, kernel tuning, or `double_crossover` source-level optimization.

This pack assumes the real target is a defensible systems / algorithms / pure-CS direction that happens to use this application as its domain. It is meant to help decide whether there is a research program here beyond “CUDA engineering worked well.”

## Core Framing

The strongest broader direction is probably not:

- “make `double_ma` even faster”
- “beat brute-force on one structured benchmark”
- “invent another generic GPU metaheuristic”

The stronger possibilities are:

- a strategy-agnostic optimizer for expensive mixed search spaces under batch-synchronous evaluation constraints
- an optimizer portfolio / algorithm-selection system for heterogeneous strategy spaces
- a multi-fidelity scheduler with principled fidelity selection
- a pruning / elimination method with exactness guarantees
- a search-space / strategy compiler that turns strategy descriptions into optimizer-aware structure
- a benchmark / artifact paper if the algorithmic contribution is not strong enough

## How To Use This Pack

- Use one question per chat.
- Attach the shared baseline files first.
- Then attach any question-specific files.
- Ask the model to compare against existing black-box optimization, mixed-space BO, algorithm-selection, multi-fidelity, and search-space-partition literature.
- Require the model to tell you whether the direction is:
  - a plausible new research project
  - only a practitioner engineering direction
  - or too saturated to pursue

## Shared Baseline Files

Attach these to most threads:

- `AGENTS.md`
- `REPO_MAP.md`
- `README.md`
- `ARCHITECTURE_NOTES.md`
- `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_external_review_findings_2026-04-03.md`
- `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

Use these when the question is specifically about current `double_ma` evidence:

- `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
- `CUDA kernel review/CUPTI/cupti_sass_metric_followup_findings_2026-04-05.md`

## Literature Anchors To Compare Against

Ask the model to compare proposed directions against at least the closest families in the literature, not just generic “black-box optimization”:

- BOHB
- TuRBO
- CoCaBO
- Think Global and Act Local
- multi-fidelity tree-search and BO work
- PS-AAS or related algorithm-selection / portfolio work
- search-space partition / tree-search black-box methods such as LA-MCTS

## Shared Answer Shape

Ask for this shape:

1. The exact research question.
2. Why it is or is not already saturated.
3. The closest prior work.
4. The real novelty delta required.
5. The minimum prototype or experiment set.
6. The right baselines and evaluation protocol.
7. Whether the result is:
   - algorithms research
   - systems research
   - compilers/tooling research
   - benchmarking/artifact research
   - or not strong enough

## Research Mode Rigor For The Top Questions

For Questions 1 through 4, require the model to answer all of the following, not just the headline question:

1. **What is the algorithmic core, independent of CUDA?**
   - Ask for the abstract optimizer or scheduler first.
   - Ask which parts would later be custom CUDA implementations and which parts are only execution details.

2. **What is the abstract problem class?**
   - Search-space type
   - objective structure
   - constraint structure
   - evaluation budget model
   - feedback timing
   - whether one batch truly corresponds to one iteration

3. **What is the minimum baseline matrix?**
   - random / Sobol or grid baseline
   - exhaustive baseline where feasible
   - strong mixed-space BO baseline
   - evolutionary / population baseline
   - portfolio or scheduler baseline when relevant

4. **How will algorithmic gains be separated from implementation gains?**
   - force at least one implementation-neutral or CPU-oriented comparison plan
   - ask what ablations would show that the search method, not just faster batching, caused the improvement

5. **How does the claim generalize beyond `double_ma`?**
   - require a minimum cross-strategy evaluation plan
   - reject answers that only work on one low-dimensional structured family

6. **Where should the method lose?**
   - tiny spaces
   - exact grids
   - noisy objectives
   - weak fidelity ladders
   - strategy families with little exploitable structure

7. **What is the honest venue fit?**
   - algorithms
   - AutoML / black-box optimization
   - systems / infrastructure
   - only engineering

## Ranked Research Questions

## Question 1: Is A Strategy-Agnostic, Batch-Synchronous Optimizer The Best Generalization Path?

Why this matters:

- This is the cleanest way to move from “fast one strategy” to a general CS question.
- The core challenge is not CUDA itself, but optimization when evaluation happens in large GPU batches and feedback arrives only once per batch.

Prompt:

```text
I want to evaluate a broader research direction for this application, not another kernel-tuning project.

Using the attached repo context and broad literature search, answer this question:

Is there a defensible research project in designing a strategy-agnostic, batch-synchronous optimizer for expensive mixed search spaces, where one large GPU batch is one iteration and the optimizer must choose what to evaluate next under that constraint?

Constraints:
- Do not reduce this to “make CUDA faster”.
- Compare against BOHB, TuRBO, CoCaBO, Think Global and Act Local, and related batched/mixed-space black-box optimization work.
- Distinguish whether the contribution would be algorithms, systems, or just implementation engineering.

Deliverables:
1. Define the strongest precise research question.
2. State whether the area is already too saturated.
3. Explain what novelty would still be needed beyond existing mixed-space and batched optimization work.
4. State the abstract problem class in implementation-neutral terms.
5. Explain what the hardware-independent algorithmic core would be.
6. Explain which components, if any, would later merit a custom CUDA implementation and which would not.
7. Define the minimum algorithmic prototype and evaluation protocol.
8. Give the minimum baseline matrix, including where exhaustive search should remain the expected winner.
9. State whether this direction is plausible enough to pursue.
```

## Question 2: Is “Beat The Efficient `double_ma` Brute-Force Baseline” Even The Right Goal?

Why this matters:

- As stated, this is probably the wrong thesis target.
- A strategy-agnostic optimizer that preserves exactness is unlikely to beat an already highly structured exhaustive GPU solver on its home problem in a scientifically meaningful way.

Prompt:

```text
I want an honest methodological answer about evaluation goals.

Using the attached repo context and broad literature search, answer this question:

Is “beat the repo’s efficient `double_ma` brute-force implementation” a scientifically sound target for a new strategy-agnostic optimization algorithm, or is that the wrong evaluation framing?

Constraints:
- Distinguish exact optimum recovery, simple regret, anytime performance, wall-clock under budget, and cross-problem generalization.
- Explain when exhaustive search is the wrong comparator and when it is the right comparator.

Deliverables:
1. Explain why this goal is or is not well-posed.
2. Propose a better evaluation target if needed.
3. Define the benchmark family that would be required for a fair claim.
4. State whether `double_ma` should be:
   - the main thesis benchmark
   - one calibration benchmark
   - or only a sanity check
5. Explain how a custom CUDA implementation would fit as execution substrate rather than novelty.
6. State what claims would still be valid if the exact same optimizer were first prototyped on CPU.
```

## Question 3: Could Multi-Fidelity Scheduling Be The Real Algorithmic Opportunity?

Why this matters:

- This application naturally has possible cheaper proxies: shorter horizons, reduced datasets, smaller contracts, reduced validation, or coarse search stages.
- Multi-fidelity optimization is saturated in general, but there may still be room if the fidelity structure here is exact, domain-grounded, and batch-native.

Prompt:

```text
I want to know whether the strongest algorithmic opportunity here is not a brand-new optimizer, but a better fidelity scheduler.

Using the attached repo context and broad literature search, answer this question:

Could this application support a novel multi-fidelity optimization research direction, where lower-cost but biased or partial backtest evaluations are used to guide batch-synchronous search over massive strategy spaces?

Constraints:
- Compare against classical and modern multi-fidelity BO / tree-search work.
- Do not assume novelty merely because the application is in finance.
- Be explicit about what fidelity controls in this repo would actually be defensible.

Deliverables:
1. Define the most credible fidelity dimensions available in this application.
2. Classify each candidate fidelity by:
   - expected cost reduction
   - expected ranking preservation
   - expected bias risk
   - whether it is exact, approximate, or semantically dangerous
3. State whether the novelty would be in the scheduler, the fidelity model, or only in the application.
4. Define the minimum prototype and theory or guarantees needed.
5. Define the ablations needed to show that gains come from fidelity management rather than cheaper execution alone.
6. Explain how a custom CUDA implementation would serve the method without becoming the claimed novelty.
7. State whether this direction is stronger than a generic new optimizer proposal.
```

## Question 4: Could Online Algorithm Selection Be Stronger Than One New Optimizer?

Why this matters:

- The repo already has heterogeneous search surfaces and strategy families.
- A portfolio selector that chooses between exhaustive, local, surrogate-guided, or fidelity-aware search may be more credible than claiming one optimizer dominates everything.

Prompt:

```text
I want to evaluate whether algorithm selection is a better research direction than inventing one new optimizer.

Using the attached repo context and broad literature search, answer this question:

Could an online algorithm-selection or optimizer-portfolio system for heterogeneous strategy search spaces be a stronger research direction for this application than proposing one new strategy-agnostic optimizer?

Constraints:
- Compare against existing automated algorithm selection / portfolio work in black-box optimization.
- Distinguish offline portfolio construction from online per-instance or per-stage switching.
- Tie the answer to the repo’s actual diversity of strategy/search surfaces.

Deliverables:
1. Explain the strongest portfolio-selection question this repo could pose.
2. Distinguish static features, cheap probe features, and online runtime features.
3. State whether the novelty would be in meta-features, online switching, cost-awareness, or GPU-batch constraints.
4. Explain what the cold-start policy should be when no prior performance data exists.
5. Define the minimal benchmark suite and baselines.
6. Include at least:
   - a fixed single-optimizer baseline
   - a heuristic portfolio baseline
   - an oracle upper-bound thought experiment
7. Explain how a custom CUDA implementation would fit the story without becoming the central claim.
8. State whether this is a stronger paper than “new optimizer X”.
```

## Question 5: Is There A Generalizable Exact-Pruning Or Certified-Elimination Direction?

Why this matters:

- If you want something more “pure CS”, exact pruning with guarantees is stronger than heuristic tuning.
- The problem is that truly strategy-agnostic exact pruning may be impossible; the real question is whether a semi-agnostic framework exists.

Prompt:

```text
I want to evaluate whether this application could support an exact or certified elimination research direction.

Using the attached repo context and broad literature search, answer this question:

Is there a plausible research direction in exact pruning or certified elimination for large strategy search spaces, where the system can provably discard candidates without losing the optimum, and can such a method be at least partly strategy-agnostic?

Constraints:
- Distinguish exact guarantees from heuristic pruning.
- Compare against branch-and-bound, optimistic optimization, hierarchical partition, and elimination-style methods.
- Be honest about whether strategy-agnostic exact pruning is realistic.

Deliverables:
1. State whether a meaningful exact-pruning question exists here.
2. Explain what assumptions would be required.
3. Define the weakest acceptable notion of guarantee.
4. State whether this direction is likely stronger or weaker than the batch-optimizer direction.
```

## Question 6: Could A Search-Space Compiler Be More Novel Than A New Optimizer?

Why this matters:

- This app already has structured strategy definitions, parameter grids, indicator contracts, and route metadata.
- A compiler that converts strategy definitions into optimizer-aware representations, fidelity ladders, and feasible search decompositions may be more original than another search heuristic.

Prompt:

```text
I want to evaluate a compiler / systems angle rather than a new black-box optimizer.

Using the attached repo context and broad literature search, answer this question:

Could a system that compiles high-level trading-strategy search definitions into optimizer-aware mixed search spaces, fidelity ladders, and evaluation plans be a more defensible research direction than inventing another optimizer?

Constraints:
- Compare against search-space design frameworks, AutoML search-space compilers, grammar-based search representations, and optimizer-aware DSL work.
- Do not treat repo-specific plumbing alone as novelty.

Deliverables:
1. Define the strongest compiler/system question available here.
2. State whether the contribution would be systems, languages, or optimization infrastructure.
3. Define the minimum prototype and evaluation plan.
4. State whether this is likely more or less defensible than the optimizer-algorithm direction.
```

## Question 7: Is There A Benchmark-Suite Or Evaluation Paper Here If Algorithmic Novelty Fails?

Why this matters:

- If the algorithmic directions are too saturated, there may still be value in a benchmark/evaluation contribution focused on expensive mixed strategy search with exact evaluators and GPU-batch constraints.
- This is weaker than a strong algorithm paper, but it is still a real fallback.

Prompt:

```text
I want to know whether the application could support a benchmark or evaluation paper if the algorithmic direction is not novel enough.

Using the attached repo context and broad literature search, answer this question:

Could this application become the basis for a useful benchmark suite or evaluation paper on expensive mixed search-space optimization under exact batch evaluation constraints?

Constraints:
- Compare against existing black-box optimization benchmarks, AutoML benchmarks, NAS benchmarks, and artifact-oriented systems papers.
- Do not assume that having many measurements is enough.

Deliverables:
1. State whether such a benchmark/evaluation paper would be genuinely useful.
2. Identify the exact benchmark gap it would fill.
3. Define the minimum benchmark packaging and baseline set.
4. State whether this is a respectable fallback or not worth pursuing.
```

## Question 8: What Three Directions Are Actually Worth A Serious Next Month?

Why this matters:

- You probably do not want a list of twenty speculative ideas.
- You want a brutally selective recommendation on what is actually worth a real block of time.

Prompt:

```text
I want a final prioritization, not a broad brainstorm.

Using the attached repo context and broad literature search, answer this question:

If I only devote one month of serious work to finding a publishable pure-CS direction from this application, which three directions are actually worth pursuing, and in what order?

Constraints:
- Prefer directions that are most defensible as CS rather than CE.
- Penalize ideas that are too saturated or require hand-waving novelty claims.
- Penalize ideas whose evaluation protocol would be weak or too easy to game.

Deliverables:
1. Rank the top three directions.
2. Give one paragraph of justification for each.
3. Give one month-scale first step for each.
4. State which tempting directions should be explicitly rejected.
```

## Directions That Are Probably Too Weak

By default, I would treat these as low-confidence or saturated unless the model can argue otherwise:

- a generic new GPU metaheuristic with no guarantees and no strong mixed-space story
- “beat `double_ma` brute force” as the main paper thesis
- another plain BO variant without a clearer computational or theoretical angle
- another purely empirical SASS-guided tuning story
- a post-compilation branch without a much stronger scientific question

## Recommended Starting Order

If you only run four threads, use this order:

1. `Question 1: Is A Strategy-Agnostic, Batch-Synchronous Optimizer The Best Generalization Path?`
2. `Question 2: Is “Beat The Efficient double_ma Brute-Force Baseline” Even The Right Goal?`
3. `Question 4: Could Online Algorithm Selection Be Stronger Than One New Optimizer?`
4. `Question 3: Could Multi-Fidelity Scheduling Be The Real Algorithmic Opportunity?`

That sequence should quickly tell you whether the real project is:

- a new optimizer
- a meta-optimizer / portfolio selector
- a fidelity scheduler
- or not strong enough as a CS direction
