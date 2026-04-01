# Double MA Backtest Performance Research Prompt Pack

Date: 2026-04-01

Purpose: a repo-grounded prompt pack for handing specific `double_ma` CUDA-performance research questions to a stronger model such as GPT-5.4 Pro.

This document is intentionally narrower than "all CUDA ideas". It is centered on the still-open performance questions for the `double_ma` family because that is the strategy family tied most directly to the repo's public performance story.

## How To Use This Pack

- Use one prompt per chat/thread. Do not bundle multiple prompts into one request unless you want a shallow answer.
- Always attach the shared baseline files first.
- Then attach the prompt-specific files for the question you want answered.
- If context is tight, keep the review docs and the hot runtime files. Drop optional surfaces last.
- Ask the model to stay repo-specific and to distinguish:
  - what this repo already tried
  - what is still genuinely unexplored
  - what it would measure before recommending code changes

## Shared Baseline Files

Attach these to every prompt:

- `AGENTS.md`
- `REPO_MAP.md`
- `README.md`
- `CUDA kernel review/double_ma_backtest_performance_unexplored_research_2026-04-01.md`

Recommended shared framing for every thread:

- the repo's public benchmark story is anchored on `double_ma`, `alma/alma`, exact-grid GPU runs
- the CUDA review corpus already rejected many obvious wrapper-level and reducer-chain-level ideas
- answers should benefit this repo tangibly, not just provide generic CUDA advice

## Prompt 1: Compact Exact Search Dataflow

### Research Question

How should this repo redesign the exact `double_ma` search-stage dataflow so the search pass stops writing and rereading full metric rows before the final winners are known, while still preserving exact finalist replay?

### Attach These Files

Required:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `CUDA kernel review/double_crossover_shadow_reduced_payload_search_experiment_plan_2026-03-30.md`
- `CUDA kernel review/double_crossover_shadow_10row_reduced_payload_experiment_plan_2026-03-30.md`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/double_ma.rs`

Optional:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

### Prompt

```text
You are analyzing the performance-critical CUDA exact optimization path for the `double_ma` family in this repo.

Using only the attached files, answer this repo-specific question:

How should we redesign the exact search-stage dataflow so the GPU search pass stops writing and rereading full metric rows before the final winners are known, while still preserving exact finalist replay semantics for `full17` and `trade22`?

Important constraints:
- Do not re-suggest ideas the attached review docs already tried and rejected.
- Keep CPU fallback behavior unchanged.
- Keep exact finalist semantics authoritative.
- Assume the public benchmark story is still anchored on `double_ma`, `alma/alma`, and exact-grid GPU runs.
- Treat `large/58300pairs` and shortlist-heavy `xlarge` as the main performance truth surfaces.

Deliverables:
1. Summarize what the repo already tried in this area and why those attempts were insufficient.
2. Propose the best 1-2 compact-search architectures that are still genuinely open.
3. Specify the minimum compact payload you would use during search and explain why.
4. Explain how the reducer or top-k layer should change under your design.
5. Explain where host involvement should remain and where it should be removed.
6. Identify the concrete code touch points by file and symbol.
7. Give a benchmark and parity-validation plan.
8. State the exact measurements or failure modes that would cause you to reject your own design.

Be concrete and repo-specific. Cite attached files and symbols rather than giving generic CUDA optimization advice.
```

## Prompt 2: Device-Side Finalist Materialization And Replay Inputs

### Research Question

Can this repo remove the current host-first finalist reconstruction from the compact search path and move finalist materialization or replay-input emission fully or mostly onto the device?

### Attach These Files

Required:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/double_ma.rs`

Optional:

- `crates/app_core/src/double_ma_validate.rs`

### Prompt

```text
You are analyzing a specific bottleneck in the `double_ma` CUDA optimization path: the current compact-search prototype still becomes host-first when finalists are materialized and replayed exactly.

Using the attached files, answer this repo-specific question:

Can we redesign finalist handling so compact finalists stay on device until exact replay launch is fully parameterized, instead of copying shortlist identities to host and rebuilding finalist params there?

Constraints:
- Exact replay remains authoritative.
- Final `full17` and `trade22` rows must stay bit-for-bit or behaviorally exact relative to the current exact route.
- Do not assume we can remove existing product surfaces as a shortcut.

Deliverables:
1. Explain what the current host-first finalist path is doing and why it may be expensive.
2. Propose the best device-side finalist or replay-input architecture that still fits this repo.
3. Explain the minimum data that must cross from device to host before exact replay can begin.
4. Identify whether the replay kernel contract should consume `(score, pair_idx)` style descriptors, packed finalist structs, or something else.
5. Map the likely code touch points by file and symbol.
6. Identify correctness risks, especially around ordering, replay determinism, and metric materialization.
7. Give a benchmark plan that isolates “kernel improvement” from “host finalist materialization improvement”.

Do not give generic GPU pipeline advice. Keep the answer grounded in the attached files and the current `double_ma` exact route.
```

## Prompt 3: One More Bounded CUDA-Body Pass On The Retained Hot Kernels

### Research Question

Is there still one bounded, source-level CUDA kernel-body optimization pass worth trying on the retained hot `double_ma` kernels that is meaningfully different from the already rejected shuffle, broadcast, dedup, and launch-only ideas?

### Attach These Files

Required:

- `CUDA kernel review/double_crossover_performance_review_final.md`
- `CUDA kernel review/double_crossover_m17_m22_implementation_plan_2026-03-28.md`
- `CUDA kernel review/double_crossover_m17_m22_valid_implementation_plan_2026-03-28.md`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

### Prompt

```text
You are reviewing the remaining source-level CUDA kernel-body opportunities for the retained hot `double_ma` wrappers in this repo.

Using the attached files, answer this question:

Is there still one bounded CUDA-body optimization pass worth attempting on the retained hot exact kernels that is materially different from the already rejected shuffle, broadcast, dedup, and generic launch-tuning ideas?

Constraints:
- This is not permission to propose broad architecture changes; focus on one bounded kernel-body pass.
- You must distinguish what the repo already tried from what is still genuinely open.
- The answer should stay focused on the retained hot kernels that matter to the repo’s benchmark story.

Deliverables:
1. Summarize the main body-local ideas already tried and rejected.
2. Identify the strongest remaining bounded kernel-body opportunity, if any.
3. Explain why it is different from the rejected ideas.
4. Explain what Nsight evidence would justify trying it.
5. Point to the likely code regions and wrapper families involved.
6. Define a narrow A/B plan and the exact signals that would prove the idea is or is not worth keeping.
7. If you think no bounded body pass remains credible, say so and defend that conclusion.

Keep the answer repo-specific and evidence-backed.
```

## Prompt 4: TM-Native MA Preparation Beyond `sma` And `alma`

### Research Question

How should this repo generalize TM-native MA preparation beyond the current `sma` and `alma` fast path so the broader `double_ma` family does not collapse back into row-major compute plus transpose?

### Attach These Files

Required:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

Optional:

- `crates/app_core/Cargo.toml`
- `src-tauri/Cargo.toml`

### Prompt

```text
You are analyzing the MA-preparation path for the `double_ma` CUDA optimizer in this repo.

Using the attached files, answer this question:

How should we generalize the current TM-native MA-preparation path beyond `sma` and `alma`, so more of the `double_ma` family can stay device-resident and avoid row-major compute followed by transpose?

Constraints:
- Stay grounded in this repo’s current runtime and strategy architecture.
- Do not assume external crates or APIs can be changed for free.
- Treat this as both a performance and orchestration problem, not just a local helper rewrite.

Deliverables:
1. Explain the current fast-path vs transpose-path split.
2. Identify the most valuable MA families to target next and why.
3. Propose the best design path:
   a. expand TM-native generation in the current runtime
   b. add broader VRAM MA support
   c. fuse transpose elimination differently
   d. some combination of the above
4. Point to code touch points by file and symbol.
5. Explain the likely performance upside and what workloads would expose it.
6. Give a benchmark plan and parity checks.
7. State the reasons you would reject the effort if the measured upside is too small.

Make the answer repo-specific rather than a generic “support more indicators” suggestion.
```

## Prompt 5: Rust Orchestration Redesign For Multi-Stream Overlap

### Research Question

How should the Rust-side `double_ma` orchestration evolve from a mostly single-stream, tile-serialized pipeline into a more overlapped multi-stream or double-buffered design?

### Attach These Files

Required:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/build.rs`

### Prompt

```text
You are analyzing the Rust-side orchestration around the `double_ma` CUDA backtest optimizer in this repo.

Using the attached files, answer this question:

How should the current mostly single-stream, tile-serialized Rust orchestration evolve into a more overlapped multi-stream or double-buffered pipeline, without breaking correctness or CUDA/CPU fallback behavior?

Constraints:
- Stay focused on the `double_ma` optimization route.
- Keep the answer grounded in the current runtime shape, not an imaginary new platform.
- Distinguish improvements that are worth implementing now from ones that are too invasive for uncertain payoff.

Deliverables:
1. Describe the current orchestration stages and the main serialization points.
2. Propose the best next-step pipeline design for overlap:
   - multi-stream compute
   - double-buffered tiling
   - event-driven H2D/compute/D2H coordination
3. Explain what scratch-ownership or buffer-lifetime changes would be required.
4. Identify the highest-risk correctness hazards.
5. Point to the code touch points by file and symbol.
6. Give a staged implementation order so the work can be benchmarked incrementally.
7. Define the measurements that would justify the added orchestration complexity.

The answer should help this repo decide whether Rust-side orchestration still has step-function upside.
```

## Prompt 6: Device-Resident Caching Of Period Banks, MA Tiles, And Replay Inputs

### Research Question

What additional device-resident caches should exist beyond resident market series and valid-pair maps, especially for period banks, TM MA tiles, and replay-side reuse on the `double_ma` route?

### Attach These Files

Required:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`

### Prompt

```text
You are reviewing device-resident caching opportunities in the `double_ma` CUDA optimizer in this repo.

Using the attached files, answer this question:

What additional device-resident caches should exist beyond resident market series and valid-pair maps, especially for period banks, TM MA tiles, and replay-side reuse, and which of those caches are actually worth the added complexity?

Constraints:
- Stay focused on the `double_ma` optimization route.
- Keep the answer tied to the current runtime shape.
- Distinguish between caches that help the search pass, caches that help exact replay, and caches that are unlikely to pay for themselves.

Deliverables:
1. Describe what is already cached today.
2. Identify the most valuable missing caches and explain why they matter.
3. Explain the likely cache keys, invalidation rules, and memory-footprint risks.
4. Identify the code touch points by file and symbol.
5. Explain how these caches interact with shortlist replay and repeated exact surfaces.
6. Give a staged benchmark plan so the repo can prove whether the caches help enough to keep.
7. State the conditions under which each proposed cache should be rejected.

I want a repo-specific answer, not a generic “add memoization” suggestion.
```

## Prompt 7: Runtime Toggle Matrix And Auto-Tuning Strategy

### Research Question

Which existing runtime toggles and launch/tile-shape knobs in the `double_ma` CUDA path are worth promoting into a disciplined tuning strategy, and which are likely just dormant scaffolding?

### Attach These Files

Required:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `README.md`

Optional:

- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `scripts/run_gpu_perf_gate.ps1`

### Prompt

```text
You are reviewing the latent performance knobs already present in the `double_ma` CUDA runtime in this repo.

Using the attached files, answer this question:

Which existing runtime toggles and launch/tile-shape knobs are worth promoting into a disciplined tuning strategy for the `double_ma` benchmark route, and which are likely just dormant scaffolding with low expected value?

I care about:
- CUDA graphs
- pinned H2D/D2H
- async alloc
- buffer persistence
- valid-pair-map toggles
- global top-k toggles
- launch-shape overrides
- target-wave and block-size heuristics

Deliverables:
1. Summarize what knobs already exist in the live runtime.
2. Group them into:
   - likely worth systematic measurement now
   - likely only worth testing after architecture changes
   - unlikely to matter enough to prioritize
3. Propose a disciplined experiment matrix for `small`, `large`, and shortlist-heavy `xlarge`.
4. Explain which knobs could become route-specific defaults rather than hidden env toggles.
5. Explain whether this repo should add lightweight auto-tuning, static per-device defaults, or keep manual overrides only.
6. Point to the main code touch points by file and symbol.

Do not give generic “benchmark all combinations” advice. I want a reasoned, repo-specific prioritization.
```

## Prompt 8: Cubin vs PTX JIT vs SASS Audit

### Research Question

Is there still a meaningful post-compilation performance opportunity in this repo's `double_ma` CUDA path, and if so, how should it be proven before any PTX or SASS-level intervention is considered?

### Attach These Files

Required:

- `CUDA kernel review/double_crossover_performance_review_final.md`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `ma_review/double_crossover_cuda13_valid_implementation_tracker_2026-03-25.md`
- `crates/app_core/build.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `RELEASE.md`

### Prompt

```text
You are reviewing whether post-compilation work is still a credible performance lane for the `double_ma` CUDA optimizer in this repo.

Using the attached files, answer this question:

Is there still a meaningful cubin-vs-PTX-JIT-vs-SASS opportunity here, and if so, what exact audit sequence should this repo follow before anyone spends time on PTX or SASS-level intervention?

Constraints:
- The attached docs already say broad PTX/SASS surgery should not be the primary path.
- I still want to know whether post-compilation analysis is underexplored in a disciplined way.
- Keep the answer tied to the actual runtime/build path in this repo.

Deliverables:
1. Explain the current build and runtime module-loading story.
2. Explain what has and has not actually been proven so far.
3. Propose the smallest credible cubin-vs-PTX-JIT A/B plan on the real hot kernels.
4. Explain what full SASS inspection should try to prove or disprove.
5. List the exact compiler or codegen failure signatures that would justify source-level follow-up vs narrow PTX/SASS intervention.
6. State the conditions under which you would conclude this lane is not worth more time.
7. Point to code and build touch points by file and symbol.

Do not answer with generic CUDA compilation folklore. Make the case from the attached repo.
```

## Prompt 9: `feasible_out` And Constrained Exact Surfaces

### Research Question

Is there a worthwhile constrained-route performance opportunity in emitting and consuming device-side feasibility side outputs such as `feasible_out`, or is that complexity likely not justified on the `double_ma` route that matters for this repo?

### Attach These Files

Required:

- `CUDA kernel review/double_crossover_m17_m22_implementation_plan_2026-03-28.md`
- `CUDA kernel review/double_crossover_m17_m22_valid_implementation_plan_2026-03-28.md`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

Optional:

- `web/src/services/optimizeRequestBuilder.ts`

### Prompt

```text
You are reviewing whether constrained exact-route acceleration is still a worthwhile research lane for the `double_ma` CUDA path in this repo.

Using the attached files, answer this question:

Is it worth pursuing a device-side feasibility side output such as `feasible_out` for constrained exact surfaces, or is that likely too much complexity for too little real-world benefit on the `double_ma` route that matters here?

Deliverables:
1. Explain what the docs and code imply about the current constrained-route costs.
2. Explain what `feasible_out` or a similar side output would actually buy us.
3. Identify the smallest viable implementation shape for this repo.
4. Identify the code touch points by file and symbol.
5. Give a benchmark plan that would determine whether this lane is hot enough to justify the added surface area.
6. State the conditions under which the repo should permanently defer this work.

Keep the answer practical and repo-specific.
```

## Prompt 10: Rust-Side Host Shim For `cub::DeviceTopK` Or Similar Selection Primitives

### Research Question

Is it worth introducing a new CUDA host-side shim or stronger cubin-first boundary so this repo can use `cub::DeviceTopK` or similar device-selection primitives for compact candidate selection on the `double_ma` route?

### Attach These Files

Required:

- `ma_review/double_crossover_cuda13_valid_implementation_tracker_2026-03-25.md`
- `crates/app_core/build.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/Cargo.toml`
- `Cargo.toml`

Optional:

- `RELEASE.md`

### Prompt

```text
You are reviewing whether this repo should introduce a stronger CUDA host-side shim so the `double_ma` optimizer can use `cub::DeviceTopK` or similar selection primitives for compact candidate selection.

Using the attached files, answer this question:

Is it worth introducing a new CUDA host-side shim or stronger cubin-first boundary so this repo can use `cub::DeviceTopK`-style primitives, or is that architectural cost too high relative to the likely payoff?

Constraints:
- The attached tracker says `DeviceTopK` exists locally but is not viable in the current PTX-module + Rust-driver architecture without a new shim.
- I want a repo-specific answer, not a generic “CUB is fast” answer.

Deliverables:
1. Explain why the current architecture blocks this lane today.
2. Explain the minimum architectural change needed to make this viable.
3. Compare the likely payoff of this lane against the internal compact-search designs that do not require a new host shim.
4. Identify build, release, portability, and maintenance risks.
5. Point to the code and build touch points by file and symbol.
6. State the conditions under which this lane becomes worth pursuing.
7. State the conditions under which the repo should reject it outright.

The answer should help decide whether `DeviceTopK` is a serious future direction or just an attractive distraction.
```

## Prompt 11: Open Measurement Surfaces And Adoption Order

### Research Question

Which still-open measurement surfaces actually matter for deciding whether a new `double_ma` performance architecture is real, and in what order should this repo close them?

### Attach These Files

Required:

- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `CUDA kernel review/double_ma_backtest_performance_unexplored_research_2026-04-01.md`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `scripts/run_gpu_perf_gate.ps1`

Optional:

- `web/src/services/optimizeRequestBuilder.ts`
- `README.md`

### Prompt

```text
You are reviewing the still-open measurement and adoption surfaces for future `double_ma` CUDA performance work in this repo.

Using the attached files, answer this question:

Which still-open measurement surfaces actually matter for deciding whether a new architecture is real, and in what order should this repo close them?

I specifically care about:
- shortlist-heavy `xlarge`
- shortlist-slice exact reruns
- `trade22` tail metrics after finalist replay
- constrained exact routes
- heatmap
- `include_all`
- export surfaces
- union/confidence rerank surfaces

Deliverables:
1. Group the open surfaces into:
   - must close before adopting a new path
   - should close soon after
   - can remain on fallback paths for now
2. Explain why each group belongs there.
3. Propose an adoption order that preserves product safety while still letting the repo ship performance wins.
4. Identify which measurement harnesses or scripts should be extended first.
5. State the exact evidence threshold you would require before calling a new path “real” for this repo.

Keep the answer grounded in the attached repo files and current `double_ma` product story.
```

## Experimental Prompts

These are intentionally lower-priority than the main architecture and orchestration prompts above.

They are still realistic enough to explore once, but they should be treated as bounded research spikes, not as the default optimization plan.

## Prompt 12: Toolchain And `ptxas` Flag Matrix For Codegen Diffs

### Research Question

Should this repo run a disciplined toolchain and `ptxas` flag matrix on the hot `double_ma` kernels to see whether compiler-version or codegen-option drift materially changes PTX or SASS quality?

### Attach These Files

Required:

- `CUDA kernel review/double_crossover_performance_review_final.md`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `ma_review/double_crossover_cuda13_valid_implementation_tracker_2026-03-25.md`
- `crates/app_core/build.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

Optional:

- `RELEASE.md`

### Prompt

```text
You are reviewing whether this repo should run a disciplined toolchain and `ptxas` flag matrix on the hot `double_ma` kernels to look for meaningful codegen differences.

Using the attached files, answer this question:

Should we run a bounded matrix across CUDA toolchain versions and `ptxas`/`nvcc` codegen flags for the hot `double_ma` exact kernels, and if so what exact matrix is worth running before we call this lane explored?

Constraints:
- Treat this as a lower-priority experimental lane, not the main optimization path.
- Stay grounded in the actual build/runtime path in this repo.
- Do not answer with a generic list of CUDA flags; I want a repo-specific matrix with a reason for each item.

Deliverables:
1. Explain what toolchain and codegen variance is already implied by the repo and review docs.
2. Propose the smallest credible matrix across:
   - CUDA versions already relevant to this repo
   - PTX vs cubin outputs
   - `NVCC_ARGS` or `ptxas` options worth testing
3. Identify which flags are too risky or semantically dangerous for this repo to test casually.
4. Explain what PTX-level and SASS-level differences would actually count as promising.
5. Point to the build and runtime touch points by file and symbol.
6. Define the exact benchmark and disassembly evidence threshold that would justify keeping any toolchain-specific build path.

Keep the answer practical and repo-specific.
```

## Prompt 13: Hand-Maintained PTX Microfork Viability

### Research Question

Is a hand-maintained PTX microfork for one retained hot `double_ma` kernel even viable in this repo, and if so what conditions would have to be true before attempting it?

### Attach These Files

Required:

- `CUDA kernel review/double_crossover_performance_review_final.md`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`
- `crates/app_core/build.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `RELEASE.md`

### Prompt

```text
You are reviewing an intentionally experimental idea for this repo: a hand-maintained PTX microfork for one retained hot `double_ma` exact kernel.

Using the attached files, answer this question:

Is a hand-maintained PTX microfork for one retained hot kernel even viable in this repo, and if so what conditions would have to be true before we attempt it?

Constraints:
- Treat this as a last-resort experimental lane.
- Keep the answer tied to the repo’s actual PTX packaging, runtime module loading, and release story.
- Do not assume broad PTX hand-editing is acceptable by default.

Deliverables:
1. Explain what would have to be true before a PTX microfork becomes rational in this repo.
2. Identify the minimum scope that could possibly be maintained:
   - one wrapper family
   - one architecture target
   - one benchmark-critical route
3. Explain how the build and runtime would need to distinguish the microfork from the ordinary path.
4. Identify maintenance, portability, release, and correctness risks.
5. Explain what kind of compiler miss or measured gap would justify this path.
6. State the exact conditions under which the repo should reject this idea without implementing it.

I do not want a generic “hand-written PTX is hard” answer. I want a repo-specific viability test.
```

## Prompt 14: Compiler-Version Archaeology On The Hot Kernels

### Research Question

Could compiler-version drift alone materially change the generated PTX or SASS for the hot `double_ma` kernels, and should this repo pin or multi-build around that possibility?

### Attach These Files

Required:

- `ma_review/double_crossover_cuda13_valid_implementation_tracker_2026-03-25.md`
- `CUDA kernel review/double_crossover_performance_review_final.md`
- `crates/app_core/build.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

Optional:

- `RELEASE.md`

### Prompt

```text
You are reviewing whether compiler-version drift itself could be a meaningful performance variable for the hot `double_ma` kernels in this repo.

Using the attached files, answer this question:

Could compiler-version drift alone materially change PTX or SASS quality for the hot `double_ma` kernels, and should this repo pin a specific CUDA toolchain or maintain multiple build artifacts because of that possibility?

Constraints:
- Keep this grounded in the actual versions and build path mentioned in the attached repo files.
- Treat this as a bounded research spike, not a pretext for endless toolchain churn.
- Distinguish “worth checking once” from “worth institutionalizing in the build/release flow”.

Deliverables:
1. Explain why compiler-version drift is or is not a plausible variable for this repo.
2. Propose the smallest credible cross-version comparison for the hot kernels.
3. Explain what PTX or SASS differences would actually matter enough to justify pinning.
4. Identify build/release implications if the repo ever chose to prefer one compiler line.
5. State the evidence threshold for:
   - no further action
   - pin one toolchain
   - maintain separate artifacts

Keep the answer repo-specific and decision-oriented.
```

## Suggested Order

If you only want to run a few threads first, use this order:

1. Prompt 1: Compact Exact Search Dataflow
2. Prompt 2: Device-Side Finalist Materialization And Replay Inputs
3. Prompt 5: Rust Orchestration Redesign For Multi-Stream Overlap
4. Prompt 6: Device-Resident Caching Of Period Banks, MA Tiles, And Replay Inputs
5. Prompt 4: TM-Native MA Preparation Beyond `sma` And `alma`
6. Prompt 8: Cubin vs PTX JIT vs SASS Audit
7. Prompt 11: Open Measurement Surfaces And Adoption Order

That order matches the current repo evidence: the biggest remaining upside is still architectural and orchestration-level, with post-compilation work treated as a narrower diagnostic lane.

If you want to explore the experimental PTX/SASS lanes later, use this order:

1. Prompt 8: Cubin vs PTX JIT vs SASS Audit
2. Prompt 12: Toolchain And `ptxas` Flag Matrix For Codegen Diffs
3. Prompt 14: Compiler-Version Archaeology On The Hot Kernels
4. Prompt 13: Hand-Maintained PTX Microfork Viability

## Feature-Oriented Prompts

These are not primarily about raw kernel speed.

They are about small or medium-scope experimental features that can be added on top of the current `double_ma` GPU route while still respecting the repo's preference for one-pass work, row-only algebra, and tiny-shortlist follow-up work.

The bias here is:

- first exploit what the current rows already support
- then exploit trade-tail fields already present in `trade22`
- only then consider tiny new carried state

## Prompt 15: Route-Unblock Existing GPU-Derivable Objectives

### Research Question

Which currently blocked or underexposed objectives are already mathematically supported by the current GPU rows, and what is the smallest repo-specific change needed to expose them safely?

### Attach These Files

Required:

- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

### Prompt

```text
You are reviewing small-scope feature additions for the `double_ma` GPU route in this repo.

Using the attached files, answer this question:

Which objectives are already mathematically supported by the current GPU metric rows but are still route-blocked, underexposed, or not promoted as first-class GPU optimization modes, and what is the smallest safe change needed to expose them?

I care especially about:
- direct GPU `Sortino`
- direct GPU `Omega`
- active-time-normalized score families
- current derived score families already present in the repo

Deliverables:
1. Identify which objectives are already reconstructible from the current rows.
2. Separate:
   - objective modes that need mostly routing/preflight changes
   - modes that need small reduction-layer additions
   - modes that still need new carried state
3. Point to the code touch points by file and symbol.
4. Recommend the best 2-3 feature additions by value vs scope.
5. State the parity and benchmark checks needed before enabling them.

Keep the answer tightly grounded in the attached repo.
```

## Prompt 16: Derived Metric Vocabulary From `full17` And `trade22`

### Research Question

What additional user-facing metrics, filters, or rerank features can this repo derive exactly from the existing `full17` and `trade22` rows without widening the default search row?

### Attach These Files

Required:

- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

### Prompt

```text
You are reviewing feature ideas that can be derived exactly from the existing `full17` and `trade22` metric rows in this repo.

Using the attached files, answer this question:

What additional user-facing metrics, filters, or rerank features can we derive exactly from `full17` and `trade22` without widening the default search row or adding another data pass?

I want you to focus on:
- exact row-only algebra
- features that could be shown in the UI, used as optional rerankers, or used as opt-in constraints
- features that are small enough to tack onto the existing application

Deliverables:
1. Group the candidate features into:
   - row-derived from `full17`
   - trade-tail-derived from `trade22`
   - already partially implemented in code but not fully surfaced
2. For each feature, explain:
   - the formula
   - whether it is exact from the current row
   - whether it should be an objective, filter, display metric, or rerank-only feature
3. Recommend the best 5 additions by product value vs implementation cost.
4. Identify any features that look attractive but should stay out because they are too noisy or too easy to misuse.

Keep the answer repo-specific and practical.
```

## Prompt 17: Expanded Device-Side Constraints From Existing Rows

### Research Question

Which additional hard or soft constraints should this repo expose on the GPU route using only values already available in the current rows?

### Attach These Files

Required:

- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

Optional:

- `web/src/services/optimizeRequestBuilder.ts`

### Prompt

```text
You are reviewing whether this repo should expose more device-side constraints on the `double_ma` GPU route using values already available in the current rows.

Using the attached files, answer this question:

Which additional hard or soft constraints should we expose on the GPU route using only values already available in the current rows, and which of those constraints are actually worth surfacing to users?

I care about candidates such as:
- min/max exposure
- max absolute net exposure
- min omega
- max ulcer
- max downside deviation
- derived active-time or chronicity thresholds

Deliverables:
1. Identify which constraints are already partly supported in code.
2. Separate:
   - constraints that are exact and low-risk
   - constraints that are exact but likely noisy or misuse-prone
   - constraints that should remain validation-stage rather than optimization-stage
3. Recommend the best small feature set to expose first.
4. Point to the code touch points by file and symbol.
5. State the benchmark and UX risks of exposing too many constraints at once.

Keep the answer practical and repo-specific.
```

## Prompt 18: Trade22-As-Feature Surface

### Research Question

Given that `trade22` already carries a small trade-tail family, which experimental product features can this repo add from those fields before considering any wider trade-level metric expansion?

### Attach These Files

Required:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `CUDA kernel review/double_crossover_next_phase_performance_implementation_plan_2026-03-30.md`

### Prompt

```text
You are reviewing what product features this repo can build on top of the existing `trade22` metric contract before adding any richer trade-level metric family.

Using the attached files, answer this question:

Given the existing `trade22` fields, which experimental features can we add now with relatively small scope, and which ideas should still wait?

I care about features derived from:
- closed_trades
- winning_trades
- losing_trades
- gross_profit
- gross_loss

Examples might include:
- profit factor
- expectancy
- win rate
- avg win / avg loss
- trade-distribution quality scores

Deliverables:
1. Identify the most valuable exact derived features from the existing `trade22` tail.
2. Separate:
   - display/reporting features
   - optional rerankers
   - optional optimization objectives
   - features that should not be promoted because they are too fragile
3. Recommend the best 3-5 additions by value vs scope.
4. Explain whether they should remain `trade22`-only or be backfilled elsewhere.
5. Point to the code touch points by file and symbol.

Keep the answer small-scope and repo-specific.
```

## Prompt 19: Multi-Head Ranking And Union Shortlist As A User-Facing Feature

### Research Question

How should this repo turn its current composite-score, extra-shortlist-objective, and shortlist-union machinery into a coherent experimental feature for users without overcomplicating the product?

### Attach These Files

Required:

- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Optional:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

### Prompt

```text
You are reviewing whether this repo should productize multi-head ranking and union-shortlist behavior as an experimental user-facing feature for `double_ma`.

Using the attached files, answer this question:

How should we turn the current composite-score, extra-shortlist-objective, and shortlist-union machinery into a coherent experimental feature for users, without overcomplicating the product or undermining the current benchmark route?

Deliverables:
1. Describe what ranking and shortlist machinery already exists in the repo.
2. Recommend the smallest user-facing version of this feature that is still meaningful.
3. Explain which score heads should be offered first.
4. Explain when the feature should use:
   - extra shortlist objectives
   - union shortlist
   - confidence rerank
   - neighborhood or plateau diagnostics
5. Point to the code touch points by file and symbol.
6. State the product and performance risks of surfacing this too broadly.

Keep the answer repo-specific and bias toward a small experimental rollout.
```

## Prompt 20: Small-Scope Explainability And Robustness Features From Existing Rows

### Research Question

What explainability or robustness-style features can this repo add from existing rows or tiny-shortlist matrix geometry, without turning optimization into a validation pipeline?

### Attach These Files

Required:

- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

### Prompt

```text
You are reviewing possible explainability and robustness-style features for the `double_ma` optimizer in this repo.

Using the attached files, answer this question:

What explainability or robustness-style features can we add from existing rows or tiny-shortlist matrix geometry, without turning optimization into a second validation pipeline?

I care about ideas such as:
- active-time-normalized metrics
- drawdown chronicity
- downside-share or asymmetry proxies
- local agreement between score families
- shortlist neighborhood / plateau summaries

Deliverables:
1. Separate the candidate features into:
   - exact row-only algebra
   - shortlist-matrix geometry
   - ideas that are too validation-like to belong in optimization
2. Recommend the best small feature set for an experimental rollout.
3. Explain which features should be display-only vs rerank-only vs default-visible.
4. Point to the code touch points by file and symbol.
5. State the risks of overselling these signals to users.

Keep the answer grounded in the repo and biased toward features that are small enough to tack on.
```
