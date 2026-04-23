# GPU ALMA Contract Gate Plan

Last updated: 2026-04-10

## Purpose

Define the specialized ALMA contract-row GPU perf lane and keep it subordinate to the main GPU perf gate.

## Scope Boundaries

In scope:

- the specialized wrapper `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`
- checked-in baselines under `scripts/baselines/gpu_alma_contract_rows/`
- ALMA-contract-row specific GPU timing checks and summary emission
- the relation between this lane and the main `run_gpu_perf_gate.ps1` authority script

Out of scope:

- the main GPU perf gate baseline
- options-related benchmarks
- general CUDA profiling
- front-end or desktop performance

## Current User-Facing Behavior

The repo already has a specialized GPU gate for ALMA contract-row behavior. It exists because that workload is important enough to watch but narrow enough that it should not silently expand the main GPU baseline. That split is correct, but it needs a dedicated doc so contributors do not read this lane as “the GPU gate” in general.

## Current Code Surface

- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1` is the user-facing wrapper for the ALMA lane.
- `scripts/run_gpu_perf_gate.ps1` remains the main GPU authority gate and provides adjacent context.
- `scripts/baselines/gpu_alma_contract_rows/` stores the checked-in ALMA-specific baseline data.
- the relevant GPU bench or runner outputs live under `target/` during candidate runs.

## Current Verification Surface

- `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`
- `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode baseline` when intentionally refreshing the ALMA lane
- review of the emitted summary JSON and ALMA-specific baseline artifacts

## Strengths Already Present

- the repo already isolates this narrow workload instead of hiding it inside the main GPU gate
- it makes specialized regression detection possible without polluting the broader gate surface
- the baseline folder is separate and therefore easier to interpret
- the lane can serve as a focused follow-up when ALMA-related GPU behavior shifts

## Weaknesses / Failure Modes

- specialized lanes are easy to overvalue if not clearly documented
- the lane still depends on local GPU/driver comparability
- it may share some infrastructure assumptions with the main GPU gate but not the same representative workload
- contributors may not realize when the ALMA lane should or should not be refreshed

## Regressions To Avoid

- treating the ALMA contract-row lane as equivalent to the main GPU perf gate
- refreshing ALMA baselines casually after unrelated GPU changes
- using the specialized lane to make broad GPU performance claims
- allowing its case identity or output meaning to drift without documentation
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because specialized GPU workloads still need disciplined boundary control.
- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant because narrow benchmark lanes should remain explicit about what they do and do not represent.
- [Cargo benchmark command docs](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are relevant because this lane relies on a benchmark-style timing surface under the hood.
- [Nsight Compute docs](https://docs.nvidia.com/nsight-compute/) are relevant as a follow-up path if the ALMA lane regresses and deeper kernel analysis is needed.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep the ALMA lane explicitly documented as a specialized supplemental authority lane, not the default GPU authority.
2. Tie its baseline refresh process to ALMA-specific workload or implementation changes.
3. Cross-reference the main GPU gate so reviewers understand which lane is broader and which is narrower.
4. Escalate unexpected ALMA regressions into targeted GPU profiling rather than broad claim changes first.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the exact reason this ALMA lane exists separately from the main GPU gate.
- [x] Document its baseline location and refresh policy.
- [x] Keep the lane non-options only.
- [x] Avoid using ALMA-only results for repo-wide GPU claims.
- [x] Cross-link the lane to the main GPU gate and profiling playbooks.
- [x] Record the hardware/driver context before trusting a refreshed baseline.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the ALMA lane in candidate mode after ALMA-related GPU changes
- compare the emitted summary against the specialized baseline
- inspect main GPU gate status before assuming the ALMA regression is broad
- use Nsight or CUPTI follow-up only if the narrow lane points to a real issue
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the separate lane instead of folding it into the main GPU gate casually
- prefer clearer provenance over more thresholds
- retire the lane only if the underlying workload becomes obsolete, not because it is narrow
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- This item closed on a small reliability hardening pass plus fresh candidate verification of the specialized ALMA lane.
- The implementation changes stayed inside the benchmarking harness surfaces that own this lane:
  - `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`
  - `scripts/run_gpu_perf_gate.ps1`
- The hardening addressed a real evidence problem for this specialized lane:
  - the main GPU gate's custom-case exact-path flow required a pre-existing `Criterion` estimates file, which made the ALMA wrapper depend on stale case-local artifacts
  - the ALMA wrapper now clears the case-local `target/criterion/...` directory before each focused case run so the recorded `new/estimates.json` belongs to the current measurement
  - the ALMA aggregate summary now records authority scope, baseline path, candidate-vs-baseline means, allowed budget, pass/fail, and GPU identity instead of only the raw candidate means
- Fresh acceptance evidence for this closure:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`
- Fresh verified artifacts:
  - aggregate summary: `target/gpu_alma_contract_rows_perf_summary.json`
  - per-case summaries:
    - `target/gpu_alma_contract_rows_perf/auto.json`
    - `target/gpu_alma_contract_rows_perf/full17.json`
    - `target/gpu_alma_contract_rows_perf/trade22.json`
- Hardware and driver context captured by the passing summaries:
  - GPU: `NVIDIA GeForce RTX 4090`
  - driver: `595.71`
- The passing candidate summary kept this lane explicitly subordinate to the main GPU gate and passed all three ALMA-specific cases against the checked-in baselines:
  - `auto`: `65.552367 ms` vs baseline `65.617356 ms`
  - `full17`: `67.481057 ms` vs baseline `70.551114 ms`
  - `trade22`: `68.405214 ms` vs baseline `67.362288 ms`
- No baseline refresh was performed. The checked-in baseline directory remains:
  - `scripts/baselines/gpu_alma_contract_rows/`
- No `README.md` or `REPO_MAP.md` update was required because both already described the ALMA lane as a specialized supplemental gate rather than the default GPU authority.

## Related Docs

- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/33_ncu_profile_playbook_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/35_cupti_sass_probe_2026-04-09.md`
