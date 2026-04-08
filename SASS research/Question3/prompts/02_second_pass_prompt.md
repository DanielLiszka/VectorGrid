# Second-Pass Prompt

Question: `Question 3: Would Optimizer Portfolios Or Online Algorithm Selection Be Stronger Than One Universal Optimizer?`

Reuse the same copied attachment set from the first pass:

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
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/strategy_presets.rs`

Use this only after the first-pass answer exists.

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
