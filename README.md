# VectorGrid

This is an upcoming open-source release of a backtest optimization Tauri desktop application capable of near instantaneous backtest optimization execution (including indicator calculation), intended to be primarily executed on a CUDA capable GPU.

It will feature a complete workflow allowing for full backtest optimization of one of several available strategies (or one defined by the user), followed by subsequent extensive validation, analysis, and reporting of all results.

The focus is primarily on stock tickers, but backtest optimization will be possible with stock options as well.

For a basic double moving average strategy, the stored historical warm app-path
snapshot reports about 2,452x speedup over its stored VectorBT CPU reference on
250,000 pairs, including indicator calculation. That snapshot used an AMD Ryzen
9 9950X and NVIDIA GeForce RTX 4090; current release performance gates remain
separate evidence.

The full desktop UI is included in this repository and is the primary application surface. Backend CLI commands remain available for direct engine runs, benchmarks, and lower-level validation.

## Quick Start

Build the frontend assets:

```powershell
npm --prefix web ci
npm --prefix web run build
```

Run the full Tauri desktop app:

```powershell
cargo run -p ta_desktop_demo_app --release
```

Windows packaging and the ML native fallback use the official, hash-pinned
CatBoost `v1.2.10` Windows x86-64 model-applier DLL plus its Apache-2.0 license.
Run `scripts/acquire_catboost_native.ps1` to acquire and verify the reviewed
asset. The upstream DLL is unsigned; a public VectorGrid release still requires
the signed/timestamped application and installer, exact-package QA, and the
release owner's redistribution/legal approval.

## Backend CLI

CPU-only synthetic `Double MA` run:

```powershell
cargo run -p ta_app_core --features cli --bin ta_cli -- `
  --backend cpu `
  --synth-len 1000 `
  --fast-ma alma `
  --slow-ma alma `
  --fast-period 5:15:5 `
  --slow-period 20:40:10 `
  --heatmap-bins 0 `
  --top-k 3
```

Optional GPU-kernel run:

```powershell
cargo run -p ta_app_core --release --features cli,cuda,cuda-backtest-kernel --bin ta_cli -- `
  --backend gpu-kernel `
  --synth-len 200000 `
  --fast-ma alma `
  --slow-ma alma `
  --fast-period 5:50:1 `
  --slow-period 20:200:5 `
  --heatmap-bins 0 `
  --top-k 10
```

## Terminal-only Options Lifecycle QA

The maintainer lifecycle harness exercises a bounded CPU options-expression run,
artifact list/load, tuple and executed-event replay, validation, CSV export, and
cold persisted-cache recovery without opening the desktop UI:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_options_expression_lifecycle_test.ps1
```

By default, the script reads the newest usable saved options-expression fixture
from the normal app-local data directory, copies only its required inputs into
an isolated directory under `output/qa`, and performs both phases there. Pass
`-AppLocalDataDir` or `-SeedArtifact` to select a different source fixture. The
source app-local data remains read-only, and the result is functional QA
evidence rather than performance evidence.

## Required CPU Quality

The prepared working-tree Windows workflow defines a future
`Required CPU quality` GitHub Actions job. Its local command is:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_cpu_ci_gate.ps1
```

It runs serialized CPU/Rust, frontend, headless Chromium, security-contract,
and JSON/CSV/XLSX parity checks, then produces a redacted support-evidence ZIP.
GPU/performance, native desktop, signing, provider-sandbox, and clean-machine
evidence remain separate lanes; see `docs/CI.md`.
It becomes active remote enforcement only after the workflow and supporting
files are committed and pushed and branch protection requires that check.

## Benchmarks

![VectorGrid vs VectorBT stored benchmark snapshots](assets/benchmarks/vectorbt-faceoff.svg)


## License

See [LICENSE](LICENSE).
