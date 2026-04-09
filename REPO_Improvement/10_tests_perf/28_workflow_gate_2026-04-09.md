# Workflow Gate Plan

Last updated: 2026-04-09

## Purpose

Define the deterministic synthetic workflow regression gate that protects optimize, validate, meta-select, and holdout timing and summary behavior.

## Scope Boundaries

In scope:

- the workflow gate wrapper `scripts/run_workflow_gate.ps1`
- the deterministic workflow bin `src-tauri/src/bin/ta_workflow_regression.rs`
- the synthetic workload pack in `scripts/workflow_benchmark_pack.json`
- baseline comparison against `scripts/baselines/workflow_regression_baseline.json`

Out of scope:

- options-related workflow bins
- UI or Tauri hydration latency
- real-market-data product latency
- CPU or GPU authority gates that layer on top of workflow output

## Current User-Facing Behavior

The workflow gate already runs a deterministic synthetic optimize -> validate -> meta-select -> holdout chain and writes a structured summary JSON. In candidate mode it also compares the result to a stored baseline. The bin itself is careful about timing-boundary notes and explicitly excludes UI and shell hydration surfaces from the workflow number. That makes it useful as an execution-regression sentinel, but only if the doc keeps its synthetic and exclusion-heavy nature explicit.

## Current Code Surface

- `scripts/run_workflow_gate.ps1` wraps the workflow regression bin and handles baseline/candidate modes.
- `src-tauri/src/bin/ta_workflow_regression.rs` implements the deterministic synthetic workflow and emits stage summaries plus exclusions.
- `scripts/workflow_benchmark_pack.json` defines the current synthetic workload contract consumed by the wrapper.
- `scripts/baselines/workflow_regression_baseline.json` stores the current baseline summary.

## Current Verification Surface

- `powershell -ExecutionPolicy Bypass -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `powershell -ExecutionPolicy Bypass -File scripts/run_workflow_gate.ps1 -Mode baseline`
- generated summary JSON under `target/workflow_regression_summary.json`
- inline tests in `ta_workflow_regression.rs` for legacy summary and stage-budget handling

## Strengths Already Present

- the workflow bin already emits timing-boundary notes and explicit exclusions, which is unusually disciplined
- the gate covers multiple pipeline stages instead of a single micro-benchmark
- the synthetic workload pack is versioned and checked in
- the gate summary is machine-readable and suitable for downstream gate wrappers

## Weaknesses / Failure Modes

- the workload is synthetic by design and therefore not a substitute for product or dataset-realistic runtime checks
- stage budgets are intentionally loose enough that moderate regressions may not trip them immediately
- the gate can be misread as an end-user latency number if the exclusions are ignored
- changing the workload pack or bin semantics without a disciplined baseline refresh would quietly invalidate comparisons

## Regressions To Avoid

- claiming the workflow runtime as full-app or UI-inclusive latency
- changing the workload pack silently without updating the baseline and the documentation
- dropping the stage-boundary and exclusion notes from the emitted summary
- using the synthetic workflow gate as the sole authority for CPU or GPU claims

## External Research Summary

- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because deterministic time-series evaluation needs explicit split semantics rather than generic random sampling assumptions.
- [Cargo profiles reference](https://doc.rust-lang.org/cargo/reference/profiles.html) is relevant because the workflow gate explicitly distinguishes `dev` and `release` build profiles.
- [JSON Schema getting started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the workflow summary behaves like a versioned contract and should be treated with schema discipline even if no separate JSON Schema file exists yet.
- [Cargo run command docs](https://doc.rust-lang.org/cargo/commands/cargo-run.html) are relevant because the wrapper’s authority depends on a reproducible build-and-run boundary for the bin.

## Proposed Improvement Approach

1. Keep the workflow gate explicitly synthetic and explicitly bounded by its emitted exclusion notes.
2. Preserve versioned workload-pack and summary-schema assumptions as part of the gate contract.
3. Use the workflow gate as a shared foundation for other authority gates, not as a catch-all performance claim.
4. Treat workload-pack changes as contract changes that require deliberate baseline and documentation updates.

## Checklist

- [ ] Document the workflow gate timing boundary and exclusions clearly.
- [ ] Document the synthetic workload-pack contents and why they exist.
- [ ] Keep baseline refresh tied to explicit pack or contract changes.
- [ ] Keep the summary schema and stage breakdown stable or deliberately versioned.
- [ ] Keep the workflow gate non-options only.
- [ ] Cross-link workflow-gate results into CPU and post-release review docs without conflating boundaries.

## Verification Plan

- run the workflow gate in candidate mode after pipeline-runtime changes
- inspect the emitted summary JSON to ensure exclusions and stage fields remain present
- re-run in baseline mode only when intentional contract changes are accepted
- compare workflow failures against downstream CPU or post-release scripts before changing budgets

## Rollout / Fallback / Compatibility Notes

- preserve the current synthetic gate while tightening documentation around what it does not measure
- prefer additive schema discipline over ad hoc threshold tweaks
- if new workflow stages are added, version the contract rather than quietly expanding the payload

## Related Docs

- `REPO_Improvement/10_tests_perf/26_cpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/30_workflow_benchmark_pack_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/39_post_release_telemetry_review_2026-04-09.md`
