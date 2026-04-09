# Nsight Systems Playbook Plan

Last updated: 2026-04-09

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

## External Research Summary

- [Nsight Systems docs](https://docs.nvidia.com/nsight-systems/) are relevant because this playbook is directly about whole-process GPU timeline capture.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because timeline captures are useful only in the context of disciplined GPU performance analysis.
- [Cargo benchmark command docs](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are relevant because the wrapper can target bench executables as well as runner binaries.
- [Cargo build command docs](https://doc.rust-lang.org/cargo/commands/cargo-build.html) are relevant because the runner path depends on explicit feature-gated build steps.

## Proposed Improvement Approach

1. Keep Nsight Systems profiling documented as a whole-process diagnostic tool rather than a benchmark authority.
2. Require every capture to record whether bounded NVTX capture actually succeeded or fell back.
3. Keep privilege and local-install caveats explicit in the playbook.
4. Use Nsight Systems when a regression needs timeline-level attribution, not merely because a GPU path exists.

## Checklist

- [ ] Document runner versus bench target modes clearly.
- [ ] Document NVTX-bounded capture, fallback behavior, and provenance expectations.
- [ ] Document the known Windows/elevation caveats for local Nsight Systems use.
- [ ] Keep the playbook non-options only.
- [ ] Tie each capture to a concrete investigation question.
- [ ] Avoid turning Nsight Systems runs into casual routine steps for unrelated changes.

## Verification Plan

- run the wrapper in the intended mode and confirm that a report and provenance file are generated
- inspect provenance for capture mode, target mode, and environment flags
- note explicitly whether bounded capture was honored or fallback was used
- reconcile the capture with the motivating gate failure or regression question before drawing conclusions

## Rollout / Fallback / Compatibility Notes

- preserve the current wrapper and improve usage discipline around it
- prefer explicit capture rationale and provenance over more capture permutations
- if local environment issues dominate, document them instead of loosening interpretive standards

## Related Docs

- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/33_ncu_profile_playbook_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/35_cupti_sass_probe_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/38_profile_runner_bins_2026-04-09.md`
