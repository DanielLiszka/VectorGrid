# Workflow Benchmark Pack Plan

Last updated: 2026-04-09

## Purpose

Define the stewardship rules for the checked-in synthetic workload pack that drives the workflow regression gate.

## Scope Boundaries

In scope:

- the JSON pack `scripts/workflow_benchmark_pack.json`
- pack-version and workload-size meaning for the workflow gate
- relationships between pack changes and workflow baseline refresh
- synthetic workload representativeness for regression detection

Out of scope:

- options-related workload packs
- real-dataset benchmark scenarios
- general benchmarking strategy outside the workflow gate
- frontend or desktop performance

## Current User-Facing Behavior

The repo already keeps a tiny checked-in JSON pack that names the workflow-regression bin and the synthetic workload dimensions: seed, candles length, train assets, and holdout assets. `run_workflow_gate.ps1` reads this pack and passes the values through to the bin. That is good because the gate’s synthetic workload is explicit and diffable, but it also means pack edits are equivalent to benchmark-contract edits and need stronger process than ordinary fixture churn.

## Current Code Surface

- `scripts/workflow_benchmark_pack.json` defines the current synthetic workflow workload.
- `scripts/run_workflow_gate.ps1` consumes the pack and passes it into the workflow regression bin.
- `src-tauri/src/bin/ta_workflow_regression.rs` is the runtime consumer of the pack values.
- `scripts/baselines/workflow_regression_baseline.json` becomes invalid or stale when the pack meaning changes materially.

## Current Verification Surface

- `powershell -ExecutionPolicy Bypass -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `powershell -ExecutionPolicy Bypass -File scripts/run_workflow_gate.ps1 -Mode baseline` after intentional pack changes
- diff review of the pack JSON itself

## Strengths Already Present

- the workload contract is explicit and checked in
- the pack is small enough to review carefully
- the wrapper already consumes it centrally rather than duplicating parameters in multiple places
- the pack provides a stable anchor for downstream CPU and post-release review scripts

## Weaknesses / Failure Modes

- the pack is intentionally small and may underrepresent larger-scale regressions
- there is no separate schema file describing allowed pack changes
- pack edits are easy to underestimate even though they invalidate comparisons
- the pack currently captures workload dimensions but not a richer taxonomy of scenario classes

## Regressions To Avoid

- changing pack size or semantics without resetting the workflow baseline deliberately
- treating the pack as if it were just sample data rather than a benchmark contract
- adding many knobs to the pack before there is a clear governance reason
- letting the pack drift away from what downstream scripts think they are measuring

## External Research Summary

- [JSON Schema getting started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the workload pack is a versioned contract and deserves schema-style discipline.
- [Cargo run command docs](https://doc.rust-lang.org/cargo/commands/cargo-run.html) are relevant because the pack is consumed through a reproducible bin invocation path.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because the pack ultimately configures a deterministic time-series workflow evaluation surface.
- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant by contrast: this pack is not a micro-benchmark config and must not be treated like one.

## Proposed Improvement Approach

1. Treat the workflow benchmark pack as a versioned performance-contract input, not as casual fixture data.
2. Keep the pack intentionally small and interpretable unless a stronger workload taxonomy is justified.
3. Tie every pack change to explicit workflow-baseline review and documentation.
4. Consider future schema validation only if pack complexity grows enough to warrant it.

## Checklist

- [ ] Document the meaning of each current pack field.
- [ ] Document how pack changes affect the workflow baseline.
- [ ] Keep the pack non-options only.
- [ ] Review pack representativeness periodically without changing it casually.
- [ ] Avoid multiplying workload knobs without a concrete regression-detection benefit.
- [ ] Cross-link pack stewardship with workflow-gate stewardship.

## Verification Plan

- review the JSON diff for every pack change
- rebuild the workflow baseline intentionally after accepted pack changes
- re-run candidate mode to make sure the wrapper and bin still agree on pack semantics
- confirm downstream docs and scripts still describe the same workload

## Rollout / Fallback / Compatibility Notes

- preserve the current simple pack until there is evidence that it misses important regressions
- prefer explicit versioning over silent field reinterpretation
- if multiple packs are ever introduced, keep one canonical authority pack for the workflow gate

## Related Docs

- `REPO_Improvement/10_tests_perf/28_workflow_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/39_post_release_telemetry_review_2026-04-09.md`
