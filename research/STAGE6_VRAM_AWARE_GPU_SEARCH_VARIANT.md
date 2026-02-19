# Stage 6: VRAM-Aware Deterministic Progressive Search (VDPS)

Last updated: 2026-02-18  
Scope: `ta_desktop_demo` research and implementation blueprint only

## 0) Evidence Base and Boundary

This document is grounded in:

- Early-stop and multi-fidelity search literature (Hyperband, ASHA, DEHB, MO-DEHB) [R1][R2][R3][R4].
- CUDA runtime and performance guidance for streams, async copies, memory pools, and graphs [R5][R6][R7][R8][R9][R10].
- GPU-parallel primitives (CUB/Thrust) for stable sorting, selection, reductions, and determinism controls [R11][R12][R13][R14].
- Reproducibility literature for floating-point parallel reductions [R15][R16].
- Tooling references for profiling and power/energy telemetry (Nsight, NVML) [R17][R18][R19].
- UE5 GPU-processing references to maintain gaming-tech relevance [R20][R21].

Boundary:

- We are not claiming a new universal optimizer class.
- We are targeting a specialized algorithmic system for large discrete strategy-parameter spaces under VRAM constraints.

## 1) Problem Statement

Current brute-force GPU batching works for moderate spaces, but breaks down when:

1. Total candidate count is much larger than what can be evaluated at full fidelity in practical time.
2. Candidate state cannot be held in VRAM, forcing repeated host-device transfers.
3. Deterministic top-K across batches is required for artifact reproducibility.

Goal:

- Replace exhaustive batched brute force with a VRAM-aware progressive search that minimizes full-fidelity evaluations while preserving top-K quality and determinism.

## 2) Why VDPS (Positioning vs Existing Methods)

What we reuse from prior art:

- Hyperband/ASHA: aggressive early elimination under bounded resources [R1][R2].
- DEHB: evolutionary refill to explore high-dimensional and discrete-heavy spaces better than pure elimination [R3].
- MO-DEHB: optional multi-objective extension when objective vectors are used [R4].

What we add for this app:

- Explicit VRAM budgeting and pool management for each rung.
- Deterministic ranking/tie-breaking across independently executed GPU batches.
- Tight integration with static compiled strategy kernels and existing run artifacts.

Practical baseline implementation strategy:

1. Reproduce ASHA/Hyperband behavior first using established scheduler semantics [R24][R25].
2. Add DE-style refill and compare against DEHB behavior [R3].
3. Keep custom work focused on VRAM residency and deterministic GPU ranking (the non-off-the-shelf part).
4. Treat EvoTorch as an external reference for GPU-accelerated evolutionary operations, not as a drop-in scheduler for this app architecture [R26].

## 3) Formal VDPS v0.1 Spec

Inputs:

- Search space `S` (mixed discrete/continuous).
- Rung budgets `B = {b0 < b1 < ... < bR}` as fractions of full data (or equivalent fidelity budget).
- Elimination rate `eta` (default 4).
- Initial population `N0`.
- Refill ratios: `rho_elite`, `rho_de`, `rho_rand`.
- Final target `K` (top-K).
- Seed `seed`.

Derived per-rung counts:

- `Nr = ceil(N0 / eta^r)` promoted survivors target.
- Evaluate at budget `br`.

Core loop:

1. Initialize queue at rung 0 with Sobol or LHS candidates plus optional historical warm-start candidates [R23].
2. Evaluate queue at `br` in VRAM-feasible batches.
3. Deterministically rank by objective and tie-break keys.
4. Promote top `Nr+1`.
5. Refill next rung queue via:
   - elite local mutation,
   - differential mutation/recombination,
   - random injection.
6. Continue until full fidelity rung.
7. Return deterministic top-K.

Promotion invariant:

- Every promoted candidate has been evaluated at all lower rungs.

## 4) Candidate Encoding and Mutation for Mixed Spaces

Encoding:

- Continuous dims: `f32` (or fixed-point integer for strict determinism).
- Integer dims: `i32`.
- Categorical dims: dictionary index (`u16`/`u32`).

Recommended mutation policy:

1. Differential mutation for numeric dims (Storn-Price style) [R22].
2. Categorical mutation by parent voting plus random perturbation.
3. Constraint projection after mutation:
   - clamp to bounds,
   - quantize to valid step,
   - enforce cross-parameter constraints.

Default coefficients:

- `F in [0.5, 0.9]`, `CR in [0.5, 0.9]` (configurable, seed-driven).

## 5) Fidelity Ladder Design

Default rung ladder:

- Long series: `[1/16, 1/8, 1/4, 1/2, 1]`.
- Mid series: `[1/8, 1/4, 1/2, 1]`.
- Short series: `[1/4, 1/2, 1]`.

Critical validation rule:

- Before production, measure rank correlation between rung `r` and full fidelity.
- If correlation is weak, increase early rung budgets and/or reduce elimination aggressiveness.

## 6) Determinism Contract (Required)

Deterministic ranking key order:

1. Primary objective (descending for max objective, ascending for min objective).
2. Risk penalty (ascending).
3. Constraint violations (ascending).
4. Lexicographic packed parameter tuple (ascending).
5. Candidate id (ascending).

Implementation rules:

- Use stable sorting (`thrust::stable_sort` or stable CUB path) [R12][R11].
- Avoid unordered atomic accumulation for ranking keys.
- Use fixed seed and fixed RNG algorithm per stage.
- Log deterministic mode in artifact metadata.

Important note:

- Floating-point reductions can differ across GPU architectures due to non-associativity. CUB documents run-to-run determinism on same GPU and optional determinism controls in modern APIs [R14][R16].

## 7) VRAM-Resident Runtime Design

Persistent pools:

1. `candidate_pool` (params, id, lineage, rung).
2. `metric_pool` (objective, risk, flags).
3. `survivor_pool` (rung winners + global top-K).
4. `scratch_pool` (sorting/select temp buffers).

Batch sizing:

- Query free memory at runtime (`cudaMemGetInfo`) [R6].
- Compute:
  - `bytes_per_candidate = bytes_params + bytes_metrics + bytes_aux`.
  - `batch_size = floor((free_vram - reserve_bytes) / bytes_per_candidate)`.
- Start with `reserve_bytes = 20%` of free VRAM.

Allocator:

- Prefer stream-ordered allocator (`cudaMallocAsync`/`cudaFreeAsync`) and memory pools to reduce global synchronization overhead from legacy allocation paths [R5].

## 8) CUDA Primitive Choices (Concrete)

Use existing eval kernels unchanged first. Add search-control primitives around them.

For ranking/select:

1. CUB `DeviceRadixSort` for numeric key-value sort (stable, linear complexity) [R11].
2. CUB `DeviceSelect` for compaction and rung survivor extraction [R13].
3. Optional `DeviceReduce::ArgMax/ArgMin` for fast extremum checks [R14].
4. Thrust stable sort fallback path for custom comparator workflows [R12].

For execution overhead:

- Capture repetitive rung workflows with CUDA Graphs once topology is stable, then relaunch executable graph to reduce CPU launch overhead [R10].

## 9) Transfer Strategy and Streams

Default strategy:

- Keep candidate and metric buffers on device across rung transitions.
- Copy only minimal telemetry and final top-K payload to host.

When host-device transfer is required:

- Use pinned host memory for async overlap with compute and non-default streams [R8][R9].
- Avoid excessive pinned memory usage (scarce resource) [R8].

Do not:

- Copy full per-candidate intermediate histories each batch.

## 10) Request and Artifact Schema Additions

New request mode:

- `search_mode = "gpu_vdps"`

Required fields:

- `vdps_eta: u32` (default `4`)
- `vdps_rungs: Vec<f32>` (default `[0.0625, 0.125, 0.25, 0.5, 1.0]`)
- `vdps_initial_candidates: u32`
- `vdps_seed: u64`
- `vdps_target_top_k: u32`
- `vdps_deterministic: bool` (default `true`)
- `vdps_random_injection_ratio: f32`
- `vdps_elite_mutation_ratio: f32`
- `vdps_differential_ratio: f32`

Artifact additions under `res.vdps`:

- per-rung counts (`evaluated`, `promoted`, `dropped`)
- per-rung runtime and effective throughput
- full-fidelity eval count saved vs brute-force baseline
- transfer bytes (`h2d`, `d2h`)
- determinism metadata (`seed`, `rng_kind`, `tie_break_schema_version`)
- final top-K overlap vs baseline (if baseline run exists)

## 11) Implementation Sequence (Code-Level)

Phase 0: plumbing and no-op toggle (1-2 days)

1. Add request fields and artifact schema.
2. Add `search_mode` enum and routing.
3. Add determinism metadata placeholders.

Phase 1: host scheduler, GPU eval reuse (3-5 days)

1. Build rung queues and elimination on host for correctness.
2. Reuse existing GPU eval kernels.
3. Validate deterministic outputs on repeated same-seed runs.

Phase 2: GPU-side select and refill (4-7 days)

1. Move ranking/select to CUB/Thrust on device.
2. Add device mutation/refill kernels.
3. Add VRAM-aware batch auto-sizing and stream-ordered allocator.

Phase 3: launch overhead and profiling (3-5 days)

1. Add CUDA Graph capture path for stable rung loops.
2. Add NVTX ranges for each stage.
3. Profile with Nsight Systems and Nsight Compute.

Phase 4: benchmark and ablation (3-5 days)

1. Run baseline comparisons.
2. Run ablations.
3. Produce artifacted report with confidence intervals.

## 12) Benchmark Matrix (Minimum)

Factors:

1. Series length: `{50k, 200k, 1M}`.
2. Search space size: `{1e4, 1e5, 1e6+}` candidates equivalent.
3. Objective mode: single-objective and multi-objective.
4. VRAM pressure: `{low, medium, near-capacity}`.
5. Determinism mode: `{on, off}`.

Baselines:

1. Current batched brute-force.
2. ASHA-like elimination without mutation.
3. DE-only fixed-budget full-fidelity search.
4. Optional DEHB baseline (host-side reference implementation) [R3].

Primary metrics:

- Time-to-top-K.
- Full-fidelity evaluations saved.
- Top-K overlap with brute-force.
- Final objective regret vs brute-force best.
- Transfer bytes and achieved GPU occupancy.
- Determinism pass rate across repeated runs.

Tooling:

- Nsight Systems for timeline and overlap [R17].
- Nsight Compute for kernel occupancy/memory analysis [R18].
- NVML for power/energy telemetry [R19].

## 13) Acceptance and Kill Criteria

Ship criteria:

1. At least 2x faster time-to-top-K vs brute-force on at least two benchmark classes.
2. At least 90% top-K overlap (or better objective regret bound) vs brute-force reference.
3. Deterministic mode yields byte-identical ranking order on repeated same-seed runs on same GPU.
4. No regression in CPU-only build path behavior.

Kill/pivot criteria:

1. No consistent speedup after ablations.
2. Quality loss is material at practical budgets.
3. Determinism overhead is unacceptable for production.

## 14) Risk Register and Mitigations

Risk: low-fidelity rank is weak predictor of full-fidelity.

- Mitigation: increase early rung budget, reduce elimination aggressiveness, add random injection.

Risk: memory fragmentation or allocator stalls.

- Mitigation: stream-ordered allocator pools and trim policies [R5].

Risk: nondeterministic ordering from floating-point ties.

- Mitigation: stable sort + explicit tie-break keys + deterministic RNG.

Risk: PCIe bottleneck from telemetry copies.

- Mitigation: only copy compact summaries per rung.

Risk: overhead from kernel launch granularity.

- Mitigation: CUDA Graphs for repeated launch patterns [R10].

## 15) Gaming-Tech Relevance (Non-Quant Framing)

The same algorithm maps to gaming workloads where candidate evaluation is expensive and parallel:

1. UE5 PCG parameter tuning with GPU compute graphs (minimize CPU-GPU boundary crossings) [R20].
2. Large-scale ECS/MassEntity behavior parameter calibration [R21].
3. Offline tuning pipelines where deterministic ranking matters for reproducible build outputs.

For UE-style A/B validation, frame the comparison explicitly as:

1. Baseline A: conventional CPU + system RAM preprocessing pipeline.
2. Variant B: GPU + VRAM-first batched compute pipeline.

Important implementation note:

- "VRAM-first" does not imply all data always fits in VRAM. When working sets exceed VRAM, the variant uses out-of-core staging (system RAM/SSD to VRAM) while keeping hot compute and ranking state resident on GPU as much as possible.

This keeps the research contribution in parallel systems engineering, not domain-specific finance claims.

## 16) First Practical Sprint (Recommended)

Week 1:

1. Implement Phase 0 + Phase 1 with deterministic host ranking.
2. Run small benchmark against brute-force.

Week 2:

1. Implement GPU-side select/refill and allocator changes.
2. Add artifact telemetry and Nsight traces.

Week 3:

1. Add CUDA Graph path.
2. Complete baselines, ablations, and acceptance checks.

## References

[R1] Hyperband: A Novel Bandit-Based Approach to Hyperparameter Optimization (JMLR).  
https://www.jmlr.org/papers/v18/16-558.html

[R2] A System for Massively Parallel Hyperparameter Tuning (ASHA).  
https://arxiv.org/abs/1810.05934

[R3] DEHB: Evolutionary Hyperband for Scalable, Robust and Efficient Hyperparameter Optimization.  
https://arxiv.org/abs/2105.09821

[R4] MO-DEHB: Evolutionary-based Hyperband for Multi-Objective Optimization.  
https://arxiv.org/abs/2305.04502

[R5] CUDA Programming Guide: Stream-Ordered Memory Allocator (`cudaMallocAsync`/`cudaFreeAsync`).  
https://docs.nvidia.com/cuda/cuda-programming-guide/04-special-topics/stream-ordered-memory-allocation.html

[R6] CUDA Runtime API Memory Group (`cudaMemGetInfo`, memory APIs).  
https://docs.nvidia.com/cuda/cuda-runtime-api/group__CUDART__MEMORY.html

[R7] CUDA Runtime API synchronization behavior.  
https://docs.nvidia.com/cuda/cuda-runtime-api/api-sync-behavior.html

[R8] CUDA C++ Best Practices Guide (pinned memory and overlap guidance).  
https://docs.nvidia.com/cuda/archive/12.6.3/cuda-c-best-practices-guide/index.html

[R9] CUDA C Programming Guide (streams and concurrent transfers discussion).  
https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html

[R10] CUDA Programming Guide: CUDA Graphs.  
https://docs.nvidia.com/cuda/cuda-programming-guide/04-special-topics/cuda-graphs.html

[R11] CUB `DeviceRadixSort` (stable sort details).  
https://nvidia.github.io/cccl/cub/api/structcub_1_1DeviceRadixSort.html

[R12] Thrust `stable_sort` / `stable_sort_by_key`.  
https://nvidia.github.io/cccl/thrust/api/function_group__sorting_1ga77581e28069f26d1c21fc6c3f9d35b25.html

[R13] CUB `DeviceSelect`.  
https://nvidia.github.io/cccl/cub/api/file_cub_device_device_select.cuh.html

[R14] CUB `DeviceReduce` determinism notes.  
https://nvidia.github.io/cccl/cub/api/structcub_1_1DeviceReduce.html

[R15] ReproBLAS project (reproducible floating-point operations background).  
https://bebop.cs.berkeley.edu/reproblas/

[R16] Numerical reproducibility for parallel reduction on multi/many-core architectures.  
https://www.sciencedirect.com/science/article/pii/S0167819115001155

[R17] Nsight Systems User Guide (NVTX and timeline correlation).  
https://docs.nvidia.com/nsight-systems/UserGuide/

[R18] Nsight Compute Profiling Guide (occupancy and memory workload analysis).  
https://docs.nvidia.com/nsight-compute/ProfilingGuide/index.html

[R19] NVML API Reference (`nvmlDeviceGetPowerUsage`, telemetry).  
https://docs.nvidia.com/deploy/nvml-api/nvml-api-reference.html

[R20] UE5: Using PCG with GPU Processing.  
https://dev.epicgames.com/documentation/en-us/unreal-engine/using-pcg-with-gpu-processing-in-unreal-engine

[R21] UE5: MassEntity Overview.  
https://dev.epicgames.com/documentation/en-us/unreal-engine/overview-of-mass-entity-in-unreal-engine

[R22] Differential Evolution original paper DOI record.  
https://doi.org/10.1023/A:1008202821328

[R23] cuRAND Quasirandom (Sobol) generator API.  
https://docs.nvidia.com/cuda/curand/group__HOST.html

[R24] Ray Tune scheduler APIs (ASHA and related schedulers).  
https://docs.ray.io/en/latest/tune/api/schedulers.html

[R25] Optuna Hyperband pruner reference.  
https://optuna.readthedocs.io/en/stable/reference/generated/optuna.pruners.HyperbandPruner.html

[R26] EvoTorch documentation (GPU-accelerated evolutionary optimization toolkit).  
https://docs.evotorch.ai/docs-latest/
