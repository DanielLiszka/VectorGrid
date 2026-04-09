# GPU Perf Gate Plan

Last updated: 2026-04-09

## Purpose

Define the authoritative non-options GPU perf gate and the strict rules that keep it comparable across driver, device, and workload changes.

## Scope Boundaries

In scope:

- the PowerShell GPU perf gate wrapper in `scripts/run_gpu_perf_gate.ps1`
- the default and custom Criterion-backed GPU cases used by that gate
- the dynamic-strategy runner ratio and GPU identity metadata collected by the gate
- baseline/schema compatibility for `scripts/baselines/gpu_perf_baseline.json`

Out of scope:

- options-related GPU paths
- Nsight or CUPTI profiling playbooks, which are supplemental diagnostics rather than gate authority
- frontend or desktop performance
- broad marketing claims that exceed the gate’s actual timing boundaries

## Current User-Facing Behavior

The current GPU perf gate already behaves like an authority wrapper. It waits for other benchmark/profiler processes to drain, resolves GPU identity with `nvidia-smi`, runs Criterion-backed GPU cases or a custom case, optionally checks the dynamic-strategy runner ratio, and compares the resulting summary against a versioned baseline. This is valuable because it keeps the GPU gate centered on a small known set of comparable surfaces, but it also makes the doc important: careless changes to device assumptions, driver environments, or case identity can invalidate the whole baseline.

## Current Code Surface

- `scripts/run_gpu_perf_gate.ps1` orchestrates GPU case execution, process-drain checks, baseline comparison, and summary generation.
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs` provides the dynamic-strategy runner surface referenced by the gate.
- `scripts/baselines/gpu_perf_baseline.json` stores the current authoritative GPU baseline.
- `crates/app_core/Cargo.toml` and the GPU-enabled bench executables define the Criterion side of the gate.

## Current Verification Surface

- `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_perf_gate.ps1 -Mode baseline` when intentionally refreshing the GPU baseline
- generated summary JSON under `target/gpu_perf_summary.json`
- candidate case-specific Criterion estimates under `target/criterion/`

## Strengths Already Present

- the script already serializes benchmark execution and explicitly waits for competing perf/profiler processes to drain
- it records GPU identity metadata, which is crucial for interpreting regressions sanely
- it supports both a default authoritative case set and an explicitly labeled custom-case mode
- compatibility assertions already protect timing boundary, build-profile, and case identity assumptions

## Weaknesses / Failure Modes

- GPU baselines are especially sensitive to device and driver changes
- Criterion output layout and bench-executable discovery under `target/release/deps` can be brittle
- the gate depends on local CUDA/NVIDIA tooling such as `nvidia-smi` being available and representative
- the dynamic-strategy ratio can fail for reasons that are related but not identical to the main bench cases

## Regressions To Avoid

- refreshing the GPU baseline casually after a hardware or driver change without recording why
- mixing profiler, runner, and gate boundaries into one implied “GPU speed” claim
- running other profiler or benchmark processes concurrently and contaminating the gate
- making the custom-case path look like the same authority as the default baseline set
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant because the GPU gate still relies on Criterion estimates for comparable benchmark cases.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because GPU timing interpretation depends on disciplined measurement and configuration control.
- [Nsight Compute docs](https://docs.nvidia.com/nsight-compute/) are relevant because the GPU gate should be clearly separated from deeper kernel profiling workflows.
- [Cargo benchmark command docs](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are relevant because the gate shells out to benchmark executables rather than running bespoke timing loops only.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep the GPU gate authoritative only for a small, clearly versioned set of cases and ratios.
2. Preserve explicit environment and case-identity metadata so baseline comparisons remain interpretable.
3. Keep process serialization and drain checks mandatory before gate execution.
4. Treat hardware, driver, or benchmark-identity changes as contract changes that require deliberate baseline review.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the authoritative default GPU cases and any allowed custom-case use.
- [ ] Document the GPU identity assumptions carried by the baseline.
- [ ] Keep process-drain serialization behavior mandatory.
- [ ] Keep schema-version and case-registry compatibility checks explicit.
- [ ] Keep the GPU gate non-options only.
- [ ] Require explanation before any GPU baseline refresh.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the GPU gate in candidate mode after relevant CUDA/runtime changes
- confirm no other benchmark/profiler processes are active before trusting the results
- inspect the summary JSON for GPU identity and case-compatibility details
- separate main-case failures from dynamic-strategy ratio failures during triage
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current narrow authoritative gate and improve documentation around it
- use Nsight or CUPTI only as follow-up diagnostics when the gate points to trouble
- if a case becomes obsolete, replace it with a documented baseline migration rather than silent drift
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/26_cpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/33_ncu_profile_playbook_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/34_nsys_profile_playbook_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/35_cupti_sass_probe_2026-04-09.md`
