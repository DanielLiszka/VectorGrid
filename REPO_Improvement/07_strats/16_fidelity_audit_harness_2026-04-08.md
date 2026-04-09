# Fidelity Audit Harness Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the real-data fidelity audit harness so exact-vs-cheaper optimization and validation comparisons remain bounded, reproducible, and useful for product decisions without destabilizing the main app.

## Scope Boundaries

In scope:

- `real_data_fidelity_audit` binary
- exact vs `GpuVdps` optimization comparisons
- baseline vs cheaper validation preset comparisons
- artifact schema and interpretability
- bounded dataset/timeframe/candidate-pool control

Out of scope:

- integrating the harness into the main UI
- changing optimization/validation algorithms themselves
- broad research claims outside the harness evidence

## Current User-Facing Behavior

There is no default UI surface. The harness is an opt-in binary that loads real CSV data, optionally resamples, runs exact grid and approximate optimization, builds a bounded validation candidate pool, compares baseline vs cheaper validation presets, evaluates holdout winners, and writes a JSON artifact.

## Current Code Surface

- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/strategy_presets.rs`

## Current Verification Surface

- `cargo test -p ta_app_core --bin real_data_fidelity_audit`
- helper tests embedded in `real_data_fidelity_audit.rs`
- existing Double-MA optimization and validation tests

## Strengths Already Present

- the harness is isolated in its own binary, reducing regression risk to the shipped app
- it already compares both optimization fidelity and validation fidelity
- candidate-pool construction is bounded rather than open-ended
- the artifact includes runtime, overlap, regret, verdict, and holdout-evaluation data

## Weaknesses / Failure Modes

- artifact interpretation thresholds are not yet documented
- live audit execution still depends on user-provided datasets and bounded scenario choices
- the harness currently focuses on real-value comparisons but does not yet define a "pass/fail" product gate
- without surrounding docs, the artifact can be informative but hard to operationalize

## Regressions To Avoid

- letting the harness silently diverge from authoritative exact or validation code paths
- expanding the harness until it becomes a shadow application
- weakening bounded candidate-pool logic and turning runs into uncontrolled sweeps
- treating one good artifact as universal proof
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [scikit-learn: TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because the harness exists to judge time-ordered fidelity, not random-split similarity.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because the harness compares cheaper GPU-oriented execution against exact references and should preserve correctness-first benchmarking discipline.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the harness artifact should stay interpretable and versioned.
- [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
  - relevant because strategy-local behavior is easy to regress silently unless tests stay close to the owning strategy/runtime surface

## Proposed Improvement Approach

1. Treat the harness as a product-evidence tool, not a replacement execution path.
2. Document the artifact schema and the meaning of each comparison metric.
3. Define explicit review thresholds for overlap, regret, and approval/recommendation agreement.
4. Keep the harness bounded:
   - limited datasets
   - limited timeframes
   - limited candidate pool
5. Preserve isolation from the main application runtime.
6. Treat strategy-surface code quality as part of the implementation scope: keep strategy-local ownership, test proximity, truth anchors, diagnostics, and route semantics explicit instead of hiding them in generic layers.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the CLI contract and artifact schema for the harness.
- [ ] Define baseline and cheaper validation presets as named, stable profiles.
- [ ] Define decision thresholds for rank overlap, regret, and verdict agreement.
- [ ] Add representative bounded dataset/timeframe recipes for repeatable runs.
- [ ] Ensure the harness keeps calling authoritative optimization/validation code paths.
- [ ] Add a review guide for interpreting holdout-winner comparisons.
- [ ] Keep the harness opt-in and separate from normal app startup/runtime.
- [ ] Avoid turning the harness into an unbounded benchmark runner.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name the truth anchor, representative fixture or dataset, and any exact-vs-approximate or perf gate evidence required to close this strategy/runtime item.

## Verification Plan

- run `cargo test -p ta_app_core --bin real_data_fidelity_audit`
- run one bounded live harness execution on a known dataset/timeframe pair when changing harness semantics
- inspect the emitted artifact for schema stability and metric completeness
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant Rust quality gates for touched strategy/runtime surfaces: `cargo check --workspace`, the closest crate or strategy tests, and the applicable workflow, CPU, or GPU perf gate when behavior or performance claims change

## Rollout / Fallback / Compatibility Notes

- keep the harness as a separate binary with no default UI dependency
- preserve current artifact fields unless a schema-version change is recorded
- if thresholds are introduced, ship them as review guidance before making them enforcement gates
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/07_strats/01_strat_double_ma_family_2026-04-08.md`
- `REPO_Improvement/05_validation/25_validation_context_compat_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/23_tauri_report_command_api_2026-04-08.md`
