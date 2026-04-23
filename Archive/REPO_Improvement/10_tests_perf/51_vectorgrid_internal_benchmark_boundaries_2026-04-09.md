# VectorGrid Internal Benchmark Boundaries Plan

Last updated: 2026-04-10

## Purpose

Define the internal benchmark vocabulary so the repo’s many timing surfaces stay distinguishable and honest.

## Scope Boundaries

In scope:

- the timing-boundary taxonomy used across perf gates, helper bins, and faceoff scripts
- same-thread app-path, CLI cold wall-clock, Python-process wall-clock, workflow regression, Criterion, and telemetry-only boundaries
- the relation between internal timing boundaries and public-facing claims
- non-options benchmark boundary consistency across docs and artifacts

Out of scope:

- options-related benchmark surfaces
- changing the underlying benchmark tools themselves
- general benchmarking theory outside the repo’s current surfaces
- marketing or product messaging beyond repo-controlled artifacts

## Current User-Facing Behavior

The repo already has a rich internal timing vocabulary scattered across scripts and bins. That is a strength, but only if the boundaries are named and reused consistently. Today the code is often more honest than the prose, which is exactly why this doc matters.

## Current Code Surface

- `src-tauri/src/bin/ta_app_path_faceoff.rs` emits same-thread app-path boundary data with detailed timing breakdowns.
- `scripts/run_cpu_perf_gate.ps1` and `scripts/run_gpu_perf_gate.ps1` combine workflow and Criterion-style boundaries into authority gates.
- `src-tauri/src/bin/ta_workflow_regression.rs` defines the synthetic workflow-regression boundary and its exclusions.
- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` emits comparison outputs across multiple distinct timing boundaries.

## Current Verification Surface

- inspection of summary JSON emitted by the various gates and helper bins
- manual cross-check of README or doc claims against those boundary labels
- rerunning the specific lane when a boundary label looks stale or ambiguous

## Active Boundary Inventory

- `app-path`
  - main owners:
    - `src-tauri/src/bin/ta_app_path_faceoff.rs`
    - `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
  - meaning:
    - same-thread harness using `AppState` plus a blocking optimizer call, not the full Tauri command path
  - authority use:
    - valid for saved warm faceoff snapshots when the scenario and artifact date are explicit
- `app-path-fresh-state`
  - main owner:
    - `src-tauri/src/bin/ta_app_path_faceoff.rs`
  - meaning:
    - fresh `AppState` plus dataset load/resample and blocking optimizer call per trial
  - authority use:
    - helper comparison boundary, not the main README snapshot surface
- `CliColdWallClock`
  - main owner:
    - `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
  - meaning:
    - external VectorGrid CLI process wall-clock timing for a cold invocation
  - authority use:
    - comparison/helper lane, not the warm app-path README snapshot
- `PythonProcessWallClock`
  - main owners:
    - `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
    - `scripts/vectorbt_double_ma_faceoff.py`
  - meaning:
    - external Python/VectorBT workflow process wall-clock timing
  - authority use:
    - valid only for explicit VectorBT faceoff comparisons with captured interpreter provenance
- `workflow-regression`
  - main owners:
    - `src-tauri/src/bin/ta_workflow_regression.rs`
    - `scripts/run_workflow_gate.ps1`
    - `scripts/run_cpu_perf_gate.ps1`
  - meaning:
    - total deterministic workflow wall time including optimize, validate, meta-select, and holdout, with explicit exclusions for UI hydration and analysis-command latency
  - authority use:
    - authoritative workflow gate boundary
- `Criterion`
  - main owners:
    - `scripts/run_cpu_perf_gate.ps1`
    - `scripts/run_gpu_perf_gate.ps1`
    - `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`
    - `scripts/run_vectorgrid_double_ma_faceoff.ps1`
  - meaning:
    - Criterion bench-profile estimate for a deterministic optimization case or ratio
  - authority use:
    - authoritative for the Criterion-backed gate cases; not interchangeable with warm faceoff or workflow-regression figures
- `RunnerSteadyStateWallClock`
  - main owner:
    - `scripts/run_gpu_perf_gate.ps1`
  - meaning:
    - release-built `profile_gpu_dynamic_runner` steady-state wall-clock ratio between dynamic-strategy off and on runs
  - authority use:
    - authoritative only for that specialized dynamic-strategy sub-check, not for general product benchmark claims
- `route-matrix`
  - main owner:
    - `src-tauri/src/bin/ta_profile_route_matrix.rs`
  - meaning:
    - helper-bin route profiling boundary
  - authority use:
    - helper-bin only; not a public perf gate
- `validation-cost`
  - main owner:
    - `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
  - meaning:
    - helper-bin validation-cost profiling boundary
  - authority use:
    - helper-bin only; not a public perf gate
- `process-memory-sampling`
  - main owners:
    - `scripts/capture_optimization_memory.ps1`
    - `scripts/baselines/ui_capture/optimization_memory_metrics.json`
  - meaning:
    - peak process working-set/private-byte sampling over a synthetic workflow run
  - authority use:
    - telemetry-only and explicitly non-authoritative as a perf gate
- `headless-output-parity`
  - main owners:
    - `scripts/run_output_parity_check.mjs`
    - `scripts/headless_output_parity_pack.json`
  - meaning:
    - deterministic headless report generation for structural JSON/XLSX parity
  - authority use:
    - supplemental verification only; not a perf gate

## Strengths Already Present

- many existing artifacts already carry timing-boundary labels explicitly
- the repo distinguishes micro-bench, workflow, faceoff, and telemetry surfaces in code
- boundary language can be standardized without changing runtime code
- honest internal taxonomy directly improves research, product, and release discussions

## Weaknesses / Failure Modes

- the taxonomy is distributed across many files rather than concentrated in one doc or enum
- different contributors can still paraphrase the same boundary differently in prose
- boundary names are only useful if they are repeated consistently in summaries and docs
- readers may still prefer simplified claims, increasing pressure to flatten unlike surfaces

## Regressions To Avoid

- inventing new prose labels for existing boundaries instead of reusing the canonical ones
- merging unlike timing surfaces into one internal chart or narrative
- allowing doc claims to outlive the artifact boundary labels they were based on
- treating telemetry-only or helper-bin outputs as gate authority
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant because several internal boundaries are Criterion-based while others are not.
- [Cargo benchmark command docs](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) are relevant because multiple internal surfaces ultimately originate from Cargo benchmark semantics.
- [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/pdf/CUDA_C_Best_Practices_Guide.pdf) is relevant because GPU timing boundaries especially require precise measurement context.
- [Python `venv` docs](https://docs.python.org/3/library/venv.html) are relevant because Python-process wall-clock boundaries depend on controlled comparison environments.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Document the internal timing-boundary vocabulary in one place and reuse it consistently across docs.
2. Tie each boundary label to one or more concrete scripts or bins.
3. Use the taxonomy as a review tool whenever benchmark prose is written or updated.
4. Prefer preserving multiple honest boundaries over creating one oversimplified aggregate number.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Inventory the active internal timing boundaries and their canonical names.
- [x] Map each boundary to its main scripts or bins.
- [x] Keep this boundary taxonomy non-options only.
- [x] Review benchmark prose and release notes against the canonical vocabulary.
- [x] Mark telemetry-only and helper-bin boundaries explicitly as non-authoritative.
- [x] Avoid introducing new ad hoc boundary names without need.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- inspect current artifact outputs for boundary-name consistency
- cross-check docs and summaries against those outputs
- re-run a lane when its emitted boundary metadata is unclear or stale
- treat inconsistent boundary naming as a documentation regression
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve existing explicit boundary labels and standardize around them
- prefer documentation and review discipline over new runtime complexity
- if a truly new boundary is added, add it explicitly to the taxonomy and surrounding docs
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closure date: `2026-04-10`
- Code changes were not required; the active timing-boundary vocabulary was already present in the helper bins, gates, and saved artifacts.
- Verified active boundary labels from current code and artifacts:
  - `src-tauri/src/bin/ta_app_path_faceoff.rs` emits `app-path` and `app-path-fresh-state`
  - `src-tauri/src/bin/ta_workflow_regression.rs` emits `workflow-regression` plus explicit exclusions
  - `scripts/run_cpu_perf_gate.ps1` and `scripts/run_gpu_perf_gate.ps1` use `Criterion` and the specialized `RunnerSteadyStateWallClock` dynamic-strategy ratio boundary
  - `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` uses `app-path`, `CliColdWallClock`, and `PythonProcessWallClock`
  - `scripts/baselines/ui_capture/optimization_memory_metrics.json` marks `process-memory-sampling` as supplemental telemetry only
  - `scripts/headless_output_parity_pack.json` marks `headless-output-parity` as supplemental verification only
- Artifact cross-check evidence:
  - `target/vectorbt_vs_vectorgrid_faceoff_verify_smoke_2026-04-10/legacy_58300_search_only-search_only_legacy/summary.json`
  - `target/workflow_regression_summary.json`
  - `target/gpu_alma_contract_rows_perf_summary.json`
  - `scripts/baselines/ui_capture/optimization_memory_metrics.json`
  - `scripts/headless_output_parity_pack.json`
- README benchmark prose already matched the canonical vocabulary:
  - README historical benchmark snapshots still describe the public numbers as warm `app-path` saved artifacts, not Criterion or perf-gate outputs
  - the Criterion anchor remains explicitly called out as a tighter engine benchmark boundary rather than being merged into the warm faceoff story
- No `README.md`, `REPO_MAP.md`, or release-note edit was required for closure because the existing prose already aligned with the code/artifact vocabulary.

## Related Docs

- `REPO_Improvement/10_tests_perf/31_vectorbt_faceoff_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/32_benchmark_claims_hygiene_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/38_profile_runner_bins_2026-04-09.md`
