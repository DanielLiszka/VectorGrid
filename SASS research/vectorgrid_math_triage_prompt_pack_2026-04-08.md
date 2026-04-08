# `VectorGrid` Math Triage Prompt Pack

Date: 2026-04-08

Purpose: a narrow prompt pack for GPT-5.4 Pro / Research Mode that is not about generating more directions. It is about killing weak math quickly and isolating the few theorem-worthy or negative-result-worthy directions that survive.

Use this after the broader CS pack or the higher-math pack when you want a harder screen.

## Why This Pack Exists

The main failure mode now is not “too few ideas.”

It is:

- fake theorem candidates built on unrealistic assumptions
- ornamental notation wrapped around a systems idea
- or mathematically attractive directions that are too repo-specific to matter as CS

This pack is meant to force three things:

1. theorem candidate screening,
2. counterexample and assumption attacks,
3. and, if needed, conversion of a failed positive-theorem direction into a stronger negative-result or limitation statement.

## Highest-Value Math Questions To Triage

These are the best math-forward candidates worth screening hard:

1. **Distortion-aware multi-fidelity screening**
   - Is there a fidelity ladder with enough ranking preservation to support safe or near-safe screening?
2. **Restricted certified elimination**
   - Not “strategy-agnostic exact pruning everywhere,” but whether some limited families admit admissible bounds or certificates.
3. **Structure-extraction correctness**
   - Can a compiler/IR layer formally preserve feasibility, conditionality, and cost/fidelity structure?
4. **Batch-delayed budgeted optimization**
   - Is the synchronous batch-cost model itself sharp enough to support a meaningful regret or cost-aware formalization?
5. **Negative result / impossibility framing**
   - Is the strongest math contribution actually a rigorous statement that certain attractive generic guarantees are not credible here?

## Priority Order

If you want the highest expected value, triage in this order:

1. distortion-aware multi-fidelity screening
2. structure-extraction correctness
3. restricted certified elimination
4. negative-result framing
5. batch-delayed regret theory

Reason:

- the first two have the best chance of being both mathematically explicit and repo-grounded
- the third has high upside but likely narrower scope
- the fourth is valuable if positive theorem directions collapse
- the fifth is plausible, but most exposed to prior-work collision and “the math is not the contribution” reviewer reactions

## Assumption Toxicity Checklist

Force the model to classify every important assumption:

### Green: repo-supported or close to it

- explicit conditional search structure from request builders / capability logic
- route-dependent feasibility or cost differences
- finite mixed discrete/continuous/conditional spaces
- validation-stage cost differences
- existence of an exact evaluator for at least some structured families
- synchronous batch evaluation as a real runtime constraint

### Yellow: plausible but not yet established

- useful ranking preservation under cheaper fidelities
- instance-dependent bounds on some restricted strategy families
- cheap meta-features that predict optimizer suitability
- cost models that are stable enough to formalize across workloads

### Red: usually fake unless directly justified

- global smoothness or convexity
- clean Lipschitz constants on the full objective
- broad submodularity or monotonicity
- exact pruning guarantees across heterogeneous strategy families
- assumptions that depend on one particular kernel implementation detail

If the theorem depends mainly on red assumptions, kill it.

## Shared Baseline Files

Attach these:

- `AGENTS.md`
- `REPO_MAP.md`
- `README.md`
- `SASS research/vectorgrid_broader_cs_research_question_pack_2026-04-05.md`
- `SASS research/vectorgrid_higher_math_cs_research_prompt_pack_2026-04-08.md`
- `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_research_artifact_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_external_review_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`
- `SASS research/double_crossover_phase2_experiment_table_2026-04-03.md`
- `CUDA kernel review/CUPTI/cupti_sass_metric_followup_findings_2026-04-05.md`
- `CUDA kernel review/Prompt1/xlarge_cache_reuse_probe_findings_2026-04-04.md`
- `CUDA kernel review/Prompt2/tm_native_vwma_pilot_findings_2026-04-04.md`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/strategy_presets.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/lib.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

## Prompt 1: Theorem Candidate Screen

Use this first.

```text
I want you to act as a theorem-candidate screener for this repository.

Using the attached repo context and broad literature search, identify which mathematically explicit research directions are actually worth theorem-level effort here, and which should be rejected before more time is spent.

Constraints:
- Do not generate a broad idea list.
- Focus on directions that are both repo-grounded and potentially valid as CS.
- For each direction, explicitly classify the critical assumptions as:
  - repo-supported
  - plausible but unverified
  - unrealistic
- Reject directions that depend mainly on unrealistic assumptions.

Candidate direction families to evaluate:
- distortion-aware multi-fidelity screening
- restricted certified elimination / exact pruning
- structure-extraction correctness for a compiler or IR layer
- batch-delayed budgeted optimization theory
- a mathematically meaningful negative-result or impossibility direction

Deliverables:
1. Rank these direction families from strongest to weakest.
2. For each family, give:
   - the exact formal object that would be studied
   - the strongest plausible theorem / guarantee / correctness claim
   - the assumptions required
   - the closest prior work
   - the strongest likely reviewer objection
3. State which directions should be killed immediately.
4. Choose one leading positive-theorem direction and one leading negative-result fallback.
5. State the minimum proof burden for each of those two survivors.
```

## Prompt 2: Counterexample Attack

Use this after Prompt 1 on the leading survivor.

```text
Attack the leading theorem candidate from the prior answer.

Do not defend it. Try to break it.

Tasks:
1. Construct the strongest plausible counterexamples, edge cases, or repo-grounded failure scenarios.
2. Identify where the assumed structure fails:
   - non-smooth objective regions
   - ranking inversions across fidelities
   - lack of admissible bounds
   - strategy-family heterogeneity
   - route-dependent behavior that breaks the claim
3. Separate:
   - fatal counterexamples
   - repairable scope restrictions
   - harmless edge cases
4. State whether the candidate can be saved only by narrowing scope to:
   - DoubleMa only
   - a restricted family with explicit assumptions
   - a correctness-only compiler/IR result
   - or a negative-result framing

Deliverables:
1. Fatal counterexamples
2. Repairable restrictions
3. The narrowest version of the claim that still looks publishable
4. The point at which the claim becomes too narrow to matter
5. A keep / narrow / abandon verdict
```

## Prompt 3: Negative-Result Conversion

Use this if the positive theorem path weakens too much.

```text
Assume the strongest positive-theorem direction for this repository is weaker than initially hoped.

Using the attached repo context and broad literature search, determine whether the stronger contribution is actually a negative result, limitation result, or impossibility-style argument.

Targets to examine:
- why broad exact pruning is not credible across heterogeneous strategy families
- why naive fidelity shortcuts are mathematically dangerous without distortion control
- why “beat exact brute force” is the wrong universal theorem target
- why batch-synchronous search may resist clean universal guarantees without stronger structure

Deliverables:
1. The strongest mathematically explicit negative-result statement that might be defensible
2. The assumptions needed
3. Whether the result would still count as useful CS
4. What empirical evidence would be needed to support the claim
5. Whether this negative-result path is stronger than forcing a weak positive theorem
```

## Prompt 4: Final Triage Memo

Use this last.

```text
Write a final triage memo for the math-forward direction as if it will be checked into the repo.

Choose one outcome:
- pursue a positive theorem direction
- pursue a correctness-and-systems direction
- pursue a negative-result direction
- or close the math-heavy path for now

Deliverables:
1. The adopted outcome
2. The exact formal target
3. The assumptions that survive and the assumptions that were rejected
4. The minimum proof or correctness obligations
5. The minimum benchmark obligations
6. The kill test
7. Whether implementation work should reopen now or only after more design work
```

## Bottom Line

This pack is useful if you suspect the next risk is not “missing another idea,” but “believing weak math too early.”
