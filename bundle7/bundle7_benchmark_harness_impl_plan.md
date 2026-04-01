
# Bundle 7 implementation plan: benchmark harnesses, perf gates, and timing boundaries

## Prompt alignment and scope

This document follows **Bundle 7: “Benchmark harnesses, perf gates, and timing boundaries”** from `gpt54_feature_review_map.md`.

### Ask-for coverage

This plan explicitly covers the bundle-7 asks:

- perf-evidence quality review
- timing-boundary review
- perf-gate relevance review
- stale-benchmark-target review

It also applies the two most relevant optional add-ons from the map:

- route/fallback audit
- benchmark-fidelity audit

### Surface under review

The surface under review is the repo’s performance evidence layer: PowerShell gate scripts, Criterion benches, warm app-path and route-matrix binaries, validation-cost and workflow-regression harnesses, and Nsight profiler entrypoints. The goal is to make the evidence trustworthy, boundary-correct, route-aware, and directly useful for regression gating without drifting into new feature work.

### Files covered

#### Scripts

- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/profile_gpu_backtest_ncu.ps1`
- `scripts/profile_gpu_backtest_nsys.ps1`
- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
- `scripts/run_vectorgrid_double_ma_faceoff.ps1`

#### Criterion benches

- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/benches/double_ma_followup_workflow.rs`
- `crates/app_core/benches/double_ma_metrics.rs`
- `crates/app_core/benches/dynamic_strategy_cpu.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`

#### Binaries / harnesses

- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`

### Extra context

The current repo already has several strong evidence surfaces:

- route-aware validation benches
- app-path timing with engine subphase timing
- route matrix artifacts
- validation-cost artifacts
- workflow regression artifacts
- Nsight capture entrypoints
- dynamic-strategy ratio checks

The main problem is **not lack of measurement primitives**. The problem is **loss of fidelity and correctness at the gate / wrapper / summary layer**, plus some stale or too-narrow gate targets.

---

## Decision rules and invariants for this implementation

These rules should remain true after the edits below.

1. **Do not mix timing boundaries silently.**
   - Criterion microbench evidence stays Criterion.
   - app-path evidence stays app-path.
   - validation-cost evidence stays validation-cost.
   - workflow-regression evidence stays workflow total wall.
   - Nsight evidence stays diagnostic/profiling-only unless explicitly promoted.

2. **Every persisted performance artifact should say what it measured.**
   Add:
   - `schema_version`
   - `timing_boundary`
   - `timing_boundary_note`
   - `build_profile`
   - `requested_backend` where relevant
   - `backend_used`
   - `backend_route`
   - `backend_fallback_reason`
   - `generated_at_utc`

3. **Every gate must use deterministic artifact selection.**
   No “latest matching estimate” fallback for pass/fail logic.

4. **Every gate comparison must compare candidate to baseline, not candidate sub-metric to another candidate sub-metric.**

5. **Prefer smaller changes before larger refactors.**
   Fix the correctness bugs first. Unify evidence schema second. Expand matrix coverage third.

6. **Do not promote Nsight into a hard gate.**
   Nsight remains diagnostic evidence, but its provenance must be recorded.

---

## File coverage matrix

| File | Current role | Priority | Main change |
|---|---|---:|---|
| `run_cpu_perf_gate.ps1` | CPU perf gate | P0 | fix workflow bootstrap bug, release-aware workflow gate, multi-case matrix |
| `run_gpu_perf_gate.ps1` | GPU perf gate | P0 | deterministic estimates, baseline-ratio comparison, release runner |
| `run_workflow_gate.ps1` | workflow gate wrapper | P0 | stop overwriting output path in baseline mode, add `--release` option |
| `profile_gpu_backtest_nsys.ps1` | Nsight Systems entrypoint | P1 | write sidecar provenance JSON |
| `profile_gpu_backtest_ncu.ps1` | Nsight Compute entrypoint | P1 | add `-NcuExePath`, write sidecar provenance JSON |
| `run_vectorbt_vs_vectorgrid_faceoff.ps1` | faceoff orchestrator | P1 | boundary-aware summary rows and canonical parity source |
| `run_vectorgrid_double_ma_faceoff.ps1` | vectorgrid-only faceoff | P1 | stop parsing human Criterion text, read structured estimates JSON |
| `double_ma_opt.rs` | primary Double-MA Criterion bench | P1/P2 | emit route/build-profile metadata for gate-relevant cases |
| `double_ma_metrics.rs` | full-metrics GPU kernel Criterion bench | P2 | emit route/build-profile/contract metadata |
| `double_ma_followup_workflow.rs` | follow-up workflow Criterion bench | P2 | emit route/build-profile/cache-mode metadata |
| `dynamic_strategy_cpu.rs` | dynamic-strategy CPU Criterion bench | P1 | explicit boundary/build-profile metadata, quick-mode env |
| `gpu_native_expansion_opt.rs` | strategy-local GPU-native Criterion bench | P2 | metadata banner only; existing route asserts are good |
| `double_ma_validation_workflow.rs` | Double-MA validation Criterion bench | P2 | strict-route env and metadata banner |
| `composite_validation_workflow.rs` | composite validation Criterion bench | P2 | metadata banner; keep existing GPU route asserts |
| `e2e_workflow_ml.rs` | end-to-end ML workflow Criterion bench | P2/P3 | env-sized workloads + stage diagnostics |
| `ta_profile_route_matrix.rs` | route matrix binary | P1 | schema/build-profile/timing-boundary fields; optional strict mode |
| `ta_app_path_faceoff.rs` | app-path timing binary | P1 | richer summary stats + schema/build-profile/route summary |
| `ta_double_ma_validation_cost.rs` | validation-cost binary | P1 | separate total/main/cross ratios + schema/build-profile |
| `ta_workflow_regression.rs` | workflow regression binary | P0 | stage breakdown + schema/build-profile |

---

## 1. Surface understanding

This bundle is already measuring the right *kinds* of things.

- `double_ma_opt.rs` is the central Criterion evidence source for CPU, GPU sweep, GPU kernel, GPU hybrid kernel, contract split, dynamic preflight overhead, dynamic program execution, and VDPS.
- `dynamic_strategy_cpu.rs` provides a CPU-only dynamic-strategy overhead surface and the CPU gate already consumes its `full_run/2k/composite_mo` pair.
- `double_ma_validation_workflow.rs`, `composite_validation_workflow.rs`, `ta_profile_route_matrix.rs`, `ta_app_path_faceoff.rs`, and `ta_double_ma_validation_cost.rs` already preserve much richer route and timing information than the gate summaries expose today.
- `ta_workflow_regression.rs` and `e2e_workflow_ml.rs` provide real workflow-level timing, but they are not yet integrated in a way that makes their timing boundary explicit.

The repository therefore already has a good measurement *base*. The missing work is:

- making gate selection deterministic
- labeling timing boundaries clearly
- ensuring build profiles match the claim being made
- preserving route/fallback information into the persisted summary
- broadening gate coverage slightly so that it tracks real production hotspots rather than only a single small case

---

## 2. Findings

### Confirmed issues

#### Finding 1

- Severity: critical
- Type: correctness / benchmark-fidelity
- Location: `run_cpu_perf_gate.ps1`, `run_workflow_gate.ps1`
- Timing boundary affected: workflow-regression
- Why it matters:
  In baseline mode, `run_cpu_perf_gate.ps1` can invoke `run_workflow_gate.ps1` in baseline mode. `run_workflow_gate.ps1` then rewrites `OutPath` to the baseline file path, but `run_cpu_perf_gate.ps1` later unconditionally reads `target/workflow_regression_summary.json`. On a clean tree this can fail; on a dirty tree it can silently reuse stale data.
- Recommended change:
  `run_workflow_gate.ps1` should always write the fresh summary to `OutPath`, and baseline mode should **copy** that summary to `BaselinePath` after the run. `run_cpu_perf_gate.ps1` should always pass an explicit workflow summary path and read that same path.

#### Finding 2

- Severity: high
- Type: benchmark-fidelity
- Location: `run_workflow_gate.ps1`, `ta_workflow_regression.rs`
- Timing boundary affected: workflow-regression
- Why it matters:
  The wrapper currently uses `cargo run`, which defaults to a non-bench dev-oriented build unless `--release` is supplied. That makes the workflow timing boundary materially different from Criterion’s bench profile.
- Recommended change:
  Add a `-UseRelease` / `-BuildProfile` switch to `run_workflow_gate.ps1`, default it to release in gate use, and persist the effective build profile into `ta_workflow_regression.rs` output.

#### Finding 3

- Severity: high
- Type: benchmark-fidelity / correctness
- Location: `run_gpu_perf_gate.ps1`
- Timing boundary affected: runner steady-state wall
- Why it matters:
  The GPU dynamic-strategy check stores `ratio_on_over_off`, but candidate-mode pass/fail compares candidate `on_mean_ms` to candidate `off_mean_ms` instead of candidate ratio to baseline ratio. That allows true regressions in incremental dynamic overhead to escape the gate.
- Recommended change:
  Compare `candidate.ratio_on_over_off` to `baseline.ratio_on_over_off`, and optionally keep a second absolute `off_mean_ms` check so both base latency and incremental overhead are protected.

#### Finding 4

- Severity: high
- Type: benchmark-fidelity
- Location: `run_gpu_perf_gate.ps1`
- Timing boundary affected: runner steady-state wall
- Why it matters:
  The dynamic runner is launched with `cargo run -q`, which means its numbers are coming from a dev-oriented profile unless `--release` is added. The script currently presents the result beside Criterion bench results without clearly labeling the build-profile difference.
- Recommended change:
  Run `profile_gpu_dynamic_runner` with `cargo run --release`, and persist `timing_boundary = "runner_steady_state_wall"` and `build_profile = "release"` in the summary block.

#### Finding 5

- Severity: high
- Type: benchmark-fidelity
- Location: `run_gpu_perf_gate.ps1`
- Timing boundary affected: Criterion
- Why it matters:
  The script falls back to the “latest GPU-kernel estimate file” when the exact glob does not match. That can turn a pass/fail check into a stale-artifact check.
- Recommended change:
  Remove the fallback entirely. Replace it with a deterministic case registry or an explicit path parameter.

#### Finding 6

- Severity: high
- Type: benchmark-fidelity
- Location: `run_vectorbt_vs_vectorgrid_faceoff.ps1`, `ta_app_path_faceoff.rs`
- Timing boundary affected: app-path + external CLI wall + external Python wall
- Why it matters:
  The faceoff summary collapses multiple timing boundaries into one timing table:
  - vectorgrid app-path same-thread wall
  - vectorgrid CLI process wall
  - vectorbt Python process wall
  They are useful, but not directly comparable unless they are labeled and surfaced separately.
- Recommended change:
  Add `timing_boundary`, `build_profile`, and boundary-specific timing columns to every faceoff row. Split the markdown summary by timing boundary.

#### Finding 7

- Severity: high
- Type: benchmark-fidelity / maintainability
- Location: `run_vectorgrid_double_ma_faceoff.ps1`
- Timing boundary affected: Criterion
- Why it matters:
  The script parses the human-readable Criterion `time: [...]` line and hard-codes strategy/backend/objective metadata. That is fragile and loses richer structured evidence already present in `estimates.json`.
- Recommended change:
  Read `estimates.json` directly. Parameterize objective/backend metadata or derive them from the benchmark ID instead of hard-coding.

#### Finding 8

- Severity: high
- Type: route / parity / benchmark-fidelity
- Location: `run_cpu_perf_gate.ps1`, `run_gpu_perf_gate.ps1`, `ta_profile_route_matrix.rs`, `ta_app_path_faceoff.rs`, `ta_double_ma_validation_cost.rs`, validation benches
- Timing boundary affected: Criterion / app-path / validation-cost / route-matrix
- Why it matters:
  The rich route/fallback evidence exists in the deeper harnesses, but the gate summaries mostly drop it. This makes route regressions harder to distinguish from generic runtime regressions.
- Recommended change:
  Preserve `requested_backend`, `backend_used`, `backend_route`, and `backend_fallback_reason` into every persisted summary and add strict-route options where exact routes are expected.

#### Finding 9

- Severity: medium
- Type: benchmark-fidelity / profiling-only
- Location: `profile_gpu_backtest_nsys.ps1`, `profile_gpu_backtest_ncu.ps1`
- Timing boundary affected: profiling-only
- Why it matters:
  The profiling scripts can produce usable reports, but they do not persist a small machine-readable provenance artifact capturing request vs actual capture mode, report path, target, dynamic mode, graphs mode, and exit code.
- Recommended change:
  Write a `.meta.json` sidecar for each run.

#### Finding 10

- Severity: medium
- Type: perf-gate relevance
- Location: `run_cpu_perf_gate.ps1`, `run_gpu_perf_gate.ps1`
- Timing boundary affected: mixed gate summary
- Why it matters:
  The current gates cover only one small CPU Criterion case, one small GPU Criterion case, one CPU dynamic-strategy ratio surface, one GPU dynamic runner ratio surface, and one workflow binary. This is better than nothing, but it undershoots the real hot path mix in the repo.
- Recommended change:
  Add one larger ALMA case to CPU and GPU coverage, and add nightly coverage for app-path, validation-cost, and e2e ML.

#### Finding 11

- Severity: medium
- Type: testing / maintainability
- Location: all PowerShell gate scripts
- Timing boundary affected: unknown
- Why it matters:
  The scripts embed parsing, path selection, and summary shaping inline, which makes them hard to test. The current bugs would have been easy to catch with fixture-driven smoke tests.
- Recommended change:
  Extract pure helper functions for path resolution and summary writing, then add fixture-based script smoke tests (or Pester if available in the environment).

#### Finding 12

- Severity: medium
- Type: benchmark-fidelity
- Location: most Criterion bench files
- Timing boundary affected: Criterion
- Why it matters:
  Several benches already print useful route or profile lines, but the pattern is inconsistent. A small standardized “bench_meta” print would make downstream log parsing and future gate promotion much easier.
- Recommended change:
  Add a small shared file-local helper in each bench file that prints:
  `benchmark_id`, `timing_boundary`, `build_profile`, `requested_backend`, and route/fallback fields where available.

### Likely issues

#### Finding 13

- Severity: medium
- Type: benchmark-fidelity
- Location: `run_vectorbt_vs_vectorgrid_faceoff.ps1`
- Timing boundary affected: unknown
- Why it matters:
  Parity currently uses the first measured artifact for each tool/backend. If any small non-determinism exists, parity can flicker even when the median result is stable.
- Recommended change:
  Select a canonical trial per group (recommended: median wall-time trial) and use that for parity comparison.

#### Finding 14

- Severity: low
- Type: testing / maintainability
- Location: `ta_app_path_faceoff.rs`, `ta_workflow_regression.rs`, `ta_double_ma_validation_cost.rs`
- Timing boundary affected: app-path / workflow-regression / validation-cost
- Why it matters:
  These binaries mostly rely on integration execution for coverage. Small summary helpers and stage-timing reducers would benefit from targeted tests.
- Recommended change:
  Add unit tests for summary reducers and stage accumulators where practical.

### Open questions

#### Finding 15

- Severity: low
- Type: open question
- Location: `double_ma_opt.rs`, `dynamic_strategy_cpu.rs`, `e2e_workflow_ml.rs`
- Timing boundary affected: Criterion
- Why it matters:
  The current sample sizes and measurement durations may or may not be ideal once larger canonical gate cases are added.
- Recommended change:
  Start by preserving current sample sizes for continuity, then tune after two or three baseline regeneration cycles.

---

## 3. Safe optimization opportunities

1. Fix the CPU workflow baseline bootstrap path bug.
2. Stop mixing workflow dev-profile timing with Criterion without labeling or release-mode execution.
3. Remove stale estimate fallback from the GPU gate.
4. Compare GPU dynamic ratio candidate vs baseline ratio.
5. Run the GPU dynamic runner in release mode.
6. Add schema/build-profile/timing-boundary metadata to every persisted summary.
7. Keep route/fallback metadata visible in the summary layer.
8. Split faceoff summaries by timing boundary.
9. Add sidecar provenance JSON to Nsight scripts.
10. Add one representative larger ALMA case to CPU and GPU gate coverage.
11. Add env-controlled metadata banners to benches so route/fallback evidence is available before adding new instrumentation.
12. Use existing route-matrix / validation-cost / app-path harnesses before creating any new measurement tool.

---

## 4. Higher-risk optimization opportunities

These are worth doing, but not before the P0/P1 fixes.

1. **Shared PowerShell perf helper module**
   - Move JSON writing, estimate resolution, and summary shaping into shared functions.
   - Benefit: less duplication.
   - Risk: touches multiple scripts at once.

2. **Shared benchmark case registry**
   - Introduce a small registry of canonical benchmark IDs and exact estimate paths.
   - Benefit: eliminates path drift and narrows stale-target risk.
   - Risk: needs maintenance when benchmark IDs legitimately change.

3. **Unified performance artifact schema across binaries**
   - Make `ta_app_path_faceoff`, `ta_profile_route_matrix`, `ta_double_ma_validation_cost`, and `ta_workflow_regression` share a common metadata envelope.
   - Benefit: easier tooling and dashboarding.
   - Risk: schema churn across multiple consumers.

4. **Representative matrix gate tiers**
   - Per-PR tier: small CPU/GPU + workflow + dynamic ratio.
   - nightly tier: large ALMA, contract split, validation cost, app-path, route matrix, e2e ML.
   - Benefit: better production alignment.
   - Risk: longer CI time.

5. **cargo-criterion JSON migration**
   - Long-term, switch some scripts to machine-readable JSON output directly from `cargo-criterion`.
   - Benefit: more explicit machine-facing format.
   - Risk: adds tool dependency and slightly changes workflow.

---

## 5. Benchmark and perf-gate review

### Boundary map

| Boundary | Source of truth | Current state | Recommended role |
|---|---|---|---|
| Criterion microbench | `double_ma_opt.rs`, `dynamic_strategy_cpu.rs`, `gpu_native_expansion_opt.rs`, `double_ma_validation_workflow.rs`, `composite_validation_workflow.rs`, `e2e_workflow_ml.rs`, `double_ma_metrics.rs`, `double_ma_followup_workflow.rs` | strong, but metadata inconsistent | primary gate evidence for engine-level changes |
| warm app-path runtime | `ta_app_path_faceoff.rs` | strong subphase timing, but not surfaced cleanly by faceoff wrapper | nightly / manual hotspot validation |
| validation-cost harness | `ta_double_ma_validation_cost.rs` | strong separation of main vs cross-ticker wall, but ratios can be richer | nightly / regression budget input |
| route matrix | `ta_profile_route_matrix.rs` | strong route/fallback surface | nightly or strict pre-release route audit |
| workflow regression | `ta_workflow_regression.rs` | useful, but build-profile ambiguity and stage opacity | per-PR regression gate once fixed |
| Nsight diagnostic evidence | `profile_gpu_backtest_nsys.ps1`, `profile_gpu_backtest_ncu.ps1` | useful but provenance-light | diagnostic only |

### Recommended gate tiers

#### Tier 1: per-PR gate

Keep this small and deterministic.

- CPU workflow regression (`ta_workflow_regression`, release)
- CPU small SMA Criterion case (continuity)
- CPU large ALMA Criterion case (representative)
- CPU dynamic-strategy ratio (`dynamic_strategy_cpu/full_run/2k/composite_mo`)
- GPU small SMA Criterion case (continuity)
- GPU dynamic-strategy steady-state ratio (release runner)

#### Tier 2: nightly / scheduled pack

- GPU large ALMA kernel case
- GPU contract-split `search_only` vs `end_to_end_product`
- app-path faceoff (vectorgrid app-path only)
- validation-cost harness (one cheap and one full-stack case)
- route matrix strict mode
- e2e ML workflow Criterion bench
- selected strategy-local GPU-native expansion cases

### Stale-target review

The current gates are not wrong because they are small; they are wrong because they are **too narrow and too loosely bound to explicit case identities**.

The fix is **not** “add many more samples everywhere”. The fix is:

- make the current cases deterministic
- preserve their boundary and route metadata
- add a small second representative case where the current case is not representative
- keep broader packs for nightly

---

## 6. Test review

### Script-level tests to add

These are the minimum high-value tests.

1. **CPU workflow baseline bootstrap test**
   - delete temp baseline
   - run `run_cpu_perf_gate.ps1 -Mode baseline` against temp output paths
   - assert both the workflow summary path and workflow baseline path were written
   - assert the summary readback path is the one just generated

2. **GPU estimate resolution test**
   - create two fake estimate files:
     - one matching the exact canonical path
     - one newer unrelated GPU estimate
   - assert the gate uses the exact canonical path and not the newer unrelated file

3. **GPU dynamic ratio baseline comparison test**
   - fake baseline with ratio 1.05
   - fake candidate with ratio 1.30
   - fake candidate off/on absolute timings that would otherwise pass
   - assert the ratio gate fails

4. **Faceoff canonical trial selection test**
   - feed three trial rows with wall times 100 / 110 / 1000
   - assert canonical trial selection uses the median wall-time trial rather than the first

5. **Faceoff boundary summary test**
   - verify that app-path rows contain `app_path_harness_median_ms`, `app_path_engine_wall_median_ms`, and `app_path_engine_runtime_median_ms`
   - verify vectorbt rows do not pretend to have engine-runtime columns

6. **Nsight sidecar test**
   - simulate normal NVTX capture
   - simulate whole-process fallback
   - assert requested and effective capture mode differ only in the fallback case

### Rust unit tests to add

1. `ta_app_path_faceoff.rs`
   - extend summary tests so total wall / engine wall / engine runtime medians are checked together

2. `ta_workflow_regression.rs`
   - test stage timing accumulation helper if factored out

3. `ta_double_ma_validation_cost.rs`
   - test ratio helper for total/main/cross cost bands

4. `run_vectorgrid_double_ma_faceoff.ps1`
   - if helper logic is factored into small pure functions, test benchmark ID parsing and path selection

### Benchmark-log tests / smoke assertions

Add simple log assertions for:
- `build_profile=...`
- `timing_boundary=...`
- `backend_route=...`
- `fallback=...`

This is especially helpful once bench metadata banners are added.

---

## 7. Suggested improvement plan

### Phase 0 — correctness and gate-trust fixes

Implement these first.

1. `run_workflow_gate.ps1`
   - stop rewriting `OutPath` in baseline mode
   - add release-mode execution
   - copy fresh summary to baseline path in baseline mode

2. `run_cpu_perf_gate.ps1`
   - always pass explicit workflow summary path
   - read the same path it asked the workflow gate to write

3. `ta_workflow_regression.rs`
   - add `schema_version`, `timing_boundary`, `build_profile`, stage timings

4. `run_gpu_perf_gate.ps1`
   - remove latest-estimate fallback
   - run dynamic runner with `--release`
   - compare candidate ratio to baseline ratio

### Phase 1 — evidence schema and boundary labeling

1. add metadata envelope to:
   - `ta_profile_route_matrix.rs`
   - `ta_app_path_faceoff.rs`
   - `ta_double_ma_validation_cost.rs`
   - `ta_workflow_regression.rs`

2. add sidecar JSON to:
   - `profile_gpu_backtest_nsys.ps1`
   - `profile_gpu_backtest_ncu.ps1`

3. add boundary-aware rows to:
   - `run_vectorbt_vs_vectorgrid_faceoff.ps1`
   - `run_vectorgrid_double_ma_faceoff.ps1`

### Phase 2 — representative matrix expansion

1. expand CPU gate to:
   - small SMA Criterion
   - large ALMA Criterion
   - dynamic CPU full-run ratio
   - workflow regression

2. expand GPU gate to:
   - small SMA Criterion
   - one representative ALMA case
   - dynamic steady-state ratio
   - optional contract split nightly case

3. add bench metadata banners to Criterion files

### Phase 3 — nightly / extended pack

Promote the following to scheduled runs, not per-PR:
- route matrix strict mode
- app-path faceoff
- validation-cost harness
- e2e ML workflow
- GPU-native expansion strategy packs

### Phase 4 — optional unification

Only after the above is stable:
- shared PowerShell helper library
- shared canonical benchmark case registry
- unified artifact envelope across all binaries

---

## 8. File-by-file implementation details and code snippets

## 8.1 Shared schema and helper snippets

### Shared persisted artifact envelope

Use this pattern everywhere a summary or profiling artifact is written.

```json
{
  "schema_version": 1,
  "timing_boundary": "Criterion | app-path | validation-cost | workflow-regression | route-matrix | profiling-only",
  "timing_boundary_note": "Human-readable note about what is and is not included.",
  "build_profile": "bench | release | dev | python",
  "generated_at_utc": "2026-03-28T00:00:00Z"
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

### Shared Rust bench metadata banner

Use a tiny helper like this in Criterion bench files.

```rust
const BENCH_BUILD_PROFILE: &str = "bench";

fn emit_bench_meta(
    benchmark_id: &str,
    timing_boundary: &str,
    requested_backend: &str,
    backend_used: Option<&str>,
    backend_route: Option<&str>,
    backend_fallback_reason: Option<&str>,
) {
    println!(
        "bench_meta benchmark_id={} timing_boundary={} build_profile={} requested_backend={} backend_used={} backend_route={} fallback={}",
        benchmark_id,
        timing_boundary,
        BENCH_BUILD_PROFILE,
        requested_backend,
        backend_used.unwrap_or("<none>"),
        backend_route.unwrap_or("<none>"),
        backend_fallback_reason.unwrap_or("<none>")
    );
}
```

### Shared binary build-profile helper

Use this in binaries that may run under `cargo run` or `cargo run --release`.

```rust
fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "dev"
    } else {
        "release"
    }
}
```

### Shared PowerShell JSON writer

Use this in every script that persists JSON.

```powershell
function Write-Utf8Json {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][object]$InputObject,
    [int]$Depth = 8
  )

  $dir = Split-Path -Parent $Path
  if ($dir) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }

  $InputObject | ConvertTo-Json -Depth $Depth | Set-Content -Encoding UTF8 $Path
}
```

---

## 8.2 Script edits

### `scripts/run_workflow_gate.ps1`

### Required edits

1. Keep `OutPath` stable in both baseline and candidate modes.
2. Add a release-mode option and default it on for gate use.
3. In baseline mode, copy the fresh output summary to the baseline file instead of replacing the output path.
4. After execution, assert that the output summary exists.

### Suggested patch

```powershell
param(
  [ValidateSet("baseline", "candidate")]
  [string]$Mode = "candidate",
  [string]$PackPath = "scripts/workflow_benchmark_pack.json",
  [string]$BaselinePath = "scripts/baselines/workflow_regression_baseline.json",
  [string]$OutPath = "target/workflow_regression_summary.json",
  [switch]$UseRelease = $true
)

$ErrorActionPreference = "Stop"

if (!(Test-Path $PackPath)) {
  throw "Missing benchmark pack: $PackPath"
}

$pack = Get-Content -Raw $PackPath | ConvertFrom-Json
$candlesLen = [int]$pack.candles_len
$trainAssets = [int]$pack.train_assets
$holdoutAssets = [int]$pack.holdout_assets

$outDir = Split-Path -Parent $OutPath
if ($outDir) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$cmd = @("run")
if ($UseRelease) {
  $cmd += "--release"
}
$cmd += @(
  "-p", "ta_desktop_demo_app",
  "--bin", "ta_workflow_regression",
  "--",
  "--out", $OutPath,
  "--candles-len", "$candlesLen",
  "--train-assets", "$trainAssets",
  "--holdout-assets", "$holdoutAssets"
)

if ($Mode -eq "candidate") {
  if (!(Test-Path $BaselinePath)) {
    throw "Missing baseline summary: $BaselinePath. Run with -Mode baseline first."
  }
  $cmd += @("--baseline", $BaselinePath)
}

Write-Host "Running workflow gate ($Mode, release=$UseRelease)..."
cargo @cmd
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

if (!(Test-Path $OutPath)) {
  throw "Workflow gate did not write expected output summary: $OutPath"
}

if ($Mode -eq "baseline") {
  $baselineDir = Split-Path -Parent $BaselinePath
  if ($baselineDir) {
    New-Item -ItemType Directory -Force -Path $baselineDir | Out-Null
  }
  Copy-Item -Force $OutPath $BaselinePath
  Write-Host "Workflow baseline written to $BaselinePath"
}

Write-Host "Summary written to $OutPath"
```

### Validation

- delete `target/workflow_regression_summary.json`
- delete `scripts/baselines/workflow_regression_baseline.json`
- run baseline mode
- verify both files are present and differ only by path, not content
- run candidate mode
- verify `target/workflow_regression_summary.json` is freshly rewritten

---

### `scripts/run_cpu_perf_gate.ps1`

### Required edits

1. Always pass an explicit workflow summary path into `run_workflow_gate.ps1`.
2. Stop assuming baseline workflow mode will also write the candidate summary path unless explicitly requested.
3. Promote the gate from a single Criterion case to a small deterministic case matrix.
4. Persist the timing boundary and build profile for both the workflow block and Criterion block.

### Suggested patch (workflow fix + case matrix)

```powershell
$workflowBaselinePath = "scripts/baselines/workflow_regression_baseline.json"
$workflowSummaryPath = "target/workflow_regression_summary.json"

$workflowMode = "candidate"
if (($Mode -eq "baseline") -and (!(Test-Path $workflowBaselinePath))) {
  $workflowMode = "baseline"
}

Write-Host "Running workflow regression gate ($workflowMode)..."
powershell -File "scripts/run_workflow_gate.ps1" `
  -Mode $workflowMode `
  -BaselinePath $workflowBaselinePath `
  -OutPath $workflowSummaryPath `
  -UseRelease:$true

if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

if (!(Test-Path $workflowSummaryPath)) {
  throw "Missing workflow summary: $workflowSummaryPath"
}

$cpuCriterionCases = @(
  @{
    name = "cpu_sma_small"
    bench_filter = "double_ma/cpu/sma_sma/200k/small"
    estimates_json = "target/criterion/double_ma_cpu_sma_sma_200k/small_1590pairs/new/estimates.json"
    regression_pct = 15.0
  },
  @{
    name = "cpu_alma_large"
    bench_filter = "double_ma/cpu/alma_alma/200k/large"
    estimates_json = "target/criterion/double_ma_cpu_alma_alma_200k/large_58300pairs/new/estimates.json"
    regression_pct = 15.0
  }
)

$criterionSummaries = @()
foreach ($case in $cpuCriterionCases) {
  Write-Host "Running criterion benchmark ($($case.bench_filter))..."
  cargo bench -p ta_app_core --bench double_ma_opt -- $case.bench_filter
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }

  if (!(Test-Path $case.estimates_json)) {
    throw "Missing criterion estimates file: $($case.estimates_json)"
  }

  $criterion = Get-Content -Raw $case.estimates_json | ConvertFrom-Json
  $criterionSummaries += [ordered]@{
    name = $case.name
    timing_boundary = "Criterion"
    build_profile = "bench"
    bench_filter = $case.bench_filter
    estimates_json = $case.estimates_json
    regression_pct = $case.regression_pct
    mean_ms = [math]::Round(([double]$criterion.mean.point_estimate) / 1000000.0, 6)
    mean_ci_lower_ms = [math]::Round(([double]$criterion.mean.confidence_interval.lower_bound) / 1000000.0, 6)
    mean_ci_upper_ms = [math]::Round(([double]$criterion.mean.confidence_interval.upper_bound) / 1000000.0, 6)
    median_ms = [math]::Round(([double]$criterion.median.point_estimate) / 1000000.0, 6)
  }
}

$workflow = Get-Content -Raw $workflowSummaryPath | ConvertFrom-Json

$summary = [ordered]@{
  schema_version = 1
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  mode = $Mode
  workflow = [ordered]@{
    timing_boundary = "workflow-regression"
    build_profile = $workflow.build_profile
    summary_json = $workflowSummaryPath
    runtime_ms = [double]$workflow.runtime_ms
    stage = $workflow.stage
  }
  criterion_cases = $criterionSummaries
}
```

### Candidate-mode comparison logic

```powershell
$baseline = Get-Content -Raw $BaselinePath | ConvertFrom-Json

$workflowPass = ([double]$summary.workflow.runtime_ms) -le (([double]$baseline.workflow.runtime_ms) * 1.5 + 1.0)
Write-PassFailLine "workflow_runtime_not_regressed_50pct" $workflowPass `
  "$($summary.workflow.runtime_ms) ms vs baseline $($baseline.workflow.runtime_ms) ms"

$criterionPass = $true
foreach ($candCase in @($summary.criterion_cases)) {
  $baseCase = @($baseline.criterion_cases | Where-Object { $_.name -eq $candCase.name }) | Select-Object -First 1
  if ($null -eq $baseCase) {
    throw "Missing baseline case for CPU criterion case '$($candCase.name)'"
  }

  $budget = 1.0 + ([double]$candCase.regression_pct / 100.0)
  $casePass = ([double]$candCase.mean_ms) -le (([double]$baseCase.mean_ms) * $budget)
  Write-PassFailLine "criterion_${($candCase.name)}_not_regressed_${($candCase.regression_pct)}pct" $casePass `
    "$($candCase.mean_ms) ms vs baseline $($baseCase.mean_ms) ms"
  $criterionPass = $criterionPass -and $casePass
}
```

### Dynamic CPU ratio block

Keep the current dynamic CPU ratio logic, but nest it under a clearly labeled block:

```powershell
$summary.dynamic_strategy_cpu = [ordered]@{
  checked = $true
  timing_boundary = "Criterion"
  build_profile = "bench"
  bench_filter = $DynamicBenchFilter
  static_estimates_json = $DynamicStaticEstimatesPath
  dynamic_estimates_json = $DynamicOnEstimatesPath
  static_mean_ms = $dynOffMean
  dynamic_mean_ms = $dynOnMean
  ratio_dynamic_over_static = [math]::Round($dynRatio, 6)
}
```

### Validation

- run baseline mode on a clean tree
- confirm workflow summary readback is from the new summary path
- confirm both CPU Criterion cases are present in the persisted JSON
- intentionally tamper one baseline case and confirm candidate mode fails only that case

---

### `scripts/run_gpu_perf_gate.ps1`

### Required edits

1. Remove latest-estimate fallback.
2. Resolve GPU Criterion estimate path deterministically from a small registered case map.
3. Run `profile_gpu_dynamic_runner` in release mode.
4. Compare candidate ratio to baseline ratio.
5. Persist boundary and build-profile metadata for both Criterion and dynamic runner sections.
6. Add one representative larger ALMA case to the gate matrix.

### Suggested deterministic estimate resolver

```powershell
$gpuCriterionCases = @{
  "double_ma/gpu_kernel/sma_sma/200k/small" = "target/criterion/double_ma_gpu_kernel_sma_sma_200k/small_1590pairs/new/estimates.json"
  "double_ma/gpu_kernel/alma_alma/200k/large" = "target/criterion/double_ma_gpu_kernel_alma_alma_200k/large_58300pairs/new/estimates.json"
  "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/search_only" = "target/criterion/double_ma_gpu_kernel_contract_split_alma_alma_200k/large_58300pairs_search_only/new/estimates.json"
  "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/end_to_end_product" = "target/criterion/double_ma_gpu_kernel_contract_split_alma_alma_200k/large_58300pairs_end_to_end_product/new/estimates.json"
}

function Resolve-GpuCriterionEstimatePath([string]$BenchFilter) {
  if (-not $gpuCriterionCases.ContainsKey($BenchFilter)) {
    throw "Unregistered GPU benchmark filter '$BenchFilter'. Add an explicit exact path before gating it."
  }
  return $gpuCriterionCases[$BenchFilter]
}
```

### Replace the old glob logic

```powershell
$CriterionEstimatesPath = Resolve-GpuCriterionEstimatePath -BenchFilter $BenchFilter

function Invoke-GpuKernelBench([string]$benchFilter, [bool]$dynamicStrategyEnabled, [string]$estimateSnapshotPath) {
  Wait-NoPerfProcesses -TimeoutSeconds 120 -Context "before GPU benchmark launch"

  if ($dynamicStrategyEnabled) {
    $env:VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL = "1"
  } else {
    Remove-Item Env:VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL -ErrorAction SilentlyContinue
  }

  $env:VECTORBT_BENCH_EMIT_ROUTE = "1"
  $env:VECTORBT_REQUIRE_EXACT_GPU_ROUTE = "1"

  $benchCmd = @(
    "bench",
    "-p", "ta_app_core",
    "--bench", "double_ma_opt",
    "--features", "cuda,cuda-backtest-kernel",
    "--",
    $benchFilter
  )

  cargo @benchCmd
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }

  if (!(Test-Path $CriterionEstimatesPath)) {
    throw "Expected exact GPU Criterion estimates file was not written: $CriterionEstimatesPath"
  }

  if ($estimateSnapshotPath) {
    Copy-Item -Force $CriterionEstimatesPath $estimateSnapshotPath
  }

  $criterion = Get-Content -Raw $CriterionEstimatesPath | ConvertFrom-Json
  return [ordered]@{
    timing_boundary = "Criterion"
    build_profile = "bench"
    estimate_path = $CriterionEstimatesPath
    estimate_snapshot_path = $estimateSnapshotPath
    mean_ms = [math]::Round(([double]$criterion.mean.point_estimate) / 1000000.0, 6)
    mean_ci_lower_ms = [math]::Round(([double]$criterion.mean.confidence_interval.lower_bound) / 1000000.0, 6)
    mean_ci_upper_ms = [math]::Round(([double]$criterion.mean.confidence_interval.upper_bound) / 1000000.0, 6)
    median_ms = [math]::Round(([double]$criterion.median.point_estimate) / 1000000.0, 6)
  }
}
```

### Run dynamic runner in release mode

```powershell
$runnerCmd = @(
  "run",
  "--release",
  "-q",
  "-p", "ta_app_core",
  "--bin", "profile_gpu_dynamic_runner",
  "--features", "cuda,cuda-backtest-kernel",
  "--",
  "--strategy", "double_ma",
  "--workload", "small",
  "--iterations", "$iterations",
  "--dynamic-mode", $dynMode,
  "--device-id", "0"
)
```

### Fix candidate vs baseline ratio comparison

```powershell
$dynamicPass = $true
if ($CheckDynamicStrategyRatio -and $null -ne $summary.dynamic_strategy) {
  if ($null -eq $baseline.dynamic_strategy) {
    throw "Baseline is missing dynamic_strategy block. Re-run GPU baseline mode."
  }

  $dynamicBudget = 1.0 + ($DynamicStrategyRegressionPct / 100.0)

  $candRatio = [double]$summary.dynamic_strategy.ratio_on_over_off
  $baseRatio = [double]$baseline.dynamic_strategy.ratio_on_over_off
  $ratioPass = $candRatio -le ($baseRatio * $dynamicBudget)

  $candOff = [double]$summary.dynamic_strategy.off_mean_ms
  $baseOff = [double]$baseline.dynamic_strategy.off_mean_ms
  $offAbsPass = $candOff -le ($baseOff * $dynamicBudget)

  Write-PassFailLine "gpu_dynamic_strategy_ratio_not_regressed_${DynamicStrategyRegressionPct}pct" $ratioPass `
    "$candRatio vs baseline $baseRatio"
  Write-PassFailLine "gpu_dynamic_strategy_off_baseline_not_regressed_${DynamicStrategyRegressionPct}pct" $offAbsPass `
    "$candOff ms vs baseline $baseOff ms"

  $dynamicPass = $ratioPass -and $offAbsPass
}
```

### Updated summary block

```powershell
$summary.dynamic_strategy = [ordered]@{
  checked = $true
  method = "profile_gpu_dynamic_runner_steady_state"
  timing_boundary = "runner_steady_state_wall"
  build_profile = "release"
  regression_pct = $DynamicStrategyRegressionPct
  off_first_wall_ms = $offDynRun.first_wall_ms
  on_first_wall_ms = $onDynRun.first_wall_ms
  off_mean_ms = $offDynRun.steady_state_wall_ms
  on_mean_ms = $onDynRun.steady_state_wall_ms
  off_runtime_ms = $offDynRun.steady_state_runtime_ms
  on_runtime_ms = $onDynRun.steady_state_runtime_ms
  ratio_on_over_off = [math]::Round($ratio, 6)
}
```

### Validation

- create stale unrelated GPU estimate files under `target/criterion`
- run the gate and confirm it only accepts the exact registered path
- baseline the gate
- tamper candidate dynamic ratio relative to baseline and confirm failure
- verify summary JSON clearly labels Criterion vs runner steady-state boundaries

---

### `scripts/profile_gpu_backtest_nsys.ps1`

### Required edits

1. Persist capture provenance sidecar JSON.
2. Record requested capture mode, effective capture mode, fallback status, report path, target, dynamic mode, graph mode, and exit code.
3. Keep Nsight diagnostic-only status clear.

### Suggested sidecar helper

```powershell
function Write-NsysSidecar {
  param(
    [string]$Path,
    [string]$ProfilerExe,
    [string]$Target,
    [string]$RequestedCaptureMode,
    [string]$EffectiveCaptureMode,
    [bool]$FallbackUsed,
    [string]$ReportPath,
    [int]$ExitCode,
    [string]$DynamicStrategyMode,
    [string]$CudaGraphsMode
  )

  $meta = [ordered]@{
    schema_version = 1
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    timing_boundary = "profiling-only"
    profiler = "nsys"
    profiler_exe = $ProfilerExe
    target = $Target
    requested_capture_mode = $RequestedCaptureMode
    effective_capture_mode = $EffectiveCaptureMode
    fallback_used = $FallbackUsed
    report_path = $ReportPath
    exit_code = $ExitCode
    dynamic_strategy_mode = $DynamicStrategyMode
    cuda_graphs_mode = $CudaGraphsMode
  }

  Write-Utf8Json -Path $Path -InputObject $meta -Depth 6
}
```

### Call it after report resolution

```powershell
$sidecarPath = "$outputPrefixPath.meta.json"
$fallbackUsed = $reportPath -like "*_whole_fallback*"
$effectiveCaptureMode = if ($fallbackUsed) { "whole_process_fallback" } else { $CaptureMode }

Write-NsysSidecar `
  -Path $sidecarPath `
  -ProfilerExe $nsysExe `
  -Target $Target `
  -RequestedCaptureMode $CaptureMode `
  -EffectiveCaptureMode $effectiveCaptureMode `
  -FallbackUsed:$fallbackUsed `
  -ReportPath $reportPath `
  -ExitCode $exitCode `
  -DynamicStrategyMode $DynamicStrategyMode `
  -CudaGraphsMode $CudaGraphsMode
```

### Validation

- run with `-CaptureMode nvtx` and confirm sidecar says `requested_capture_mode=nvtx`
- force a fallback and confirm `effective_capture_mode=whole_process_fallback`
- confirm the sidecar points to the actual report path that exists

---

### `scripts/profile_gpu_backtest_ncu.ps1`

### Required edits

1. Add an explicit `-NcuExePath` parameter for determinism and symmetry with the Nsight Systems script.
2. Write a provenance sidecar JSON after a successful run.
3. Record kernel filter usage and launch count.

### Suggested patch

```powershell
param(
  [ValidateSet("runner", "bench")]
  [string]$Target = "runner",
  # ... existing params ...
  [ValidateSet("inherit", "off", "on")]
  [string]$CudaGraphsMode = "off",
  [string]$NcuExePath = ""
)

if (-not [string]::IsNullOrWhiteSpace($NcuExePath)) {
  $ncuExe = $NcuExePath
} else {
  # existing auto-detection block
}
```

### Suggested sidecar writer

```powershell
$sidecar = [ordered]@{
  schema_version = 1
  generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
  timing_boundary = "profiling-only"
  profiler = "ncu"
  profiler_exe = $ncuExe
  target = $Target
  output_prefix = $OutputPrefix
  report_path = $reportPath
  exit_code = $exitCode
  kernel_filter_enabled = $UseKernelNameFilter
  kernel_name = $KernelName
  launch_count = $LaunchCount
  dynamic_strategy_mode = $DynamicStrategyMode
  cuda_graphs_mode = $CudaGraphsMode
}
Write-Utf8Json -Path "$OutputPrefix.meta.json" -InputObject $sidecar -Depth 6
```

### Optional follow-up

If you later want stronger automation, add a post-run query step that confirms at least one kernel matched when `UseKernelNameFilter = $true`. Keep that out of the initial P1 patch to reduce risk.

---

### `scripts/run_vectorgrid_double_ma_faceoff.ps1`

### Required edits

1. Stop parsing human-friendly Criterion `time: [...]` output.
2. Read the exact `estimates.json` file.
3. Parameterize or derive benchmark metadata instead of hard-coding it.
4. Add `timing_boundary` and `build_profile`.

### Suggested patch

```powershell
param(
  [string]$BenchmarkId = "double_ma/gpu_kernel/alma_alma/200k/large/58300pairs",
  [int]$SampleSize = 10,
  [int]$MeasurementTimeSec = 3,
  [int]$WarmUpTimeSec = 1,
  [string]$Features = "cuda,cuda-backtest-kernel",
  [string]$Objective = "sharpe",
  [string]$OutputJson = "target/vectorgrid_double_ma_faceoff.json",
  [string]$RawOutputPath = "target/vectorgrid_double_ma_faceoff.raw.txt",
  [string]$CriterionEstimatesPath = ""
)

function Resolve-CriterionEstimatesPath {
  param([string]$BenchmarkId, [string]$OverridePath)

  if (-not [string]::IsNullOrWhiteSpace($OverridePath)) {
    return $OverridePath
  }

  $known = @{
    "double_ma/gpu_kernel/alma_alma/200k/large/58300pairs" = "target/criterion/double_ma_gpu_kernel_alma_alma_200k/large_58300pairs/new/estimates.json"
    "double_ma/gpu_kernel/sma_sma/200k/small/1590pairs" = "target/criterion/double_ma_gpu_kernel_sma_sma_200k/small_1590pairs/new/estimates.json"
  }

  if (-not $known.ContainsKey($BenchmarkId)) {
    throw "No exact estimates path registered for benchmark id '$BenchmarkId'. Pass -CriterionEstimatesPath explicitly."
  }

  return $known[$BenchmarkId]
}

function Parse-BenchmarkMetadata {
  param([string]$Id)

  $parts = $Id.Split('/')
  if ($parts.Count -lt 5) {
    throw "Unexpected benchmark id format: $Id"
  }

  $backend = switch ($parts[1]) {
    "gpu_kernel" { "GpuKernel" }
    "gpu_hybrid_kernel" { "GpuHybridKernel" }
    "cpu" { "CpuOnly" }
    default { $parts[1] }
  }

  $barsToken = $parts[3]
  $bars = if ($barsToken -match '^(?<n>\d+)k$') {
    [int64]$matches['n'] * 1000
  } elseif ($barsToken -match '^(?<n>\d+)m$') {
    [int64]$matches['n'] * 1000000
  } else {
    $null
  }

  $pairMatch = [regex]::Match($Id, '(?<pairs>\d+)pairs')
  $pairCount = if ($pairMatch.Success) { [int64]$pairMatch.Groups['pairs'].Value } else { $null }

  return [ordered]@{
    strategy = $parts[0]
    backend = $backend
    ma_case = $parts[2]
    bars = $bars
    pair_count = $pairCount
    pair_bars = if ($bars -and $pairCount) { $bars * $pairCount } else { $null }
  }
}
```

### Read the structured Criterion file

```powershell
$joined = Invoke-LoggedProcess -Exe "cargo" -CmdArgs $cargoArgs -OutputPath $RawOutputPath

$estimatesPath = Resolve-CriterionEstimatesPath -BenchmarkId $BenchmarkId -OverridePath $CriterionEstimatesPath
if (!(Test-Path $estimatesPath)) {
  throw "Missing Criterion estimates file: $estimatesPath"
}

$criterion = Get-Content -Raw $estimatesPath | ConvertFrom-Json
$meta = Parse-BenchmarkMetadata -Id $BenchmarkId

$result = [ordered]@{
  schema_version = 1
  generated_at_utc = [DateTime]::UtcNow.ToString("o")
  timing_boundary = "Criterion"
  build_profile = "bench"
  engine = "vectorgrid"
  benchmark_id = $BenchmarkId
  strategy = $meta.strategy
  ma_case = $meta.ma_case
  objective = $Objective
  backend = $meta.backend
  pair_count = $meta.pair_count
  bars = $meta.bars
  pair_bars = $meta.pair_bars
  runtime = [ordered]@{
    low_ms = [math]::Round(([double]$criterion.mean.confidence_interval.lower_bound) / 1000000.0, 6)
    point_estimate_ms = [math]::Round(([double]$criterion.mean.point_estimate) / 1000000.0, 6)
    high_ms = [math]::Round(([double]$criterion.mean.confidence_interval.upper_bound) / 1000000.0, 6)
    median_ms = [math]::Round(([double]$criterion.median.point_estimate) / 1000000.0, 6)
  }
  criterion_estimates_path = $estimatesPath
  raw_output_path = $RawOutputPath
  features = $Features
  sample_size = $SampleSize
  measurement_time_sec = $MeasurementTimeSec
  warmup_time_sec = $WarmUpTimeSec
}
```

### Validation

- delete the raw output file and rerun
- confirm the persisted JSON still works even if Criterion stdout wording changes
- change `-Objective` and confirm the JSON reflects it without source edits

---

### `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`

### Required edits

1. Add timing boundary fields to all trial artifacts and summary rows.
2. For vectorgrid app-path mode, surface `harness_wall_ms`, `engine_wall_ms`, and `engine_runtime_ms`.
3. Use canonical median trial for parity instead of first trial.
4. Split markdown summary by timing boundary.

### Suggested helper snippets

#### Canonical trial selection

```powershell
function Get-CanonicalTrial {
  param([object[]]$Items)

  if ($null -eq $Items -or $Items.Count -eq 0) {
    return $null
  }

  $sorted = @($Items | Sort-Object { [double]$_.runtime_ms_wall })
  $mid = [int][Math]::Floor($sorted.Count / 2)
  return $sorted[$mid]
}
```

#### Boundary-aware stats

```powershell
function Get-MedianValue {
  param([double[]]$Values)

  $vals = @($Values | Sort-Object)
  if ($vals.Count -eq 0) { return $null }
  if ($vals.Count % 2 -eq 1) {
    return [math]::Round($vals[[int]($vals.Count / 2)], 3)
  }
  return [math]::Round((($vals[($vals.Count / 2) - 1] + $vals[$vals.Count / 2]) / 2.0), 3)
}

function Get-AppPathTimingStats {
  param([object[]]$Trials)

  return [ordered]@{
    harness_wall_median_ms = Get-MedianValue @($Trials | ForEach-Object { [double]$_.result.timing.harness_wall_ms })
    state_lookup_median_ms = Get-MedianValue @($Trials | ForEach-Object { [double]$_.result.timing.state_lookup_ms })
    engine_wall_median_ms = Get-MedianValue @($Trials | ForEach-Object { [double]$_.result.timing.engine_wall_ms })
    engine_runtime_median_ms = Get-MedianValue @($Trials | ForEach-Object { [double]$_.result.timing.engine_runtime_ms })
  }
}
```

#### Add boundary labels to artifacts

For CLI vectorgrid trials:

```powershell
$artifact = [ordered]@{
  tool = "vectorgrid"
  backend = $BackendName
  vectorgrid_mode = "cli-cold"
  timing_boundary = "external_cli_wall"
  build_profile = "release"
  # ... existing fields ...
}
```

For app-path vectorgrid trials:

```powershell
$items.Add([ordered]@{
  tool = "vectorgrid"
  backend = $BackendName
  vectorgrid_mode = "app-path"
  timing_boundary = "app-path"
  build_profile = "release"
  # ... existing fields ...
})
```

For vectorbt trials:

```powershell
$artifact = [ordered]@{
  tool = "vectorbt"
  backend = "cpu"
  timing_boundary = "external_python_wall"
  build_profile = "python"
  # ... existing fields ...
}
```

#### Summary rows

```powershell
foreach ($group in $grouped) {
  $items = @($group.Group)
  $stats = Get-TrialStats -Trials $items
  $first = $items[0]

  $appPathStats = $null
  if ($first.tool -eq "vectorgrid" -and $first.timing_boundary -eq "app-path") {
    $appPathStats = Get-AppPathTimingStats -Trials $items
  }

  $summaryRows.Add([ordered]@{
    tool = $first.tool
    backend = $first.backend
    timing_boundary = $first.timing_boundary
    build_profile = $first.build_profile
    min_ms = $stats.min_ms
    median_ms = $stats.median_ms
    max_ms = $stats.max_ms
    app_path_harness_median_ms = if ($appPathStats) { $appPathStats.harness_wall_median_ms } else { $null }
    app_path_state_lookup_median_ms = if ($appPathStats) { $appPathStats.state_lookup_median_ms } else { $null }
    app_path_engine_wall_median_ms = if ($appPathStats) { $appPathStats.engine_wall_median_ms } else { $null }
    app_path_engine_runtime_median_ms = if ($appPathStats) { $appPathStats.engine_runtime_median_ms } else { $null }
    pair_count = $first.pair_count
    bars = $first.bars
    pair_bars = $first.pair_bars
    backend_route = if ($first.tool -eq "vectorgrid") { $first.backend_route } else { "cpu_python_workflow" }
    best = $first.best
    trial_paths = @($items | ForEach-Object { $_.stdout_path })
  })
}
```

#### Canonical parity source

```powershell
$vectorGridCpu = Get-CanonicalTrial @($measuredArtifacts | Where-Object { $_.tool -eq "vectorgrid" -and $_.backend -eq "cpu" })
$vectorGridGpu = Get-CanonicalTrial @($measuredArtifacts | Where-Object { $_.tool -eq "vectorgrid" -and $_.backend -eq "gpu-kernel" })
$vectorBtCpu = Get-CanonicalTrial @($measuredArtifacts | Where-Object { $_.tool -eq "vectorbt" })
```

### Markdown summary change

Create one section per boundary instead of one mixed table.

Recommended headings:

- `## Timings — app-path`
- `## Timings — external CLI wall`
- `## Timings — external Python wall`

And add this note near the top:

```markdown
This summary intentionally keeps timing boundaries separate.  
`vectorgrid` app-path rows are same-thread app-state + optimizer-call timings.  
`vectorgrid` CLI rows are external process wall times.  
`vectorbt` rows are external Python process wall times.
```

### Validation

- run app-path mode and confirm app-path summary rows include engine-runtime medians
- run CLI-cold mode and confirm those columns are null
- verify parity uses the canonical median-time trial

---

## 8.3 Rust binary edits

### `src-tauri/src/bin/ta_workflow_regression.rs`

### Required edits

1. Add `schema_version`, `timing_boundary`, and `build_profile`.
2. Add stage breakdown:
   - optimize total
   - validate total
   - meta-select
   - holdout
3. Keep the existing total `runtime_ms`.

### Suggested patch

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
struct WorkflowStageBreakdown {
    optimize_ms_total: u64,
    validate_ms_total: u64,
    meta_select_ms: u64,
    holdout_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WorkflowSummary {
    schema_version: u32,
    timing_boundary: String,
    timing_boundary_note: String,
    build_profile: String,
    candles_len: usize,
    train_assets: usize,
    holdout_assets: usize,
    artifacts_used: usize,
    groups: usize,
    top_meta_score: f64,
    robust_score_p10: f64,
    max_dd_p90: f64,
    holdout_score_p10: f64,
    holdout_max_dd_p90: f64,
    holdout_pass_rate: f64,
    runtime_ms: u64,
    stage: WorkflowStageBreakdown,
}

fn build_profile() -> &'static str {
    if cfg!(debug_assertions) { "dev" } else { "release" }
}
```

### Accumulate stage timings

```rust
let mut optimize_ms_total = 0u64;
let mut validate_ms_total = 0u64;

for (i, (label, candles)) in train.iter().enumerate() {
    // ...
    let opt_start = Instant::now();
    let opt_res =
        run_composite_cpu_optimization_blocking_with_candles(opt_req, candles, cancel, None)?;
    optimize_ms_total += opt_start.elapsed().as_millis() as u64;

    // ...
    let val_start = Instant::now();
    let val_res =
        run_composite_validation_blocking_with_candles(val_req, candles, cancel, None)?;
    validate_ms_total += val_start.elapsed().as_millis() as u64;

    // ...
}

let meta_start = Instant::now();
let mut meta_res = meta_select_from_artifacts(&meta_req, &artifacts, None);
let meta_elapsed_ms = meta_start.elapsed().as_millis() as u64;

let holdout_start = Instant::now();
let holdout_summary =
    evaluate_holdout_light(&meta_req, &mut meta_res.groups, &holdout_pairs, cancel)?;
let holdout_elapsed_ms = holdout_start.elapsed().as_millis() as u64;
```

### Populate summary

```rust
Ok(WorkflowSummary {
    schema_version: 1,
    timing_boundary: "workflow-regression".to_string(),
    timing_boundary_note: "total deterministic workflow wall time including optimize, validate, meta-select, and holdout".to_string(),
    build_profile: build_profile().to_string(),
    candles_len,
    train_assets: train.len(),
    holdout_assets: holdout.len(),
    artifacts_used: meta_res.artifacts_used,
    groups: meta_res.groups.len(),
    top_meta_score: top.map(|g| g.meta_score).unwrap_or(0.0),
    robust_score_p10: top.map(|g| g.robust_score_p10).unwrap_or(0.0),
    max_dd_p90: top.map(|g| g.max_dd_p90).unwrap_or(0.0),
    holdout_score_p10: hold.map(|h| h.score_p10).unwrap_or(0.0),
    holdout_max_dd_p90: hold.map(|h| h.max_dd_p90).unwrap_or(0.0),
    holdout_pass_rate: hold.map(|h| h.pass_rate).unwrap_or(0.0),
    runtime_ms: t0.elapsed().as_millis() as u64,
    stage: WorkflowStageBreakdown {
        optimize_ms_total,
        validate_ms_total,
        meta_select_ms: meta_elapsed_ms,
        holdout_ms: holdout_elapsed_ms,
    },
})
```

### Validation

- run under `cargo run` and `cargo run --release`
- confirm `build_profile` changes as expected
- confirm stage totals are all non-zero on real runs

---

### `src-tauri/src/bin/ta_double_ma_validation_cost.rs`

### Required edits

1. Add metadata envelope.
2. Separate:
   - total ratio vs optimization
   - main validation ratio vs optimization
   - cross-ticker ratio vs optimization
3. Keep total ratio for backward compatibility.
4. Preserve `requested_backend`.

### Suggested struct changes

```rust
#[derive(Serialize)]
struct CostReport {
    schema_version: u32,
    timing_boundary: &'static str,
    timing_boundary_note: &'static str,
    build_profile: String,
    csv_path: String,
    backend_requested: String,
    total_bars: usize,
    optimization: OptimizationSummary,
    validation_cases: Vec<ValidationCaseSummary>,
    module_feasibility: Vec<ModuleFeasibilitySummary>,
}

#[derive(Serialize)]
struct ValidationCaseSummary {
    label: String,
    status: String,
    repeats: usize,
    avg_wall_ms: u64,
    avg_main_validation_wall_ms: u64,
    avg_cross_ticker_wall_ms: u64,
    avg_reported_runtime_ms: u64,
    versus_optimization_ratio: f64,
    main_validation_ratio_vs_optimization: f64,
    cross_ticker_ratio_vs_optimization: f64,
    cheapness_band: String,
    main_validation_cheapness_band: String,
    cross_ticker_cheapness_band: String,
    backend_used: Option<String>,
    backend_route: Option<String>,
    backend_fallback_reason: Option<String>,
    // ... existing fields ...
}
```

### Ratio computation

```rust
let total_ratio = if opt_wall_ms > 0 {
    avg_wall_ms as f64 / opt_wall_ms as f64
} else {
    0.0
};
let main_ratio = if opt_wall_ms > 0 {
    avg_main_wall_ms as f64 / opt_wall_ms as f64
} else {
    0.0
};
let cross_ratio = if opt_wall_ms > 0 {
    avg_cross_wall_ms as f64 / opt_wall_ms as f64
} else {
    0.0
};
```

### Summary population

```rust
summaries.push(ValidationCaseSummary {
    label: case.label.to_string(),
    status: if agg.last_error.is_some() { "error".to_string() } else { "ok".to_string() },
    repeats: agg.repeats,
    avg_wall_ms,
    avg_main_validation_wall_ms: avg_main_wall_ms,
    avg_cross_ticker_wall_ms: avg_cross_wall_ms,
    avg_reported_runtime_ms,
    versus_optimization_ratio: total_ratio,
    main_validation_ratio_vs_optimization: main_ratio,
    cross_ticker_ratio_vs_optimization: cross_ratio,
    cheapness_band: cheapness_band(total_ratio),
    main_validation_cheapness_band: cheapness_band(main_ratio),
    cross_ticker_cheapness_band: cheapness_band(cross_ratio),
    backend_used: agg.last_backend_used,
    backend_route: agg.last_backend_route,
    backend_fallback_reason: agg.last_backend_fallback_reason,
    // ... existing fields ...
});
```

### Report envelope

```rust
let report = CostReport {
    schema_version: 1,
    timing_boundary: "validation-cost",
    timing_boundary_note: "wall-time comparison between optimization baseline and validation module stacks",
    build_profile: build_profile().to_string(),
    csv_path: cli.csv.to_string_lossy().to_string(),
    backend_requested: requested_backend_label(&backend).to_string(),
    total_bars: candles.close.len(),
    optimization: OptimizationSummary {
        wall_ms: opt_wall_ms,
        runtime_ms: opt_res.runtime_ms,
        backend_used: opt_backend_used,
        backend_route: opt_res.backend_route,
        backend_fallback_reason: opt_res.backend_fallback_reason,
        top_candidates: candidates.len(),
    },
    validation_cases: summaries,
    module_feasibility,
};
```

### Validation

- run one no-cross case and one cross-ticker case
- verify the report shows:
  - total ratio
  - main ratio
  - cross ratio
- confirm cross ratio is zero on cases without cross-ticker work

---

### `src-tauri/src/bin/ta_app_path_faceoff.rs`

### Required edits

1. Add metadata envelope and build profile.
2. Extend trial summaries so total wall, state lookup, and engine wall medians are surfaced.
3. Add requested backend and a measured route summary.

### Struct additions

```rust
#[derive(Debug, Serialize)]
struct RouteSummaryArtifact {
    backend_used_counts: std::collections::BTreeMap<String, usize>,
    backend_route_counts: std::collections::BTreeMap<String, usize>,
    fallback_count: usize,
}

#[derive(Debug, Serialize)]
struct TrialTimingSummaryArtifact {
    count: usize,
    total_wall_ms_min: f64,
    total_wall_ms_median: f64,
    total_wall_ms_max: f64,
    state_lookup_ms_min: f64,
    state_lookup_ms_median: f64,
    state_lookup_ms_max: f64,
    harness_wall_ms_min: f64,
    harness_wall_ms_median: f64,
    harness_wall_ms_max: f64,
    engine_wall_ms_min: f64,
    engine_wall_ms_median: f64,
    engine_wall_ms_max: f64,
    engine_runtime_ms_min: u64,
    engine_runtime_ms_median: f64,
    engine_runtime_ms_max: u64,
    // existing GPU/hybrid subphase medians remain
}

#[derive(Debug, Serialize)]
struct AppPathFaceoffArtifact {
    schema_version: u32,
    tool: &'static str,
    mode: &'static str,
    timing_boundary: &'static str,
    timing_boundary_note: &'static str,
    build_profile: &'static str,
    requested_backend: String,
    backend: String,
    dataset: FaceoffDatasetArtifact,
    route_summary: RouteSummaryArtifact,
    timing_summary: FaceoffTimingSummaryArtifact,
    warmup_trials: Vec<TrialArtifact>,
    measured_trials: Vec<TrialArtifact>,
    generated_at_utc: String,
}
```

### Summary helper additions

```rust
fn summarize_trials(trials: &[TrialArtifact]) -> Option<TrialTimingSummaryArtifact> {
    if trials.is_empty() {
        return None;
    }

    let mut total_wall_ms: Vec<f64> = trials.iter().map(|t| t.timing.total_wall_ms).collect();
    let mut state_lookup_ms: Vec<f64> = trials.iter().map(|t| t.timing.state_lookup_ms).collect();
    let mut harness_wall_ms: Vec<f64> = trials.iter().map(|t| t.timing.harness_wall_ms).collect();
    let mut engine_wall_ms: Vec<f64> = trials.iter().map(|t| t.timing.engine_wall_ms).collect();
    let mut engine_runtime_ms: Vec<u64> = trials.iter().map(|t| t.timing.engine_runtime_ms).collect();

    Some(TrialTimingSummaryArtifact {
        count: trials.len(),
        total_wall_ms_min: total_wall_ms.iter().copied().fold(f64::INFINITY, f64::min),
        total_wall_ms_median: median_f64(total_wall_ms.as_mut_slice()),
        total_wall_ms_max: total_wall_ms.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        state_lookup_ms_min: state_lookup_ms.iter().copied().fold(f64::INFINITY, f64::min),
        state_lookup_ms_median: median_f64(state_lookup_ms.as_mut_slice()),
        state_lookup_ms_max: state_lookup_ms.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        harness_wall_ms_min: harness_wall_ms.iter().copied().fold(f64::INFINITY, f64::min),
        harness_wall_ms_median: median_f64(harness_wall_ms.as_mut_slice()),
        harness_wall_ms_max: harness_wall_ms.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        engine_wall_ms_min: engine_wall_ms.iter().copied().fold(f64::INFINITY, f64::min),
        engine_wall_ms_median: median_f64(engine_wall_ms.as_mut_slice()),
        engine_wall_ms_max: engine_wall_ms.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        engine_runtime_ms_min: engine_runtime_ms.iter().copied().min().unwrap_or(0),
        engine_runtime_ms_median: median_u64_as_f64(engine_runtime_ms.as_mut_slice()),
        engine_runtime_ms_max: engine_runtime_ms.iter().copied().max().unwrap_or(0),
        // ... existing fields ...
    })
}
```

### Route summary helper

```rust
fn summarize_routes(trials: &[TrialArtifact]) -> RouteSummaryArtifact {
    let mut backend_used_counts = std::collections::BTreeMap::new();
    let mut backend_route_counts = std::collections::BTreeMap::new();
    let mut fallback_count = 0usize;

    for trial in trials {
        *backend_used_counts
            .entry(format!("{:?}", trial.result.backend_used))
            .or_insert(0) += 1;

        let route = trial
            .result
            .backend_route
            .clone()
            .unwrap_or_else(|| "<none>".to_string());
        *backend_route_counts.entry(route).or_insert(0) += 1;

        if trial.result.backend_fallback_reason.is_some() {
            fallback_count += 1;
        }
    }

    RouteSummaryArtifact {
        backend_used_counts,
        backend_route_counts,
        fallback_count,
    }
}
```

### Artifact creation

```rust
let artifact = AppPathFaceoffArtifact {
    schema_version: 1,
    tool: "vectorgrid",
    mode: "app-path",
    timing_boundary: "app-path",
    timing_boundary_note: "same-thread harness using AppState plus blocking optimizer call; not the full Tauri command path",
    build_profile: build_profile(),
    requested_backend: format!("{:?}", backend_from_arg(cli.backend.clone())),
    backend: format!("{:?}", backend_from_arg(cli.backend)),
    dataset: FaceoffDatasetArtifact { /* existing fields */ },
    route_summary: summarize_routes(measured_trials.as_slice()),
    timing_summary: FaceoffTimingSummaryArtifact {
        warmup: summarize_trials(warmup_trials.as_slice()),
        measured: summarize_trials(measured_trials.as_slice()),
    },
    warmup_trials,
    measured_trials,
    generated_at_utc: chrono::Utc::now().to_rfc3339(),
};
```

### Validation

- run CPU app-path and GPU app-path
- verify measured summary exposes total wall, state lookup, engine wall, and engine runtime medians
- verify route summary correctly counts route/fallback occurrences

---

### `src-tauri/src/bin/ta_profile_route_matrix.rs`

### Required edits

1. Add metadata envelope.
2. Add optional strict route mode.
3. Persist `route_ok` and expected route.

### Suggested patch

```rust
#[derive(Serialize)]
struct RouteMatrixArtifact {
    schema_version: u32,
    timing_boundary: &'static str,
    timing_boundary_note: &'static str,
    build_profile: &'static str,
    csv_path: String,
    timeframe: String,
    total_bars: usize,
    cases: Vec<RouteMatrixCase>,
}

#[derive(Serialize)]
struct RouteMatrixCase {
    case_label: String,
    requested_backend: String,
    expected_backend_route: Option<String>,
    backend_used: String,
    backend_route: Option<String>,
    backend_fallback_reason: Option<String>,
    route_ok: bool,
    runtime_ms: u64,
    wall_ms: u64,
    best_fast_len: u16,
    best_slow_len: u16,
    best_sharpe: f64,
    best_pnl: f64,
}
```

### Strict flag

```rust
fn parse_strict() -> bool {
    std::env::args().any(|arg| arg == "--strict")
}
```

### Route verdict in `run_case`

```rust
let expected_backend_route = match spec.backend {
    Backend::CpuOnly => Some("CpuOnly".to_string()),
    Backend::GpuKernel { .. } => Some("GpuKernel".to_string()),
    Backend::GpuHybridKernel { .. } => Some("GpuHybridKernel".to_string()),
    _ => None,
};

let route_ok = match expected_backend_route.as_deref() {
    Some("CpuOnly") => res.backend_fallback_reason.is_none() && matches!(res.backend_used, BackendUsed::Cpu),
    Some(expected) => res.backend_fallback_reason.is_none() && res.backend_route.as_deref() == Some(expected),
    None => true,
};

Ok(RouteMatrixCase {
    case_label: spec.case_label.to_string(),
    requested_backend: requested_backend_label(&spec.backend).to_string(),
    expected_backend_route,
    backend_used: backend_used_label(res.backend_used).to_string(),
    backend_route: res.backend_route,
    backend_fallback_reason: res.backend_fallback_reason,
    route_ok,
    runtime_ms: res.runtime_ms,
    wall_ms,
    best_fast_len: res.best_params.fast_len,
    best_slow_len: res.best_params.slow_len,
    best_sharpe: res.best_metrics.sharpe,
    best_pnl: res.best_metrics.pnl,
})
```

### Fail in strict mode

```rust
let strict = parse_strict();
for spec in case_specs(device_id) {
    let case = run_case(&spec, &candles)?;
    if strict && !case.route_ok {
        return Err(format!("route mismatch for {}", case.case_label));
    }
    cases.push(case);
}
```

### Validation

- run without `--strict` and verify JSON is written
- run with `--strict` on a healthy machine and verify success
- intentionally force a fallback and verify strict mode fails

---

## 8.4 Criterion bench edits

### Shared guidance for all bench files

Use the small `emit_bench_meta(...)` helper from Section 8.1 and call it once per gate-relevant case or benchmark group.

This does **not** change timing. It only makes the logs parseable and the evidence self-describing.

---

### `crates/app_core/benches/double_ma_opt.rs`

### Required edits

1. Add `emit_bench_meta` helper.
2. Add optional route probe for gate-relevant GPU groups.
3. Add optional strict exact-route assertion controlled by env var.
4. Leave the existing benchmark names intact to avoid unnecessary path churn.

### Suggested route probe pattern

```rust
let bench_id = format!("double_ma/gpu_kernel/{}/200k/{}", ma.label, name);
if std::env::var_os("VECTORBT_BENCH_EMIT_ROUTE").is_some() {
    let probe = ta_app_core::run_double_ma_optimization_blocking_with_candles(
        req.clone(),
        &candles,
        &cancel,
        None,
    )
    .expect("gpu kernel route probe");
    emit_bench_meta(
        &bench_id,
        "Criterion",
        "GpuKernel",
        Some(match probe.backend_used {
            ta_app_core::BackendUsed::Cpu => "Cpu",
            ta_app_core::BackendUsed::Gpu => "Gpu",
        }),
        probe.backend_route.as_deref(),
        probe.backend_fallback_reason.as_deref(),
    );

    if std::env::var_os("VECTORBT_REQUIRE_EXACT_GPU_ROUTE").is_some() {
        assert_eq!(probe.backend_route.as_deref(), Some("GpuKernel"));
        assert!(probe.backend_fallback_reason.is_none());
    }
}
```

### Where to apply

Apply this pattern to:

- `bench_double_ma_gpu_kernel`
- `bench_double_ma_gpu_hybrid_kernel`
- `bench_double_ma_gpu_kernel_contract_split`
- `bench_double_ma_gpu_kernel_strategy_profiles`
- `bench_double_ma_gpu_kernel_vdps`

### Optional CPU metadata banner

For CPU groups, emit a simpler banner without a route probe:

```rust
emit_bench_meta(
    &format!("double_ma/cpu/{}/200k/{}", ma.label, name),
    "Criterion",
    "CpuOnly",
    None,
    None,
    None,
);
```

### Validation

- run with `VECTORBT_BENCH_EMIT_ROUTE=1`
- confirm route lines appear once per case
- run with `VECTORBT_REQUIRE_EXACT_GPU_ROUTE=1` on a healthy GPU path and confirm no regressions

---

### `crates/app_core/benches/double_ma_metrics.rs`

### Recommended edits

This file is not currently a core gate, but it is a strong nightly / diagnostic benchmark surface for the full-metrics contract.

Add:

1. `emit_bench_meta` helper
2. route probe banner
3. explicit `metric_contract = "full_metrics"` log note
4. `objective = "Pnl"` log note

### Suggested snippet

```rust
let bench_id = format!("double_ma/gpu_kernel_pnl_full_metrics/{}/200k/{}", ma_label, name);
if std::env::var_os("VECTORBT_BENCH_EMIT_ROUTE").is_some() {
    let probe = ta_app_core::run_double_ma_optimization_blocking_with_candles(
        req.clone(),
        &candles,
        &cancel,
        None,
    )
    .expect("gpu kernel pnl full metrics route probe");

    emit_bench_meta(
        &bench_id,
        "Criterion",
        "GpuKernel",
        Some(match probe.backend_used {
            ta_app_core::BackendUsed::Cpu => "Cpu",
            ta_app_core::BackendUsed::Gpu => "Gpu",
        }),
        probe.backend_route.as_deref(),
        probe.backend_fallback_reason.as_deref(),
    );

    println!(
        "bench_contract benchmark_id={} metric_contract=full_metrics objective=Pnl",
        bench_id
    );
}
```

### Use

Promote this into nightly only after the core gate fixes are stable.

---

### `crates/app_core/benches/double_ma_followup_workflow.rs`

### Recommended edits

This bench is already useful, but its output should say that it is a **follow-up workflow** boundary, not an optimize hot path.

Add:

1. metadata banner
2. route probe print from the already-executed optimization probe
3. explicit cached vs cold wording

### Suggested snippet

```rust
let route = opt_res.backend_route.as_deref();
let fallback = opt_res.backend_fallback_reason.as_deref();
emit_bench_meta(
    &format!("double_ma_followup/{label}/50k"),
    "Criterion",
    match backend {
        Backend::CpuOnly => "CpuOnly",
        Backend::GpuKernel { .. } => "GpuKernel",
        _ => "Other",
    },
    Some(match opt_res.backend_used {
        ta_app_core::BackendUsed::Cpu => "Cpu",
        ta_app_core::BackendUsed::Gpu => "Gpu",
    }),
    route,
    fallback,
);
println!(
    "bench_mode benchmark_id=double_ma_followup/{}/50k cached_key_present={} cold_and_cached_followup=true",
    label,
    !followup_cache_key.is_empty()
);
```

### Why this matters

If this file is later promoted into a user-visible responsiveness pack, its logs will already state what it measured.

---

### `crates/app_core/benches/dynamic_strategy_cpu.rs`

### Required edits

1. Add metadata banners for:
   - `dynamic_strategy_cpu/eval/50k`
   - `dynamic_strategy_cpu/staging/50k`
   - `dynamic_strategy_cpu/full_run/2k`
2. Add optional quick-mode env for shorter local reruns while keeping default gate numbers unchanged.

### Suggested snippet

```rust
fn quick_mode() -> bool {
    std::env::var_os("VECTORBT_DYNAMIC_CPU_BENCH_QUICK").is_some()
}

let mut group = c.benchmark_group("dynamic_strategy_cpu/full_run/2k");
group.measurement_time(if quick_mode() {
    Duration::from_secs(4)
} else {
    Duration::from_secs(8)
});
group.sample_size(if quick_mode() { 6 } else { 10 });

emit_bench_meta(
    "dynamic_strategy_cpu/full_run/2k",
    "Criterion",
    "CpuOnly",
    None,
    None,
    None,
);
```

### Why this matters

The CPU gate already consumes this file, so it benefits immediately from explicit metadata without changing the benchmark ID.

---

### `crates/app_core/benches/gpu_native_expansion_opt.rs`

### Recommended edits

This file already has the most important property: exact GPU route assertions in GPU-native paths. Keep those.

Add only:

1. metadata banner
2. candidate count / pair count note for easier nightly log analysis

### Suggested snippet

```rust
emit_bench_meta(
    route_label,
    "Criterion",
    "GpuKernel",
    Some("Gpu"),
    Some("GpuKernel"),
    None,
);
println!("bench_shape route_label={} cases=small|medium|large", route_label);
```

### Why not change more?

Because the strong route assertions are already doing the important work here.

---

### `crates/app_core/benches/double_ma_validation_workflow.rs`

### Recommended edits

1. Add metadata banner.
2. Add strict exact-route env for the sparse GPU workflow, similar to other GPU benches.

### Suggested snippet

```rust
fn require_exact_gpu_route() -> bool {
    std::env::var_os("VECTORBT_REQUIRE_EXACT_GPU_ROUTE").is_some()
}

print_double_ma_route_probe(
    "double_ma_validate/gpu_kernel/sparse_finalists/100k/8candidates",
    &route_probe,
);

emit_bench_meta(
    "double_ma_validate/gpu_kernel/sparse_finalists/100k/8candidates",
    "Criterion",
    "GpuKernel",
    Some(match route_probe.backend_used {
        ta_app_core::BackendUsed::Cpu => "Cpu",
        ta_app_core::BackendUsed::Gpu => "Gpu",
    }),
    route_probe.backend_route.as_deref(),
    route_probe.backend_fallback_reason.as_deref(),
);

if require_exact_gpu_route() {
    assert!(matches!(route_probe.backend_used, ta_app_core::BackendUsed::Gpu));
    assert_eq!(route_probe.backend_route.as_deref(), Some("GpuKernel"));
    assert!(route_probe.backend_fallback_reason.is_none());
}
```

---

### `crates/app_core/benches/composite_validation_workflow.rs`

### Recommended edits

This file already asserts exact GPU route in the GPU workflows. Keep those asserts.

Add only:

1. metadata banner
2. explicit boundary text in route prints

### Suggested snippet

```rust
emit_bench_meta(
    "composite_validate/gpu_kernel/asr/100k/64candidates",
    "Criterion",
    "GpuKernel",
    Some(match route_probe.backend_used {
        ta_app_core::BackendUsed::Cpu => "Cpu",
        ta_app_core::BackendUsed::Gpu => "Gpu",
    }),
    route_probe.backend_route.as_deref(),
    route_probe.backend_fallback_reason.as_deref(),
);
```

---

### `crates/app_core/benches/e2e_workflow_ml.rs`

### Required edits for nightly usefulness

1. Make train and holdout asset counts configurable via env.
2. Add optional stage diagnostics.
3. Emit benchmark metadata banner.

### Suggested snippet

```rust
fn parse_usize_env(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default)
}

fn diag_enabled() -> bool {
    std::env::var_os("VTA_E2E_BENCH_DIAG").is_some()
}
```

Use the envs in `bench_e2e_workflow_with_ml`:

```rust
let train_assets = parse_usize_env("VTA_E2E_BENCH_TRAIN_ASSETS", 12);
let holdout_assets = parse_usize_env("VTA_E2E_BENCH_HOLDOUT_ASSETS", 8);

emit_bench_meta(
    &format!("e2e/composite_cpu_validate_meta_holdout_ml/len{len}/train{train_assets}/holdout{holdout_assets}"),
    "Criterion",
    "CpuOnly",
    None,
    None,
    None,
);
```

Add stage timing prints inside `run_full_workflow_once`:

```rust
if diag_enabled() {
    println!(
        "e2e_stage len={} train_assets={} holdout_assets={} optimize_validate_ms={} meta_select_ms={} holdout_ms={} ml_train_apply_ms={}",
        candles_len,
        train_assets,
        holdout_assets,
        optimize_validate_ms,
        meta_select_ms,
        holdout_ms,
        ml_train_apply_ms
    );
}
```

### Why this matters

The file becomes much more useful as a nightly performance surface without turning it into a large new subsystem.

---

## 9. Recommended validation commands and rollout order

## Rollout order

1. `run_workflow_gate.ps1`
2. `ta_workflow_regression.rs`
3. `run_cpu_perf_gate.ps1`
4. `run_gpu_perf_gate.ps1`
5. `run_vectorgrid_double_ma_faceoff.ps1`
6. `run_vectorbt_vs_vectorgrid_faceoff.ps1`
7. `ta_app_path_faceoff.rs`
8. `ta_double_ma_validation_cost.rs`
9. `ta_profile_route_matrix.rs`
10. profiling scripts
11. Criterion metadata banners
12. matrix expansion

## Suggested commands

### Workflow gate

```powershell
powershell -File scripts/run_workflow_gate.ps1 -Mode baseline -UseRelease
powershell -File scripts/run_workflow_gate.ps1 -Mode candidate -UseRelease
```

### CPU gate

```powershell
powershell -File scripts/run_cpu_perf_gate.ps1 -Mode baseline
powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate
```

### GPU gate

```powershell
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode baseline
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

### Route matrix

```powershell
cargo run --release -p ta_desktop_demo_app --bin ta_profile_route_matrix -- `
  --csv data/benchmarks/AAPL.csv `
  --timeframe 15m `
  --out target/profile_route_matrix.json `
  --strict
```

### App-path faceoff

```powershell
cargo build -p ta_desktop_demo_app --release --bin ta_app_path_faceoff --features cuda,cuda-backtest-kernel

target/release/ta_app_path_faceoff.exe `
  --backend gpu-kernel `
  --csv data/benchmarks/AAPL.csv `
  --output-json target/app_path_faceoff.json `
  --warmup-trials 3 `
  --measured-trials 5
```

### Validation cost

```powershell
cargo run --release -p ta_desktop_demo_app --bin ta_double_ma_validation_cost -- `
  --csv data/benchmarks/AAPL.csv `
  --backend gpu-kernel `
  --out-json target/validation_cost.json `
  --repeats 1 `
  --validate-max-candidates 8 `
  --optimize-top-k 16
```

### Nsight Systems

```powershell
powershell -File scripts/profile_gpu_backtest_nsys.ps1 `
  -Target runner `
  -RunnerStrategy breakout_momentum `
  -CaptureMode nvtx `
  -NvtxMode on `
  -OutputPrefix target/nsight/example_run
```

### Nsight Compute

```powershell
powershell -File scripts/profile_gpu_backtest_ncu.ps1 `
  -Target runner `
  -RunnerStrategy breakout_momentum `
  -OutputPrefix target/ncu/example_run `
  -UseKernelNameFilter:$true
```

---

## 10. Detailed implementation checklist

### Phase 0 — correctness and trust

- [ ] Update `scripts/run_workflow_gate.ps1` so baseline mode does **not** overwrite `OutPath`.
- [ ] Add `-UseRelease` (or equivalent build-profile control) to `scripts/run_workflow_gate.ps1`.
- [ ] Make baseline mode copy the fresh workflow summary to `BaselinePath`.
- [ ] Update `scripts/run_cpu_perf_gate.ps1` to pass an explicit workflow summary path into `run_workflow_gate.ps1`.
- [ ] Update `scripts/run_cpu_perf_gate.ps1` to read the same workflow summary path it requested.
- [ ] Add `schema_version`, `timing_boundary`, `timing_boundary_note`, `build_profile`, and `stage` to `src-tauri/src/bin/ta_workflow_regression.rs`.
- [ ] Verify `ta_workflow_regression.rs` reports different `build_profile` values under dev vs release runs.
- [ ] Remove latest-estimate fallback from `scripts/run_gpu_perf_gate.ps1`.
- [ ] Replace GPU estimate glob fallback with deterministic exact-path resolution.
- [ ] Run `profile_gpu_dynamic_runner` via `cargo run --release` inside `scripts/run_gpu_perf_gate.ps1`.
- [ ] Change GPU dynamic candidate-mode comparison from candidate `on` vs candidate `off` to candidate ratio vs baseline ratio.
- [ ] Keep a second absolute baseline check for `off_mean_ms` in the GPU dynamic block.

### Phase 1 — evidence schema and boundary labeling

- [ ] Add the shared JSON metadata envelope to `ta_workflow_regression.rs`.
- [ ] Add the shared JSON metadata envelope to `ta_app_path_faceoff.rs`.
- [ ] Add the shared JSON metadata envelope to `ta_double_ma_validation_cost.rs`.
- [ ] Add the shared JSON metadata envelope to `ta_profile_route_matrix.rs`.
- [ ] Add a route summary block to `ta_app_path_faceoff.rs`.
- [ ] Extend `ta_app_path_faceoff.rs` summary medians to include total wall, state lookup, engine wall, and engine runtime.
- [ ] Add requested/effective capture provenance sidecar JSON to `scripts/profile_gpu_backtest_nsys.ps1`.
- [ ] Add `-NcuExePath` to `scripts/profile_gpu_backtest_ncu.ps1`.
- [ ] Add provenance sidecar JSON to `scripts/profile_gpu_backtest_ncu.ps1`.
- [ ] Update `scripts/run_vectorgrid_double_ma_faceoff.ps1` to read structured `estimates.json`.
- [ ] Remove hard-coded `strategy`, `ma_case`, `objective`, and `backend` values in `scripts/run_vectorgrid_double_ma_faceoff.ps1`.
- [ ] Parameterize `objective` in `scripts/run_vectorgrid_double_ma_faceoff.ps1`.
- [ ] Add `timing_boundary="Criterion"` and `build_profile="bench"` to `scripts/run_vectorgrid_double_ma_faceoff.ps1`.
- [ ] Add `timing_boundary` and `build_profile` to vectorgrid CLI trial artifacts in `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`.
- [ ] Add `timing_boundary` and `build_profile` to vectorgrid app-path trial artifacts in `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`.
- [ ] Add `timing_boundary` and `build_profile` to vectorbt trial artifacts in `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`.
- [ ] Split faceoff markdown summary by timing boundary.

### Phase 2 — representative matrix expansion

- [ ] Expand `scripts/run_cpu_perf_gate.ps1` from one Criterion case to a deterministic two-case CPU matrix.
- [ ] Keep the existing small SMA CPU case for continuity.
- [ ] Add a large ALMA CPU case to `scripts/run_cpu_perf_gate.ps1`.
- [ ] Label each CPU Criterion case with boundary and build profile in the persisted summary.
- [ ] Expand `scripts/run_gpu_perf_gate.ps1` from one Criterion case to at least two registered cases.
- [ ] Keep the existing small SMA GPU case for continuity.
- [ ] Add a representative large ALMA GPU case to `scripts/run_gpu_perf_gate.ps1`.
- [ ] Decide whether contract-split `search_only` and `end_to_end_product` belong in nightly only or also in GPU gate candidate mode.
- [ ] Add `VECTORBT_BENCH_EMIT_ROUTE` logging to `crates/app_core/benches/double_ma_opt.rs`.
- [ ] Add optional `VECTORBT_REQUIRE_EXACT_GPU_ROUTE` assertion path to `crates/app_core/benches/double_ma_opt.rs`.
- [ ] Add route/build-profile metadata banner to `crates/app_core/benches/dynamic_strategy_cpu.rs`.
- [ ] Add route/build-profile metadata banner to `crates/app_core/benches/double_ma_metrics.rs`.
- [ ] Add route/build-profile metadata banner to `crates/app_core/benches/double_ma_followup_workflow.rs`.
- [ ] Add route/build-profile metadata banner to `crates/app_core/benches/gpu_native_expansion_opt.rs`.
- [ ] Add route/build-profile metadata banner to `crates/app_core/benches/double_ma_validation_workflow.rs`.
- [ ] Add route/build-profile metadata banner to `crates/app_core/benches/composite_validation_workflow.rs`.
- [ ] Add route/build-profile metadata banner to `crates/app_core/benches/e2e_workflow_ml.rs`.

### Phase 3 — nightly / extended evidence pack

- [ ] Decide the nightly benchmark pack membership:
  - [ ] route matrix
  - [ ] app-path faceoff
  - [ ] validation cost
  - [ ] e2e ML
  - [ ] GPU-native expansion
  - [ ] contract split
- [ ] Add stage diagnostics to `crates/app_core/benches/e2e_workflow_ml.rs`.
- [ ] Add env-configurable train/holdout asset counts to `crates/app_core/benches/e2e_workflow_ml.rs`.
- [ ] Add strict route mode to `src-tauri/src/bin/ta_profile_route_matrix.rs`.
- [ ] Decide whether route matrix should be a nightly hard fail or only a report-producing audit.
- [ ] Add `main_validation_ratio_vs_optimization` and `cross_ticker_ratio_vs_optimization` to `ta_double_ma_validation_cost.rs`.
- [ ] Decide which validation-cost cases are cheap enough for nightly automation.
- [ ] Use app-path faceoff only as app-path evidence, not as a replacement for Criterion.

### Script smoke tests / fixture tests

- [ ] Add a clean-tree baseline bootstrap test for `scripts/run_cpu_perf_gate.ps1`.
- [ ] Add an exact-estimates-path resolution test for `scripts/run_gpu_perf_gate.ps1`.
- [ ] Add a dynamic ratio candidate-vs-baseline comparison test for `scripts/run_gpu_perf_gate.ps1`.
- [ ] Add a canonical-trial selection test for `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`.
- [ ] Add a capture-provenance sidecar test for `scripts/profile_gpu_backtest_nsys.ps1`.
- [ ] Add a capture-provenance sidecar test for `scripts/profile_gpu_backtest_ncu.ps1`.

### Rust unit / helper tests

- [ ] Extend `ta_app_path_faceoff.rs` tests to cover richer summary medians.
- [ ] Add helper tests for stage breakdown accumulation in `ta_workflow_regression.rs` if factored.
- [ ] Add helper tests for total/main/cross ratio calculation in `ta_double_ma_validation_cost.rs`.
- [ ] Add helper tests for benchmark ID parsing in `run_vectorgrid_double_ma_faceoff.ps1` if that logic is extracted into a pure helper.

### Baseline regeneration and verification

- [ ] Regenerate workflow baseline after the workflow gate fix.
- [ ] Regenerate CPU perf baseline after CPU matrix expansion.
- [ ] Regenerate GPU perf baseline after GPU exact-path and release-runner changes.
- [ ] Archive old baselines separately rather than overwriting them blindly.
- [ ] Record the old vs new schema differences in the baseline commit message.
- [ ] Confirm that all new summary JSON files include `schema_version`, `timing_boundary`, and `build_profile`.

### Final acceptance checks

- [ ] No gate reads a file path it did not explicitly request.
- [ ] No gate uses “latest matching artifact” for pass/fail logic.
- [ ] All persisted summaries identify their timing boundary.
- [ ] All persisted summaries identify build profile.
- [ ] GPU gate dynamic ratio compares candidate vs baseline ratio.
- [ ] CPU workflow gate uses release mode by default (or explicitly labels non-release mode).
- [ ] Faceoff summary no longer mixes app-path and external wall numbers in one unlabeled table.
- [ ] Nsight runs produce both the report and a provenance sidecar.
- [ ] Route/fallback metadata is visible end-to-end in at least the nightly pack.
- [ ] Per-PR gate remains small enough to be practical.
- [ ] Nightly pack covers the missing representative surfaces without duplicating the per-PR gate.
