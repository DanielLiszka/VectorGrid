# Optimization Workflows

Optimization searches candidate strategy configurations and ranks the results under the selected objective and constraints.

## Simple Mode Strategies

Simple Mode is release-candidate public copy until a signed desktop release ships and verifies it. For a release that promotes Simple Mode, it focuses on:

- Double MA crossover.
- Composite signal.

These are the recommended first public workflows because they provide a manageable path from data to optimization to optional validation.

## Advanced Strategy Surfaces

Advanced Mode exposes additional strategy and research surfaces. Availability and CPU/GPU support vary by route.

Public docs should describe advanced strategies as research surfaces unless a release explicitly promotes one as a primary supported workflow.

Current strategy surfaces include:

- Double MA;
- Dynamic Double MA;
- PIT;
- SET;
- MTS;
- ORC;
- MARRT;
- VFCT;
- ASR;
- VCC;
- VEMR;
- ROCB;
- VCET;
- KQSR;
- CARM;
- KVDF;
- MKCS;
- CDMC;
- Composite CPU;
- Composite Trend;
- Regime Hybrid;
- Breakout Momentum;
- Composite MO;
- ML Signal.

Some of these are strategy-specific research workflows with fixed grid behavior, no heatmap, or CPU-preferred validation.

## Backend Labels

VectorGrid uses backend labels to describe execution routes:

| Label | Meaning |
|---|---|
| `CpuOnly` | Indicators and backtest run on CPU. |
| `GpuOnly` | Indicators may run on GPU, then backtest continues on CPU. |
| `GpuKernel` | Supported route runs through a GPU kernel path. |
| `GpuHybridKernel` | Supported hybrid GPU route, often with some CPU-preferred component. |
| `Auto` | Request-time route selection rather than a separate execution backend. |

If a run says CPU, treat it as CPU even if a GPU is installed. If a run says GPU hybrid, do not present it as pure GPU execution.

## Search Controls

Common optimization controls can include:

- strategy family;
- parameter ranges;
- objective;
- backend;
- optimization mode;
- Top-K count;
- include-all results;
- heatmap bins;
- export controls;
- metric-contract controls;
- shortlist confidence and union controls where the selected route supports them.

Not every strategy supports every control. Unsupported controls should be hidden, disabled, or explained by the UI.

## Top-K Results

Top-K results keep the best-ranked candidates under the selected objective. They are useful for:

- reviewing the leading candidates;
- choosing a validation target;
- comparing nearby parameter sets;
- avoiding very large exports.

Top-K is not a guarantee that the best candidate is robust. Always review validation evidence.

## Heatmaps

Heatmaps are available on supported Double-MA-family routes. They help users see broader parameter sensitivity rather than only one best row.

Heatmaps are not available on every advanced strategy.

## Include-All And Export-All

Include-all and export-all can produce large outputs. Use them when you need broad candidate review or downstream analysis.

For large datasets and broad grids, these outputs can consume more memory, disk, and UI time than a Top-K-only run.

## Composite-Profile Routes

Composite Trend, Breakout Momentum, and Regime Hybrid are product-facing profile routes built on the Double-MA family path.

General route policy:

- Composite Trend: GPU kernel when ADX is off, GPU hybrid when ADX is on, CPU fallback.
- Breakout Momentum: GPU kernel when ADX is off, GPU hybrid when ADX is on, CPU fallback.
- Regime Hybrid: GPU hybrid in its current ADX-bearing form, CPU fallback.

Treat route labels as part of the result. Do not assume every profile uses the same GPU path.

## Composite MO

Composite MO is a CPU-side multi-objective workflow. It can use sampler families and optional prescan behavior, but its main result is Pareto-style output rather than Double-MA Top-K/heatmap/export-all output.

Use Composite MO when you are intentionally researching tradeoffs across objectives rather than running the primary Simple Mode workflow.

## ML Signal

ML Signal does not use the global optimization-mode selector. It uses a dedicated CPU research path:

- export features;
- import external scores;
- build/validate score caches;
- validate signal policies.

ML Signal outputs dataset-export, score-cache, and validation artifacts rather than normal optimizer heatmaps.

## Reading Optimization Results

When reviewing an optimization result, record:

- dataset and timeframe;
- date range;
- strategy family;
- objective;
- candidate parameters;
- backend route;
- ranked count;
- best metrics;
- warnings;
- whether validation has been run.

A strong optimization result without validation is an incomplete research artifact.

## Candidate Detail And Local Stability

Advanced Mode can expose candidate-detail, drilldown, chart, and local-stability follow-ups where the selected strategy supports them.

Use these surfaces to inspect:

- selected candidate trades;
- nearby parameter behavior;
- route-specific diagnostics;
- whether a result appears isolated or stable across adjacent settings.

Candidate detail is still historical research evidence. It should not replace validation.
