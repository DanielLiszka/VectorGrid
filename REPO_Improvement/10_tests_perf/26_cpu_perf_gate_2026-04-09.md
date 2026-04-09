# CPU Perf Gate Plan

Last updated: 2026-04-09

## Purpose

Define the authoritative non-options CPU perf gate that combines deterministic workflow wall-clock checks, Criterion CPU anchors, and the dynamic-strategy CPU ratio check.

## Scope Boundaries

In scope:

- the PowerShell CPU perf gate wrapper in `scripts/run_cpu_perf_gate.ps1`
- the workflow-regression gate it delegates to for end-to-end synthetic CPU runtime checking
- Criterion CPU anchor cases for `double_ma_opt` and the dynamic-strategy CPU ratio bench
- baseline/schema compatibility rules for `scripts/baselines/cpu_perf_baseline.json`

Out of scope:

- options-related benches or perf baselines
- GPU perf gating
- browser or desktop frontend performance
- broad product-latency claims outside the specific timing boundaries emitted by the gate

## Current User-Facing Behavior

The current CPU perf gate already acts as a structured authority script rather than a loose benchmark launcher. It first delegates to the workflow regression gate, then runs selected Criterion benches, reads hard-coded estimate paths from `target/criterion`, compares the resulting workflow and bench summaries against a versioned baseline, and emits a candidate summary JSON. This is a strong starting point because it distinguishes timing boundaries and schema compatibility explicitly, but the implementation plan needs to keep that authority narrow and honest.

## Current Code Surface

- `scripts/run_cpu_perf_gate.ps1` orchestrates workflow-gate execution, Criterion cases, baseline comparison, and summary emission.
- `scripts/run_workflow_gate.ps1` and `src-tauri/src/bin/ta_workflow_regression.rs` provide the workflow wall-clock portion of the gate.
- `crates/app_core/Cargo.toml` declares the Criterion bench targets the gate depends on.
- `scripts/baselines/cpu_perf_baseline.json` stores the current authoritative CPU perf baseline snapshot.

## Current Verification Surface

- `powershell -ExecutionPolicy Bypass -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`
- `powershell -ExecutionPolicy Bypass -File scripts/run_cpu_perf_gate.ps1 -Mode baseline` when intentionally refreshing the baseline
- generated summary JSON under `target/cpu_perf_summary.json`
- workflow summary JSON under `target/workflow_regression_summary.json`

## Strengths Already Present

- the gate already distinguishes workflow timing from Criterion timing rather than merging them into one opaque number
- baseline compatibility checks are explicit about schema, case registry, and bench identity
- the script already prints pass/fail lines and writes a machine-readable summary artifact
- the dynamic-strategy CPU ratio check is treated as a separate subordinate metric rather than hidden inside a generic benchmark result

## Weaknesses / Failure Modes

- the gate is brittle to hard-coded `target/criterion` path drift if benchmark naming or layout changes
- workflow-gate failure can cascade into CPU gate failure even when the underlying issue is not a CPU regression
- the chosen Criterion anchors are intentionally narrow and do not represent the full CPU surface
- baseline refresh is powerful and therefore risky if used casually

## Regressions To Avoid

- changing the timing boundary or benchmark set without updating the baseline schema and explanation
- treating the emitted numbers as universal product-performance claims
- loosening compatibility checks until baseline drift becomes silent
- adding many more Criterion cases without preserving a clear reason for each one
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant because the CPU gate relies on Criterion estimates as one of its authoritative timing boundaries.
- [Cargo benchmark command docs](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are relevant because the gate shells out to `cargo bench` and therefore inherits Cargo benchmark semantics.
- [Cargo profiles reference](https://doc.rust-lang.org/cargo/reference/profiles.html) is relevant because the workflow and bench portions of the gate depend on build-profile distinctions that must stay explicit.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because the delegated workflow gate is explicitly about deterministic time-series workflow evaluation, not generic random-split benchmarking.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep the CPU perf gate authoritative only for the specific workflow and Criterion boundaries it currently measures.
2. Preserve baseline/schema/case-registry compatibility checks as a first-class safeguard against silent drift.
3. Document the exact meaning of each emitted number so reviewers know what changed when the gate fails.
4. Treat benchmark-set changes and baseline refreshes as explicit contract changes that require deliberate review.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the CPU gate timing boundaries and which numbers come from workflow versus Criterion.
- [ ] Document the current authoritative bench cases and why they exist.
- [ ] Document the baseline refresh procedure and its review requirements.
- [ ] Keep schema-version and case-registry compatibility checks mandatory.
- [ ] Keep the CPU gate non-options only.
- [ ] Cross-link CPU gate failures back to workflow-gate or Criterion-root causes before changing thresholds.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the CPU gate in candidate mode after relevant CPU/runtime changes
- inspect the emitted summary JSON and pass/fail lines before modifying any baseline
- re-run the delegated workflow gate separately when CPU gate failures may be workflow-rooted
- verify hard-coded estimate paths still match the actual Criterion output layout after bench renames
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current gate authority while improving documentation and traceability
- prefer adding explanation and compatibility structure before broadening the benchmark set
- if a case is no longer representative, replace it deliberately rather than silently retargeting its path
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/27_gpu_perf_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/28_workflow_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/52_app_core_criterion_benches_2026-04-09.md`
