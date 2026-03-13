# VectorBT vs TA Desktop Demo Double-MA Faceoff Results

Date: 2026-03-12

## Frozen Scenario

- Scenario file: `scripts/vectorbt_double_ma_faceoff_scenario.json`
- Strategy: plain `double_ma`
- MA pair: `alma_alma`
- Objective: `Sharpe`
- Fast range: `5:200:1`
- Slow range: `10:400:1`
- Valid pairs: `58,300`
- Bars: `200,000`
- Pair-bars: `11,660,000,000`
- Timed run settings:
  - warmup: `1`
  - measured trials: `5`
  - app `top_k`: `10`
  - app `heatmap_bins`: `0`

## Frozen Dataset

- CSV: `target/vectorbt_vs_vectorgrid_faceoff/faceoff_dataset_synth_200k.csv`
- Metadata: `target/vectorbt_vs_vectorgrid_faceoff/faceoff_dataset_synth_200k.meta.json`
- Dataset id: `double_ma_faceoff_synth_200000_v1`
- SHA-256: `7ec6dd8151c6dbc922657b8471ccb9b5c7337123e4ff3b721eb27fe0f12ed640`

The shared dataset is the deterministic synthetic benchmark-harness export. Both the app and the Python/VectorBT-side workflow were run against the same CSV.

## Environment

- CPU: `AMD Ryzen 9 9950X 16-Core Processor`
- GPU: `NVIDIA GeForce RTX 4090`
- RAM: `100,485,406,720` bytes
- Python: `3.10.6`
- VectorBT: `0.28.4`
- NumPy: `2.2.6`
- Pandas: `2.3.3`
- Numba: `0.64.0`

The Python environment was frozen with:

- `scripts/requirements-vectorbt-faceoff.lock.txt`

## Measured Wall-Clock Results

These are the recorded median/min wall-clock timings from:

- `target/vectorbt_vs_vectorgrid_faceoff/summary.json`
- `target/vectorbt_vs_vectorgrid_faceoff/summary.md`

| Tool | Backend | Median ms | Min ms | Pairs | Bars | Pair-bars | Best fast | Best slow | Best Sharpe | Route |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| VectorBT-side workflow | CPU | `90,745.661` | `89,682.635` | `58,300` | `200,000` | `11,660,000,000` | `5` | `10` | `0.845219932663` | `cpu_python_workflow` |
| VectorGrid app | CPU | `4,487.878` | `4,441.820` | `58,300` | `200,000` | `11,660,000,000` | `5` | `10` | `0.845256161020` | `CpuOnly` |
| VectorGrid app | GPU kernel | `1,831.014` | `1,075.852` | `58,300` | `200,000` | `11,660,000,000` | `5` | `10` | `0.845566129824` | `GpuKernel` |

Derived ratios from the measured medians:

- app `GpuKernel` vs VectorBT-side CPU workflow: about `49.56x`
- app CPU vs VectorBT-side CPU workflow: about `20.22x`
- app `GpuKernel` vs app CPU: about `2.451x`

## Parity Notes

Best-parameter parity held across the measured runs:

- app CPU best: `fast=5`, `slow=10`
- app GPU kernel best: `fast=5`, `slow=10`
- VectorBT-side CPU workflow best: `fast=5`, `slow=10`

Metric deltas were small but not zero:

- app CPU vs VectorBT-side CPU Sharpe delta: `0.000036228356`
- app CPU vs app GPU-kernel Sharpe delta: `0.000309968804`

So the current honest statement is:

- best parameters match exactly
- objective values are materially close but not bit-identical
- the comparison is suitable for an exact-search parity sanity check, with metric-semantic caveats noted

## App Benchmark Anchor

For the app-side benchmark harness, the reproducible Criterion helper now emits:

- JSON: `target/vectorgrid_double_ma_faceoff.json`
- Raw output: `target/vectorgrid_double_ma_faceoff.raw.txt`

Current rerun result:

- benchmark id: `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`
- point estimate: `373.01 ms`
- interval: `[372.66 ms, 373.37 ms]`

This is a different timing boundary than the CLI wall-clock faceoff, so it should be presented as an app benchmark anchor rather than mixed into the side-by-side wall-clock table.

## Commands Used

Shared dataset / repeated faceoff:

```powershell
$py = Join-Path (Get-Location) '.venv-vectorbt-faceoff\Scripts\python.exe'
.\scripts\run_vectorbt_vs_vectorgrid_faceoff.ps1 `
  -ScenarioPath scripts/vectorbt_double_ma_faceoff_scenario.json `
  -OutDir target/vectorbt_vs_vectorgrid_faceoff `
  -PythonExe $py
```

App benchmark anchor:

```powershell
.\scripts\run_vectorgrid_double_ma_faceoff.ps1
```

## Recording Status

The benchmark workflow, frozen dataset, repeated trials, parity summary, and publication table are complete.

The actual side-by-side video capture is still manual. The repo now contains the artifacts and runbook needed to record it honestly without changing the methodology.
