# Final Consolidation Prompt

Question: `Question 8: Which Venue Family Best Fits The Strongest Remaining Direction?`

Reuse the same copied attachment set from the first and second passes:

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
- `SASS research/double_crossover_phase2_paper_outline_2026-04-03.md`

Use this only after the second-pass answer exists.

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
