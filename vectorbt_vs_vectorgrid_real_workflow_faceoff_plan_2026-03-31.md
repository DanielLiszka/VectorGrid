# VectorBT vs VectorGrid Real-Workflow Faceoff Plan

Date: 2026-03-31

Status: canonical implementation planning document for the next revision of the `VectorBT vs VectorGrid` double-ALMA face-off.

Scope: repo-grounded implementation plan for upgrading the existing face-off harness from a Sharpe-only Python scorer into a realistic workflow comparison that times the default shipped `VectorGrid` product surface and a matching Python / VectorBT-side workflow over the same per-candidate metric payload.

This document is intentionally focused on the current face-off workflow owned by:

- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
- `scripts/vectorbt_double_ma_faceoff.py`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`

It is separate from:

- `benchmark_run_summary_2026-02-26.md`
- `benchmark_unification_matrix_alma_2026-02-28.md`
- `CUDA kernel review/double_crossover_shadow_10row_reduced_payload_experiment_plan_2026-03-30.md`

Those documents track benchmark outputs or kernel architecture work. This document tracks the benchmark-harness implementation plan for a fairer real-workflow face-off.

## Purpose

The current face-off is useful, but it is not yet a realistic default-workflow comparison.

Current state:

- the Python side computes ALMA locally
- the timed search computes only `step_returns`, `mean`, `std`, and `sharpe`
- `pnl` is computed only once for the winning candidate
- `vectorbt` is imported for environment capture, not for portfolio-stat execution

That makes the current benchmark a valid search microbenchmark, but not a valid comparison against the shipped `VectorGrid` default product workflow, which now defaults to the `10`-row reduced product contract for common `Sharpe` surfaces.

This plan exists to produce two benchmark lanes with clear semantics:

1. a realistic default-workflow face-off where both sides compute the same `10` per-candidate metrics
2. an optional max-throughput face-off preset that exercises the current `auto` / `10`-row one-pass VRAM ceiling

## Current Repo Truth

### Existing face-off harness

Current orchestrator:

- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`

Current Python side:

- `scripts/vectorbt_double_ma_faceoff.py`

Current VectorGrid app-path harness:

- `src-tauri/src/bin/ta_app_path_faceoff.rs`

Current frozen scenarios:

- `scripts/vectorbt_double_ma_faceoff_scenario.json`
- `scripts/vectorbt_double_ma_faceoff_scenario_250k.json`

### Current Python-side behavior

The current Python face-off does not run a real VectorBT portfolio/stat workload. It:

- builds ALMA caches itself
- builds position arrays itself
- scores candidates with local NumPy / Numba Sharpe logic
- computes `pnl` only for the winner

This is correct for a search-only benchmark, but it is narrower than the current shipped VectorGrid default path.

### Current VectorGrid default metric contract

The current GPU kernel defaults matter:

- `PRODUCT_REDUCED_METRICS_COUNT = 10`
- `METRICS_COUNT = 17`
- `TRADE_METRICS_COUNT = 22`

The default reduced product contract for the current shipped `Sharpe` workflow is the `10`-metric path.

Those `10` metrics are:

- `pnl`
- `trades`
- `max_dd`
- `mean`
- `std`
- `exposure`
- `net_exposure`
- `downside_deviation`
- `ulcer_index`
- `omega`

### Current face-off workload presets

Existing wider realistic preset:

- `scripts/vectorbt_double_ma_faceoff_scenario_250k.json`
- fast range: `5:504:1`
- slow range: `505:1004:1`
- valid pairs: `250,000`

Current max one-pass `auto` / `10`-row local ceiling established for this workstation session:

- fast count: `8,294`
- slow count: `8,295`
- valid pairs: `68,798,730`

To make every pair valid under `fast < slow`, the corresponding disjoint ranges are:

- fast range: `5:8298:1`
- slow range: `8299:16593:1`

This ceiling is tied to the current measured VRAM budget on the local RTX 4090 run context. The first implementation should treat this as an explicit benchmark preset, not as a dynamic auto-discovery feature.

## Benchmark Goal

The new primary face-off should answer:

- how fast is `VectorGrid` versus a realistic Python / VectorBT-side workflow when both compute the same `10` default product metrics per candidate?

The benchmark should not answer:

- how fast is `VectorGrid` versus a minimal Sharpe-only scorer?

That older question remains useful, but it should be preserved as a secondary benchmark lane, not the main headline.

## Architecture Target

Preferred benchmark family after implementation:

### Lane A: Real Workflow Default

Purpose:

- benchmark the shipped default `Sharpe` product workflow

Rules:

- both sides compute the same `10` per-candidate metrics
- same dataset
- same candidate grid
- same next-bar semantics
- same ALMA parameters
- same objective winner selection
- same warmup and measured trial structure

### Lane B: Max-VRAM `auto` Stress Option

Purpose:

- benchmark the largest current one-pass `auto` / `10`-row surface

Rules:

- same `10` per-candidate metrics as Lane A
- explicit preset, not a hidden benchmark-only hardcode in the runner
- report completion status, wall time, and memory pressure
- if the Python side cannot complete within a bounded runtime or memory envelope, the harness must record that outcome explicitly rather than silently truncating the workload

## Fairness Rules

This benchmark is fair only if the timed work matches.

Required fairness rules:

- both sides must compute the same `10` metrics in the timed loop
- both sides must use the same signal semantics:
  - ALMA crossover
  - `fast > slow` long-entry condition
  - next-bar position shift
  - same long-only / allow-flip defaults
- both sides must use the same return semantics:
  - step returns derived from per-bar position and bar-to-bar price move
- both sides must choose winners from the same objective:
  - default: `Sharpe`
- both sides must use the same candidate universe:
  - `250,000` disjoint-pair preset
  - `68,798,730` disjoint-pair max preset

Not required for the first implementation:

- a built-in VectorBT portfolio object for every candidate, if that makes the benchmark impossible to complete

But required:

- the Python side must become a real workflow implementation rather than a search-only microkernel
- if pure built-in VectorBT APIs are too slow or memory-heavy for the max preset, the harness must say so clearly in the results

## Implementation Strategy

### Principle 1: Preserve the current benchmark

Do not destroy the current Sharpe-only face-off.

Instead:

- keep the current Python script behavior available as a `search_only` or legacy mode
- add a new `real_workflow_10row` mode
- make `real_workflow_10row` the preferred face-off mode in the orchestrator and summaries

### Principle 2: Use explicit workload presets

Do not bury workload logic in multiple scripts.

The scenario layer should expose explicit workload presets:

- `standard_250k_auto10`
- `max_vram_auto10_68798730`

The first implementation can keep these as scenario JSON files. A later refinement can add a scenario generator if needed.

### Principle 3: Keep VectorGrid-side metric contract explicit

The VectorGrid app-path runner should record enough metadata to make the comparison auditable:

- resolved metric contract or metrics count
- whether `include_trade_metrics` is off
- objective
- top-k
- heatmap bins

The benchmark already uses the default `auto` behavior. The revised harness should make that visible in the emitted artifact instead of leaving it implicit.

### Principle 4: Treat the max preset as a stress lane

The max preset is a valid benchmark option, but it is not the primary fairness headline.

The harness and summary should keep the two presets distinct:

- standard realistic lane
- max one-pass stress lane

### Principle 5: Front-load clean VectorGrid captures

Benchmark execution order should be explicit, not incidental.

Preferred execution order for each scenario:

1. `VectorGrid gpu-kernel`
2. `VectorGrid cpu`
3. `VectorBT` / Python-side workflow

Reason:

- the `VectorGrid gpu-kernel` number is the most GPU-sensitive capture and should be recorded before any long-running background GPU work begins
- the `VectorGrid cpu` number is still materially more stable if it is recorded before the long Python / VectorBT-side run
- the `VectorBT` leg is the longest expected leg and is the best place to allow optional background GPU work if the operator is willing to trade some system cleanliness for wall-clock convenience

Operational note:

- if the goal is strictly clean benchmarking, do not start background GPU tasks until both `VectorGrid gpu-kernel` and `VectorGrid cpu` have completed
- if the goal is specifically to free the GPU as early as possible, the harness should still front-load and persist the `VectorGrid gpu-kernel` result first, then continue with the remaining legs

## Proposed Code Touchpoints

Primary files:

- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
- `scripts/vectorbt_double_ma_faceoff.py`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`

New scenario files:

- `scripts/vectorbt_double_ma_faceoff_scenario_real_workflow_250k.json`
- `scripts/vectorbt_double_ma_faceoff_scenario_real_workflow_max_vram_auto.json`

Likely optional helper files:

- `scripts/vectorbt_double_ma_faceoff_metrics.py`
- `scripts/vectorbt_double_ma_faceoff_common.py`

Optional supporting docs after implementation:

- update summary templates generated by `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`

## Detailed Work Items

### 1. Redesign the Python-side benchmark mode surface

Add explicit Python benchmark modes:

- `search_only_legacy`
- `real_workflow_10row`

`real_workflow_10row` requirements:

- compute all `10` default product metrics per candidate in the timed evaluation loop
- return the winner plus the timed metric payload metadata
- report whether the implementation uses:
  - VectorBT portfolio/stats APIs
  - VectorBT signal machinery plus local metric aggregation
  - local metric aggregation only

The result artifact must make that choice explicit.

### 2. Implement the `10`-metric Python-side metric set

For every candidate, compute:

- `pnl`
- `trades`
- `max_dd`
- `mean`
- `std`
- `exposure`
- `net_exposure`
- `downside_deviation`
- `ulcer_index`
- `omega`

The metric semantics must be documented inside the Python artifact output so parity disputes can be traced later.

### 3. Add explicit workload presets

Create or replace scenario files so the face-off can run:

- `standard_250k_auto10`
- `max_vram_auto10_68798730`

Preset definitions:

- `standard_250k_auto10`
  - fast: `5:504:1`
  - slow: `505:1004:1`
  - valid pairs: `250,000`
- `max_vram_auto10_68798730`
  - fast: `5:8298:1`
  - slow: `8299:16593:1`
  - valid pairs: `68,798,730`

The scenario metadata should include:

- intended metric mode: `real_workflow_10row`
- intended VectorGrid contract: `auto`
- intended pair count
- note that the max preset is tied to the current local one-pass VRAM ceiling

### 4. Upgrade the PowerShell orchestrator

`scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` should gain:

- mode selection for Python side:
  - legacy search-only
  - real workflow 10-row
- explicit execution order:
  - `VectorGrid gpu-kernel`
  - `VectorGrid cpu`
  - `VectorBT`
- clearer scenario labels in output paths
- explicit summary fields:
  - metric mode
  - metric count
  - preset id
  - completion status
  - failure reason
  - peak memory if captured
- support for the new max preset without manual script edits
- partial artifact flushes after:
  - the `VectorGrid gpu-kernel` leg
  - the full `VectorGrid` block

The partial flush requirement exists so an operator can recover and use the early clean captures even if the later Python / VectorBT leg runs for hours or fails.

### 5. Upgrade VectorGrid app-path artifact metadata

`src-tauri/src/bin/ta_app_path_faceoff.rs` should emit enough metadata to prove what path ran:

- objective
- resolved backend
- metric contract request
- resolved metric count if available
- `include_trade_metrics`
- `top_k`
- `heatmap_bins`
- ranges and pair count

This does not need to change the optimization route. It only needs to make the route auditable.

### 6. Add bounded failure handling for the max preset

The max preset is expected to be stressful, especially on the Python side.

The harness must support explicit outcomes:

- `completed`
- `timed_out`
- `oom`
- `failed`

The summary must preserve partial evidence:

- elapsed wall time before failure
- Python stderr / exception
- pair count attempted
- hardware and environment info

### 7. Add memory capture for the Python side

For realism and auditability, capture at minimum:

- peak working set or RSS
- peak private bytes if available on Windows
- final process exit status

This can be done in the PowerShell runner without changing benchmark semantics.

## Acceptance Criteria

The new face-off implementation is done when all of the following are true:

- the repo has a new default `real_workflow_10row` face-off lane
- the current `search_only` benchmark is preserved
- the Python side computes the same `10` per-candidate metrics as the VectorGrid default product path
- the orchestrator supports both workload presets:
  - `250,000`
  - `68,798,730`
- the summary clearly distinguishes:
  - realistic default workflow results
  - max one-pass stress results
- failures in the max preset are recorded explicitly instead of being treated as missing data
- the benchmark output captures enough metadata to explain metric semantics and route shape

## Rejection Criteria

Reject the implementation if any of these happen:

- the new benchmark quietly remains Sharpe-only while being labeled as real-workflow
- the Python side computes a different metric set than VectorGrid default and the mismatch is not disclosed
- the max preset is wired as the only benchmark path
- the old search-only face-off is deleted
- the summary collapses standard and max-stress results into one headline without labeling
- the max preset silently batches or truncates work while claiming to represent the full one-pass ceiling

## Verification Matrix

### Functional verification

- run the revised face-off on the `250,000` preset in legacy search-only mode
- run the revised face-off on the `250,000` preset in `real_workflow_10row` mode
- confirm summary labeling and artifact schema
- confirm VectorGrid and Python pair counts match the scenario

### Stress verification

- run the revised face-off on the max preset with bounded trials
- confirm explicit completion or failure capture
- confirm memory capture is written
- confirm the summary distinguishes the max preset from the standard preset

### Route verification

- confirm VectorGrid artifact records the effective default `auto` / `10`-row path on the standard `Sharpe` face-off
- confirm the summary includes that contract metadata

## Phase Order

### Phase 0: Spec Lock

Goal:

- freeze the target benchmark semantics before code changes

Deliverables:

- this plan
- final metric list
- final preset definitions

### Phase 1: Python Workflow Upgrade

Goal:

- add the real `10`-metric Python-side workflow while preserving legacy search-only mode

Deliverables:

- Python mode selection
- `10`-metric candidate computation
- artifact schema updates

### Phase 2: Orchestrator And Scenario Upgrade

Goal:

- support both benchmark presets and both benchmark modes cleanly

Deliverables:

- new scenario files
- updated PowerShell runner
- updated output summaries

### Phase 3: VectorGrid Artifact Upgrade

Goal:

- make the VectorGrid side self-describing in benchmark artifacts

Deliverables:

- app-path faceoff metadata additions
- route / metric metadata in emitted JSON

### Phase 4: Max-Preset Stress Validation

Goal:

- prove that the max preset is runnable as an explicit benchmark option and captures failure cleanly when not runnable

Deliverables:

- bounded max-preset runs
- explicit failure taxonomy
- memory capture

## Checklist

### Phase 0 Checklist

- [x] Freeze the real-workflow metric list at the shipped `10`-row default contract.
- [x] Freeze the two preset ranges:
- [x] `250,000` valid pairs via `5:504:1` and `505:1004:1`.
- [x] `68,798,730` valid pairs via `5:8298:1` and `8299:16593:1`.
- [x] Preserve the current Sharpe-only face-off as a legacy benchmark lane.

### Phase 1 Checklist

- [x] Add a Python-side benchmark mode switch.
- [x] Keep `search_only_legacy` behavior intact.
- [x] Implement `real_workflow_10row`.
- [x] Compute `pnl` per candidate inside the timed loop.
- [x] Compute `trades` per candidate inside the timed loop.
- [x] Compute `max_dd` per candidate inside the timed loop.
- [x] Compute `mean` per candidate inside the timed loop.
- [x] Compute `std` per candidate inside the timed loop.
- [x] Compute `exposure` per candidate inside the timed loop.
- [x] Compute `net_exposure` per candidate inside the timed loop.
- [x] Compute `downside_deviation` per candidate inside the timed loop.
- [x] Compute `ulcer_index` per candidate inside the timed loop.
- [x] Compute `omega` per candidate inside the timed loop.
- [x] Emit metric-semantics metadata in the Python result JSON.

### Phase 2 Checklist

- [x] Add a new scenario file for the standard `250,000` real-workflow preset.
- [x] Add a new scenario file for the max-VRAM `68,798,730` real-workflow preset.
- [x] Update the orchestrator to accept the new scenarios without manual edits.
- [x] Make backend execution order explicit instead of relying on incidental array ordering.
- [x] Run `VectorGrid gpu-kernel` before `VectorGrid cpu`.
- [x] Run the full `VectorGrid` block before `VectorBT`.
- [x] Add summary labeling for benchmark mode and preset id.
- [x] Add summary labeling for completion status and failure reason.
- [x] Keep output directories distinct per preset and per metric mode.
- [x] Flush partial artifacts after the `VectorGrid gpu-kernel` leg completes.
- [x] Flush partial artifacts after the `VectorGrid cpu` leg completes.

### Phase 3 Checklist

- [x] Add VectorGrid artifact fields for objective, range specs, and pair count.
- [x] Add VectorGrid artifact fields for `top_k` and `heatmap_bins`.
- [x] Add VectorGrid artifact fields for metric-contract request and trade-metric flags.
- [x] Add resolved route metadata needed to audit that the default `auto` path ran.

### Phase 4 Checklist

- [x] Add bounded timeout handling for Python-side max-preset runs.
- [x] Add explicit `completed`, `timed_out`, `oom`, and `failed` result states.
- [x] Capture peak memory for Python-side trials.
- [x] Capture max-preset summary output even when the Python side fails.
- [ ] Verify the standard preset still completes cleanly after max-preset support is added.

### Reporting Checklist

- [x] Make the standard `250,000` preset the default benchmark reported in summaries.
- [x] Report the max preset as a separate stress option, not the default headline.
- [x] Keep legacy search-only results available for search-engine microbenchmark comparisons.
- [x] Document clearly whether the Python side used actual VectorBT APIs, local metric aggregation, or a hybrid workflow.
- [x] Document benchmark execution order in the summary metadata so later readers know GPU captures were front-loaded intentionally.

## Definition Of Done

This benchmark plan is complete when the repo has:

1. a realistic `10`-metric face-off lane that matches the shipped VectorGrid default product workflow
2. explicit support for both the `250,000` standard preset and the `68,798,730` max-VRAM preset
3. preserved legacy Sharpe-only search benchmarking for narrow engine-speed comparisons
4. auditable summaries that distinguish normal workflow benchmarking from max-throughput stress results

At that point, the repo will support both a fair default-workflow comparison and a separate maximum-throughput comparison without conflating the two.
