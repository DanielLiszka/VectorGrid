# Double MA Backtest Performance Prompt Pack

Last revised: 2026-04-03

Purpose: a trimmed, high-ROI prompt pack for handing the remaining worthwhile `double_ma` CUDA performance questions to a stronger model such as GPT-5.4 Pro.

This file intentionally replaces the broader April 1 prompt list. Most of the earlier prompts were overtaken by the completed Phase 2 tracker work:

- the bounded lockstep-body pass was already retained
- the shortlist-slice replay study was already completed
- the replay-batching prototype was already retained
- the remaining product-facing upside is no longer in generic SASS re-review, toggle hunting, or another small kernel-local pass

The only prompts kept here are the ones that still look plausibly worth the time.

## How To Use This Pack

- Use one prompt per chat or thread.
- Attach the shared baseline files first.
- Then attach the prompt-specific files.
- Ask the model to stay repo-specific and to distinguish:
  - what the repo already completed
  - what is still genuinely open
  - what should be rejected as low ROI

Important instruction for every thread:

- do not reopen broad PTX or SASS surgery
- do not re-suggest the already completed lockstep-body or replay-batching work
- do not give generic CUDA advice
- keep CPU fallback behavior unchanged
- keep exact finalist semantics unchanged unless the prompt explicitly says otherwise

## Shared Baseline Files

Attach these to every prompt:

- `AGENTS.md`
- `REPO_MAP.md`
- `README.md`
- `SASS research/double_crossover_sass_phase2_runtime_profiling_and_optimization_plan_2026-04-03.md`
- `SASS research/double_crossover_phase2_runtime_profiling_findings_2026-04-03.md`
- `SASS research/double_crossover_phase2_external_review_response_and_task_tracker_2026-04-03.md`

Recommended shared framing for every thread:

- the public benchmark story is still anchored on `double_ma`, `alma/alma`, and exact-grid GPU runs
- the repo already retained one bounded lockstep-body win and one shortlist replay-batching win
- the remaining worthwhile work should be justified by either:
  - broader product payoff, or
  - a clearly differentiated architecture change

## Shared Answer Shape

For any prompt in this pack, ask the model to keep the answer concrete and implementation-oriented.

Minimum answer shape to request when you want a full-quality response:

1. What the repo already completed
2. What remains genuinely unresolved
3. The best remaining direction, if any
4. What should explicitly remain unchanged
5. Concrete code touch points by file and symbol
6. A staged benchmark and validation order
7. Rejection criteria and failure modes

If you need more structure, also ask for:

- a `do now / defer / reject` table
- a short artifact plan
- a note explaining which previously plausible ideas should now be considered closed

## First-Pass Prompt Discipline

For every primary prompt in this pack, prefer answers that do all of the following:

- cite concrete repo files, symbols, runtime paths, and benchmark surfaces
- compare at least:
  - one conservative direction
  - one recommended direction
  - one more aggressive direction
- state which option should actually win now
- separate:
  - adopt now
  - defer
  - reject
- explain what should remain unchanged because it is already working
- translate any web-researched reference back into repo-specific implications rather than leaving it as generic CUDA inspiration

What to avoid:

- generic CUDA best-practice lists detached from this repo
- broad architecture brainstorms with no benchmark plan
- answers that silently reopen already completed lockstep or replay-batching work
- answers that treat every option as equally viable

## Second-Pass Prompt Discipline

For every second-pass prompt in this pack:

- do not merely restate the first answer
- identify contradictions, weak assumptions, and over-designed ideas
- explicitly trim advice that is low-confidence or low-ROI
- collapse the answer toward one winning direction
- use targeted current web research only when it materially sharpens a repo decision
- say what new evidence changed the recommendation, if any

## Final Consolidation Prompt Discipline

For every final consolidation prompt in this pack:

- write as if the result will be checked into `CUDA kernel review/`
- present one coherent repo direction, not an equal-weight summary of alternatives
- include explicit:
  - adopted direction
  - deferred ideas
  - rejected ideas
- include file-level implementation touch points
- include a staged benchmark and verification order
- keep open questions narrow and specific

## Prompt 1: Xlarge Cache, Reuse, And Data Layout Feasibility

### Why This Prompt Still Looks Worth Doing

This is the only cache-oriented lane that still looks plausibly interesting after the current evidence.

Current repo evidence says:

- `large` already looks cache-friendly
- `xlarge`, especially `full17`, is materially more cache-stressed
- current measurements do not yet isolate whether the remaining stress is really price-data reuse, other read-mostly state, or simple working-set growth

That makes an `xlarge`-specific cache/reuse feasibility review potentially worthwhile. A generic "add more caching" prompt is not.

### Attach These Files

Required:

- `SASS research/double_crossover_phase2_cache_hit_measurements_2026-04-03.md`
- `target/nsight/cache_l1l2_alma_double_ma_summary_2026-04-03.json`
- `target/nsight/double_crossover_phase2_ncu_summary_2026-04-03.txt`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

Optional:

- `SASS research/double_crossover_phase2_windowed_replay_kernel_study_2026-04-03.md`
- `SASS research/double_crossover_phase2_replay_batching_log_2026-04-03.md`

### Prompt

```text
You are reviewing the remaining cache and reuse opportunities in the `double_ma` CUDA optimizer in this repo.

Using only the attached files, answer this repo-specific question:

Given that `large` already looks cache-friendly but `xlarge`, especially `full17`, shows materially weaker L1/TEX and L2 hit rates, is there one targeted cache, reuse, or data-layout intervention still worth trying?

Constraints:
- Do not give generic CUDA cache advice.
- Do not suggest broad PTX or SASS work.
- Treat `large` as largely solved unless the attached evidence says otherwise.
- Focus on `xlarge`, especially `alma/alma`, `synth`, and `full17`.
- Keep CPU fallback behavior unchanged.
- Keep exact finalist semantics unchanged.
- Distinguish clearly between:
  - cache-policy changes,
  - data-layout changes,
  - replay/dataflow changes,
  - and ideas that should be rejected.

Deliverables:
1. Summarize what the attached repo evidence already proves about cache behavior on `large` vs `xlarge`.
2. State whether the remaining `xlarge` cache stress looks more like:
   - price-data reuse failure,
   - derived-state footprint,
   - row-contract width pressure,
   - replay/dataflow shape,
   - or working-set growth that cache policy alone will not fix.
3. Propose the single best remaining cache or reuse intervention, if any.
4. Explain why that intervention is more plausible than:
   - explicit price-data persistence alone,
   - generic "add a cache" ideas,
   - or another local kernel-body tweak.
5. Identify the concrete code touch points by file and symbol.
6. Define the smallest benchmark and profiler plan needed to validate or reject the idea.
7. State the exact measurements that would cause you to reject the intervention before implementation.

Be concrete, repo-specific, and evidence-backed.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 1 as a critical second pass.

I want you to pressure-test the cache/reuse recommendation you just made for this repo.

Tasks:
1. Identify weak assumptions, vague claims, and places where your recommendation sounds plausible but is not yet well defended by the attached repo evidence.
2. Identify any ways your proposal may be mistaking:
   - replay/dataflow cost,
   - working-set growth,
   - row-contract width effects,
   - or generic xlarge pressure
   for a true cache-policy problem.
3. If targeted current web research would materially improve the answer, use it selectively to study:
   - Ada L2 persistence behavior
   - practical L1/TEX and L2 cache-policy constraints on RTX 4090
   - software-managed cache or relayout patterns for irregular GPU workloads
4. Compare your proposal against at least one more conservative alternative and one more aggressive alternative.
5. Decide which cache/reuse direction should actually win for this repo now, if any.

Deliverables:
1. `Keep / revise / reject` verdict on your first-pass recommendation
2. The refined cache/reuse direction
3. The strongest competing explanation for the xlarge cache evidence
4. The top implementation risks
5. The exact conditions that would make the repo defer the idea without implementation

Stay repo-specific. Do not broaden this into generic GPU memory-system advice.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 1, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`CUDA kernel review/xlarge_cache_reuse_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current repo evidence summary
3. Final recommendation
4. Explicitly retained current strengths
5. Explicitly deferred ideas
6. Explicitly rejected ideas
7. Concrete code touch points by file and symbol
8. Benchmark and profiler plan
9. Verification plan
10. Rejection criteria and stop conditions

Do not restate all brainstormed options equally. Make one decision and present one coherent direction for this repo.
```

## Prompt 2: TM-Native MA Expansion Beyond SMA And ALMA

### Why This Prompt Still Looks Worth Doing

This is not the best next prompt for immediate `alma/alma` benchmark speed, but it is still one of the few remaining prompts with credible product upside.

If the goal is to broaden GPU coverage and avoid regressing into row-major compute plus transpose for other MA families, this is still a real architectural question. It is more valuable than another prompt about generic runtime toggles or another body-local CUDA tweak.

### Attach These Files

Required:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/Cargo.toml`
- `src-tauri/Cargo.toml`

Optional:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

### Prompt

```text
You are analyzing the MA-preparation path for the `double_ma` CUDA optimizer in this repo.

Using only the attached files, answer this repo-specific question:

How should this repo generalize TM-native MA preparation beyond `sma` and `alma`, so more of the `double_ma` family can stay device-resident and avoid row-major compute followed by transpose?

Constraints:
- Stay grounded in this repo's current runtime and strategy architecture.
- Do not assume external crates or APIs can be changed for free.
- Treat this as a product-facing architecture question, not an excuse for generic indicator expansion.
- Distinguish clearly between:
  - what helps the current benchmark story,
  - what broadens useful GPU coverage,
  - and what is likely too invasive for too little payoff.

Deliverables:
1. Explain the current TM-native fast path vs transpose-path split in this repo.
2. Identify which MA families are the highest-value next targets and why.
3. Propose the best design path:
   - expand TM-native generation in the current runtime,
   - add broader VRAM MA support,
   - change transpose ownership,
   - or some combination.
4. Identify the concrete code touch points by file and symbol.
5. Explain what benchmark surfaces would actually reveal payoff from this work.
6. Give a staged implementation and parity-validation plan.
7. State the reasons this effort should be rejected if the measured upside is too small.

Keep the answer repo-specific and decision-oriented.
```

### Second-Pass Prompt

```text
Review your prior answer to Prompt 2 as a critical second pass.

I want you to pressure-test the TM-native MA expansion recommendation you just made for this repo.

Tasks:
1. Identify weak assumptions, vague claims, and places where your recommendation sounds plausible but is not yet well defended by the attached repo evidence.
2. Identify any ways your proposal may over-prioritize architectural cleanliness over actual product payoff for this repo.
3. Identify the strongest arguments for:
   - keeping the current `sma` / `alma` scope,
   - expanding TM-native preparation narrowly,
   - or redesigning VRAM MA support more broadly.
4. If targeted current web research would materially improve the answer, use it selectively to study:
   - practical GPU-resident MA preparation patterns
   - transpose-avoidance strategies for MA families
   - maintainability tradeoffs in broader indicator/GPU support
5. Refine the recommendation down to the strongest direction for this repo now.

Deliverables:
1. `Keep / revise / reject` verdict on your first-pass recommendation
2. Refined TM-native MA expansion direction
3. Must-have versus optional scope
4. Top implementation and maintenance risks
5. The exact conditions that would make the repo defer or reject the effort

Keep the answer grounded in this repo's current runtime and strategy architecture.
```

### Final Consolidation Prompt

```text
Using your first-pass and second-pass answers for Prompt 2, produce one final implementation-ready markdown document for this repo.

Write the output as if it will become:
`CUDA kernel review/tm_native_ma_expansion_direction_FINAL.md`

The document must contain:
1. Executive summary
2. Current repo MA-path summary
3. Final recommendation
4. Explicitly retained current strengths
5. Explicitly deferred ideas
6. Explicitly rejected ideas
7. Concrete code touch points by file and symbol
8. Staged implementation order
9. Benchmark and parity-validation plan
10. Rejection criteria and stop conditions

Make one concrete recommendation set. Do not leave this as an open-ended architecture brainstorm.
```

## Removed Prompt Categories

The following prompt families were removed from this pack because they now look stale, redundant, or low ROI relative to the completed Phase 2 work:

- one more generic bounded CUDA-body pass
- generic Rust orchestration redesign
- runtime toggle matrices and auto-tuning strategy
- open measurement surfaces and adoption order
- constrained exact-route side outputs such as `feasible_out`
- `cub::DeviceTopK` or new host-shim speculation
- broad cubin vs PTX JIT vs SASS audits
- toolchain archaeology, `ptxas` flag fishing, or hand-maintained PTX microforks

If any of those lanes are reopened later, they should be rewritten against the current Phase 2 evidence first rather than reused from the earlier April 1 framing.

## Suggested Order

If you only run one thread, use:

1. Prompt 1: Xlarge Cache, Reuse, And Data Layout Feasibility

If you want a second thread that is still plausibly worthwhile, use:

2. Prompt 2: TM-Native MA Expansion Beyond SMA And ALMA

That order matches the current repo evidence:

- cache or reuse questions only still look interesting on `xlarge`
- MA-path broadening is the remaining product-oriented architecture lane that is not obviously stale
