# OptiX and RT-Core Prove-Or-Kill Decision

Status: killed for product implementation; no spike accepted
Date: 2026-04-27
Dossier: 6 - OptiX And RT-Core Prove-Or-Kill

## Decision

Do not add OptiX or RT-core product code to VectorGrid.

This means no OptiX backend, RT-core route, Cargo feature, dependency, SDK probe, Tauri command, capability entry, frontend toggle, route metadata, report field, artifact schema field, release variant, or installer/package step.

Current CPU-only and CUDA behavior must remain unchanged. Any future RT-core or OptiX work is blocked unless a separate profiling-backed research spike is accepted.

## Why

The current VectorGrid workloads do not naturally map to ray or BVH traversal strongly enough to justify OptiX as a product backend.

- Options-expression barrier hits are chronological financial first-hit scans with exact CPU-oracle semantics. They include stop/target priority, cutoff behavior, stress lanes, fill assumptions, and multi-leg alignment.
- Current options-expression GPU scoring is aggregate tuple-lane tensor scoring over precomputed outcome data, not path discovery.
- Double-MA optimization and validation are branch-heavy temporal simulations with strategy state, stops, overlays, regime filters, metric accumulation, and route diagnostics.
- Validation, report generation, artifact provenance, and route transparency are not geometric workloads.
- CPU-only builds are the default invariant, and current CUDA support is already feature-gated without any OptiX runtime requirement.

The hard problem behind this dossier is deterministic earliest-first-hit path evaluation under exact financial semantics, not generic ray tracing.

## Explicitly Forbidden Without A New Accepted Spike

The following are not allowed as follow-on implementation work from this decision:

- OptiX dependency, SDK wrapper, Rust binding, C++ bridge, `bindgen` surface, or loader.
- Cargo feature such as `optix`, `rt-core`, `rtx`, `ray_tracing`, or similar.
- Backend route such as `Optix`, `GpuRtCore`, `GpuRay`, or `GpuExperimentalRt`.
- Tauri command, Tauri capability entry, generated command-map change, frontend service/store field, or UI toggle.
- Report, artifact, validation, or route-provenance field implying an OptiX product route exists.
- Release packaging or installer promise for OptiX, RT cores, OptiX DLLs, SDK paths, or driver probes.
- Generic shared RT execution layer across options, Double-MA, validation, or reporting.
- Speculative CPU sparse-table/RMQ, segment-tree, binary-lifting, or CUDA RMQ implementation as part of this Dossier 6 scope.

These terms are acceptable inside this decision document and future research notes. They should not appear in product code, build files, generated command files, capabilities, schemas, or release docs unless a future spike is separately accepted.

## Candidate Mapping

| VectorGrid computation | Natural ray/BVH fit? | Simpler path before OptiX | Decision |
|---|---:|---|---|
| Options single-leg stop/target barrier hit | Weak | Profile first; then CPU sparse table/RMQ, segment tree, binary lifting, blocked threshold index, or CUDA scan/RMQ if justified. | Kill product implementation. |
| Options multi-leg barrier hit | Poor | Precomputed net paths, CPU/CUDA interval index, improved summary cache. | Kill. |
| Options event-path stress lanes | Weak | Existing outcome tensors, host exactification, summary-cache profiling. | Kill product OptiX. |
| Options candidate lookup by delay, DTE, moneyness, hold, target, stop | No | Dense offsets, sorted vectors, buckets, direct tensor indexing. | Kill. |
| Contract eligibility, expiration, DTE, moneyness lookup | No | Partition-point, per-expiration buckets, sorted indexes. | Kill. |
| Options aggregate GPU scoring | No | Existing CUDA scorer, chunk tuning, cache reuse, CUDA Graphs only if measured. | Kill. |
| Double-MA crossover/backtest | No | Existing CUDA kernel path, GPU-resident validation work, top-k/reduction and launch-overhead profiling. | Kill. |
| Validation/report/provenance | No | Existing validation and reporting pipeline. | Not applicable. |
| Hypothetical static range-min/max workload | Plausible research only | CPU/CUDA RMQ first, with exact parity proof. | Standalone research sandbox only after evidence. |

## Future Spike Admission Criteria

A future owner may ask to reopen the decision only after profiling proves all of the following:

1. A concrete current phase dominates wall time, such as options candidate compile, options summary compile, CPU score, GPU score, exactify, validation, report assembly, or a Double-MA GPU phase.
2. The dominant phase is specifically a first-hit, path-range, interval-query, or range-min/max workload.
3. The workload is not primarily aggregate tensor scoring, direct offset lookup, branch-heavy temporal simulation, CPU exactification, report assembly, Tauri orchestration, memory transfer, or host synchronization.
4. The query geometry is static or mostly static across many repeated queries, so acceleration-structure build cost can be amortized.
5. Query volume is large enough to matter across events, tuples, thresholds, stress lanes, contracts, and repeated immutable paths.
6. Existing benchmark surfaces show the bottleneck with timing breakdowns, not just intuition.
7. Simpler CPU and CUDA baselines have been implemented or benchmarked and are insufficient.

OptiX cannot be the first attempted implementation. It is admissible only after current-path profiling and lower-risk CPU/CUDA baselines fail.

## Required Baselines Before OptiX

| Suspected bottleneck | Required baseline evidence before OptiX |
|---|---|
| Single-leg barrier first-hit | Existing scan plus CPU sparse table/RMQ, segment tree or binary lifting, and CUDA scan/RMQ where practical. |
| Multi-leg barrier first-hit | Existing aligned scan plus precomputed net high/low paths, CPU interval index, and CUDA scan/RMQ where practical. |
| Event-path threshold lookup | Existing CPU evaluator plus per-contract range preprocessing and CUDA preprocessing where practical. |
| Options aggregate tuple scoring | Existing CUDA scorer, chunk-size tuning, cache reuse analysis, and transfer/sync breakdown. |
| Double-MA route cost | Existing CUDA kernel profiling, GPU-resident validation work, top-k/reduction profiling, and CUDA Graph launch-overhead analysis where measured. |
| Candidate lookup | Direct tensor offsets, sorted vectors, bucket indexes, and binary search. |

## Minimum Evidence For Any Future OptiX Sandbox

A standalone OptiX microbenchmark remains killed unless it demonstrates:

- at least 2x end-to-end speedup versus the best simple CPU/CUDA baseline;
- timing that includes acceleration-structure build/update, traversal, memory transfer, host synchronization, and exactification;
- no CPU-oracle drift for exit timestamp, exit reason, exit price, stop/target priority, fill model, stress lane, or multi-leg alignment;
- no CPU-only build impact;
- no current CUDA build impact;
- no release packaging regression;
- no route, fallback, report, or artifact provenance ambiguity.

## Kill Criteria

Keep the idea killed before or during any future proposal if any of these are true:

1. The workload is not naturally expressible as ray/BVH traversal, interval intersection, path hit testing, or range query.
2. The workload requires frequent acceleration-structure rebuilds that cannot be amortized.
3. The workload is actually aggregate tensor scoring, direct offset lookup, branch-heavy temporal simulation, validation report assembly, or CPU exactification.
4. Custom OptiX intersection programs would perform most financial logic, making the path SM-bound rather than traversal-bound.
5. CPU sparse table/RMQ, segment tree, binary lifting, or plain CUDA scan/RMQ closes the performance gap with lower risk.
6. End-to-end speedup is less than 2x versus the best simple CPU/CUDA baseline, including build, update, transfer, synchronization, and exactification costs.
7. CPU oracle parity fails for exit timestamp, exit reason, exit price, stop/target priority, fill model, stress lane, cutoff, or multi-leg alignment.
8. CPU-only builds require OptiX, CUDA Toolkit, NVIDIA driver, SDK paths, DLLs, or runtime probes.
9. Current CUDA builds require OptiX or acquire new packaging assumptions.
10. A UI, API, report, route, or artifact field is proposed before parity and speed are proven.
11. Route/fallback metadata cannot clearly distinguish CPU, CUDA, and any experimental OptiX path.
12. Release docs or installer behavior would need to promise a new GPU runtime surface before it is proven.
13. Nsight or benchmark evidence shows build cost, host sync, memory transfer, or exactification dominates traversal savings.
14. The spike requires touching optional UI/Tauri files before those files are reviewed.
15. The implementation creates a large generic execution layer instead of a bounded strategy-local experiment.

## Verification Authority

No benchmark or test gate is required for this documentation-only decision. Review should confirm:

- only this decision document is added;
- no production Rust, CUDA, Tauri, frontend, schema, capability, generated command, release, or build files changed;
- no Cargo feature, dependency, environment variable, SDK path, or package step was added;
- the decision names future spike blockers, baseline requirements, hard kill criteria, and optional-file boundaries.

If repository policy requires hygiene checks for documentation-only changes, use `git diff --check`. Broader repo quality gates are not required by this decision.

If a future accepted spike challenges this decision, the minimum evidence package must include:

- CPU-only build proof;
- CUDA build proof that does not require OptiX;
- isolated OptiX sandbox build proof, outside product code;
- CPU oracle parity tests for options exit semantics and stress lanes where applicable;
- current CPU/CUDA baseline comparisons;
- GPU performance and profiling evidence when practical;
- release/package proof showing no CPU-only or current-CUDA regression;
- route/provenance design before any user-visible integration.

## Optional-File Boundary

No accepted Dossier 6 step depends on optional UI or Tauri files.

The following files become required only if a future scope proposes options path replay, event inspection, UI presentation, or a Tauri-facing experimental route:

- `web/src/components/OptionsExpressionPanel.vue`
- `web/src/stores/optionsExpressionStore.ts`
- `src-tauri/src/commands_options_expression.rs`

Until that future scope is explicitly accepted, those files should not be modified for OptiX or RT-core work.

## Current User-Facing Answer

If asked whether VectorGrid currently uses RT cores or OptiX:

> No. OptiX and RT-core acceleration are intentionally not implemented. Current GPU acceleration uses existing feature-gated CUDA paths. OptiX remains killed for product implementation unless future profiling proves a static reusable interval/path-hit workload and simpler CPU/CUDA baselines fail.

## Rollback

Rollback is deletion of this decision document. No runtime, build, package, schema, report, UI, or artifact cleanup is needed.
