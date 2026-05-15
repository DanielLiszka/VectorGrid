# GPU, CUDA, And Performance Expansion Plan

Date: 2026-04-30

## Goal

Expand GPU acceleration with strict CPU fallback, measured residency claims, explicit PTX/native asset handling, and serialized benchmark/profiling evidence. GPU work should improve product routes without making CPU-only builds harder.

## Current Owner Files

Core GPU/runtime:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/vram_ma.rs`
- `crates/app_core/src/cuda_device.rs`
- `crates/app_core/src/standalone_gpu_runtime.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/kernels/double_crossover.prebuilt.ptx`
- `crates/app_core/src/kernels/options_expression.cu`
- `crates/app_core/src/kernels/options_expression.prebuilt.ptx`

Strategy GPU modules:

- `crates/app_core/src/*_gpu_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/options_expression_gpu.rs`

Scripts:

- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_validation_perf_gate.ps1`
- `scripts/run_gpu_followup_perf_gate.ps1`
- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`
- `scripts/profile_gpu_backtest_nsys.ps1`
- `scripts/profile_options_expression_ncu.ps1`
- `scripts/profile_options_expression_nsys.ps1`
- `scripts/profile_options_expression_compute_sanitizer.ps1`
- `scripts/benchmark_lock.ps1`

## Research Basis

- CUDA Best Practices emphasizes device-resident intermediate data, batched host/device transfers, coalesced global memory access, and occupancy as performance considerations: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
- CUDA C++ Programming Guide is the baseline for programming model, memory hierarchy, streams, and runtime behavior: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/contents.html>
- CUDA Graphs can reduce repeated launch overhead for stable execution patterns, but only after correctness, residency, and workload shape are proven: <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html#cuda-graphs>
- NVIDIA's CCCL/CUB primitives are candidates for reductions/top-k style work when custom kernels would duplicate mature building blocks: <https://nvidia.github.io/cccl/cub/>
- Compute Sanitizer should be used for memory correctness investigations before treating profiling numbers as meaningful: <https://docs.nvidia.com/compute-sanitizer/ComputeSanitizer/index.html>
- NVTX ranges can make Nsight traces explainable across Rust orchestration, CUDA API calls, and kernels: <https://nvidia.github.io/NVTX/>
- Nsight Systems is suited for timeline, CPU/GPU interaction, CUDA API, memory-transfer, and process-level tracing: <https://docs.nvidia.com/nsight-systems/UserGuide/>
- Nsight Compute CLI is suited for kernel-level metric collection and report generation: <https://docs.nvidia.com/nsight-compute/NsightComputeCli/index.html>

## Global GPU Rules

- CPU-only builds must compile and run without CUDA toolkit, NVIDIA driver, or GPU.
- `cuda` and `cuda-backtest-kernel` feature wiring must stay explicit.
- Prebuilt PTX routes must be release-manifested when public claims depend on them.
- No GPU route should silently fall back to CPU under a GPU performance claim.
- Benchmark and profiling runs must be serialized.

## Expansion Track A: Resident Buffer Inventory

### Problem

GPU expansion requires knowing which data is actually resident and which paths copy to host.

### Implementation

Create a docs/code-adjacent inventory first:

- inputs: prices, candles, indicators, masks, candidates;
- owner allocation;
- lifetime;
- device pointer type;
- host copy points;
- downstream consumer;
- fallback path.

Potential new diagnostic:

- `crates/app_core/src/gpu_residency_probe.rs`
- `src-tauri/src/bin/ta_gpu_residency_probe.rs`

### Acceptance Criteria

- Double-MA pure VRAM path remains clear.
- Dynamic indicator path ownership/keepalive rules are documented.
- Any returned device matrix used downstream keeps owner alive.

## Expansion Track B: GPU Validation Productization

### Problem

Validation GPU route claims need metadata and perf evidence distinct from optimization.

### Implementation

Current owners:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `scripts/run_gpu_validation_perf_gate.ps1`

Add:

- validation-specific route metadata;
- device/driver/build identity in perf summaries;
- CPU tail attribution;
- workload profile labels;
- quick vs release-authority evidence labels.

### Tests/Gates

- app-core validation tests;
- `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`;
- `-PlanOnly` for case registry review when GPU is busy.

## Expansion Track C: Kernel Profiling Workflow

### Problem

Performance work needs repeatable profiling with scope and artifact naming.

### Implementation

Extend profiling scripts to require:

- no competing benchmark/profiler process;
- case id;
- output folder;
- git/toolchain/device identity;
- timing boundary;
- profiler version;
- elevated/admin note for Windows GPU counters.

Use:

- Nsight Systems for timeline/transfer/overlap questions;
- Nsight Compute for kernel occupancy, memory throughput, instruction-level questions;
- compute-sanitizer for options kernel correctness when memory behavior changes.

### Acceptance Criteria

- A profile artifact can be tied to source revision and case id.
- Profiling output is companion evidence, not correctness authority.
- Scripts do not overwrite baseline summaries.

## Expansion Track D: Options PTX Release Policy

### Problem

The repo contains `options_expression.prebuilt.ptx`, but current release authority notes do not make it a public-required resource.

### Implementation

Only if options GPU is promoted:

- add options PTX to release manifest required resources for relevant variant;
- add CPU fallback when PTX missing;
- add options GPU route label to release docs;
- add options promotion gate to release candidate gate only if public claims depend on it.

### Kill Criteria

- CPU/GPU parity is unstable;
- PTX cannot be packaged reproducibly;
- route requires CUDA toolkit at runtime for public builds.

## Expansion Track E: New GPU Strategy Promotion

### Problem

Composite-side GPU modules exist for multiple strategies. Promotion needs route/evidence uniformity.

### Implementation

For each strategy:

- define CPU oracle fixture;
- define candidate grid and objective fixture;
- define expected route metadata;
- add CPU/GPU parity;
- add microbench only after correctness;
- add route capability to frontend.

Do not promote `CompositeMo` to GPU without separate multi-objective design.

### Verification

- strategy-local unit tests;
- app-core parity tests;
- GPU perf gate for performance claims;
- workflow gate for product workflow timing claims.

## Second-Pass Expansion: Launch, Transfer, And Reduction Experiments

### Problem

The first-pass GPU plan emphasizes residency, but the next performance gains may depend on launch overhead, pinned transfers, or reduction primitives rather than new strategy logic.

### Implementation

Create experiments, not product commitments:

- pinned host memory for large one-time uploads;
- persistent device buffers for repeated validation windows;
- CUDA Graph capture for stable repeated kernel sequences;
- CUB/CCCL reductions for top-k, min/max, and aggregate metrics;
- NVTX range annotations around Rust stages and CUDA kernels;
- Compute Sanitizer pre-check for kernels touched by memory-layout changes.

Experiment record:

- case id;
- workload dimensions;
- feature flags;
- device identity;
- driver/runtime version;
- timing boundary;
- host/device transfer count;
- launch count;
- CPU fallback count;
- before/after summary.

Kill criteria:

- experiment requires CUDA toolkit or driver for CPU-only builds;
- improvement is within noise after serialized rerun;
- added complexity obscures route ownership;
- CUDA Graph capture cannot tolerate current dynamic workload shape.

## Second-Pass Expansion: PTX And Native Asset Matrix

### Problem

GPU and ML release resources are split across kernels and native libraries. A release plan needs exact resource names and variant rules.

### Current Resource Anchors

- `crates/app_core/src/kernels/double_crossover.prebuilt.ptx`
- `crates/app_core/src/kernels/options_expression.prebuilt.ptx`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/kernels/options_expression.cu`
- CatBoost native resources referenced by ML Signal release scripts and manifests.

### Policy

- CPU public variant: must not require PTX or native ML DLLs at runtime.
- GPU double-crossover variant: requires `double_crossover.prebuilt.ptx` if a public GPU claim depends on it.
- Options GPU variant: requires `options_expression.prebuilt.ptx` only if options GPU route is public or release-supported.
- ML native variant: requires model/native resource hashes only when native ML inference is part of the selected release variant.

### Verification

- `scripts/generate_release_manifest.ps1` records resource presence and hash;
- release doctor verifies required resources by variant;
- missing optional resources produce explicit unsupported/fallback labels, not startup failure.

## Non-Goals

- No CUDA requirement for CPU public variant.
- No tensor-core or RT-core production work without CPU oracle and benchmark proof.
- No concurrent benchmarks/profilers.
- No moving all strategy logic into one generic GPU abstraction.

## Verification

Targeted:

- `cargo test -p ta_app_core`
- strategy-specific app-core tests.

GPU:

- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_followup_perf_gate.ps1 -Mode candidate`

Profiling:

- `powershell -File scripts/profile_gpu_backtest_nsys.ps1`
- `powershell -File scripts/profile_gpu_backtest_ncu.ps1`

Broad:

- `powershell -File scripts/run_repo_quality_gate.ps1`
