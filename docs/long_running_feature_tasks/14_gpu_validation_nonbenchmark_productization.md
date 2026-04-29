# GPU Validation Non-Benchmark Productization

Status: docs and disabled-CI metadata implemented while CUDA benchmarks are deferred
Risk: high for route/runtime changes; low for documentation and disabled metadata
Primary value: preserve GPU validation evidence boundaries without running CUDA benchmarks during unrelated GPU load

## Summary

GPU validation route metadata already exists in backend and frontend/report surfaces. The current safe pass should not change CUDA kernels, route policy, residency behavior, or performance claims. It should tighten documentation and disabled-CI metadata so future CUDA evidence can be collected intentionally once the GPU is available and idle.

NVIDIA tooling remains the right future validation stack: Compute Sanitizer for correctness-sensitive CUDA memory/race/uninitialized checks, Nsight Systems for workflow timeline and transfers, and Nsight Compute for kernel-level profiling after the timeline identifies a kernel bottleneck.

Sources:

- NVIDIA Compute Sanitizer: https://docs.nvidia.com/compute-sanitizer/ComputeSanitizer/index.html
- NVIDIA Nsight Systems User Guide: https://docs.nvidia.com/nsight-systems/UserGuide/
- NVIDIA Nsight Compute Profiling Guide: https://docs.nvidia.com/nsight-compute/ProfilingGuide/index.html

## Current Anchors

- Existing parent plan: `04_gpu_validation_route_productization.md`
- Backend validation metadata: `crates/app_core/src/double_ma_validate.rs`, `crates/app_core/src/composite_validate.rs`
- Frontend route details: `web/src/services/validationService.ts`
- Report metadata: `src-tauri/src/commands_reports_validation_common.rs`
- Release evidence authority: `docs/verification_benchmark_release_authority.md`
- Release gate script: `scripts/run_release_candidate_gate.ps1`
- Disabled workflows: `.github/workflows.disabled/`

## Existing Behavior

- Validation result metadata can include requested backend, residency tier, route details, and CPU-tail timing.
- GPU validation perf authority is separate from generic GPU perf authority.
- Release gate only invokes GPU validation perf when explicitly requested for the relevant backend.
- GitHub workflows are disabled and non-authoritative.

## Additive Boundary

- No `.cu`, PTX, VectorTA, route planner, backend policy, or runtime behavior changes in this slice.
- No CUDA benchmarks or Nsight runs while another GPU-intensive task may be running.
- No baseline refresh.
- No active workflow reactivation.
- Disabled-CI metadata must not redefine local release authority.

## Non-Goals

- No new GPU kernel.
- No route-policy expansion.
- No performance claims.
- No residency guarantee changes.
- No generated benchmark evidence.

## Implementation Checklist

### Phase 1: Documentation Tightening

- [x] Update GPU task docs to state the current slice is docs/metadata only.
- [x] Cross-link release authority and validation-heavy benchmark authority.
- [x] Mark CUDA benchmarks/profiling as deferred until GPU is idle.
- [x] Document which scripts are inventory-only versus benchmark-producing.

### Phase 2: Disabled-CI Metadata

- [x] Update `.github/workflows.disabled/README.md` with future optional GPU validation requirements.
- [ ] If YAML comments are added, keep workflows disabled.
- [ ] Require hardware labels, manual trigger, unique artifact names, and no default PR inclusion for future GPU jobs.

### Phase 3: Future CUDA Evidence Plan

- [ ] Run correctness checks before profiling when CUDA code changes.
- [ ] Run Nsight Systems before Nsight Compute.
- [ ] Keep benchmarks serialized and verify no other benchmark/profiling process is active.
- [ ] Record GPU identity, driver/runtime versions, dataset hash, and route metadata.

## Validation Checklist

- [ ] `git diff --check`
- [ ] Optional YAML parse/check if disabled workflow YAML changes.
- [ ] If frontend route text changes: `npm --prefix web run test -- validationService` and `npm --prefix web run typecheck`.
- [ ] If report metadata changes: `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests`.
- [ ] Do not run CUDA benchmark/profiling gates in this pass.

## Regression Watchpoints

- Docs must not imply fresh CUDA performance evidence.
- Disabled workflows must remain disabled.
- Release authority must remain local scripts until CI is intentionally reactivated.
- Inventory commands must not produce benchmark baselines or mutate evidence.

## Definition Of Done

- GPU validation docs and disabled-CI metadata clearly describe future evidence collection.
- CUDA benchmark/profiling work is explicitly deferred.
- No runtime route behavior changed.
