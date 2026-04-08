# GPT-5.4 Pro Prompt Pack For Options-Expression Research

Date: 2026-04-08
Status: repo-specific prompt pack for researching the current options-expression lane with GPT-5.4 Pro, with emphasis on the current full-GPU target rather than generic options-simulator feature growth

Use this document when you want higher-quality external reasoning about the current options-expression optimizer without losing the repo's actual constraints.

This pack is intentionally shaped around the current product and architecture:

- artifact-driven follow-up workflow
- current parent-family support only
- full-GPU productization as the target direction
- dense compiled tensors as the core abstraction
- robustness and validation as part of the same workflow
- strict skepticism toward ideas that weaken tensor regularity, cache reuse, or measured GPU economics

The goal is not to ask GPT-5.4 Pro for broad "what features should I add?" advice.

The goal is to get better insight into:

- what to research next
- what to reject early
- what evidence actually matters
- how to sharpen the current full-GPU options path without drifting into a general simulator

## Recommended Usage Pattern

Before using any of the prompts below:

1. Paste a short repo-grounded context block.
2. Paste only the evidence relevant to the question.
3. Ask for explicit assumptions, measurable gates, and failure modes.
4. Reject hand-wavy or simulator-bloat answers.

Recommended context block:

```text
You are helping me research next-step architecture and product decisions for a GPU-first options-expression optimizer inside an existing desktop trading app.

Hard constraints:
- This is an artifact-driven follow-up workflow over already-validated parent signals.
- The current product direction is full GPU execution wherever feasible.
- The system depends on compiling irregular options data into dense candidate tensors and dense exit-summary tensors.
- Any proposed expansion must preserve tensor regularity, cache reuse, and measurable GPU advantage.
- CPU correctness matters, but the research target is the full-GPU path, not a CPU-first product.
- I do not want generic “add more features” advice or LEAN-style simulator sprawl.

When you answer:
- Separate observations, hypotheses, and recommendations.
- Call out hidden assumptions explicitly.
- Prefer measurable gates, failure modes, and experiments.
- Reject ideas that likely destroy GPU economics.
- Treat architecture drift as a real cost.
```

Recommended evidence block template:

```text
Current state:
- supported parent families: DoubleMa, CompositeTrend, RegimeHybrid, BreakoutMomentum
- supported structure families: long_single, debit_vertical, long_pair
- realism lanes: aggregate, derived-mid, quote-aware
- direct provider download: bounded 1m aggregate path only
- current GPU auto policy: density >= 30%, invalid-slot rate <= 70%, compiled tuples >= 256, search events >= 8, projected work >= 10,000, measured speedup >= 1.25x

Recent evidence:
- candidate density: [value]
- invalid-slot rate: [value]
- structural cache reuse: [value]
- summary cache reuse: [value]
- GPU ranking speedup vs CPU: [value]
- estimated backend preview wall speedup vs CPU: [value]
- dominant invalid reasons: [list]
```

## Output Rules To Add To Most Prompts

These add friction in a good way. Paste them after the main question when you want stronger answers.

```text
Required output:
- observations
- hypotheses
- recommendations
- assumptions that could invalidate the answer
- one highest-value next experiment
- clear success and failure gates

Do not:
- propose generic options-platform expansion
- assume CPU-first workflow priorities
- recommend broad feature growth without explaining tensor and GPU impact
- hide uncertainty behind vague language
```

## Prompt 1: Research Direction Triage

Use this when you want GPT-5.4 Pro to tell you where to spend effort next.

```text
Act as a skeptical research reviewer for a GPU-first options-expression optimizer.

Current state:
[paste context]

Evidence:
[paste evidence]

Question:
What are the best research directions from here if I care most about full-GPU productization, validation quality, and preserving the current options-expression lane?

Rank the top 5 directions.

For each direction provide:
- why it matters
- why it is compatible or incompatible with full GPU execution
- expected implementation complexity
- expected benchmarkability
- main architecture risk
- a one-line kill criterion

Then give:
- the single best next step
- the single most tempting but misleading direction to avoid
```

## Prompt 2: Full-GPU Bottleneck Analysis

Use this when you already have benchmark or feasibility output and want a hard diagnosis.

```text
You are acting as a GPU performance economist, not a general architecture advisor.

I have a GPU-first options-expression optimizer that compiles candidate and exit-summary tensors and then scores them on CPU or GPU.

Current evidence:
[paste feasibility report, phase05 output, or summarized numbers]

Question:
Given this evidence, what are the top 3 bottlenecks preventing the current options lane from becoming confidently full-GPU-first in product use?

For each bottleneck provide:
- what the evidence says
- whether it is a data problem, compile-shape problem, search-space problem, planner problem, or UI/policy problem
- what experiment would best reduce uncertainty
- what metric would prove the bottleneck is improved

Conclude with:
- whether the next step should focus on compile density, invalid-slot reduction, planner policy, or validation presentation
```

## Prompt 3: Invalid-Slot Reason Analysis

Use this when the main issue is density or search waste.

```text
I want you to analyze invalid-slot reasons in a GPU-first options-expression optimizer.

Current invalid-reason breakdown:
[paste reasons and counts]

Current search-space shape:
- delay buckets: [paste]
- DTE buckets: [paste]
- moneyness buckets: [paste]
- structure family: [paste]
- realism lane: [paste]

Question:
Which invalid reasons should be attacked directly, and which should instead reshape the default search space?

Output:
- one diagnosis per invalid reason
- which reasons are reducible by better data
- which reasons are reducible by better contract selection
- which reasons are reducible by search-default changes
- which reasons should be accepted as a product boundary
- the best ablation study to run next
- the risk of improving density in a misleading way
```

## Prompt 4: Auto-Backend Policy Critique

Use this when deciding whether `auto` is too conservative or too aggressive.

```text
Act as a reviewer trying to prevent a bad product default.

Current auto-backend policy:
[paste thresholds]

Recent benchmark results:
[paste evidence]

Question:
Critique the current auto-backend policy for a GPU-first options-expression optimizer.

Output:
- where the policy is too conservative
- where it is too aggressive
- which thresholds should remain static
- which thresholds should be benchmark-derived
- a proposed revised policy
- how the policy should explain itself to users in UI/artifacts
- what evidence would justify promoting GPU more aggressively
```

## Prompt 5: Candidate-Compiler Research Prompt

Use this when you suspect the biggest gains are before scoring.

```text
I want research ideas for improving the candidate compiler in a GPU-first options-expression optimizer.

Current system:
- parent signals are fixed
- candidate compilation maps each event into deterministic candidate slots
- scoring only happens after irregular data has been compiled away

Current evidence:
[paste density, invalid rates, dominant failure modes, cache reuse]

Question:
What are the highest-leverage candidate-compiler research directions that improve full-GPU viability without turning the system into dynamic chain search?

Output:
- top 5 ideas
- why each helps or hurts tensor density
- whether each belongs in structural compilation, static filtering, or search-space defaults
- expected impact on cache reuse
- easiest idea to benchmark honestly
- ideas that look attractive but would likely degrade GPU economics
```

## Prompt 6: Structure-Family Viability Prompt

Use this before expanding beyond current structure families.

```text
I want to evaluate whether a new options structure family belongs in a GPU-first compiled-tensor optimizer.

Current supported structures:
- long_single
- debit_vertical
- long_pair

Candidate structure:
[describe it]

Assume these constraints:
- deterministic contract selection
- no arbitrary scripting
- no rolling
- no assignment/exercise modeling
- no mixed-expiry generalization unless explicitly justified
- full GPU execution remains the target

Evaluate the candidate against:
- fixed geometry
- same-entry timing compatibility
- same-expiry compatibility
- deterministic companion-leg construction
- candidate density risk
- exit-summary regularity
- scoring branchiness
- likely cache behavior

Output:
- viable / maybe viable / not viable
- what must be precompiled
- what should remain outside the hot path
- what benchmark would validate inclusion
- what failure would cause you to reject it
```

## Prompt 7: Validation-Depth Research Prompt

Use this when you want better decision quality without adding a separate workflow.

```text
My current options-expression workflow already includes:
- search ranking
- holdout metrics
- sealed-finalist evaluation
- stress lanes
- validation verdicts

I want research ideas that improve trust and selection quality without creating a separate validation product branch.

Question:
What are the best next-step validation improvements that fit a GPU-first options-expression optimizer?

Output:
- 5 concrete improvements
- whether each belongs in hot-path scoring, finalist-only rescoring, or UI/reporting only
- what decision failure each improvement is meant to prevent
- which improvements are likely low-cost enough to justify inclusion
- which improvements are likely to look helpful but mainly add workflow drag
```

## Prompt 8: Search-Winner Vs Robust-Winner Prompt

Use this when the search winner and the trustworthy winner may diverge.

```text
I want you to analyze disagreement between search winners and robust winners in an options-expression optimizer.

Current metrics available:
- selection metrics
- holdout metrics
- sealed-finalist metrics
- stress metrics
- validation verdict
- robust score

Question:
How should I reason about ranking, promotion, and rejection when the search winner is not the most robust winner?

Output:
- a principled ranking philosophy
- when to preserve search ranking
- when to override with robust ranking
- when to surface disagreement without overriding ranking
- what product language should distinguish “best search tuple” from “best trustworthy tuple”
- a decision table with concrete conditions
```

## Prompt 9: Realism-Lane Research Prompt

Use this when comparing aggregate, derived-mid, and quote-aware lanes.

```text
My options-expression workflow supports these realism lanes:
- aggregate
- derived-mid
- quote-aware

Current constraint:
- direct provider download remains bounded to the aggregate lane
- quote-aware and derived-mid are currently import-driven lanes

Question:
How should I deepen realism-lane quality and interpretation without compromising the full-GPU product path?

Output:
- what should remain the canonical optimization lane
- what should be finalist-only or comparison-only
- how to interpret disagreement across lanes
- what metrics best capture realism fragility
- when quote-aware research materially changes product decisions
- which realism ideas should stay out of the hot path
```

## Prompt 10: Planner And Memory Prompt

Use this for GPU planner and chunking work.

```text
I want critique and research ideas for the GPU planner in a compiled-tensor options-expression scorer.

Current GPU planner evidence:
[paste tuple chunk count, chunk size, memory budget source, tensor bytes, total device bytes, fit notes, cache usage]

Question:
What planner changes are most likely to improve product reliability and measured GPU advantage without destabilizing correctness?

Output:
- planner weaknesses
- planner improvements worth testing
- what to keep static vs adaptive
- what telemetry should be surfaced
- what failure modes should be guarded explicitly
- what benchmark scenarios the planner must survive
```

## Prompt 11: Benchmark-Pack Design Prompt

Use this when you need research discipline around performance claims.

```text
Design a benchmark pack for a GPU-first options-expression optimizer.

The benchmark pack must answer:
- whether compile density is good enough
- whether invalid-slot rate is acceptable
- whether cache reuse is strong enough
- whether GPU scorer speedup is meaningful
- whether estimated end-to-end preview wall time favors GPU
- whether validation coverage quality is improving or degrading

Current product shape:
[paste current scope]

Output:
- benchmark matrix dimensions
- minimum canonical workloads
- what should count as a representative workload
- promotion gates vs diagnostic-only metrics
- how to avoid misleading small-workload wins
- how to detect fake progress
```

## Prompt 12: Research Roadmap Prompt

Use this when you want a disciplined path forward rather than isolated ideas.

```text
Design a research roadmap for moving from:
"GPU works on bounded canonical workloads"
to
"GPU is the normal path for this options-expression lane."

Constraints:
- full GPU execution is the target
- do not widen into new workflow branches
- do not recommend generic simulator features
- preserve the current artifact-driven options lane

Output:
- 3 phases maximum
- objective exit criteria for each phase
- what must be measured in each phase
- what would count as failure
- what should explicitly remain out of scope
- what evidence would justify stopping early
```

## Prompt 13: Scope-Rejection Prompt

Use this when an idea is attractive but may not fit the architecture.

```text
I want you to act as a hostile reviewer of this proposed feature for a GPU-first options-expression optimizer:

[paste feature idea]

Question:
Why might this feature be a bad idea in the current architecture, even if it sounds productively “more advanced”?

Output:
- strongest case against the idea
- what it would do to tensor regularity
- what it would do to cache reuse
- what it would do to planner/scorer simplicity
- whether it belongs in a CPU post-pass instead
- the one condition under which it might become acceptable later
```

## Prompt 14: Parent-Adapter Research Prompt

Use this when you want to improve supported parent-family quality without widening scope.

```text
My current options-expression workflow supports these parent families only:
- DoubleMa
- CompositeTrend
- RegimeHybrid
- BreakoutMomentum

Question:
How should I improve adapter quality, replay determinism, and parent-family-specific diagnostics without widening into more parent families?

Output:
- current likely weak points in parent adaptation
- what should be family-local vs shared
- tests that best protect replay determinism
- what metadata should be surfaced to help diagnose adapter-specific issues
- what not to abstract prematurely
```

## Prompt 15: Product-Language Prompt

Use this when you want better artifact/UI language around backend selection and trust.

```text
I want better product language for a GPU-first options-expression optimizer.

Current concepts that need to be explained well:
- auto backend selection
- GPU eligibility
- fallback reason
- search winner
- holdout-supported
- sealed finalist
- stress-fragile
- aggregate vs derived-mid vs quote-aware lanes

Question:
Propose product language that is technically honest, concise, and useful for advanced users.

Output:
- labels
- short descriptions
- warning text
- fallback text
- what should appear in live run results vs saved artifacts
- terms I should avoid because they are misleading
```

## Prompt 16: Master Prompt For Broad Research Sessions

Use this when you want one strong prompt instead of a specialized one.

```text
Act as a skeptical research reviewer for a GPU-first options-expression optimizer.

My target is not a general options simulator. My target is a full-GPU, artifact-driven options optimization lane that preserves dense compiled tensors, high cache reuse, and meaningful embedded validation.

Current state:
[paste concise current-state summary]

Evidence:
[paste benchmark and feasibility numbers]

Question:
What are the most defensible research directions from here if I care most about full-GPU productization and not feature sprawl?

Required output:
- 3 best research directions
- 3 bad-but-tempting directions to avoid
- the single most informative next experiment
- explicit success/failure gates
- what evidence would change your recommendation
- any assumption you think I am underrating
```

## Prompt 17: "Tell Me What I Am Missing" Prompt

Use this when you suspect your current framing is too narrow or biased.

```text
Assume I am too close to this system and may be missing an important architectural or product risk.

Current framing:
[paste your current thesis]

Current evidence:
[paste evidence]

Question:
What are the top 5 risks, blind spots, or false-confidence patterns in my current full-GPU options-expression research direction?

Output:
- one risk per item
- why it is easy to miss
- what evidence would reveal it
- how to test for it cheaply
- whether the risk is architecture, data, benchmarking, or product-interpretation related
```

## Prompt 18: Compare Two Candidate Directions Prompt

Use this when you are choosing between two next steps.

```text
Compare these two research directions for a GPU-first options-expression optimizer:

Direction A:
[paste]

Direction B:
[paste]

Current constraints:
[paste]

Current evidence:
[paste]

Output:
- better near-term direction
- better long-term direction
- expected insight yield of each
- risk of fake progress in each
- benchmarkability of each
- which one should happen first and why
```

## Prompt 19: "Design The Next Experiment" Prompt

Use this when you already know the area and just need a disciplined experiment.

```text
Design the next experiment for this GPU-first options-expression research question:

[paste question]

Current evidence:
[paste]

Output:
- exact hypothesis
- independent variables
- dependent variables
- controls
- benchmark workloads
- expected positive outcome
- expected negative outcome
- what result would be ambiguous
- how to avoid fooling myself
```

## Prompt 20: Anti-Bloat Prompt

Use this when you want GPT-5.4 Pro to actively defend the architecture against sprawl.

```text
Review these candidate ideas as a defender of a narrow, high-performance, GPU-first options-expression optimizer:

[paste ideas]

For each idea, decide:
- keep
- defer
- reject

Your criteria must prioritize:
- full GPU execution viability
- tensor regularity
- cache reuse
- benchmarked product value
- validation clarity

Do not reward feature breadth by default.
```

## Suggested Roles To Assign GPT-5.4 Pro

These role framings usually improve answer quality:

- skeptical architecture reviewer
- GPU performance economist
- benchmark-design critic
- quant product strategist constrained by tensor regularity
- failure-analysis reviewer
- scope-discipline reviewer

Avoid generic roles like:

- expert software engineer
- quant expert
- product manager

Those often drift toward vague breadth and away from the repo's actual constraints.

## What To Paste From The Repo

Good evidence sources for these prompts:

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/commands_options_data.rs`
- latest `output/options_expression_phase05/*.json`
- latest `output/options_expression_phase05_gpu_gate/**/phase05_summary.json`

High-signal values to paste:

- candidate density
- invalid-slot rate
- dominant invalid reasons
- structural cache reuse
- summary cache reuse
- GPU ranking speedup vs CPU
- estimated preview wall speedup vs CPU
- current `auto` thresholds
- supported parent families
- supported structure families
- realism-lane constraints

## Prompting Anti-Patterns

Avoid prompts like:

- "What options features should I add next?"
- "How do I make this comparable to LEAN?"
- "What other strategies should I support?"
- "How can I make this more advanced?"
- "What would make this best-in-class?" without constraints

These tend to produce:

- simulator sprawl
- mixed abstractions
- CPU-shaped advice
- weak benchmark discipline
- architecture drift disguised as ambition

## Short Final Reminder

When in doubt, ask GPT-5.4 Pro to do one of these three things:

1. rank the next research directions under full-GPU constraints
2. critique a proposed expansion against tensor regularity and cache reuse
3. design the next decisive experiment instead of the next large implementation

That will usually generate much better insight than asking for broad product ideation.
