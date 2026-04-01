# Bundle 7 addendum / v2 improvements

This document is an addendum to `bundle7_benchmark_harness_impl_plan.md`.

It covers:

- items I would add to the original plan
- a few places where the original plan should be tightened before implementation
- optional performance/correctness upgrades that go beyond the strict bundle-7 prompt but still fit your stated goal: **better performance evidence, better optimization guidance, and better correctness without removing features or breaking functionality**

This addendum stays grounded in the same Bundle 7 surface from `gpt54_feature_review_map.md`:

- benchmark harnesses
- perf gates
- timing-boundary fidelity
- route/fallback evidence
- regression safety

---

## 1. What I would add or tighten from the first plan

I did miss a few worthwhile things, and I would tighten a couple of my earlier snippets.

The biggest additions are:

1. **remove measurement contamination from env-var mutation inside Criterion iterations**
2. **use the canonical trial for faceoff summary rows, not only for parity checks**
3. **add baseline compatibility validation before pass/fail comparisons**
4. **add optional repeat/median support to route-matrix runs**
5. **capture more benchmark-shape metadata for contract-split and dynamic-mode benches**
6. **add stage-specific regression budgets on workflow timing, not only total runtime**

Those are the highest-value v2 additions.

---

## 2. Important missed item: env-var mutation is inside some Criterion iterations

### Why this matters

In `double_ma_opt.rs`, several helper wrappers mutate process environment variables and restore them inside closures that are called by `b.iter(...)`. That means the measured time includes:

- environment mutation
- environment restoration
- the actual work

That is not fatal for large workloads, but it is still timing-boundary contamination, and it is avoidable. It especially matters for smaller cases and for benches whose purpose is to compare two execution shapes closely.

The main wrappers are:

- `run_with_dynamic_strategy_opt_env(...)`
- `run_with_finalist_recompute_env(...)`
- `run_with_product_reduced_rows_env(...)`

These are used in the dynamic-preflight and contract-split surfaces in [`double_ma_opt.rs`](sandbox:/mnt/data/double_ma_opt.rs). fileciteturn7file1

### Recommended change

Move env-setting outside the measured loop whenever possible.

There are two safe patterns:

1. **set env once before `bench_function`, restore after `group.finish()`** when one benchmark/group always uses one fixed mode
2. **use an RAII-style guard created outside `b.iter(...)`** when you need per-benchmark isolation but not per-iteration toggling

### Recommended Rust helper

```rust
struct EnvVarGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let prev = std::env::var(key).ok();
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        Self { key, prev }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.prev.as_deref() {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}
```

### Example fix for dynamic-preflight benchmark variants

Instead of doing this inside every iteration:

```rust
group.bench_function(&name, |b| {
    b.iter(|| {
        let req2 = req.clone();
        run_with_dynamic_strategy_opt_env(dynamic_enabled, || {
            let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
                req2, candles, &cancel, None,
            )
            .expect("gpu kernel dynamic-preflight strategy profile run");
        });
    })
});
```

Do this:

```rust
group.bench_function(&name, |b| {
    let _dynamic_guard = if dynamic_enabled {
        EnvVarGuard::set("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", Some("1"))
    } else {
        EnvVarGuard::set("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", None)
    };

    b.iter(|| {
        let req2 = req.clone();
        let _ = ta_app_core::run_double_ma_optimization_blocking_with_candles(
            req2,
            candles,
            &cancel,
            None,
        )
        .expect("gpu kernel dynamic-preflight strategy profile run");
    })
});
```

### Why this is safe

- it does not change product code paths
- it does not remove any benchmark mode
- it reduces harness overhead inside the Criterion boundary
- it makes the timing closer to the actual route under test

### Priority

**P1**. I would do this before trusting fine-grained comparisons within the `double_ma_opt.rs` dynamic/contract-split surfaces.

---

## 3. Important missed item: canonical trial should drive summary rows too

### Why this matters

In `run_vectorbt_vs_vectorgrid_faceoff.ps1`, my first plan correctly moved parity comparison toward a canonical trial, but I did **not** fully tighten the summary-row construction.

The current script groups measured artifacts and computes min/median/max timing, but summary-row metadata like:

- `best`
- `backend_route`
- `pair_count`
- `bars`

is currently sourced from the first item in the group, not from a canonical representative row in [`run_vectorbt_vs_vectorgrid_faceoff.ps1`](sandbox:/mnt/data/run_vectorbt_vs_vectorgrid_faceoff.ps1).

If trial artifacts are all deterministic, this is harmless. If any trial-level jitter or metadata drift exists, the row becomes internally inconsistent: timing stats come from the whole group, but descriptive fields come from a single arbitrary artifact.

### Recommended change

Use the canonical median-time trial as the source of row-level descriptive fields.

### Corrected helper

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

### Corrected summary-row construction

```powershell
foreach ($group in $grouped) {
  $items = @($group.Group)
  $stats = Get-TrialStats -Trials $items
  $canonical = Get-CanonicalTrial -Items $items

  $appPathStats = $null
  if ($canonical.tool -eq "vectorgrid" -and $canonical.timing_boundary -eq "app-path") {
    $appPathStats = Get-AppPathTimingStats -Trials $items
  }

  $summaryRows.Add([ordered]@{
    tool = $canonical.tool
    backend = $canonical.backend
    timing_boundary = $canonical.timing_boundary
    build_profile = $canonical.build_profile
    min_ms = $stats.min_ms
    median_ms = $stats.median_ms
    max_ms = $stats.max_ms
    app_path_harness_median_ms = if ($appPathStats) { $appPathStats.harness_wall_median_ms } else { $null }
    app_path_state_lookup_median_ms = if ($appPathStats) { $appPathStats.state_lookup_median_ms } else { $null }
    app_path_engine_wall_median_ms = if ($appPathStats) { $appPathStats.engine_wall_median_ms } else { $null }
    app_path_engine_runtime_median_ms = if ($appPathStats) { $appPathStats.engine_runtime_median_ms } else { $null }
    pair_count = $canonical.pair_count
    bars = $canonical.bars
    pair_bars = $canonical.pair_bars
    backend_route = if ($canonical.tool -eq "vectorgrid") { $canonical.backend_route } else { "cpu_python_workflow" }
    best = $canonical.best
    canonical_trial_path = $canonical.stdout_path
    trial_paths = @($items | ForEach-Object { $_.stdout_path })
  })
}
```

### Priority

**P1**. This is a direct quality fix for faceoff evidence.

---

## 4. Important missed item: baseline compatibility must be validated before comparing numbers

### Why this matters

Once you add more metadata to summaries, you should also **refuse invalid comparisons**. Otherwise, a candidate run can be compared against an incompatible baseline and still produce a misleading pass/fail.

The gates should fail early when any of these differ unexpectedly:

- `schema_version`
- `timing_boundary`
- `build_profile`
- `bench_filter` / case identity
- GPU model / driver (for GPU gates)
- workflow pack identity (for workflow gate)
- optional `case_registry_version`

This is especially important once the plan broadens CPU/GPU gate matrices and adds release-mode workflow timing.

### Recommended summary additions

Add to CPU/GPU/workflow summaries:

```json
{
  "schema_version": 1,
  "case_registry_version": 1,
  "hardware": {
    "gpu": "...",
    "driver": "...",
    "cpu": "..."
  },
  "git": {
    "commit": "...",
    "dirty": false
  }
}
```

### PowerShell comparison helper

```powershell
function Assert-BaselineCompatibility {
  param(
    [Parameter(Mandatory = $true)][object]$Candidate,
    [Parameter(Mandatory = $true)][object]$Baseline,
    [Parameter(Mandatory = $true)][string]$GateName
  )

  $checks = @(
    @{ name = "schema_version"; cand = $Candidate.schema_version; base = $Baseline.schema_version },
    @{ name = "case_registry_version"; cand = $Candidate.case_registry_version; base = $Baseline.case_registry_version },
    @{ name = "timing_boundary"; cand = $Candidate.timing_boundary; base = $Baseline.timing_boundary },
    @{ name = "build_profile"; cand = $Candidate.build_profile; base = $Baseline.build_profile }
  )

  foreach ($check in $checks) {
    if ("$($check.cand)" -ne "$($check.base)") {
      throw "$GateName baseline mismatch for $($check.name): candidate='$($check.cand)' baseline='$($check.base)'"
    }
  }
}
```

### GPU-specific compatibility helper

```powershell
function Assert-GpuHardwareCompatibility {
  param(
    [Parameter(Mandatory = $true)][object]$Candidate,
    [Parameter(Mandatory = $true)][object]$Baseline
  )

  if ($null -ne $Candidate.hardware -and $null -ne $Baseline.hardware) {
    if ("$($Candidate.hardware.gpu)" -ne "$($Baseline.hardware.gpu)") {
      throw "GPU mismatch: candidate='$($Candidate.hardware.gpu)' baseline='$($Baseline.hardware.gpu)'"
    }
    if ("$($Candidate.hardware.driver)" -ne "$($Baseline.hardware.driver)") {
      Write-Warning "GPU driver mismatch: candidate='$($Candidate.hardware.driver)' baseline='$($Baseline.hardware.driver)'. Consider regenerating baseline."
    }
  }
}
```

### Priority

**P1**. This is cheap to add and prevents bogus comparisons.

---

## 5. Important missed item: route matrix should optionally support repeats and medians

### Why this matters

`ta_profile_route_matrix.rs` currently runs each case once and stores one `runtime_ms` and one `wall_ms` per case. That is fine for a route audit, but noisy for nightly regression evidence.

The route matrix is strong because it captures:

- requested backend
- backend used
- backend route
- fallback reason

in [`ta_profile_route_matrix.rs`](sandbox:/mnt/data/ta_profile_route_matrix.rs). fileciteturn7file8

But if it is going to be promoted from pure route audit to nightly performance evidence, single-shot timing is too thin.

### Recommended change

Keep the existing one-shot default, but add optional:

- `--warmup-runs`
- `--measured-runs`
- median/min/max per case

### Suggested CLI additions

```rust
fn parse_warmup_runs() -> usize {
    parse_flag("--warmup-runs")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0)
}

fn parse_measured_runs() -> usize {
    parse_flag("--measured-runs")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
}
```

### Suggested artifact structure

```rust
#[derive(Serialize)]
struct RouteMatrixCase {
    case_label: String,
    requested_backend: String,
    expected_backend_route: Option<String>,
    backend_used: String,
    backend_route: Option<String>,
    backend_fallback_reason: Option<String>,
    route_ok: bool,
    warmup_runs: usize,
    measured_runs: usize,
    runtime_ms_min: u64,
    runtime_ms_median: f64,
    runtime_ms_max: u64,
    wall_ms_min: u64,
    wall_ms_median: f64,
    wall_ms_max: u64,
    best_fast_len: u16,
    best_slow_len: u16,
    best_sharpe: f64,
    best_pnl: f64,
}
```

### Why this is optional

- for pure route auditing, one run is enough
- for nightly timing trends, medians are much better

### Priority

**P2**. Strong optional improvement.

---

## 6. Important missed item: add benchmark-shape metadata for contract-split and related variants

### Why this matters

The contract-split benchmarks in [`double_ma_opt.rs`](sandbox:/mnt/data/double_ma_opt.rs) differ by more than just the benchmark name. They also differ by env-controlled execution shape such as:

- `VECTORBT_KERNEL_FINALIST_RECOMPUTE`
- `VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS`
- `heatmap_bins`
- `include_trade_metrics`
- shortlist/score/hurdle env-derived settings

That means the benchmark ID alone is not always enough to fully reconstruct the shape that was measured. fileciteturn7file1

### Recommended change

Emit a structured metadata banner for the contract-split variants and dynamic benches.

### Suggested helper

```rust
fn emit_variant_meta(
    benchmark_id: &str,
    requested_backend: &str,
    heatmap_bins: Option<u16>,
    finalist_recompute: bool,
    product_reduced_rows: Option<bool>,
    include_trade_metrics: bool,
    score_mode: Option<&str>,
    shortlist_union_mode: Option<&str>,
    hurdle_tau: Option<f64>,
) {
    println!(
        concat!(
            "bench_variant_meta ",
            "benchmark_id={} requested_backend={} build_profile=bench ",
            "heatmap_bins={} finalist_recompute={} product_reduced_rows={} ",
            "include_trade_metrics={} score_mode={} shortlist_union_mode={} hurdle_tau={}"
        ),
        benchmark_id,
        requested_backend,
        heatmap_bins.map(|v| v.to_string()).unwrap_or_else(|| "<none>".to_string()),
        finalist_recompute,
        product_reduced_rows.map(|v| v.to_string()).unwrap_or_else(|| "<none>".to_string()),
        include_trade_metrics,
        score_mode.unwrap_or("<none>"),
        shortlist_union_mode.unwrap_or("<none>"),
        hurdle_tau.map(|v| format!("{v:.6}")).unwrap_or_else(|| "<none>".to_string()),
    );
}
```

### Priority

**P2**. This improves interpretability with almost no product risk.

---

## 7. Additional in-scope upgrade: stage-specific workflow budgets

### Why this matters

The original plan adds stage breakdown to `ta_workflow_regression.rs`, which is the right first step. I would go one step further and add **optional stage budgets** in candidate mode.

Right now the workflow gate is effectively:

- quality metrics vs baseline
- total runtime <= 1.5x baseline

That can hide a case where total runtime still passes but one stage regresses badly while another stage gets faster.

This matters because `ta_workflow_regression.rs` is already a realistic workflow surface in [`ta_workflow_regression.rs`](sandbox:/mnt/data/ta_workflow_regression.rs). fileciteturn7file11

### Recommended change

Add optional checks such as:

- optimize+validate stage budget
- meta-select stage budget
- holdout stage budget

### Suggested summary shape

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
struct WorkflowStageBreakdown {
    optimize_validate_ms: u64,
    meta_select_ms: u64,
    holdout_ms: u64,
}
```

### Suggested candidate-mode gate checks

```powershell
$stageBudget = 1.0 + 0.25  # 25% stage budget, example

$optStagePass = ([double]$summary.workflow.stage.optimize_validate_ms) -le \
  (([double]$baseline.workflow.stage.optimize_validate_ms) * $stageBudget + 1.0)
Write-PassFailLine "workflow_optimize_validate_stage_not_regressed_25pct" $optStagePass \
  "$($summary.workflow.stage.optimize_validate_ms) ms vs baseline $($baseline.workflow.stage.optimize_validate_ms) ms"

$metaStagePass = ([double]$summary.workflow.stage.meta_select_ms) -le \
  (([double]$baseline.workflow.stage.meta_select_ms) * $stageBudget + 1.0)
Write-PassFailLine "workflow_meta_stage_not_regressed_25pct" $metaStagePass \
  "$($summary.workflow.stage.meta_select_ms) ms vs baseline $($baseline.workflow.stage.meta_select_ms) ms"

$holdoutStagePass = ([double]$summary.workflow.stage.holdout_ms) -le \
  (([double]$baseline.workflow.stage.holdout_ms) * $stageBudget + 1.0)
Write-PassFailLine "workflow_holdout_stage_not_regressed_25pct" $holdoutStagePass \
  "$($summary.workflow.stage.holdout_ms) ms vs baseline $($baseline.workflow.stage.holdout_ms) ms"
```

### Priority

**P2**. Optional, but highly useful once stage breakdown exists.

---

## 8. Additional in-scope upgrade: normalized throughput metrics

### Why this matters

Raw wall time is the primary metric, but some of these harnesses would become much more decision-useful if they also emitted normalized throughput.

This is especially true for:

- `ta_double_ma_validation_cost.rs`
- `ta_profile_route_matrix.rs`
- `ta_app_path_faceoff.rs`

Those files already measure real work, but the summaries could better answer: “is this actually more efficient per unit of work?”

### Recommended normalized metrics

#### For route/app-path surfaces

- `pairs_per_sec`
- `pair_bars_per_sec`

#### For validation-cost surface

- `wall_ms_per_candidate`
- `wall_ms_per_approved_candidate`
- `main_validation_wall_ms_per_candidate`
- `cross_ticker_wall_ms_per_dataset`

### Suggested `ta_double_ma_validation_cost.rs` additions

```rust
#[derive(Serialize)]
struct ValidationCaseSummary {
    // existing fields...
    wall_ms_per_candidate: Option<f64>,
    wall_ms_per_approved_candidate: Option<f64>,
    main_validation_wall_ms_per_candidate: Option<f64>,
    cross_ticker_wall_ms_per_dataset: Option<f64>,
}
```

```rust
let wall_ms_per_candidate = agg.last_candidate_count
    .filter(|count| *count > 0)
    .map(|count| avg_wall_ms as f64 / count as f64);

let wall_ms_per_approved_candidate = agg.last_approved_count
    .filter(|count| *count > 0)
    .map(|count| avg_wall_ms as f64 / count as f64);
```

### Priority

**P2**. Useful for tuning without changing any core feature.

---

## 9. Additional in-scope upgrade: cold-vs-warm app-path mode split

### Why this matters

`ta_app_path_faceoff.rs` already does a good job for the **warm same-thread app-path** boundary and records:

- harness wall
- state lookup
- engine wall
- engine runtime

That is already one of the strongest harnesses in the bundle in [`ta_app_path_faceoff.rs`](sandbox:/mnt/data/ta_app_path_faceoff.rs). fileciteturn7file9

But there is still one optional improvement that would make it more useful: explicitly support two modes:

- `warm_state`
- `cold_reload`

That avoids overloading one harness to represent two different user experiences.

### Suggested CLI extension

```rust
#[derive(ValueEnum, Clone, Debug)]
enum TimingModeArg {
    WarmState,
    ColdReload,
}

#[derive(Parser, Debug)]
struct Cli {
    // existing args...
    #[arg(long, value_enum, default_value_t = TimingModeArg::WarmState)]
    timing_mode: TimingModeArg,
}
```

### Suggested cold trial behavior

```rust
fn run_trial_cold(
    cli: &Cli,
    trial_index: usize,
) -> Result<TrialArtifact, String> {
    let cache_dir = cli
        .cache_dir
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join("ta_desktop_demo_faceoff_cache"));

    let total_started = Instant::now();
    let state = AppState::default();

    let load_started = Instant::now();
    let load = state.load_price_data(
        cli.csv.to_str().ok_or_else(|| "invalid csv path".to_string())?,
        &cache_dir,
    )?;
    let load_ms = load_started.elapsed().as_secs_f64() * 1000.0;

    let data_id = if let Some(raw_tf) = cli.timeframe.as_deref() {
        state.resample_loaded_data(&load.id, raw_tf)?.id
    } else {
        load.id.clone()
    };

    let req = make_request(cli, &data_id);
    let candles = state.get_candles(&data_id)?;
    let cancel = AtomicBool::new(false);

    let engine_started = Instant::now();
    let result = ta_app_core::run_double_ma_optimization_blocking_with_candles(
        req,
        candles.as_ref(),
        &cancel,
        None,
    )?;
    let engine_wall_ms = engine_started.elapsed().as_secs_f64() * 1000.0;
    let total_wall_ms = total_started.elapsed().as_secs_f64() * 1000.0;

    // extend timing artifact with load_ms when in cold mode
    // ...

    Ok(TrialArtifact {
        trial_index,
        runtime_ms_wall: total_wall_ms,
        timing: TrialTimingArtifact {
            total_wall_ms,
            harness_wall_ms: total_wall_ms,
            state_lookup_ms: load_ms,
            engine_wall_ms,
            engine_runtime_ms: result.runtime_ms,
            gpu_kernel_route_wall_ms: result.gpu_kernel_timing_ms.as_ref().map(|t| t.route_wall_ms),
            gpu_kernel_gpu_indicator_prep_ms: result.gpu_kernel_timing_ms.as_ref().and_then(|t| t.gpu_indicator_prep_ms),
            gpu_kernel_h2d_upload_ms: result.gpu_kernel_timing_ms.as_ref().and_then(|t| t.h2d_upload_ms),
            gpu_kernel_launch_submit_us: result.gpu_kernel_timing_ms.as_ref().and_then(|t| t.launch_submit_us),
            gpu_kernel_d2h_copy_ms: result.gpu_kernel_timing_ms.as_ref().and_then(|t| t.d2h_copy_ms),
            gpu_kernel_host_decode_ms: result.gpu_kernel_timing_ms.as_ref().and_then(|t| t.host_decode_ms),
            hybrid_planner_ms: result.hybrid_timing_ms.as_ref().map(|t| t.planner_ms),
            hybrid_cpu_indicator_prep_ms: result.hybrid_timing_ms.as_ref().map(|t| t.cpu_indicator_prep_ms),
            hybrid_gpu_indicator_prep_ms: result.hybrid_timing_ms.as_ref().map(|t| t.gpu_indicator_prep_ms),
            hybrid_h2d_upload_ms: result.hybrid_timing_ms.as_ref().map(|t| t.h2d_upload_ms),
            hybrid_kernel_eval_ms: result.hybrid_timing_ms.as_ref().map(|t| t.kernel_eval_ms),
        },
        result,
    })
}
```

### Priority

**P3 optional**. Good for nightly/manual analysis; not needed for the first gate fixes.

---

## 10. Additional in-scope upgrade: add a follow-up workflow performance surface to nightly

### Why this matters

`double_ma_followup_workflow.rs` is not part of the current gate core, but it is a useful performance surface for:

- cached vs cold follow-up work
- drilldown + local stability reuse
- follow-up cache effectiveness

in [`double_ma_followup_workflow.rs`](sandbox:/mnt/data/double_ma_followup_workflow.rs). fileciteturn7file2

If follow-up workflows are important in actual usage, they deserve at least nightly visibility.

### Recommended change

Do **not** add this to the per-PR gate yet. Add it to the nightly pack after the P0/P1 fixes stabilize.

Suggested nightly cases:

- `double_ma_followup/cpu/50k/drilldown_then_local_stability_cold`
- `double_ma_followup/cpu/50k/drilldown_then_local_stability_cached`
- `double_ma_followup/gpu_kernel/50k/drilldown_then_local_stability_cached` (when CUDA path is available)

### Optional summary metric

```text
followup_cache_speedup = cold_median_ms / cached_median_ms
```

### Priority

**P3 optional**.

---

## 11. Optional safe change: make Nsight scripts restore environment in `try/finally`

### Why this matters

Both Nsight entry scripts set several environment variables and then restore them afterward, but the restoration is not currently guarded by `try/finally`. If the profiling command throws early, env restoration can be skipped.

This is a correctness and operability issue in:

- [`profile_gpu_backtest_nsys.ps1`](sandbox:/mnt/data/profile_gpu_backtest_nsys.ps1)
- [`profile_gpu_backtest_ncu.ps1`](sandbox:/mnt/data/profile_gpu_backtest_ncu.ps1)

### Recommended change

Wrap the profiling invocation and report resolution in `try/finally`.

### Suggested pattern

```powershell
try {
  $exitCode = Invoke-NsysProfileCommand -ExePath $nsysExe -CommandArgs $nsysArgs -WorkingDirectory $nsysWorkDir

  # existing report handling here
}
finally {
  [System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_FAST_PATH", $prevFastPath, "Process")
  [System.Environment]::SetEnvironmentVariable("VECTORBT_KERNEL_USE_CUDA_GRAPHS", $prevGraphs, "Process")
  [System.Environment]::SetEnvironmentVariable("VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL", $prevDynamic, "Process")
  [System.Environment]::SetEnvironmentVariable("VECTORBT_GPU_NATIVE_BENCH_ONLY", $prevBenchOnly, "Process")
  [System.Environment]::SetEnvironmentVariable("VECTORBT_ENABLE_NVTX", $prevNvtx, "Process")
  [System.Environment]::SetEnvironmentVariable("NSYS_NVTX_PROFILER_REGISTER_ONLY", $prevNsysNvtxRegisterOnly, "Process")
}
```

### Priority

**P1** for reliability.

---

## 12. Optional safe change: gate instability policy

### Why this matters

The GPU gate already warns when Criterion confidence interval width is large. That is good. I would optionally formalize a simple instability policy instead of only printing a warning.

### Recommended change

Introduce a policy like:

- CI width <= 10%: normal
- CI width 10–20%: warning + optional one automatic rerun
- CI width > 20%: fail as unstable evidence, not as perf regression

### Suggested helper

```powershell
function Get-CiWidthPct {
  param(
    [double]$Mean,
    [double]$Lower,
    [double]$Upper
  )

  if ($Mean -le 0.0) { return 0.0 }
  return (($Upper - $Lower) / $Mean) * 100.0
}
```

```powershell
$ciWidthPct = Get-CiWidthPct -Mean $summary.criterion_mean_ms -Lower $summary.criterion_mean_ci_lower_ms -Upper $summary.criterion_mean_ci_upper_ms
if ($ciWidthPct -gt 20.0) {
  throw "GPU perf evidence unstable: CI width is $([math]::Round($ciWidthPct, 2))%"
}
```

### Why this helps

It keeps a noisy machine from generating false performance conclusions.

### Priority

**P3 optional**.

---

## 13. Optional safe change: benchmark manifest / registry file

This goes slightly beyond the strict prompt, but it is one of the cleanest long-term improvements.

### Why this matters

Right now benchmark identity is partly encoded in:

- PowerShell script params
- hard-coded estimate paths
- benchmark IDs
- implicit expectations in human reasoning

A small manifest makes the whole system easier to keep correct.

### Suggested file

`benchmarks/perf_case_registry.json`

### Suggested shape

```json
{
  "schema_version": 1,
  "cases": [
    {
      "name": "cpu_sma_small",
      "timing_boundary": "Criterion",
      "build_profile": "bench",
      "bench_file": "double_ma_opt",
      "bench_filter": "double_ma/cpu/sma_sma/200k/small",
      "estimates_json": "target/criterion/double_ma_cpu_sma_sma_200k/small_1590pairs/new/estimates.json",
      "requested_backend": "CpuOnly",
      "expected_route": "CpuOnly",
      "regression_pct": 15.0,
      "tier": "pr"
    },
    {
      "name": "gpu_alma_large",
      "timing_boundary": "Criterion",
      "build_profile": "bench",
      "bench_file": "double_ma_opt",
      "bench_filter": "double_ma/gpu_kernel/alma_alma/200k/large",
      "estimates_json": "target/criterion/double_ma_gpu_kernel_alma_alma_200k/large_58300pairs/new/estimates.json",
      "requested_backend": "GpuKernel",
      "expected_route": "GpuKernel",
      "regression_pct": 10.0,
      "tier": "nightly"
    }
  ]
}
```

### Why this is worth it

- better determinism
- clearer case ownership
- easier baseline invalidation when a case changes
- easier per-PR vs nightly split

### Priority

**P3 optional**, but high leverage.

---

## 14. Optional safe change: add environment fingerprint to all summary artifacts

This also goes a bit beyond the prompt, but it is extremely useful for benchmark trust.

### Suggested fields

```json
{
  "producer": {
    "hostname": "...",
    "os": "...",
    "rustc": "...",
    "cargo": "...",
    "git_commit": "...",
    "git_dirty": false
  },
  "hardware": {
    "cpu": "...",
    "gpu": "...",
    "driver": "..."
  }
}
```

### Why this helps

It makes baseline mismatches easier to diagnose and gives you a much better answer to “is this slowdown real or just a different machine / toolchain / driver?”

### Priority

**P3 optional**.

---

## 15. Corrections to the first plan’s code snippets

These are the concrete places where I would update the original document before implementation.

### Correction A — GPU estimate path should be passed per case

In the original GPU gate patch, I used a single outer `$CriterionEstimatesPath`. If you expand the gate into a multi-case matrix, that is too loose.

Use this instead:

```powershell
function Invoke-GpuKernelBench {
  param(
    [Parameter(Mandatory = $true)][string]$BenchFilter,
    [Parameter(Mandatory = $true)][string]$CriterionEstimatesPath,
    [Parameter(Mandatory = $true)][bool]$DynamicStrategyEnabled,
    [string]$EstimateSnapshotPath = ""
  )

  Wait-NoPerfProcesses -TimeoutSeconds 120 -Context "before GPU benchmark launch"

  if ($DynamicStrategyEnabled) {
    $env:VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL = "1"
  } else {
    Remove-Item Env:VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL -ErrorAction SilentlyContinue
  }

  cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- $BenchFilter
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }

  if (!(Test-Path $CriterionEstimatesPath)) {
    throw "Expected exact GPU Criterion estimates file was not written: $CriterionEstimatesPath"
  }

  if (-not [string]::IsNullOrWhiteSpace($EstimateSnapshotPath)) {
    $snapshotDir = Split-Path -Parent $EstimateSnapshotPath
    if ($snapshotDir) {
      New-Item -ItemType Directory -Force -Path $snapshotDir | Out-Null
    }
    Copy-Item -Force $CriterionEstimatesPath $EstimateSnapshotPath
  }

  $criterion = Get-Content -Raw $CriterionEstimatesPath | ConvertFrom-Json
  return [ordered]@{
    timing_boundary = "Criterion"
    build_profile = "bench"
    estimate_path = $CriterionEstimatesPath
    mean_ms = [math]::Round(([double]$criterion.mean.point_estimate) / 1000000.0, 6)
    mean_ci_lower_ms = [math]::Round(([double]$criterion.mean.confidence_interval.lower_bound) / 1000000.0, 6)
    mean_ci_upper_ms = [math]::Round(([double]$criterion.mean.confidence_interval.upper_bound) / 1000000.0, 6)
    median_ms = [math]::Round(([double]$criterion.median.point_estimate) / 1000000.0, 6)
  }
}
```

### Correction B — faceoff summary row source should be canonical, not first

Covered in Section 3 above.

### Correction C — add compatibility checks before baseline comparisons

Covered in Section 4 above.

---

## 16. Suggested revised priority order

If I were updating the original plan today, I would use this order.

### P0

1. `run_workflow_gate.ps1` baseline/output-path fix
2. `run_cpu_perf_gate.ps1` workflow-path fix
3. `run_gpu_perf_gate.ps1` deterministic estimate selection + ratio-vs-baseline fix
4. `run_gpu_perf_gate.ps1` dynamic runner release-mode execution
5. `ta_workflow_regression.rs` schema/build-profile/stage output

### P1

6. move env-var toggles outside Criterion iterations in `double_ma_opt.rs`
7. add baseline compatibility validation in CPU/GPU/workflow gates
8. add Nsight sidecar provenance and `try/finally` env restoration
9. update faceoff summary rows to use canonical trials
10. add richer route/fallback/build-profile metadata to app-path / route-matrix / validation-cost outputs

### P2

11. add representative ALMA case to CPU/GPU per-PR or nightly gate as planned
12. add contract-split variant metadata banners
13. add route-matrix optional repeats/medians
14. add stage-specific workflow budgets
15. add normalized throughput metrics to validation-cost/app-path/route-matrix

### P3

16. add warm-vs-cold mode split to `ta_app_path_faceoff.rs`
17. add nightly follow-up workflow surface
18. add instability/rerun policy
19. add manifest-based case registry
20. add environment fingerprinting

---

## 17. Implementation checklist addendum

Use this together with the original checklist.

### High-priority corrections and additions

- [ ] In `crates/app_core/benches/double_ma_opt.rs`, remove env-var mutation from inside measured Criterion iterations where the mode is fixed for the whole benchmark.
- [ ] Add an `EnvVarGuard` helper or equivalent pattern so benchmark-mode env changes are scoped outside `b.iter(...)`.
- [ ] In `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`, use the canonical median-time trial for summary-row metadata, not only for parity.
- [ ] Add `canonical_trial_path` to faceoff summary rows.
- [ ] In the GPU gate implementation, pass exact estimates-path per case instead of relying on a single outer estimates-path variable.
- [ ] Add baseline compatibility validation before all pass/fail comparisons.
- [ ] Fail or warn explicitly on `schema_version`, `timing_boundary`, `build_profile`, and case-registry mismatches.
- [ ] Add GPU hardware compatibility checking for GPU baselines.

### Route-matrix improvements

- [ ] Decide whether `ta_profile_route_matrix.rs` stays one-shot only or gets optional repeat/median support.
- [ ] If repeat support is added, add `--warmup-runs` and `--measured-runs`.
- [ ] If repeat support is added, emit min/median/max for runtime and wall time per case.
- [ ] Keep `--strict` route mode even if repeat support is added.

### Contract-split and benchmark-shape metadata

- [ ] Add contract-split benchmark metadata banners for `heatmap_bins`, `finalist_recompute`, `product_reduced_rows`, `include_trade_metrics`, shortlist mode, score mode, and hurdle tau.
- [ ] Confirm that every gate-relevant benchmark case can be reconstructed from logs and/or persisted JSON without guessing hidden env modes.

### Workflow-regression improvements

- [ ] After adding stage timings to `ta_workflow_regression.rs`, decide whether to add stage-specific candidate-vs-baseline budgets.
- [ ] If stage budgets are added, wire them into `run_workflow_gate.ps1` and/or `run_cpu_perf_gate.ps1`.
- [ ] Keep total-runtime gating even if stage gates are added.

### Validation-cost and throughput metrics

- [ ] Add `wall_ms_per_candidate` to `ta_double_ma_validation_cost.rs`.
- [ ] Add `wall_ms_per_approved_candidate` to `ta_double_ma_validation_cost.rs`.
- [ ] Add `main_validation_wall_ms_per_candidate` to `ta_double_ma_validation_cost.rs`.
- [ ] Decide whether `cross_ticker_wall_ms_per_dataset` is useful enough to include.
- [ ] Add `pairs_per_sec` and/or `pair_bars_per_sec` where relevant for app-path or route-matrix artifacts.

### App-path optional split

- [ ] Decide whether `ta_app_path_faceoff.rs` should gain a `warm_state` vs `cold_reload` mode split.
- [ ] If yes, preserve the current warm same-thread boundary exactly as-is and add cold mode as a separate explicit boundary.
- [ ] Do not silently redefine the current app-path metric.

### Follow-up workflow optional nightly surface

- [ ] Decide whether `double_ma_followup_workflow.rs` should be added to the nightly pack.
- [ ] If promoted, emit route/build-profile/cache-mode metadata.
- [ ] If promoted, add a cached-vs-cold speedup metric to the nightly summary.

### Nsight hardening

- [ ] Wrap env restoration in `try/finally` in both Nsight entry scripts.
- [ ] Keep sidecar provenance JSON even when profiler exit code is non-zero but report generation succeeded.
- [ ] Decide whether a missing-kernel-match condition should remain warning-only or become an explicit failure when filters are enabled.

### Optional operational hardening

- [ ] Decide whether to implement a CI-width instability policy.
- [ ] Decide whether to add one automatic rerun for unstable GPU Criterion evidence.
- [ ] Decide whether to add a case manifest / registry file.
- [ ] Decide whether to add environment fingerprint fields (git/toolchain/hardware).
- [ ] Decide whether baselines should be auto-invalidated on manifest or hardware mismatch.

### Final validation before rollout

- [ ] Re-read the original implementation plan and apply these v2 corrections to any snippet you copy into the repo.
- [ ] Confirm that no new measurement field silently changes an existing timing boundary.
- [ ] Confirm that all new fields are additive and do not break existing consumers.
- [ ] Confirm that no feature or execution path is removed while implementing the measurement/gating changes.
- [ ] Regenerate baselines only after schema, route metadata, and build-profile fields are finalized.

