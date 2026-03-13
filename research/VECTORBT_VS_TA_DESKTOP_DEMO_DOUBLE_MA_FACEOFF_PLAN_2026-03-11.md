# VectorBT vs TA Desktop Demo Double-MA Faceoff Plan

Date: 2026-03-11

## Execution Status (2026-03-12)

Implemented artifacts now exist for the frozen faceoff workflow:

- scenario file: `scripts/vectorbt_double_ma_faceoff_scenario.json`
- shared dataset generator: `scripts/generate_double_ma_faceoff_dataset.py`
- repeated cross-tool runner: `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
- VectorBT-side script: `scripts/vectorbt_double_ma_faceoff.py`
- app benchmark anchor helper: `scripts/run_vectorgrid_double_ma_faceoff.ps1`
- Python lock snapshot: `scripts/requirements-vectorbt-faceoff.lock.txt`

Completed execution artifacts:

- repeated-trial summary: `target/vectorbt_vs_vectorgrid_faceoff/summary.json`
- repeated-trial Markdown table: `target/vectorbt_vs_vectorgrid_faceoff/summary.md`
- frozen dataset metadata: `target/vectorbt_vs_vectorgrid_faceoff/faceoff_dataset_synth_200k.meta.json`
- app benchmark anchor: `target/vectorgrid_double_ma_faceoff.json`

Supporting repo notes:

- results note: `research/VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_FACEOFF_RESULTS_2026-03-12.md`
- capture runbook: `research/VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_CAPTURE_RUNBOOK_2026-03-12.md`

Current status:

- methodology / scenario freeze: done
- repeated timed trials: done
- parity sanity: done, with small metric deltas documented
- publication summary table: done
- final side-by-side screen recording: still manual

## Goal

Produce a fair, screen-captured, side-by-side benchmark and product demonstration comparing:

- this app's exact `double_ma` optimization path
- `vectorbt` running an equivalent double moving-average optimization workload

The intended story is not "GPU is fast" in the abstract. The intended story is:

1. exact brute-force optimization can be made shockingly cheap
2. the app can keep exact-grid search practical at scales where CPU-only tools become visibly slower
3. the comparison is fair enough to stand up to technical scrutiny

## Primary Question

For a large exact double-MA crossover optimization workload, how much faster is this app than a CPU-only `vectorbt` workflow, and what parts of that gap come from:

- GPU-executed indicator computation
- GPU-executed backtest evaluation
- VRAM-resident candidate processing
- lower orchestration overhead

## Fairness Principle

This should be treated as a methodology-first comparison, not a marketing-only speed clip.

The faceoff should avoid:

- mismatched datasets
- mismatched parameter grids
- comparing exact search in one tool against approximation in the other
- hiding setup cost in one tool but not the other
- comparing release-mode Rust against a debug-mode Python stack

If the methodology is weak, the screen capture is not persuasive.

## Baseline Repo Evidence

The current strongest internal benchmark reference for the app is already documented in:

- `research/ARTICLE_TECHNICAL_BRIEF_APP_STACK_AND_BENCHMARKS_2026-03-10.md`

Headline local result:

- benchmark: `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`
- point estimate: about `89.383 ms`
- workload: `58,300` exact ALMA/ALMA backtests on `200,000` bars

That benchmark is the best starting anchor for the faceoff because:

- it is already exact-grid
- it is already large enough to be visually compelling
- `ALMA` is strongly GPU-favored in `gpu_speedups.csv`

## Benchmark Scenario

### Primary scenario

- strategy: plain double crossover
- MA pair: `alma_alma`
- series length: `200,000` bars
- search space: `58,300` valid `fast < slow` pairs
- objective: `Sharpe`
- exact mode only

### Candidate grid

Use the same ranges as the app's existing large ALMA benchmark family:

- fast range: `(5, 200, 1)`
- slow range: `(10, 400, 1)`

This preserves alignment with the existing benchmark corpus and keeps the pair count stable.

### Dataset

Use one frozen dataset for both tools.

Preferred order:

1. synthetic candles copied from the repo benchmark harness for maximal reproducibility
2. one frozen real market dataset as a secondary realism check

Do not use live-downloaded data during the capture.

## Comparison Matrix

### Minimum comparison set

- app exact GPU kernel path
- app CPU-only exact path
- `vectorbt` CPU-only exact path

### Optional comparison set

- app `GpuOnly`
- app `GpuHybridKernel`
- `vectorbt` best optimized CPU-only implementation variant

The minimum set is enough to show:

- what the app gains from GPU
- where `vectorbt` lands relative to the app's CPU and GPU baselines

## What To Measure

Record all of these separately:

- total wall-clock optimization time
- time to first usable top-k result, if exposed
- exact number of evaluated pairs
- bars per pair
- total pair-bars
- backend route used
- whether heatmap/top-k/export features were enabled
- peak memory, if practical

Do not reduce the comparison to only one number.

## Product Capture Plan

### Capture A: methodology setup

Show:

- same CSV or in-memory dataset loaded in both environments
- same parameter ranges
- same exact-search intent
- same objective

Purpose:

- establish fairness before the timer starts

### Capture B: live optimization faceoff

Show side by side:

- app optimization launch
- `vectorbt` optimization launch

Preferred outcome:

- both runs visible on screen at once
- monotonic timing capture
- no manual cuts during the timed portion

### Capture C: result parity sanity

Show:

- top candidate parameters from each tool
- objective metric from each tool
- whether the best candidate matches exactly or is materially close

Purpose:

- prove the comparison was exact-search versus exact-search, not "fast but wrong" versus "slow but right"

## Implementation Work Needed Before Capture

### App-side

- add or verify a reproducible benchmark runner for the exact `alma_alma/200k/58300pairs` scenario
- ensure route metadata is printed or exported
- disable unrelated overhead for the benchmark mode:
  - no CSV export
  - no artifact serialization
  - no UI-induced delay
- optionally add a dedicated CLI entry for the faceoff scenario

### VectorBT-side

- implement the equivalent exact-grid double-ALMA optimization script
- freeze dependency versions
- use a production-style CPU-only execution mode
- log:
  - total runtime
  - evaluated pair count
  - best parameters
  - best objective

## Recommended Execution Rules

- run each scenario at least `5` times after warmup
- report median and min
- note CPU model, GPU model, RAM size, Python version, and OS
- keep background load minimal
- use release-mode Rust and an optimized Python stack

## Risks

### Risk 1: unfair orchestration comparison

If one side includes setup/loading and the other does not, the result is not credible.

Mitigation:

- define timing boundaries explicitly and keep them symmetric

### Risk 2: result mismatch

Different fill, warmup, or fee semantics can invalidate the comparison.

Mitigation:

- align semantics first
- show best-parameter parity or explain any mismatch explicitly

### Risk 3: small workload hides the GPU advantage

Tiny grids can make GPU overhead dominate.

Mitigation:

- lead with the `58,300 x 200,000` case, not a toy case

### Risk 4: ALMA over-generalization

`alma_alma` is a strong GPU case, but not every strategy will scale like that.

Mitigation:

- frame it honestly as a representative GPU-favorable exact-search case
- optionally add one `sma_sma` or real-data secondary scenario

## Success Criteria

The faceoff is successful if it produces:

- one reproducible exact-search benchmark scenario
- one screen-captured side-by-side optimization run
- one parity sanity check on top results
- one concise summary table of:
  - runtime
  - pair count
  - pair-bars
  - backend route

## Execution Checklist

- [x] Freeze the exact faceoff scenario parameters.
- [x] Freeze the dataset used by both tools.
- [x] Add or verify an app-side reproducible runner for the scenario.
- [x] Build the equivalent `vectorbt` exact-grid script.
- [x] Confirm result parity or document any semantic mismatch.
- [x] Run repeated timed trials and record median/min.
- [ ] Capture the side-by-side optimization run.
- [ ] Capture the result-parity sanity check.
- [x] Write a short benchmark summary table for publication or presentation.
