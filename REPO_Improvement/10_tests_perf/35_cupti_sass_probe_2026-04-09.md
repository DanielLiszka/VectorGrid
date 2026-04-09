# CUPTI SASS Probe Plan

Last updated: 2026-04-09

## Purpose

Define the low-level CUPTI SASS inventory and capture workflow as a specialized diagnostic surface rather than routine verification.

## Scope Boundaries

In scope:

- the wrapper `scripts/run_cupti_sass_probe.ps1`
- the CUDA/CUPTI probe binary `crates/app_core/src/bin/cupti_sass_probe.rs`
- inventory versus capture modes, workload, metric-contract, kernel-filter, and output-dir choices
- process-exclusivity rules for running the probe safely

Out of scope:

- authoritative performance gating
- options-related CUDA probes
- whole-process timeline analysis
- browser or desktop performance

## Current User-Facing Behavior

The repo already exposes a PowerShell wrapper around the CUPTI SASS probe binary. The wrapper refuses to start if other benchmark or profiler processes are active, constructs a cargo run invocation with CUDA features enabled, and writes the last command into the output directory. This is exactly the kind of surface that needs a narrow implementation doc, because it is both valuable and easy to misuse.

## Current Code Surface

- `scripts/run_cupti_sass_probe.ps1` is the user-facing entry point for probe runs.
- `crates/app_core/src/bin/cupti_sass_probe.rs` implements the underlying CUDA/CUPTI probe behavior.
- `target/cupti_sass_probe/` or the configured output directory stores probe artifacts.
- `scripts/profile_gpu_backtest_ncu.ps1` and `scripts/profile_gpu_backtest_nsys.ps1` are adjacent diagnostic surfaces that must stay distinct from this one.

## Current Verification Surface

- `powershell -ExecutionPolicy Bypass -File scripts/run_cupti_sass_probe.ps1 -Mode inventory`
- `powershell -ExecutionPolicy Bypass -File scripts/run_cupti_sass_probe.ps1 -Mode capture -Workload large -MetricContract full17`
- inspection of generated probe outputs and the logged last-command file

## Strengths Already Present

- the wrapper already serializes against other active profiler and benchmark processes
- inventory and capture modes are separated cleanly
- key capture parameters are explicit on the command line
- the output directory and command log improve reproducibility compared with ad hoc CUPTI usage

## Weaknesses / Failure Modes

- the tool requires a functioning CUDA/CUPTI environment and kernel-filter alignment
- no-record or unsupported-device outcomes are easy to misread as app regressions
- the probe is specialized enough that most contributors should not touch it casually
- there is no broader gate integration, which is correct but increases the risk of overinterpretation when it is run

## Regressions To Avoid

- running the probe concurrently with other profilers or benchmark processes
- treating a specialized SASS metric capture as a release gate
- changing capture parameters without recording them in provenance or notes
- using a failing or empty probe run as proof of application breakage without checking environment support first
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CUPTI docs](https://docs.nvidia.com/cupti/) are relevant because this workflow is specifically about CUPTI-backed metric collection.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because low-level GPU metrics need careful interpretation within a broader performance-analysis method.
- [Nsight Compute docs](https://docs.nvidia.com/nsight-compute/) are relevant because Nsight and CUPTI diagnostics overlap in user expectations but not in exact role.
- [Cargo run command docs](https://doc.rust-lang.org/cargo/commands/cargo-run.html) are relevant because the wrapper invokes a feature-gated probe binary through Cargo.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep the CUPTI SASS probe explicitly classified as a specialized low-level diagnostic tool.
2. Preserve process-exclusivity checks and explicit capture parameters as part of the contract.
3. Document environment prerequisites and common benign failure modes such as no-record captures.
4. Use CUPTI only when a kernel- or metric-level question justifies it.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document inventory and capture modes separately.
- [ ] Document the kernel-filter and metric-contract parameters that change probe meaning.
- [ ] Keep process-exclusivity checks mandatory.
- [ ] Keep the CUPTI probe non-options only.
- [ ] Record environment prerequisites and expected artifact locations.
- [ ] Avoid promoting probe outputs into release authority.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run inventory mode to confirm environment support before a deeper capture
- run capture mode only with a concrete kernel/metric question in mind
- inspect output artifacts and the logged last command before drawing conclusions
- verify no competing benchmark/profiler processes were active during the run
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current wrapper discipline and add documentation instead of making it easier-but-looser
- keep CUPTI invocation rare and explicit
- if repeated capture questions emerge, prefer playbook refinement over broadening the tool’s authority
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/33_ncu_profile_playbook_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/34_nsys_profile_playbook_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
