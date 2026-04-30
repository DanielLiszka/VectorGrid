# Glossary

This glossary defines common VectorGrid terms in plain language.

## Artifact

A saved local record of a workflow result. Artifacts can preserve optimization output, validation output, ML policy output, options-expression output, live-session summaries, batch summaries, and related metadata.

## Backtest

A historical simulation of a strategy over past data. A backtest is not a prediction and does not guarantee future performance.

## Backend

The execution route used by a workflow. Common labels include `CpuOnly`, `GpuOnly`, `GpuKernel`, `GpuHybridKernel`, and `Auto`.

## Candidate

One tested strategy configuration. For example, a moving-average strategy candidate may include fast and slow moving-average periods plus other settings.

## CPU-Only

Execution that does not require NVIDIA GPU support. CPU-only is the broadest compatibility path.

## CSV

Comma-separated values file. VectorGrid uses CSV files for local OHLCV data and some exported/imported workflow data.

## CSCV/PBO

Combinatorially symmetric cross-validation / probability of backtest overfitting style evidence. It helps estimate overfit risk, but it is not a final deployment verdict.

## Dataset

The loaded historical bars used for optimization or validation. Dataset identity, timeframe, range, and quality matter.

## Double MA

Double moving-average crossover strategy family. This is the clearest first-run optimization workflow.

## Dry Run

An execution mode that checks request flow without submitting real broker orders.

## GPU Kernel

A GPU-backed execution route where a supported workflow runs through a dedicated kernel path.

## GPU Hybrid

A mixed route where some work uses GPU execution and some work remains CPU-preferred or CPU-required.

## Heatmap

A visual or tabular summary of performance across parameter combinations. Heatmaps help show sensitivity beyond the single top result.

## Holdout

A data segment used to evaluate a candidate outside the exact optimization slice.

## Include-All

A setting that retains a broader set of candidate results. It can produce larger outputs and use more memory/disk.

## Local Paper

A local simulation of broker-style order state. It is useful for workflow review but is not a real broker.

## ML Signal

A research workflow for exported features, external scores, score caches, and signal-policy validation.

## OHLCV

Open, high, low, close, and volume. This is the common market-bar format for strategy research.

## Options Expression

A research workflow for evaluating options-expression candidates from supported parent artifacts and provider-backed options datasets.

## Polygon.io

The supported provider path for historical market data and stock live streams in current public docs. Access depends on the user's Polygon.io plan.

## Reoptimization Walk-Forward

A validation workflow that repeats optimization across sequential windows and evaluates the resulting behavior.

## Replay Class

The app's classification of whether a saved artifact can be reopened, replayed, repaired, or only inspected.

## Route Metadata

Information describing what actually ran, including CPU/GPU route, fallback reason, warnings, and supported module behavior.

## Simple Mode

The recommended public workflow: load data, choose a streamlined strategy, run optimization, optionally validate, and save/copy a summary.

## Top-K

The best-ranked candidate rows retained from an optimization run.

## Validation

Additional evidence after optimization. Validation can include holdout-style checks, walk-forward, robustness modules, cross-dataset reruns, and probability-style overfit indicators.

## Walk-Forward

A validation method that evaluates behavior across sequential time windows.

