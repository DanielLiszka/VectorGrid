# Nsight Compute Playbook Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Nsight Compute docs](https://docs.nvidia.com/nsight-compute/) are relevant because this playbook is built directly around Nsight Compute capture behavior.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because kernel profiling is useful only when tied back to sound GPU measurement discipline.
- [Cargo benchmark command docs](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are relevant because one profiling mode targets built bench executables.
- [Cargo build command docs](https://doc.rust-lang.org/cargo/commands/cargo-build.html) are relevant because the runner mode depends on feature-gated release builds of a dedicated profiling binary.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep Nsight Compute profiling documented as a repeatable diagnostic path, not as perf-gate authority.
2. Require every capture to record target mode, workload, filter, environment flags, and output provenance.
3. Treat runner and bench captures as different investigative tools rather than interchangeable numbers.
4. Use Nsight Compute when the GPU gate or route matrix points to a kernel-level question, not as the first-line tool for every change.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document both runner and bench capture modes clearly.
- [x] Document the capture parameters that materially change report meaning.
- [x] Keep provenance JSON generation mandatory.
- [x] Keep Nsight Compute playbooks non-options only.
- [x] Tie capture review back to a concrete regression or investigation question.
- [x] Avoid quoting kernel-profile outputs as standalone performance claims.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the wrapper in the intended mode and confirm a `.ncu-rep` and provenance JSON are produced
- inspect provenance before trusting the report interpretation
- confirm the kernel filter actually matches the target kernel when filtering is enabled
- cross-check the profiled mode against the route or gate failure that motivated the run
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current wrapper and improve documentation around when and how to use it
- prefer better provenance and narrower capture questions over adding more flags casually
- if local Nsight path drift becomes common, improve wrapper discovery before weakening the playbook
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- This item closed on a real runner-mode Nsight Compute capture rather than a source change.
- Fresh acceptance evidence for this closure:
  - `powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_ncu.ps1 -Target runner -RunnerWorkload small -RunnerStrategy breakout_momentum -RunnerIterations 1 -RunnerWarmupIterations 0 -LaunchCount 1 -OutputPrefix target/nsight/doc33_runner_small`
- The wrapper produced both required artifacts:
  - report: `target/nsight/doc33_runner_small.ncu-rep`
  - provenance: `target/nsight/doc33_runner_small.provenance.json`
- The verified provenance captured the playbook details that materially change report meaning:
  - target mode: `runner`
  - build profile: `release`
  - profiler executable: `C:\Program Files\NVIDIA Corporation\Nsight Compute 2026.1.0\ncu.bat`
  - strategy: `breakout_momentum`
  - workload: `small`
  - `launch-count`: `1`
  - `dynamic-mode`: `off`
  - `cuda-graphs`: `off`
  - `heatmap_bins`: `64`
  - `device_id`: `0`
- The capture question was intentionally narrow: verify that the wrapper can build the runner target, invoke Nsight Compute in a repeatable mode, and emit provenance beside the `.ncu-rep`. This item remains a diagnostic playbook, not a perf-gate authority surface.
- Bench mode remains documented in the playbook and wrapper, but a second bench-mode capture was not required for this closure because the runner-mode execution already verified the wrapper’s artifact/provenance contract on the current machine.
- No `README.md` or `REPO_MAP.md` update was required because this item verified the existing diagnostic workflow rather than changing any public benchmark or routing contract.

## Related Docs

- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/34_nsys_profile_playbook_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/35_cupti_sass_probe_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/38_profile_runner_bins_2026-04-09.md`
