# `VectorGrid` Higher-Math CS Research Prompt Pack

Date: 2026-04-08

Purpose: a math-forward prompt pack for GPT-5.4 Pro / Research Mode that is still grounded in this repo and still aimed at a valid CS contribution.

This file is for pressure-testing whether there is a mathematically explicit research problem here that is stronger than:

- more CUDA engineering
- a generic optimizer story
- or domain-specific finance claims that do not travel outside the application

## Core Requirement

The target is not "more math-sounding language."

The target is:

1. a formal problem statement that is actually induced by this repository,
2. a plausible theorem-level or guarantee-level claim,
3. a clear distinction between assumptions that are real here and assumptions that are wishful thinking,
4. and an empirical program that would still matter if the final runtime were implemented with some fast backend other than the current CUDA path.

If the prompt produces only:

- notation around a generic black-box optimizer,
- a proof target with unrealistic smoothness or convexity assumptions,
- or a claim that depends entirely on the current kernel implementation,

then the direction is not strong enough.

## What Counts As “Higher-Level Math” Here

The strongest math-forward directions are likely to look like one of these:

1. **Exact pruning / certified elimination**
   - admissible bounds
   - partial orders
   - branch-and-bound style elimination
   - instance-dependent certificates
2. **Distortion-aware multi-fidelity optimization**
   - formal ranking preservation
   - bounded distortion
   - safe early elimination under fidelity ladders
3. **Batch-delayed regret or budgeted optimization theory**
   - one synchronous GPU batch as one optimization round
   - delayed feedback at batch boundaries
   - wall-clock or cost-aware simple-regret targets
4. **Structure extraction with formal semantics**
   - a compiler / IR / normalizer that extracts conditional spaces, feasibility constraints, fidelity ladders, and route-dependent costs
   - correctness claims about the extracted structure
5. **Impossibility / no-free-lunch style negative result**
   - a rigorous argument that some tempting direction is mathematically too weak or too assumption-heavy in this domain

## What Does Not Count

These are weak forms of “math” for this repo:

- pure finance-specific modeling claims with no reusable CS question
- complexity notation wrapped around a generic optimizer with no sharper formal setting
- Lipschitz, convexity, or smoothness assumptions that are not credible for the actual objective surfaces
- proofs about a kernel implementation that do not survive a backend change
- a theorem that only applies to `double_ma` unless the paper is explicitly about that restricted family

## Shared Baseline Files

Attach these to the math-focused prompt:

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

## Additional Math-Focused Files

Attach these in addition:

- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/strategy_presets.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/lib.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`

## Shared Framing

Force the model to keep these distinctions explicit:

1. **What is the mathematical object?**
   - search space
   - conditional structure
   - objective family
   - fidelity family
   - cost model
   - batch / feedback model
2. **What assumptions are really supported by the repo?**
   - repo-supported
   - plausible but unverified
   - unrealistic
3. **What is the strongest claim type?**
   - exact guarantee
   - approximation guarantee
   - regret bound
   - distortion / rank-preservation statement
   - impossibility / limitation result
   - correctness property for an extracted IR or compiler
4. **What survives without the current CUDA backend?**
   - the math should survive backend substitution
   - CUDA can matter as substrate, cost model, or batch regime enabler
   - CUDA should not be the theorem

## Shared Answer Shape

Ask for this structure:

1. The single best formal problem statement, with notation.
2. A ranked list of mathematically plausible directions.
3. For each direction:
   - assumptions
   - closest prior work
   - theorem candidate or impossibility claim
   - repo fit
   - reviewer risk
4. The leading direction.
5. The minimum proof program.
6. The minimum benchmark program.
7. The kill test.
8. A keep / revise / reject verdict.

## First-Pass Prompt

```text
I want a math-forward, CS-valid evaluation of this repository.

Using the attached repo context and broad literature search, identify the strongest mathematically explicit research problem that this application actually supports.

Constraints:
- Do not give generic optimizer brainstorming.
- Do not give finance-specific mathematical claims unless they induce a reusable CS problem.
- Do not rely on unrealistic smoothness, convexity, or Lipschitz assumptions unless you explicitly mark them as unrealistic.
- Treat the current CUDA implementation as execution substrate unless the mathematical claim truly depends on the synchronous batch-cost model rather than on kernel internals.
- For every proposed theorem, guarantee, or impossibility statement, label its assumptions as:
  - repo-supported
  - plausible but unverified
  - unrealistic

Deliverables:
1. Define the single best formal problem statement with notation:
   - search space X
   - objective f
   - cost model c
   - fidelity parameter or hierarchy if relevant
   - batch budget / delayed feedback model if relevant
2. Rank the strongest mathematically plausible directions, such as:
   - exact pruning / certified elimination
   - distortion-aware multi-fidelity optimization
   - batch-delayed regret or cost-aware simple-regret analysis
   - structure extraction / compiler semantics with correctness guarantees
   - a mathematically strong negative-result direction
3. For each ranked direction, provide:
   - the required assumptions
   - the closest 3 to 5 prior works
   - the strongest plausible theorem or guarantee form
   - the strongest likely reviewer objection
   - whether the direction generalizes beyond double_ma
4. Choose one leading direction and explain why it is strongest.
5. Define the minimum proof program:
   - what must actually be proved
   - what would remain empirical only
6. Define the minimum benchmark and ablation matrix needed to keep the work honest.
7. State the kill test that would end the direction early.
8. Give a keep / revise / reject verdict.
```

## Expanded Sub-Questions To Force

- Does the repo expose any real:
  - monotonicity
  - decomposability
  - admissible upper/lower bounds
  - partial orders
  - fidelity distortions that might preserve ranking
  - delayed-feedback cost structure
  - route-dependent feasibility or cost algebra
- Which of those are actually strong enough to support a theorem-like claim?
- Which assumptions would only hold for `DoubleMa`, and which might extend to `CompositeTrend`, `BreakoutMomentum`, or `RegimeHybrid`?
- Is the strongest mathematical contribution:
  - a positive theorem,
  - a correctness theorem for a structure extractor,
  - a regret or approximation result,
  - or a rigorous negative result showing why a tempting direction fails?
- If exact pruning is too weakly supported, is distortion-aware multi-fidelity mathematically cleaner?
- If theorem-level guarantees are unrealistic, is there still a systems-and-math hybrid direction with formal semantics and strong empirical evaluation?

## Second-Pass Prompt Discipline

Use the second pass to kill fake math.

Require the model to:

- identify which assumptions are doing all the work,
- cut theorem claims that depend on unrealistic structure,
- state whether the leading direction is still interesting if only a weaker guarantee is possible,
- compare the chosen direction against one more empirical fallback,
- and explain where reviewers would say the math is ornamental rather than essential.

## Reusable Second-Pass Prompt

```text
Review your prior answer as a critical second pass.

Do not restate the first answer. Pressure-test the mathematics.

Tasks:
1. Identify the assumptions that are carrying the claimed theorem, guarantee, or impossibility result.
2. Reclassify each important assumption as:
   - repo-supported
   - plausible but unverified
   - unrealistic
3. Cut any direction whose mathematical claim depends mainly on unrealistic assumptions.
4. State whether the leading direction still has value if the theorem is weakened to:
   - a local guarantee
   - an instance-dependent guarantee
   - a distortion bound
   - or only a correctness property plus strong empirical evidence
5. Identify the strongest likely reviewer objection that the math is ornamental rather than essential.
6. Compare the leading direction against:
   - one more conservative systems-oriented fallback
   - one more ambitious but riskier theorem-oriented alternative

Deliverables:
1. Keep / revise / reject verdict on the first-pass leading direction
2. Revised leading direction
3. Strongest fallback direction
4. Explicitly rejected directions
5. Minimum remaining proof obligations
6. Minimum remaining benchmark obligations
7. Kill test
```

## Final Consolidation Prompt Discipline

The final pass should produce one repo-ready answer, not a menu of options.

It must state:

- the adopted math-forward direction,
- the exact formal problem,
- the minimum theorem or correctness claim worth chasing,
- the empirical benchmark slate that keeps the theory honest,
- the directions that were rejected,
- and whether this is strong enough to reopen implementation work.

## Reusable Final Consolidation Prompt

```text
Write a final consolidation memo for this math-forward research direction as if it will be checked into the repo.

Do not provide an equal-weight summary. Choose one outcome.

Deliverables:
1. A one-paragraph statement of the adopted direction
2. The exact formal problem statement
3. The assumptions the direction requires, labeled as:
   - repo-supported
   - plausible but unverified
   - unrealistic but intentionally rejected
4. The minimum theorem, guarantee, impossibility result, or correctness claim worth pursuing
5. Why the rejected alternatives were rejected
6. The minimum empirical benchmark matrix
7. The direction-specific kill test
8. A final verdict:
   - reopen implementation
   - design-only follow-up
   - or close the direction

Write it in a repo-ready style with crisp sections and no decorative math language.
```

## Bottom Line

If you want one additional prompt that is more mathematical than the broader CS pack, this is the right one to run.

It should tell you whether there is:

- a real theorem-worthy direction,
- a weaker but still respectable systems-and-math direction,
- or only attractive-sounding math that does not survive contact with the repo.
