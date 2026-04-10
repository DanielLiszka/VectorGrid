# Nsight Systems Playbook Plan

Last updated: 2026-04-10

## Purpose

Define a repeatable, non-authoritative Nsight Systems capture workflow for whole-process CUDA timeline analysis.

## Scope Boundaries

In scope:

- the wrapper `scripts/profile_gpu_backtest_nsys.ps1`
- runner and bench target modes for whole-process timeline capture
- NVTX-bounded and fallback whole-process capture modes
- privilege, path, and environment issues that affect Nsight Systems on Windows

Out of scope:

- authoritative perf gating
- kernel-level metric investigation, which belongs to Nsight Compute or CUPTI
- options-related profiling
- general Windows performance tuning beyond the repo’s profiling needs

## Current User-Facing Behavior

The current Nsight Systems wrapper already does significant work: it locates `nsys.exe`, warns about known elevation-sensitive installs, supports runner and bench modes, toggles NVTX-related environment variables, handles bounded capture versus fallback, and writes provenance. That means the repo already has a usable timeline playbook, but it still needs a doc that explains when NVTX-bounded capture is the right tool, what counts as fallback, and why these captures remain diagnostic rather than gate authority.

## Current Code Surface

- `scripts/profile_gpu_backtest_nsys.ps1` orchestrates Nsight Systems capture, environment setup, and provenance output.
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs` provides the runner target surface.
- `crates/app_core/Cargo.toml` and the selected benchmark executable provide the bench target surface.
- generated `.nsys-rep` and provenance files under the chosen output prefix record the actual capture context.

## Current Verification Surface

- `powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target runner -NvtxMode on -CaptureMode nvtx`
- `powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target bench`
- review of generated `.nsys-rep` and provenance artifacts

## Strengths Already Present

- the wrapper already handles several painful local setup details that would otherwise be tribal knowledge
- it distinguishes NVTX-bounded capture from broader whole-process capture
- it restores environment variables after the run rather than leaving profiler state behind
- it records provenance instead of relying on remembered command lines

## Weaknesses / Failure Modes

- Nsight Systems on Windows can be sensitive to privilege level and installation state
- NVTX-bounded capture can produce empty results and fall back, which complicates interpretation
- timeline capture is heavier and broader than the question many regressions actually require
- runner and bench targets again have different boundaries that can be misread if the doc is weak

## Regressions To Avoid

- treating Nsight Systems timeline captures as primary gate authority
- ignoring fallback behavior and assuming every capture was NVTX-bounded as requested
- profiling in an unelevated or broken local environment without recording that limitation
- comparing runner and bench timeline captures as if they were the same measurement surface
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Nsight Systems docs](https://docs.nvidia.com/nsight-systems/) are relevant because this playbook is directly about whole-process GPU timeline capture.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because timeline captures are useful only in the context of disciplined GPU performance analysis.
- [Cargo benchmark command docs](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are relevant because the wrapper can target bench executables as well as runner binaries.
- [Cargo build command docs](https://doc.rust-lang.org/cargo/commands/cargo-build.html) are relevant because the runner path depends on explicit feature-gated build steps.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep Nsight Systems profiling documented as a whole-process diagnostic tool rather than a benchmark authority.
2. Require every capture to record whether bounded NVTX capture actually succeeded or fell back.
3. Keep privilege and local-install caveats explicit in the playbook.
4. Use Nsight Systems when a regression needs timeline-level attribution, not merely because a GPU path exists.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document runner versus bench target modes clearly.
- [x] Document NVTX-bounded capture, fallback behavior, and provenance expectations.
- [x] Document the known Windows/elevation caveats for local Nsight Systems use.
- [x] Keep the playbook non-options only.
- [x] Tie each capture to a concrete investigation question.
- [x] Avoid turning Nsight Systems runs into casual routine steps for unrelated changes.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the wrapper in the intended mode and confirm that a report and provenance file are generated
- inspect provenance for capture mode, target mode, and environment flags
- note explicitly whether bounded capture was honored or fallback was used
- reconcile the capture with the motivating gate failure or regression question before drawing conclusions
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current wrapper and improve usage discipline around it
- prefer explicit capture rationale and provenance over more capture permutations
- if local environment issues dominate, document them instead of loosening interpretive standards
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- This item closed on a real runner-mode Nsight Systems capture rather than a wrapper change.
- Fresh acceptance evidence for this closure:
  - `powershell -ExecutionPolicy Bypass -File scripts/profile_gpu_backtest_nsys.ps1 -Target runner -RunnerStrategy breakout_momentum -RunnerWorkload small -RunnerIterations 1 -RunnerWarmupIterations 0 -RunnerHeatmapBins 0 -NvtxMode on -CaptureMode nvtx -OutputPrefix target/nsight/doc34_runner_small_nsys`
- The wrapper produced both required artifacts:
  - report: `target/nsight/doc34_runner_small_nsys.nsys-rep`
  - provenance: `target/nsight/doc34_runner_small_nsys.provenance.json`
- The verified provenance captured the playbook details that materially change report meaning:
  - profiler executable: `C:\Program Files\NVIDIA Corporation\Nsight Systems 2023.3.3\target-windows-x64\nsys.exe`
  - target mode: `runner`
  - build profile: `release`
  - strategy: `breakout_momentum`
  - workload: `small`
  - `nvtx_mode`: `on`
  - `capture_mode_requested`: `nvtx`
  - `capture_mode_applied`: `nvtx`
  - `fallback_used`: `false`
  - `dynamic_strategy_mode`: `off`
  - `cuda_graphs_mode`: `off`
  - `heatmap_bins`: `0`
  - `device_id`: `0`
- The capture question was intentionally narrow: verify that the wrapper can build the runner target, honor bounded NVTX capture on the current machine, and emit provenance beside the `.nsys-rep`. This item remains a diagnostic playbook, not a perf-gate authority surface.
- Bench mode remains documented in the playbook and wrapper, but a second bench-mode capture was not required for this closure because the runner-mode execution already verified the current machine’s artifact/provenance contract and bounded-capture path.
- No `README.md` or `REPO_MAP.md` update was required because this item verified the existing diagnostic workflow rather than changing a public benchmark or routing contract.

## Related Docs

- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/33_ncu_profile_playbook_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/35_cupti_sass_probe_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/38_profile_runner_bins_2026-04-09.md`
