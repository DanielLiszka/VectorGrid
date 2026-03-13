# VectorBT vs TA Desktop Demo Double-MA Capture Runbook

Date: 2026-03-12

## Goal

Record an honest side-by-side video of the app versus the VectorBT-side CPU workflow using the frozen faceoff scenario and the already measured summary artifacts.

Use these files as the source of truth:

- `scripts/vectorbt_double_ma_faceoff_scenario.json`
- `target/vectorbt_vs_vectorgrid_faceoff/faceoff_dataset_synth_200k.csv`
- `target/vectorbt_vs_vectorgrid_faceoff/summary.md`
- `target/vectorbt_vs_vectorgrid_faceoff/summary.json`
- `research/VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_FACEOFF_RESULTS_2026-03-12.md`

## What The Viewer Should See

1. Both sides clearly configured for the same dataset and the same search space.
2. Real-time playback at the beginning.
3. The app finishing at normal speed.
4. A short hold so the viewer can register that the app is already done.
5. If the clip is accelerated after that point, an explicit speed label such as `8x speed` or `20x speed`.
6. The timer visible for the full clip.
7. A final frame with both completion times and the hardware plainly listed.

Do not hide the speed-up edit. The speed label should be visually obvious.

## Frozen Scenario To Show

- Strategy: `double_ma`
- MA pair: `alma_alma`
- Objective: `Sharpe`
- Fast range: `5:200:1`
- Slow range: `10:400:1`
- Dataset: `target/vectorbt_vs_vectorgrid_faceoff/faceoff_dataset_synth_200k.csv`
- Bars: `200,000`
- Valid pairs: `58,300`

## Timings To Use In The End Frame

From `target/vectorbt_vs_vectorgrid_faceoff/summary.md`:

- VectorGrid app, GPU kernel median: `1,831.014 ms`
- VectorGrid app, CPU median: `4,487.878 ms`
- VectorBT-side CPU workflow median: `90,745.661 ms`

Hardware:

- CPU: `AMD Ryzen 9 9950X 16-Core Processor`
- GPU: `NVIDIA GeForce RTX 4090`
- RAM: `100,485,406,720` bytes

## Recommended Recording Structure

### Capture A: setup / fairness

Show:

- the frozen CSV path on both sides
- the same fast and slow ranges
- `ALMA offset=0.85`
- `ALMA sigma=6.0`
- objective `Sharpe`
- exact-search intent

### Capture B: live timed run

Left side:

- the app UI configured for the frozen scenario

Right side:

- the Python / VectorBT-side workflow terminal or notebook using the same CSV

Start both as close together as possible and keep the timer visible.

### Capture C: parity sanity

Show the final best parameters and note that the current parity result is:

- app CPU best: `fast=5`, `slow=10`
- app GPU-kernel best: `fast=5`, `slow=10`
- VectorBT-side CPU best: `fast=5`, `slow=10`
- metric deltas are small but non-zero, so say “same best parameters, materially close objective values”

## Commands For The Python / VectorBT Side

Run from the repo root:

```powershell
$py = Join-Path (Get-Location) '.venv-vectorbt-faceoff\Scripts\python.exe'
& $py scripts/vectorbt_double_ma_faceoff.py `
  --csv target/vectorbt_vs_vectorgrid_faceoff/faceoff_dataset_synth_200k.csv `
  --fast-range 5:200:1 `
  --slow-range 10:400:1 `
  --alma-offset 0.85 `
  --alma-sigma 6.0 `
  --output-json target/vectorbt_vs_vectorgrid_faceoff/capture_vectorbt.json
```

## Commands For The App CLI Evidence Path

If you need a terminal-based corroboration shot:

```powershell
target\release\ta_cli.exe `
  --backend gpu-kernel `
  --fast-ma alma `
  --slow-ma alma `
  --csv target/vectorbt_vs_vectorgrid_faceoff/faceoff_dataset_synth_200k.csv `
  --fast-period 5:200:1 `
  --slow-period 10:400:1 `
  --alma-offset 0.85 `
  --alma-sigma 6.0 `
  --top-k 10 `
  --heatmap-bins 0
```

For the CPU corroboration shot:

```powershell
target\release\ta_cli.exe `
  --backend cpu `
  --fast-ma alma `
  --slow-ma alma `
  --csv target/vectorbt_vs_vectorgrid_faceoff/faceoff_dataset_synth_200k.csv `
  --fast-period 5:200:1 `
  --slow-period 10:400:1 `
  --alma-offset 0.85 `
  --alma-sigma 6.0 `
  --top-k 10 `
  --heatmap-bins 0
```

## Editing Rules

- Keep the start in real time.
- Keep the app finish in real time.
- If you accelerate the remainder, place a clear on-screen label before or at the transition.
- Do not crop away the timer.
- Do not present the Criterion benchmark number as if it were the same measurement boundary as the side-by-side wall-clock clip.

## End Card Template

- VectorGrid app, GPU kernel: `1.831 s`
- VectorGrid app, CPU: `4.488 s`
- VectorBT-side CPU workflow: `90.746 s`
- same strategy, same search space, same dataset, same hardware

## Current Limitation

This environment produced the benchmark artifacts and the capture runbook, but it did not record the final MP4 directly. Use the frozen artifacts above when recording the manual side-by-side clip.
