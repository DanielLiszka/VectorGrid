# Workflow Benchmark Pack Plan

Last updated: 2026-04-10

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
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema getting started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the workload pack is a versioned contract and deserves schema-style discipline.
- [Cargo run command docs](https://doc.rust-lang.org/cargo/commands/cargo-run.html) are relevant because the pack is consumed through a reproducible bin invocation path.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because the pack ultimately configures a deterministic time-series workflow evaluation surface.
- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant by contrast: this pack is not a micro-benchmark config and must not be treated like one.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Treat the workflow benchmark pack as a versioned performance-contract input, not as casual fixture data.
2. Keep the pack intentionally small and interpretable unless a stronger workload taxonomy is justified.
3. Tie every pack change to explicit workflow-baseline review and documentation.
4. Consider future schema validation only if pack complexity grows enough to warrant it.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the meaning of each current pack field.
- [x] Document how pack changes affect the workflow baseline.
- [x] Keep the pack non-options only.
- [x] Review pack representativeness periodically without changing it casually.
- [x] Avoid multiplying workload knobs without a concrete regression-detection benefit.
- [x] Cross-link pack stewardship with workflow-gate stewardship.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- review the JSON diff for every pack change
- rebuild the workflow baseline intentionally after accepted pack changes
- re-run candidate mode to make sure the wrapper and bin still agree on pack semantics
- confirm downstream docs and scripts still describe the same workload
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current simple pack until there is evidence that it misses important regressions
- prefer explicit versioning over silent field reinterpretation
- if multiple packs are ever introduced, keep one canonical authority pack for the workflow gate
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- This item closed on verification of the existing checked-in pack and its consumer path rather than a code change.
- The current pack remains intentionally small and explicit:
  - `version`: `1`
  - `name`: `regpack-v1`
  - `description`: deterministic synthetic workflow pack for optimize -> validate -> meta -> holdout regression gates
  - `workflow_bin`: `ta_workflow_regression`
  - `seed`: `1`
  - `candles_len`: `10000`
  - `train_assets`: `10`
  - `holdout_assets`: `6`
- Pack stewardship and workflow-gate stewardship remain directly linked:
  - `scripts/run_workflow_gate.ps1` reads the pack and passes its values into `ta_workflow_regression`
  - any material change to the pack semantics invalidates `scripts/baselines/workflow_regression_baseline.json`
  - the CPU perf gate depends on the workflow gate and therefore indirectly depends on this same pack contract
- Fresh acceptance evidence for this closure:
  - `powershell -ExecutionPolicy Bypass -File scripts/run_workflow_gate.ps1 -Mode candidate`
- The fresh candidate workflow run proved the wrapper and bin still agree on the current pack semantics:
  - `target/workflow_regression_summary.json` was rewritten successfully
  - candidate runtime and stage totals stayed within the current baseline budgets
  - no pack change or baseline refresh was required
- No `README.md` or `REPO_MAP.md` update was required because the pack file, its field meanings, and its downstream workflow-gate relationship were confirmed rather than changed.

## Related Docs

- `REPO_Improvement/10_tests_perf/28_workflow_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/39_post_release_telemetry_review_2026-04-09.md`
