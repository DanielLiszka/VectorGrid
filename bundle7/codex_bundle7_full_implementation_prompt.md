# Codex implementation prompt: Bundle 7 benchmark harnesses, perf gates, and timing boundaries

Copy/paste the prompt below into an OpenAI Codex agent that has write access to the repo **and** access to the attached guide documents.

---

You are working inside my Rust/Tauri repo. This is an **implementation task**, not a review task.

Your job is to **fully implement** the benchmark/perf-gate/timing-boundary improvements described in these attached guide documents:

1. `gpt54_feature_review_map.md` — use **Section `## 7. Benchmark harnesses, perf gates, and timing boundaries`** as the scope and guardrail source of truth.
2. `bundle7_benchmark_harness_impl_plan.md` — use this as the primary implementation guide.
3. `bundle7_benchmark_harness_impl_plan_addendum_v2.md` — use this as the correction/tightening guide and safe-extension guide.

## Source-of-truth precedence

If these documents disagree:

- **Scope / guardrails:** `gpt54_feature_review_map.md` wins.
- **Implementation detail:** `bundle7_benchmark_harness_impl_plan_addendum_v2.md` overrides `bundle7_benchmark_harness_impl_plan.md`.
- **If something is still ambiguous:** prefer the smallest additive, parity-safe change that preserves current behavior and makes performance evidence more trustworthy.
- **Repo-validated implementation scope:** the validated checklist in this prompt wins over both plan docs. If a later section of either plan suggests work that is not represented in the checklist below, treat that later plan text as background only.

## Repo-state validation outcome

The Bundle 7 guides are only **partially valid** for the repo as it exists now.

Use these repo-state findings as guardrails before implementing anything:

- `scripts/run_workflow_gate.ps1` still has the baseline-mode `OutPath` bug and still lacks explicit release-profile control.
- `scripts/run_cpu_perf_gate.ps1` still reads a hard-coded workflow summary path and still gates only one CPU Criterion case.
- `scripts/run_gpu_perf_gate.ps1` already builds and runs `profile_gpu_dynamic_runner` in release mode and already emits a GPU CI-width warning, but it still relies on latest/glob estimate fallback and still compares candidate `on` vs candidate `off` instead of candidate ratio vs baseline ratio.
- `src-tauri/src/bin/ta_app_path_faceoff.rs` already records per-trial harness/state/engine timing and already carries `timing_boundary_note`, but it still lacks the full metadata envelope, missing summary medians for the main timing fields, and route-summary counts.
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs` already persists backend route/fallback plus separate average main-validation and cross-ticker wall times, but it still lacks the shared metadata envelope and separate main/cross ratios vs optimization.
- `src-tauri/src/bin/ta_profile_route_matrix.rs` already records requested backend plus actual route/fallback per case, but it still lacks `expected_backend_route`, `route_ok`, shared metadata, and `--strict`.
- `scripts/profile_gpu_backtest_nsys.ps1` already supports `-NsysExePath` and already has a whole-process fallback path. `scripts/profile_gpu_backtest_ncu.ps1` still lacks `-NcuExePath`.
- `scripts/run_vectorgrid_double_ma_faceoff.ps1` still parses human-readable Criterion text and still hard-codes metadata that should be derived from the benchmark id.
- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` already preserves app-path subphase timing and backend route in trial artifacts, but its grouped summary rows still use the first artifact rather than a canonical median-time trial and still do not carry explicit timing-boundary/build-profile labels.
- `crates/app_core/benches/double_ma_opt.rs` already has optional route-probe logging via `VECTORBT_BENCH_LOG_ROUTE_PROBES`, but several env-shaped variants still mutate env vars inside measured `b.iter(...)` loops.
- `crates/app_core/benches/double_ma_validation_workflow.rs`, `crates/app_core/benches/composite_validation_workflow.rs`, and `crates/app_core/benches/gpu_native_expansion_opt.rs` already emit route probes and/or exact-route assertions. Extend metadata around them; do not re-implement their route logic.
- `crates/app_core/benches/e2e_workflow_ml.rs` already supports env-configurable candle lengths via `VTA_E2E_BENCH_LENS`. Only the missing train/holdout controls and optional stage diagnostics remain.

## Current-repo validation lock

The guide documents were written against an earlier understanding of the repo. This prompt has already been validated against the **current** repo state and now overrides stale guide bullets.

Use the validated checklist below as the implementation authority. The original plan/addendum remain useful background, but **do not** implement anything from them that conflicts with this validated scope or that asks you to redo behavior already present in the repo.
For day-to-day implementation tracking, use `bundle7_benchmark_harness_validated_tracking_checklist.md` in this folder as the authoritative progress tracker.

### Already present in the repo; preserve and extend, do not re-implement

- [x] `scripts/workflow_benchmark_pack.json` already exists and is the workflow-pack source for `scripts/run_workflow_gate.ps1`. Preserve it rather than replacing it with ad hoc workflow-pack logic.
- [x] `scripts/run_gpu_perf_gate.ps1` already builds and runs `profile_gpu_dynamic_runner` in **release** mode. Keep that behavior; do not add redundant release-mode work here.
- [x] `scripts/run_gpu_alma_contract_rows_perf_gate.ps1` already exists as a separate extended GPU contract-row surface. Preserve/extend that script rather than forcing contract-row cases into the main per-PR GPU gate unless there is a clear low-risk reason.
- [x] `scripts/profile_gpu_backtest_nsys.ps1` already supports `-NsysExePath` and already has whole-process fallback behavior for bounded NVTX capture failure. Preserve that; only add missing provenance/restoration hardening.
- [x] `scripts/run_gpu_perf_gate.ps1` already emits a CI-width instability warning. Keep it; any stronger instability policy stays optional.
- [x] `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` already persists detailed per-trial artifacts. Fix canonical-trial selection and timing-boundary/build-profile labeling rather than redesigning the harness.
- [x] `src-tauri/src/bin/ta_app_path_faceoff.rs` already records per-trial `total_wall_ms`, `harness_wall_ms`, `state_lookup_ms`, `engine_wall_ms`, `engine_runtime_ms`, GPU/hybrid subphase timing, and a `timing_boundary_note`. Extend the schema/summary without redefining this boundary.
- [x] `src-tauri/src/bin/ta_double_ma_validation_cost.rs` already captures requested backend plus backend used/route/fallback metadata and already records raw main-vs-cross timing. Add the missing ratios/metadata rather than redesigning the harness.
- [x] `src-tauri/src/bin/ta_profile_route_matrix.rs` already captures requested backend, backend used/route/fallback, runtime, wall, and best-result fields. Extend it; do not rewrite the binary around a new shape.
- [x] `crates/app_core/benches/gpu_native_expansion_opt.rs` and `crates/app_core/benches/composite_validation_workflow.rs` already contain exact GPU route assertions / route probe prints. Preserve them and only add missing metadata banners.
- [x] `crates/app_core/benches/double_ma_validation_workflow.rs` already prints route/profile diagnostics. Preserve those and only add missing boundary/build-profile metadata / optional strict gating.
- [x] `crates/app_core/benches/double_ma_opt.rs` already has route-probe logging and env-shaped contract-split helpers. Reuse the existing route-probe path instead of introducing a second env-variable name just to match the older plan wording.
- [x] `crates/app_core/benches/dynamic_strategy_cpu.rs` already keeps its dynamic env toggles outside measured `b.iter(...)` loops. Do not treat addendum env-mutation cleanup as missing work in this file.
- [x] `crates/app_core/benches/e2e_workflow_ml.rs` already supports env-configurable candle lengths via `VTA_E2E_BENCH_LENS`. Only train/holdout asset controls and optional stage diagnostics remain.

### Do not introduce churn just to match guide wording

- Keep existing benchmark IDs stable.
- Keep existing env var names when they already exist; guide names like `VECTORBT_BENCH_EMIT_ROUTE` are examples, not mandatory renames.
- Do **not** force contract-row cases into `scripts/run_gpu_perf_gate.ps1` if `scripts/run_gpu_alma_contract_rows_perf_gate.ps1` remains the extended/nightly surface.
- Do **not** require `cargo run --release` specifically for the GPU dynamic runner if executing a release-built binary preserves the same timing boundary and keeps the implementation simpler.
- Use deterministic per-script case maps for this pass unless a manifest file is clearly necessary. Do **not** add a new manifest/registry file just because the addendum mentions one.
- Treat later code snippets in the plan/addendum as background only. They are not authoritative if they conflict with the current repo code or with this prompt’s validated checklist.

## Validated implementation checklist

This checklist is the authoritative in-scope work for the current repo state.

### Validation status

- [x] Re-read `gpt54_feature_review_map.md` Section 7 plus the Bundle 7 plan and addendum.
- [x] Validate the proposed work against the current repo scripts, benches, binaries, and baseline artifacts.
- [x] Identify already-implemented surfaces that should be preserved rather than re-implemented.

### Required implementation scope

- [ ] Fix `scripts/run_workflow_gate.ps1` so baseline mode keeps `OutPath` stable, writes a fresh summary to `OutPath`, copies that summary to `BaselinePath`, asserts the requested summary exists, and supports release-mode execution control suitable for gate use.
- [ ] Fix `scripts/run_cpu_perf_gate.ps1` so it passes an explicit workflow summary path into `run_workflow_gate.ps1`, reads back that exact path, stops assuming baseline mode also writes the candidate path, and validates baseline compatibility before pass/fail comparisons.
- [ ] Expand `scripts/run_cpu_perf_gate.ps1` from a single Criterion CPU case to a deterministic two-case matrix using the current repo’s live surfaces: keep the existing small SMA CPU case and add a representative large ALMA CPU case.
- [ ] Persist explicit timing-boundary and build-profile metadata for the CPU workflow block, each CPU Criterion case, and the dynamic CPU ratio block in `scripts/run_cpu_perf_gate.ps1`.
- [ ] Fix `scripts/run_gpu_perf_gate.ps1` so it no longer uses “latest estimate” or glob fallback for pass/fail logic and instead resolves exact Criterion estimate paths deterministically **per named case**.
- [ ] Expand `scripts/run_gpu_perf_gate.ps1` from one GPU Criterion case to a deterministic two-case matrix using the current repo’s live surfaces: keep the existing small SMA GPU case and add a representative large ALMA GPU case, while preserving existing contract-split surfaces as optional/nightly evidence rather than deleting them.
- [ ] Change the GPU dynamic gate in `scripts/run_gpu_perf_gate.ps1` from candidate `on` vs candidate `off` to candidate ratio vs baseline ratio, while also keeping an absolute baseline guard on static/off latency and labeling the GPU Criterion and dynamic-runner evidence by timing boundary/build profile.
- [ ] Add baseline compatibility validation to CPU and GPU gate comparisons, including schema/boundary/build-profile/case identity checks and GPU hardware compatibility checks where relevant.
- [ ] Add metadata envelope fields plus stage breakdowns to `src-tauri/src/bin/ta_workflow_regression.rs`, keeping the existing total `runtime_ms` and adding at least optimize total, validate total, meta-select, and holdout timings.
- [ ] Add additive metadata envelope fields to `src-tauri/src/bin/ta_double_ma_validation_cost.rs`, preserve existing route/fallback data plus raw main-vs-cross timing, and separate total-vs-optimization, main-validation-vs-optimization, and cross-ticker-vs-optimization ratios.
- [ ] Add explicit `schema_version`, `timing_boundary`, `build_profile`, and `requested_backend` fields to `src-tauri/src/bin/ta_app_path_faceoff.rs`, preserve the current same-thread app-path boundary plus existing per-trial timing fields, add summary medians for total wall/state lookup/harness wall/engine wall/engine runtime, and add a route-summary block across measured trials.
- [ ] Add additive envelope/build-profile fields plus `expected_backend_route`, `route_ok`, and `--strict` handling to `src-tauri/src/bin/ta_profile_route_matrix.rs` without redesigning the existing one-shot surface.
- [ ] Add provenance sidecar JSON plus `try/finally`-style env restoration to `scripts/profile_gpu_backtest_nsys.ps1` and `scripts/profile_gpu_backtest_ncu.ps1`; for NCU also add `-NcuExePath`.
- [ ] Update `scripts/run_vectorgrid_double_ma_faceoff.ps1` to read structured Criterion `estimates.json` rather than parsing human-readable CLI timing text, derive metadata from the benchmark ID where practical, and label artifacts with `timing_boundary = "Criterion"` and `build_profile = "bench"`.
- [ ] Update `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1` so every persisted artifact is labeled by timing boundary/build profile, summary rows use the canonical median-time trial rather than the first artifact, parity also uses the canonical trial, and summary output preserves app-path-specific stats instead of mixing boundaries.
- [ ] In `crates/app_core/benches/double_ma_opt.rs`, keep the current benchmark IDs and existing route-probe behavior but add broader benchmark-shape metadata and move env-var mutation out of measured `b.iter(...)` loops where the mode is fixed for the whole benchmark.
- [ ] Add additive metadata banners to the currently live Bundle 7 bench surfaces without renaming benchmark IDs: `double_ma_metrics.rs`, `double_ma_followup_workflow.rs`, `dynamic_strategy_cpu.rs`, `gpu_native_expansion_opt.rs`, `double_ma_validation_workflow.rs`, `composite_validation_workflow.rs`, and `e2e_workflow_ml.rs`.
- [ ] In `crates/app_core/benches/dynamic_strategy_cpu.rs`, add metadata banners for the existing `eval`, `staging`, and `full_run` groups without changing benchmark IDs or default gate numbers.
- [ ] In `crates/app_core/benches/e2e_workflow_ml.rs`, add metadata banners plus env-configurable train/holdout asset counts while keeping the benchmark small enough for practical use.

### Optional follow-up only

- [ ] Optional: add repeat/median support to `ta_profile_route_matrix.rs` while preserving one-shot default behavior.
- [ ] Optional: add stage-specific workflow budgets to the workflow gate path after stage timing exists, while preserving the total-runtime gate.
- [ ] Optional: add normalized throughput metrics to validation-cost, route, and app-path surfaces if they are additive and low-risk.
- [ ] Optional: add an explicit warm/cold split to `ta_app_path_faceoff.rs` only as a separate named boundary, never by silently redefining the current app-path metric.
- [ ] Optional: add a manifest / registry file only if deterministic per-script case maps prove insufficient.
- [ ] Optional: add environment fingerprinting or stronger instability policy only if it is clearly additive and low-risk.
- [ ] Optional: add a quick local rerun mode to `crates/app_core/benches/dynamic_strategy_cpu.rs` only if it stays fully additive.
- [ ] Optional: add stage diagnostics to `crates/app_core/benches/e2e_workflow_ml.rs` only if they stay fully additive.

## High-level objective

Implement the Bundle 7 improvements so the repo’s performance evidence becomes:

- more correct
- more deterministic
- better labeled by timing boundary
- more route/fallback aware
- safer for regression gating
- better aligned with real production hotspots

Primary priorities are:

- performance
- optimization guidance
- correctness
- parity safety
- benchmark fidelity

Do **not** remove features, remove measurement surfaces, silently redefine boundaries, or break existing functionality.

## Hard constraints

Follow these constraints throughout the implementation:

1. **Do not drift out of Bundle 7 scope.**
   - No broad UI work.
   - No broad product ideation.
   - No new strategies unless directly required to fix a measured perf problem.
   - No generic architecture rewrite.

2. **Prefer smaller parity-safe changes before larger refactors.**
   - Fix correctness and evidence-trust bugs first.
   - Then add schema/boundary metadata.
   - Then expand representative coverage.
   - Then add safe optional hardening.

3. **Do not silently mix timing boundaries.**
   Keep these boundaries explicit and separate:
   - `Criterion`
   - `app-path`
   - `validation-cost`
   - `workflow-regression`
   - `route-matrix`
   - `profiling-only`

4. **Every persisted artifact/summary must say what it measured.**
   Add/preserve these fields where relevant:
   - `schema_version`
   - `timing_boundary`
   - `timing_boundary_note`
   - `build_profile`
   - `generated_at_utc`
   - `requested_backend`
   - `backend_used`
   - `backend_route`
   - `backend_fallback_reason`

5. **Every pass/fail gate must use deterministic artifact selection.**
   - No “latest matching artifact” fallback for pass/fail logic.
   - No glob-based fallback to the most recent Criterion estimate.

6. **Baseline comparisons must validate compatibility before comparing numbers.**
   Candidate vs baseline comparisons must validate at least:
   - `schema_version`
   - `timing_boundary`
   - `build_profile`
   - case identity / `bench_filter`
   - optional `case_registry_version`
   - GPU hardware compatibility for GPU gates

7. **Do not silently change benchmark IDs or paths that gates depend on.**
   Keep existing benchmark names stable unless there is no safe alternative.

8. **Use structured evidence instead of terminal text parsing wherever possible.**
   For example:
   - read Criterion `estimates.json`
   - do not parse human-readable `time: [...]` CLI output when structured JSON exists

9. **Move harness-only overhead out of measured Criterion loops when possible.**
   Especially env-var mutation when the mode is fixed for the whole benchmark.

10. **Preserve compatibility for existing consumers as much as possible.**
   - Prefer additive fields over destructive renames.
   - If a field must change semantics, keep the old field if possible and add a new explicit field.

11. **Nsight remains diagnostic.**
   Improve provenance and sidecars, but do **not** turn Nsight into a hard regression gate.

12. **Do not fabricate measurements or baselines.**
   - Run what you can in the current environment.
   - If GPU/profiler execution is unavailable, implement the code paths, compile/test what is possible, and report exactly what remains to be run on a suitable machine.

## Repo files to modify

Work on the actual repo files at these paths (search if layout differs slightly):

### Scripts
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`
- `scripts/profile_gpu_backtest_nsys.ps1`
- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
- `scripts/run_vectorgrid_double_ma_faceoff.ps1`

### Criterion benches
- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/benches/double_ma_followup_workflow.rs`
- `crates/app_core/benches/double_ma_metrics.rs`
- `crates/app_core/benches/dynamic_strategy_cpu.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`

### Binaries / harnesses
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

### Optional new files if justified by the guides
Only add new files if they are clearly useful and low-risk. Likely candidates:
- helper test fixtures for PowerShell script logic
- optional manifest/registry file such as `benchmarks/perf_case_registry.json`, but only if deterministic per-script case maps prove insufficient for this pass

Do **not** create broad new subsystems.

## Cross-cutting implementation requirements

Implement these shared conventions first, then apply them consistently.

### A. Shared artifact/schema conventions

For persisted JSON summaries/artifacts, use a shared additive envelope like:

```json
{
  "schema_version": 1,
  "timing_boundary": "Criterion | app-path | validation-cost | workflow-regression | route-matrix | profiling-only",
  "timing_boundary_note": "Human-readable note about what is and is not included.",
  "build_profile": "bench | release | dev | python",
  "generated_at_utc": "..."
}
```

Where relevant, extend with:

```json
{
  "requested_backend": "CpuOnly | GpuKernel | GpuHybridKernel | Auto | python_cpu",
  "backend_used": "Cpu | Gpu",
  "backend_route": "GpuKernel | GpuHybridKernel | CpuOnly | cpu_python_workflow | ...",
  "backend_fallback_reason": null
}
```

If safe and low-risk, also add environment fingerprinting later:
- git commit / dirty state
- rustc/cargo version
- hostname / OS
- CPU model
- GPU model / driver

### B. Shared Rust helper patterns

Add/adopt these patterns where needed:

1. `build_profile()` helper in binaries that can be run in dev or release:

```rust
fn build_profile() -> &'static str {
    if cfg!(debug_assertions) { "dev" } else { "release" }
}
```

2. Small Criterion metadata banner helper in bench files so logs are self-describing without changing measured behavior.

3. When fixed benchmark modes are currently toggled via env vars, move env mutation outside `b.iter(...)` using an RAII-style guard or equivalent. The addendum explicitly calls this out for `double_ma_opt.rs`.

### C. Shared PowerShell helper patterns

Add/adopt these patterns where needed:

1. UTF-8 JSON writer helper.
2. Baseline compatibility validation helper.
3. GPU hardware compatibility validation helper.
4. Deterministic estimate-path resolver.
5. Prefer pure/helper functions for logic that can be smoke-tested.

## Implementation order

Only implement the validated checklist above. The long plan/addendum snippets that follow are supporting detail only and must not expand scope beyond that checklist.

Follow this order. Do not stop after the first phase unless blocked by something real.

### Phase 0 — correctness and gate-trust fixes (must do first)

These are the highest priority and should all be implemented.

#### 1. `scripts/run_workflow_gate.ps1`
Implement all of the following:

- Keep `OutPath` stable in both baseline and candidate modes.
- Add a `-UseRelease` (or equivalent) option and default it on for gate usage.
- In baseline mode, run the workflow binary and write the fresh result to `OutPath`, then copy it to `BaselinePath`.
- Do **not** change `OutPath` to the baseline file path.
- Assert that the output summary exists after the run.
- Preserve/verify the workflow benchmark pack usage.
- If easy and low-risk, surface pack identity in the summary or comparison path.

#### 2. `scripts/run_cpu_perf_gate.ps1`
Implement all of the following:

- Always pass an explicit workflow summary path into `run_workflow_gate.ps1`.
- Read back the same summary path you requested.
- Stop assuming baseline mode also writes the candidate summary path unless explicitly requested.
- Expand the Criterion portion from one case to a deterministic small matrix:
  - keep the current small SMA CPU case
  - add a representative large ALMA CPU case
- Persist boundary/build-profile metadata for the workflow section and each Criterion case.
- Keep the dynamic CPU ratio block, but label it explicitly with timing boundary and build profile.
- In candidate mode, compare each named Criterion case against the matching named baseline case.
- Add baseline compatibility validation before pass/fail comparisons.

#### 3. `scripts/run_gpu_perf_gate.ps1`
Implement all of the following:

- Remove any “latest estimate file” or glob fallback.
- Resolve GPU Criterion estimate paths deterministically from an explicit registered case map.
- Expand the GPU Criterion portion into at least a deterministic two-case matrix:
  - keep the current small SMA GPU case
  - add a representative large ALMA GPU case
- Keep contract-split cases available; if they are too expensive for per-PR, wire them for nightly or keep them as optional registered cases rather than deleting them.
- Run the dynamic runner / `cargo run` portion in **release** mode.
- Change candidate-mode GPU dynamic comparison from candidate `on_mean` vs candidate `off_mean` to **candidate ratio vs baseline ratio**.
- Keep an additional absolute baseline check for `off_mean_ms` / static-side latency so both absolute latency and overhead are protected.
- Persist boundary/build-profile metadata.
- Add baseline compatibility validation before pass/fail.
- Add GPU hardware compatibility checks for baseline comparison.
- If straightforward, add an instability warning/fail policy based on CI width, but treat it as a stability check, not a performance regression.

#### 4. `src-tauri/src/bin/ta_workflow_regression.rs`
Implement all of the following:

- Add the metadata envelope fields:
  - `schema_version`
  - `timing_boundary`
  - `timing_boundary_note`
  - `build_profile`
- Keep the existing total `runtime_ms`.
- Add stage breakdown for at least:
  - optimize total
  - validate total
  - meta-select
  - holdout
- Make sure `build_profile` differs between dev and release runs.
- Keep the output deterministic and machine-readable.

### Phase 1 — evidence schema and boundary labeling

Implement these next.

#### 5. `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
Implement all of the following:

- Add metadata envelope / build profile / requested backend.
- Preserve current fields where possible.
- Separate and persist:
  - total ratio vs optimization
  - main validation ratio vs optimization
  - cross-ticker ratio vs optimization
- Keep the total ratio for backward compatibility.
- Preserve route/fallback metadata.
- If low-risk, add normalized metrics such as:
  - `wall_ms_per_candidate`
  - `wall_ms_per_approved_candidate`
  - `main_validation_wall_ms_per_candidate`
  - `cross_ticker_wall_ms_per_dataset`

#### 6. `src-tauri/src/bin/ta_app_path_faceoff.rs`
Implement all of the following:

- Add metadata envelope:
  - `schema_version`
  - `timing_boundary = "app-path"`
  - `timing_boundary_note`
  - `build_profile`
  - `requested_backend`
- Extend summary medians to surface:
  - total wall
  - state lookup
  - harness wall
  - engine wall
  - engine runtime
- Add a measured route summary block across measured trials (backend_used counts, backend_route counts, fallback count).
- Keep existing GPU/hybrid subphase timing fields.
- Do **not** silently redefine the current app-path boundary.
- Optional but desirable if low-risk: add an explicit cold-vs-warm split as a **separate named boundary/mode**, while preserving the current warm same-thread boundary exactly.

#### 7. `src-tauri/src/bin/ta_profile_route_matrix.rs`
Implement all of the following:

- Add metadata envelope and build profile.
- Add `expected_backend_route` and `route_ok` per case.
- Add `--strict` mode that fails on route mismatches.
- Preserve existing runtime/wall and best-result fields.
- Optional but desirable if low-risk: add `--warmup-runs` and `--measured-runs`, plus min/median/max timing fields, while keeping current one-shot behavior as default.

#### 8. `scripts/profile_gpu_backtest_nsys.ps1`
Implement all of the following:

- Write a sidecar provenance JSON next to the report.
- Capture at least:
  - requested capture mode
  - actual capture mode used
  - report path
  - exit code
  - fallback reason if any
  - key filter/options used
- Use `try/finally`-style environment restoration so env changes are always restored.
- Keep Nsight diagnostic-only; do not make it a hard gate.

#### 9. `scripts/profile_gpu_backtest_ncu.ps1`
Implement all of the following:

- Add `-NcuExePath` support.
- Write a provenance sidecar JSON similar to the Nsight Systems script.
- Use `try/finally`-style env restoration.
- Keep it profiling-only.

#### 10. `scripts/run_vectorgrid_double_ma_faceoff.ps1`
Implement all of the following:

- Stop parsing human-readable Criterion terminal output for timing.
- Read Criterion `estimates.json` directly.
- Derive metadata from benchmark ID instead of hard-coding strategy/objective/backend values wherever possible.
- Add explicit `timing_boundary = "Criterion"` and `build_profile = "bench"` to its artifacts/summary.
- Preserve compatibility with existing benchmark IDs and expected paths.

#### 11. `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
Implement all of the following:

- Add `timing_boundary` and `build_profile` to all trial artifacts:
  - vectorgrid Criterion CLI evidence
  - vectorgrid app-path evidence
  - vectorbt Python workflow evidence
- Keep different boundaries distinct in the persisted summary.
- Split markdown/reporting by timing boundary.
- Use the **canonical median-time trial** for summary-row metadata, not the first grouped artifact.
- Use canonical trial selection for parity source too.
- Preserve app-path-specific stats (harness wall, state lookup, engine wall/runtime) in the summary.
- Add fields like `canonical_trial_path` and `trial_paths` if helpful.

### Phase 2 — representative matrix expansion and benchmark metadata

Implement these after Phase 0 and 1 are stable.

#### 12. `crates/app_core/benches/double_ma_opt.rs`
Implement all of the following:

- Add a small `emit_bench_meta(...)`-style helper.
- Emit metadata for gate-relevant groups/cases without changing benchmark IDs.
- Add optional route probe emission controlled by env (for example `VECTORBT_BENCH_EMIT_ROUTE`).
- Add optional exact-route assertion path controlled by env (for example `VECTORBT_REQUIRE_EXACT_GPU_ROUTE`).
- Apply route-probe/metadata patterns to at least:
  - GPU kernel bench group(s)
  - GPU hybrid kernel group(s)
  - contract-split group(s)
  - strategy profile group(s)
  - VDPS group(s)
- Keep existing benchmark names/paths intact unless absolutely necessary.
- **Important addendum requirement:** move env-var mutation out of measured `b.iter(...)` loops where the mode is fixed for the whole benchmark. Use an RAII-style env guard or equivalent.
- **Important addendum requirement:** add structured benchmark-shape / variant metadata for contract-split and env-shaped variants, including where relevant:
  - `heatmap_bins`
  - `finalist_recompute`
  - `product_reduced_rows`
  - `include_trade_metrics`
  - score/shortlist/hurdle mode metadata

#### 13. `crates/app_core/benches/double_ma_metrics.rs`
Implement all of the following:

- Add bench metadata banner.
- Add optional route probe banner.
- Emit explicit contract/objective metadata for the full-metrics surface.
- Keep existing benchmark IDs stable.

#### 14. `crates/app_core/benches/double_ma_followup_workflow.rs`
Implement all of the following:

- Add bench metadata banner.
- Reuse the already-executed route probe to emit route/build-profile metadata.
- Emit explicit cached vs cold/follow-up wording so the boundary is self-describing.
- Keep this as a follow-up workflow boundary, not an optimize hot-path claim.
- If promoted to nightly, add a cached-vs-cold speedup metric if easy.

#### 15. `crates/app_core/benches/dynamic_strategy_cpu.rs`
Implement all of the following:

- Add metadata banners for the key groups:
  - eval
  - staging
  - full_run
- Add an optional quick-mode env for shorter local reruns while keeping default gate numbers unchanged.
- Keep benchmark IDs stable.

#### 16. `crates/app_core/benches/gpu_native_expansion_opt.rs`
Implement all of the following:

- Keep existing exact GPU route assertions.
- Add metadata banner(s).
- Add lightweight shape/count notes only; do not over-refactor this file.

#### 17. `crates/app_core/benches/double_ma_validation_workflow.rs`
Implement all of the following:

- Add bench metadata banner.
- Add optional strict exact-route env behavior for sparse GPU workflow.
- Preserve existing validation behavior.

#### 18. `crates/app_core/benches/composite_validation_workflow.rs`
Implement all of the following:

- Keep existing exact GPU route assertions.
- Add metadata banner.
- Make route prints boundary-explicit.

#### 19. `crates/app_core/benches/e2e_workflow_ml.rs`
Implement all of the following:

- Add metadata banner.
- Make train/holdout asset counts configurable via env.
- Add optional stage diagnostics.
- Keep default sizes small enough for reasonable benchmark use.
- Do not turn this into a product redesign.

### Phase 3 — optional safe hardening from the addendum

Implement these if they are low-risk and repo-local. If one is too invasive for this pass, leave a clean TODO and note it in the final summary.

#### 20. Baseline compatibility and environment fingerprinting
If low-risk, add:

- `case_registry_version`
- git/toolchain/hardware fingerprints
- GPU hardware compatibility checks
- baseline mismatch warnings/errors that fail fast on incompatible comparisons

#### 21. Route-matrix repeat/median support
If low-risk, add repeat support to `ta_profile_route_matrix.rs` while preserving one-shot default behavior.

#### 22. Stage-specific workflow budgets
After `ta_workflow_regression.rs` exposes stage timing, optionally add candidate-vs-baseline stage budgets in the workflow gate path while preserving the existing total-runtime gate.

#### 23. Normalized throughput metrics
If low-risk, add normalized metrics to route/app-path/validation-cost outputs.

#### 24. App-path warm/cold split
If low-risk, add an explicit warm-vs-cold mode split to `ta_app_path_faceoff.rs`, but do **not** silently redefine the current boundary.

#### 25. Case manifest / registry
If low-risk and actually helpful, add a manifest such as `benchmarks/perf_case_registry.json` and wire gates to it. If that is too much churn for one pass, keep deterministic per-script case maps and leave a clear TODO.

## Required testing and validation work

Do not only edit code. Validate what you can.

### A. Compile / test expectations

Run as much of this as the environment supports:

- `cargo check --all-targets`
- relevant `cargo test` invocations for touched crates/binaries
- `cargo bench --no-run` or equivalent compile-only validation for touched bench files where practical
- PowerShell syntax validation / smoke runs where practical

If the environment does not support GPU/CUDA/Nsight execution, still:
- compile the changed Rust code
- syntax-check the PowerShell scripts as much as practical
- leave exact commands for GPU/profiler validation in your final summary

### B. Script smoke / fixture coverage

Add or improve testability for these logic surfaces where practical:

- clean-tree baseline bootstrap for `run_cpu_perf_gate.ps1`
- exact-estimates-path resolution for `run_gpu_perf_gate.ps1`
- candidate-vs-baseline dynamic ratio comparison for `run_gpu_perf_gate.ps1`
- canonical-trial selection for `run_vectorbt_vs_vectorgrid_faceoff.ps1`
- capture-provenance sidecar behavior for Nsight scripts

If the repo has no PowerShell test harness, extract pure helper logic where possible and add lightweight smoke fixtures or a minimal test script structure.

### C. Rust helper tests

Add helper/unit tests where practical for:

- app-path route-summary / envelope helpers in `ta_app_path_faceoff.rs`
- stage breakdown accumulation in `ta_workflow_regression.rs`
- total/main/cross ratio calculations in `ta_double_ma_validation_cost.rs`
- benchmark ID parsing / structured-estimate reading logic if extracted cleanly

### D. Validation commands to run or leave for the final report

Run what you can. Leave the rest exactly as commands to run on the target machine.

Workflow gate examples:

```powershell
powershell -File scripts/run_workflow_gate.ps1 -Mode baseline
powershell -File scripts/run_workflow_gate.ps1 -Mode candidate
```

CPU gate examples:

```powershell
powershell -File scripts/run_cpu_perf_gate.ps1 -Mode baseline
powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate
```

GPU gate examples:

```powershell
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode baseline
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

Route matrix example:

```bash
cargo run --release -p ta_desktop_demo_app --bin ta_profile_route_matrix -- \
  --csv data/benchmarks/AAPL.csv \
  --timeframe 15m \
  --out target/profile_route_matrix_aapl_15m.json \
  --strict
```

App-path faceoff example:

```bash
cargo run --release -p ta_desktop_demo_app --bin ta_app_path_faceoff -- \
  --csv data/benchmarks/AAPL.csv \
  --output-json target/app_path_faceoff.json \
  --backend gpu-kernel \
  --fast-ma alma \
  --slow-ma alma \
  --fast-period 5:200:1 \
  --slow-period 10:400:1 \
  --warmup-trials 1 \
  --measured-trials 5
```

Validation-cost example:

```bash
cargo run --release -p ta_desktop_demo_app --bin ta_double_ma_validation_cost -- \
  --csv data/benchmarks/AAPL.csv \
  --out-json target/double_ma_validation_cost.json
```

Nsight scripts: run only on a suitable machine and report exact command/status.

## Acceptance criteria

Your work is not complete until these are true (or explicitly blocked by environment limitations):

1. No gate reads a file path it did not explicitly request.
2. No gate uses “latest matching artifact” or equivalent fallback for pass/fail logic.
3. CPU and GPU gate comparisons validate baseline compatibility before numeric comparisons.
4. Workflow gate baseline mode does not silently break candidate-summary readback.
5. GPU dynamic gate compares candidate ratio to baseline ratio, not candidate `on` vs candidate `off`.
6. All persisted summaries identify timing boundary and build profile.
7. `ta_workflow_regression.rs` emits stage breakdown plus total runtime.
8. `ta_app_path_faceoff.rs` preserves the current app-path medians, adds route summary, and labels the artifact boundary/build profile explicitly.
9. `ta_double_ma_validation_cost.rs` separates total/main/cross ratios.
10. `ta_profile_route_matrix.rs` supports strict route checks.
11. `run_vectorgrid_double_ma_faceoff.ps1` uses structured Criterion JSON rather than CLI text parsing.
12. `run_vectorbt_vs_vectorgrid_faceoff.ps1` uses canonical trials for summary rows and parity source.
13. `double_ma_opt.rs` no longer mutates env vars inside measured loops where the mode is fixed.
14. Existing features and execution paths are preserved.
15. Benchmark IDs remain stable unless absolutely necessary.
16. Any optional additions you skipped are clearly called out with reasons.

## What to report at the end

Your final response must include:

1. A concise summary of what you changed, grouped by phase.
2. The exact list of modified files.
3. Any new files added.
4. The exact commands you ran.
5. Which commands could not be run because of environment/hardware limits.
6. Whether baselines now need regeneration, and which baseline files are affected.
7. Any optional P3 items you implemented.
8. Any optional P3 items you intentionally deferred, with reasons.

## Important reminders

- This is an implementation pass, not another design review.
- Do not stop at writing a plan.
- Make the code changes.
- Keep changes focused, additive, and benchmark-safe.
- Preserve feature behavior.
- Improve evidence quality, timing-boundary fidelity, and regression safety.
