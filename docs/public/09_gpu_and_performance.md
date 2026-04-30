# GPU And Performance

VectorGrid includes CPU and GPU execution routes. GPU support is powerful where the selected workflow is built for it, but it is not a universal speed switch.

## Release Variants

Public release pages should identify the variant clearly:

| Variant | Meaning |
|---|---|
| CPU-only | Broadest compatibility, no NVIDIA runtime required. |
| GPU MA | GPU-enabled moving-average/indicator route where released. |
| GPU kernel | GPU kernel route for supported full-kernel workflows. |

If a release does not publish a GPU variant, do not use GPU benchmark claims as install expectations.

## Backend Labels

Use the app's route label as the source of truth:

- `CpuOnly`: CPU execution.
- `GpuOnly`: GPU indicator work with CPU backtest continuation.
- `GpuKernel`: supported GPU kernel route.
- `GpuHybridKernel`: hybrid route.
- `Auto`: selector that resolves to one of the real routes.

Screenshots, summaries, release notes, and support reports should include this label.

## What Usually Benefits From GPU

GPU acceleration is most useful when:

- the workload has many similar candidate evaluations;
- the route keeps useful buffers resident on the GPU;
- the cost of transferring data is small compared with computation;
- the strategy is implemented in a kernel-backed path;
- the dataset and grid are large enough to amortize launch overhead.

Double-MA-family exact-grid workloads are the clearest public example.

## What May Not Benefit From GPU

GPU acceleration may not help when:

- the dataset is small;
- the candidate set is tiny;
- the strategy has branch-heavy or CPU-preferred logic;
- a validation module requires CPU-side orchestration;
- the route copies large intermediate results back and forth;
- a GPU-intensive process is already running on the machine.

Some advanced validation routes intentionally prefer CPU because current measurements show CPU faster for that workload.

## Benchmark Claims

Benchmark claims must name:

- VectorGrid version or commit;
- release variant;
- hardware;
- dataset size;
- candidate count;
- strategy;
- objective/metrics boundary;
- warm/cold run boundary;
- whether data was already loaded;
- CPU/GPU route;
- benchmark date.

Do not compare a warm GPU kernel path against an unrelated cold full workflow without saying so.

## User Expectations

Users should expect:

- CPU-only workflows to be the most predictable;
- GPU routes to be workload- and strategy-specific;
- first run to include setup/warmup overhead;
- repeated runs on the same data to be more representative for resident-buffer workflows;
- other GPU-heavy programs to affect timing.

## GPU Troubleshooting

If a GPU run fails or falls back:

- verify you installed a GPU release variant;
- update NVIDIA drivers;
- confirm the app diagnostics see the GPU;
- reduce dataset size or grid size;
- close unrelated GPU-heavy applications;
- try CPU-only to separate data issues from GPU runtime issues;
- include route metadata and warnings in bug reports.

## CUDA Toolkit And Profiling

End users should not need NVIDIA profiling tools.

Maintainers doing CUDA development may use Nsight Compute, Nsight Systems, CUDA profiling scripts, and GPU perf gates. Those tools may require an elevated PowerShell session.

