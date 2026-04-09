# GPU Route Matrix Profile Plan

Last updated: 2026-04-09

## Purpose

Define the route-matrix runtime audit that checks whether shipped GPU strategy-route combinations actually take the expected backend route.

## Scope Boundaries

In scope:

- the route-matrix bin `src-tauri/src/bin/ta_profile_route_matrix.rs`
- route correctness across `CompositeTrend`, `BreakoutMomentum`, and `RegimeHybrid` cases
- strict versus non-strict route checking and measured/warmup run handling
- per-case runtime and route metadata emitted by the bin

Out of scope:

- options-related strategies
- frontend behavior
- authoritative GPU perf gating
- kernel-level profiling details

## Current User-Facing Behavior

The route-matrix bin already enumerates profile-family cases across requested backends, runs them with warmup and measured passes, records backend used, backend route, fallback reason, and per-run timing, and can fail in strict mode when the realized route is not what the case expected. This makes it a valuable route-audit surface that sits between correctness and performance, but it needs documentation so users do not misread it as a generic benchmark or ignore its route-drift detection value.

## Current Code Surface

- `src-tauri/src/bin/ta_profile_route_matrix.rs` implements the route-matrix audit and emits per-case artifacts.
- `crates/app_core/src/composite_profile_route.rs` and related route logic determine the backend route expectations being audited.
- `crates/app_core/src/strategy_presets.rs` provides strategy-specific presets and routing assumptions for these profiles.
- output JSON under `target/profile_route_matrix*.json` stores the current artifact surface.

## Current Verification Surface

- `cargo run -p ta_desktop_demo_app --release --bin ta_profile_route_matrix --features cuda,cuda-backtest-kernel -- --csv data/benchmarks/AAPL.csv --strict --warmup-runs 1 --measured-runs 3`
- inspection of the emitted route-matrix JSON artifact
- inline tests in `ta_profile_route_matrix.rs` for route checking and canonical-run selection

## Strengths Already Present

- the bin already separates route correctness from ordinary perf-gate concerns
- strict mode makes backend-route drift visible instead of silently tolerated
- measured-run summaries plus canonical-run selection provide more context than a single timing sample
- the audited cases line up with current profile-family strategy-policy decisions in the repo

## Weaknesses / Failure Modes

- the route matrix depends on local GPU capability and dataset availability
- route-correctness failures can be environmental, expected fallbacks, or genuine regressions, which need interpretation
- it is easy to mistake the emitted runtimes for benchmark authority instead of route-audit context
- case coverage is intentional but not exhaustive for the entire app

## Regressions To Avoid

- dropping strict route checks and turning the tool into another generic runtime sampler
- treating route-matrix wall-clock numbers as headline performance claims
- changing route expectations without updating the audited cases and their rationale
- ignoring fallback reasons and only looking at timing values
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because route choice and backend fallback affect how GPU results should be interpreted.
- [Cargo run command docs](https://doc.rust-lang.org/cargo/commands/cargo-run.html) are relevant because the route matrix is a headless bin-level audit, not a benchmark harness hidden inside tests.
- [Tauri state management guidance](https://v2.tauri.app/develop/state-management/) is relevant by contrast because this route audit intentionally bypasses the shell and focuses on backend/runtime behavior.
- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant as a contrast: the route matrix records runtime context but is not a Criterion-style benchmark authority.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep the route matrix documented as a route-correctness and fallback audit first, runtime sampler second.
2. Preserve strict mode as the preferred regression-detection setting for supported environments.
3. Treat fallback reasons and route metadata as first-class outputs during review.
4. Use route-matrix findings to guide deeper perf or correctness investigation rather than collapsing it into a single speed number.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current audited cases and their expected backend routes.
- [ ] Document strict versus non-strict usage clearly.
- [ ] Keep route metadata and fallback reasons central to artifact review.
- [ ] Keep the route matrix non-options only.
- [ ] Cross-check route-audit outputs against current profile-route policy docs.
- [ ] Avoid marketing or release claims based on route-matrix timings alone.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the route matrix in strict mode after route-policy or backend changes
- inspect the emitted JSON for route correctness and fallback reasons
- compare route-matrix failures against route-policy code before changing expectations
- use timing values as supporting context only after route correctness is understood
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current route-audit emphasis and improve its documentation
- add new audited cases only when route policy expands or shifts materially
- keep route-matrix outputs supplemental to the main perf gates
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/03_opt/19_eng_dm_backend_routing_2026-04-08.md`
- `REPO_Improvement/07_strats/25_strategy_runtime_gpu_stack_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/38_profile_runner_bins_2026-04-09.md`
