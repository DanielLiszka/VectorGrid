# First-Pass Prompt

Attach the copied files from this bundle that mirror these original repo-relative paths:

- `AGENTS.md`
- `REPO_MAP.md`
- `README.md`
- `SASS research/vectorgrid_higher_math_cs_research_prompt_pack_2026-04-08.md`
- `SASS research/double_crossover_phase2_research_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_research_artifact_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_external_review_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_micro_attribution_2026-04-03.md`
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

Use this prompt:

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

Expanded sub-questions to force:

- Does the repo expose any real monotonicity, decomposability, admissible upper/lower bounds, partial orders, fidelity distortions that might preserve ranking, delayed-feedback cost structure, or route-dependent feasibility/cost algebra?
- Which of those are actually strong enough to support a theorem-like claim?
- Which assumptions would only hold for `DoubleMa`, and which might extend to `CompositeTrend`, `BreakoutMomentum`, or `RegimeHybrid`?
- Is the strongest mathematical contribution a positive theorem, a correctness theorem for a structure extractor, a regret or approximation result, or a rigorous negative result showing why a tempting direction fails?
- If exact pruning is too weakly supported, is distortion-aware multi-fidelity mathematically cleaner?
- If theorem-level guarantees are unrealistic, is there still a systems-and-math hybrid direction with formal semantics and strong empirical evaluation?
