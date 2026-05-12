# VectorGrid

This is an upcoming open-source release of a backtest optimization Tauri desktop application capable of near instantaneous backtest optimization execution (including indicator calculation), intended to be primarily executed on a CUDA capable GPU.

It will feature a complete workflow allowing for full backtest optimization of one of several available strategies (or one defined by the user), followed by subsequent extensive validation, analysis, and reporting of all results.

The focus is primarily on stock tickers, but backtest optimization will be possible with stock options as well.

For a basic double moving average strategy, the speedup can be up to 2,451x faster relative to VectorBT (inclusive of indicator calculation time), and only slightly slower for more complex strategies.

Currently, the backend is nearly complete, while work is still ongoing with the frontend. Operations are currently limited to a backend CLI interface at the moment.

## Quick Start

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

## Benchmarks

![VectorGrid vs VectorBT stored benchmark snapshots](assets/benchmarks/vectorbt-faceoff.svg)


## License

See [LICENSE](LICENSE).
