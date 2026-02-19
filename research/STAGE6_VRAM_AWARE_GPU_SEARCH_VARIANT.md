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

## 17) Standalone Implementation Blueprint (Blank Repo)

This section is a full build blueprint so the algorithm can be implemented without this repository.

Reference stack (recommended):

1. Rust `1.77+` for host scheduler and artifacts.
2. CUDA `12.x` for kernels.
3. `cust` for CUDA driver API bindings on Rust host.
4. CUB/Thrust for deterministic sort/select [R11][R12][R13][R14].

### 17.1 Minimal Fresh Repo Layout

```text
vdps_gpu_search/
  Cargo.toml
  build.rs
  src/
    main.rs
    config.rs
    model.rs
    rng.rs
    scheduler.rs
    gpu.rs
    artifacts.rs
    kernels/
      vdps_kernels.cu
  benches/
    vdps_vs_grid.rs
  data/
    sample_prices.csv
  scripts/
    run_vdps.ps1
```

### 17.2 `Cargo.toml` (reference)

```toml
[package]
name = "vdps_gpu_search"
version = "0.1.0"
edition = "2021"

[features]
default = []
cuda = ["dep:cust"]

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
rand = "0.8"
rayon = "1"
cust = { version = "0.3", optional = true }

[build-dependencies]
which = "7"
```

### 17.3 `build.rs` (compile CUDA -> PTX)

```rust
use std::{env, path::PathBuf, process::Command};

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let src = "src/kernels/vdps_kernels.cu";
    let ptx = out.join("vdps_kernels.ptx");

    let nvcc = which::which("nvcc").expect("nvcc not found");
    let status = Command::new(nvcc)
        .args([
            "-ptx",
            "-O3",
            "--use_fast_math",
            "-lineinfo",
            src,
            "-o",
            ptx.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run nvcc");
    assert!(status.success(), "nvcc failed");
    println!("cargo:rerun-if-changed={src}");
}
```

## 18) Core Data Contracts (Drop-In)

Use these exact contracts as a starting point.

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VdpsConfig {
    pub eta: u32,
    pub rungs: Vec<f32>,              // e.g. [0.0625, 0.125, 0.25, 0.5, 1.0]
    pub initial_candidates: u32,      // N0
    pub top_k: u32,                   // final K
    pub deterministic: bool,
    pub seed: u64,
    pub random_injection_ratio: f32,
    pub elite_mutation_ratio: f32,
    pub differential_ratio: f32,
    pub max_survivor_seeds: u32,
    pub max_period_candidates: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub id: u32,
    pub fast_len: u16,
    pub slow_len: u16,
    pub aux_a: f32,                   // generic numeric dim
    pub aux_b: f32,                   // generic numeric dim
    pub kind: u16,                    // categorical dim
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Metric {
    pub objective: f32,               // higher is better
    pub risk: f32,                    // lower is better
    pub violations: u32,              // lower is better
    pub valid: u8,                    // 1 valid, 0 invalid
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RungTelemetry {
    pub rung_index: u32,
    pub budget_fraction: f32,
    pub evaluated: u32,
    pub promoted: u32,
    pub dropped: u32,
    pub runtime_ms: u64,
    pub h2d_bytes: u64,
    pub d2h_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct VdpsResult {
    pub best: Option<(Candidate, Metric)>,
    pub top: Vec<(Candidate, Metric)>,
    pub rungs: Vec<RungTelemetry>,
    pub deterministic: bool,
    pub seed: u64,
    pub tie_break_schema_version: u32,
}
```

## 19) Determinism Contract and RNG (Concrete)

Tie-break order (must be fixed globally):

1. `objective` descending
2. `risk` ascending
3. `violations` ascending
4. `(fast_len, slow_len, aux_a_q, aux_b_q, kind)` ascending
5. `id` ascending

Reference deterministic comparator:

```rust
fn key_tuple(c: &Candidate, m: &Metric) -> (u32, u32, u32, u16, u16, i32, i32, u16, u32) {
    // objective as descending key via IEEE bit inversion trick for f32
    let obj_bits = (!m.objective.to_bits()) ^ 0x8000_0000;
    let risk_bits = m.risk.to_bits() ^ 0x8000_0000;
    let aux_a_q = (c.aux_a * 1_000_000.0).round() as i32;
    let aux_b_q = (c.aux_b * 1_000_000.0).round() as i32;
    (obj_bits, risk_bits, m.violations, c.fast_len, c.slow_len, aux_a_q, aux_b_q, c.kind, c.id)
}

fn stable_rank(cands: &mut [(Candidate, Metric)]) {
    cands.sort_by_key(|(c, m)| key_tuple(c, m)); // stable in Rust
}
```

Reference RNG (`splitmix64` seed expander):

```rust
#[derive(Clone)]
pub struct SplitMix64 { state: u64 }

impl SplitMix64 {
    pub fn new(seed: u64) -> Self { Self { state: seed } }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as u32) as f32 / ((1u32 << 24) as f32)
    }
}
```

## 20) Scheduler Algorithm (Reference Implementation)

The scheduler below is intentionally complete enough to build directly.

```rust
pub fn run_vdps(
    cfg: &VdpsConfig,
    mut pool: Vec<Candidate>,
    eval_gpu: impl Fn(&[Candidate], f32) -> anyhow::Result<Vec<Metric>>,
    mutate_elite: impl Fn(&[Candidate], usize, u64) -> Vec<Candidate>,
    mutate_de: impl Fn(&[Candidate], usize, u64) -> Vec<Candidate>,
    inject_random: impl Fn(usize, u64) -> Vec<Candidate>,
) -> anyhow::Result<VdpsResult> {
    anyhow::ensure!(!cfg.rungs.is_empty(), "rungs empty");
    anyhow::ensure!(cfg.top_k > 0, "top_k must be > 0");

    let mut result = VdpsResult {
        deterministic: cfg.deterministic,
        seed: cfg.seed,
        tie_break_schema_version: 1,
        ..Default::default()
    };

    // Deduplicate by parameter tuple + id normalization if needed
    pool.sort_by_key(|c| (c.fast_len, c.slow_len, c.kind, c.id));
    pool.dedup_by_key(|c| (c.fast_len, c.slow_len, c.kind));

    let mut current = pool;
    let eta = cfg.eta.max(2) as usize;

    for (r_idx, &budget) in cfg.rungs.iter().enumerate() {
        let t0 = std::time::Instant::now();
        let metrics = eval_gpu(&current, budget)?;
        anyhow::ensure!(metrics.len() == current.len(), "metrics size mismatch");

        let mut scored: Vec<(Candidate, Metric)> = current.iter().copied().zip(metrics).collect();
        stable_rank(&mut scored);

        let target_next = if r_idx + 1 < cfg.rungs.len() {
            (scored.len() / eta).max(1)
        } else {
            cfg.top_k as usize
        };
        let survivors: Vec<Candidate> = scored.iter().take(target_next).map(|x| x.0).collect();

        result.rungs.push(RungTelemetry {
            rung_index: r_idx as u32,
            budget_fraction: budget,
            evaluated: scored.len() as u32,
            promoted: survivors.len() as u32,
            dropped: (scored.len().saturating_sub(survivors.len())) as u32,
            runtime_ms: t0.elapsed().as_millis() as u64,
            h2d_bytes: 0,
            d2h_bytes: 0,
        });

        if r_idx + 1 == cfg.rungs.len() {
            result.top = scored.into_iter().take(cfg.top_k as usize).collect();
            result.best = result.top.first().copied();
            break;
        }

        // Refill
        let next_target = (cfg.initial_candidates as usize / eta.pow((r_idx + 1) as u32)).max(target_next);
        let refill_needed = next_target.saturating_sub(survivors.len());
        let n_elite = ((refill_needed as f32) * cfg.elite_mutation_ratio).round() as usize;
        let n_de = ((refill_needed as f32) * cfg.differential_ratio).round() as usize;
        let n_rand = refill_needed.saturating_sub(n_elite + n_de);

        let s = cfg.seed ^ ((r_idx as u64) << 32);
        let mut next = survivors.clone();
        next.extend(mutate_elite(&survivors, n_elite, s ^ 0xA5A5_A5A5));
        next.extend(mutate_de(&survivors, n_de, s ^ 0x5A5A_5A5A));
        next.extend(inject_random(n_rand, s ^ 0x1234_5678));

        // Constraint projection and dedup
        project_constraints(&mut next);
        next.sort_by_key(|c| (c.fast_len, c.slow_len, c.kind, c.id));
        next.dedup_by_key(|c| (c.fast_len, c.slow_len, c.kind));
        current = next;
    }

    Ok(result)
}

fn project_constraints(cands: &mut [Candidate]) {
    for c in cands {
        c.fast_len = c.fast_len.max(2).min(5000);
        c.slow_len = c.slow_len.max(c.fast_len.saturating_add(1)).min(5000);
        c.aux_a = c.aux_a.clamp(-10.0, 10.0);
        c.aux_b = c.aux_b.clamp(-10.0, 10.0);
    }
}
```

## 21) CUDA Interfaces and Kernel Contracts

### 21.1 Host-side GPU API

```rust
#[cfg(feature = "cuda")]
pub fn eval_candidates_gpu(
    candidates: &[Candidate],
    budget_fraction: f32,
    prices: &[f32],           // full data; budget selects prefix length
    cfg: &GpuRuntimeConfig,
) -> anyhow::Result<Vec<Metric>> { /* launch path below */ }
```

### 21.2 Kernel signatures (C++)

```cpp
extern "C" __global__
void eval_candidates_kernel(
    const Candidate* __restrict__ cands,
    int n_cands,
    const float* __restrict__ prices,
    int t_budget,
    Metric* __restrict__ out_metrics);

extern "C" __global__
void mutate_elite_kernel(
    const Candidate* __restrict__ elite,
    int n_elite,
    uint64_t seed,
    Candidate* __restrict__ out_cands,
    int out_n);

extern "C" __global__
void mutate_de_kernel(
    const Candidate* __restrict__ surv,
    int n_surv,
    uint64_t seed,
    Candidate* __restrict__ out_cands,
    int out_n);
```

### 21.3 Device deterministic scoring notes

1. Keep per-candidate reduction in one thread or fixed tree order.
2. Avoid unordered atomics in ranking key accumulation.
3. If using block reductions, ensure fixed lane order and no race-dependent tie handling.

## 22) VRAM Budgeting and Out-of-Core Batch Sizing

Reference formula:

```rust
fn choose_batch_size(
    free_vram_bytes: usize,
    reserve_ratio: f32,       // e.g. 0.20
    bytes_per_candidate: usize,
    max_batch: usize,
) -> usize {
    let reserve = (free_vram_bytes as f32 * reserve_ratio) as usize;
    let usable = free_vram_bytes.saturating_sub(reserve);
    let raw = usable / bytes_per_candidate.max(1);
    raw.clamp(1, max_batch)
}
```

Out-of-core execution loop:

1. Split candidate list into `batch_size` chunks.
2. H2D copy chunk.
3. Launch eval kernel.
4. D2H copy compact metrics only.
5. Keep global top-K heap on device if possible; otherwise host stable merge.

## 23) Artifact JSON (Write Exactly)

```json
{
  "deterministic": true,
  "seed": 42,
  "tie_break_schema_version": 1,
  "best": {
    "candidate": { "id": 17, "fast_len": 21, "slow_len": 55, "aux_a": 0.3, "aux_b": -0.2, "kind": 1 },
    "metric": { "objective": 1.42, "risk": 0.19, "violations": 0, "valid": 1 }
  },
  "top": [],
  "rungs": [
    { "rung_index": 0, "budget_fraction": 0.125, "evaluated": 4096, "promoted": 1024, "dropped": 3072, "runtime_ms": 84, "h2d_bytes": 0, "d2h_bytes": 0 }
  ]
}
```

## 24) CLI Contract (Fresh Repo)

Example commands:

```powershell
# Run brute-force grid baseline
cargo run --release --features cuda -- `
  --mode grid `
  --data data/sample_prices.csv `
  --fast-start 5 --fast-end 500 --fast-step 1 `
  --slow-start 10 --slow-end 1000 --slow-step 1 `
  --top-k 20 `
  --out artifacts/grid.json

# Run VDPS
cargo run --release --features cuda -- `
  --mode vdps `
  --data data/sample_prices.csv `
  --eta 4 `
  --rungs 0.0625,0.125,0.25,0.5,1.0 `
  --initial-candidates 4096 `
  --top-k 20 `
  --deterministic true `
  --seed 42 `
  --out artifacts/vdps.json
```

## 25) Minimal Test Suite Required for "Done"

Implement these tests exactly:

1. Determinism:
   same seed + same GPU + same data -> byte-identical top-K tuple list.
2. Constraint projection:
   all produced candidates satisfy hard bounds and cross-parameter constraints.
3. Rung monotonicity:
   promoted candidate must have appeared in previous rung evaluated set.
4. Quality guard:
   on reference datasets, top-K overlap vs grid >= target threshold (e.g. 0.90).
5. Scale guard:
   on large out-of-core case, VDPS wall time < grid wall time by target ratio.

## 26) Benchmark Harness (From Scratch)

Criterion benchmark groups (or equivalent):

1. `grid/small`, `grid/medium`, `grid/large`, `grid/xlarge_batched`.
2. `vdps/small`, `vdps/medium`, `vdps/large`, `vdps/xlarge_batched`.
3. Report:
   `time_to_topk`, `best_regret`, `topk_overlap`, `h2d_bytes`, `d2h_bytes`.

This is enough to reproduce the crossover behavior:

- small spaces: brute force often faster,
- larger VRAM-constrained spaces: VDPS often wins.

## 27) UE-Oriented A/B Implementation Mapping

If applying this to UE loading-screen compute:

1. Baseline A:
   CPU + RAM preprocessing.
2. Variant B:
   GPU + VRAM-first batched pipeline.

When data exceeds VRAM:

- stage chunk windows from RAM/SSD to VRAM,
- keep scoring state and top-K merge state GPU-resident where possible,
- return only compact telemetry and final outputs to CPU.

This maps directly to out-of-core behavior in Section 22 and preserves the A/B framing.

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
