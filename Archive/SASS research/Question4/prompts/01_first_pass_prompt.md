# First-Pass Prompt

Question: `Question 4: Can A Batch-Synchronous, Strategy-Agnostic Optimizer Still Be A Real Contribution?`

Attach the copied files from this bundle that mirror these original repo-relative paths:

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
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `SASS research/double_crossover_phase2_experiment_table_2026-04-03.md`

Use the question-specific prompt below together with the shared guidance from the source pack.

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
