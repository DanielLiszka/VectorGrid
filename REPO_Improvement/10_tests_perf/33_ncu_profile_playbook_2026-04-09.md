# Nsight Compute Playbook Plan

Last updated: 2026-04-09

## Purpose

Define a repeatable, non-authoritative Nsight Compute profiling workflow for GPU kernel investigation.

## Scope Boundaries

In scope:

- the profiling wrapper `scripts/profile_gpu_backtest_ncu.ps1`
- runner and bench target modes for Nsight Compute
- kernel-filter, workload, metric-contract, dynamic-strategy, and CUDA-graphs parameters that influence capture meaning
- provenance output for `.ncu-rep` captures

Out of scope:

- options-related CUDA profiling
- authoritative perf gating
- whole-process timeline analysis, which belongs to Nsight Systems
- CUPTI metric inventory/capture, which belongs to a separate doc

## Current User-Facing Behavior

The repo already has a substantial Nsight Compute wrapper that can profile either the standalone runner binary or a benchmark executable, discover a local `ncu` path, set relevant CUDA environment variables, export a report, and write a provenance JSON beside it. That is good because the repo does not depend on manual profiler invocation only, but it also means capture meaning depends heavily on mode, kernel filter, workload, and environment flags.

## Current Code Surface

- `scripts/profile_gpu_backtest_ncu.ps1` orchestrates Nsight Compute capture and provenance emission.
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs` provides the runner target surface.
- `crates/app_core/Cargo.toml` and `double_ma_opt` bench provide the bench target surface.
- `target/nsight/` or another chosen output prefix stores `.ncu-rep` and provenance files.

## Current Verification Surface

- `powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1 -Target runner`
- `powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1 -Target bench -BenchFilter <id>`
- review of generated `.ncu-rep` and `.provenance.json` files

## Strengths Already Present

- the wrapper already records profiler provenance and requested arguments
- it supports both runner and bench modes, which helps isolate different GPU questions
- environment toggles such as dynamic strategy and CUDA graphs are surfaced explicitly
- the script checks that a report was actually generated instead of silently succeeding with no profile

## Weaknesses / Failure Modes

- local Nsight Compute installation paths and versions vary
- kernel-filter mistakes can easily produce empty or misleading captures
- runner and bench targets have different timing boundaries and should not be compared naively
- heatmap, shortlist, and graph flags can change the code path being profiled more than users may expect

## Regressions To Avoid

- treating Nsight Compute results as gate authority instead of diagnostic evidence
- capturing without a provenance file or without recording the exact target mode
- profiling one route while assuming the report represents another route
- forgetting that filter and mode changes alter the meaning of the capture

## External Research Summary

- [Nsight Compute docs](https://docs.nvidia.com/nsight-compute/) are relevant because this playbook is built directly around Nsight Compute capture behavior.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because kernel profiling is useful only when tied back to sound GPU measurement discipline.
- [Cargo benchmark command docs](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are relevant because one profiling mode targets built bench executables.
- [Cargo build command docs](https://doc.rust-lang.org/cargo/commands/cargo-build.html) are relevant because the runner mode depends on feature-gated release builds of a dedicated profiling binary.

## Proposed Improvement Approach

1. Keep Nsight Compute profiling documented as a repeatable diagnostic path, not as perf-gate authority.
2. Require every capture to record target mode, workload, filter, environment flags, and output provenance.
3. Treat runner and bench captures as different investigative tools rather than interchangeable numbers.
4. Use Nsight Compute when the GPU gate or route matrix points to a kernel-level question, not as the first-line tool for every change.

## Checklist

- [ ] Document both runner and bench capture modes clearly.
- [ ] Document the capture parameters that materially change report meaning.
- [ ] Keep provenance JSON generation mandatory.
- [ ] Keep Nsight Compute playbooks non-options only.
- [ ] Tie capture review back to a concrete regression or investigation question.
- [ ] Avoid quoting kernel-profile outputs as standalone performance claims.

## Verification Plan

- run the wrapper in the intended mode and confirm a `.ncu-rep` and provenance JSON are produced
- inspect provenance before trusting the report interpretation
- confirm the kernel filter actually matches the target kernel when filtering is enabled
- cross-check the profiled mode against the route or gate failure that motivated the run

## Rollout / Fallback / Compatibility Notes

- preserve the current wrapper and improve documentation around when and how to use it
- prefer better provenance and narrower capture questions over adding more flags casually
- if local Nsight path drift becomes common, improve wrapper discovery before weakening the playbook

## Related Docs

- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/34_nsys_profile_playbook_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/35_cupti_sass_probe_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/38_profile_runner_bins_2026-04-09.md`
