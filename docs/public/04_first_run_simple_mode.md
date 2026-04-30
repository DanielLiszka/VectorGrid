# First Run And Simple Mode

Simple Mode is the recommended public workflow. It keeps the app focused on one task: run an optimization, optionally validate the selected result, and produce a compact summary.

## What Simple Mode Supports

Simple Mode is intended for:

- bundled synthetic sample data;
- local OHLCV CSV data;
- cached Polygon.io datasets loaded through the data panel;
- Double MA crossover optimization;
- Composite signal optimization;
- optional validation after optimization;
- a compact Markdown summary for review or sharing.

Advanced strategy families and specialized controls remain available in Advanced Mode.

## Step 1: Load Data

Use one of these data sources:

- Bundled synthetic sample.
- Local CSV file.
- Polygon.io cached dataset.

For a first run, use the bundled synthetic sample. It verifies that the app can load data, run optimization, and produce output without requiring market data or provider credentials.

For real research, use a local CSV or provider-backed dataset and review data quality before trusting results.

## Step 2: Choose A Strategy

Simple Mode exposes streamlined strategy choices:

- Double MA crossover.
- Composite signal.

Double MA is the best first workflow because its optimization and validation routes are the most mature.

Composite signal is useful when you want a broader signal recipe without immediately opening every advanced control.

## Step 3: Choose Search Depth And Objective

The search depth controls how broad the candidate search is. Larger searches may take longer and may generate more output.

The objective controls the primary ranking target. Example objectives can include return/risk-oriented metrics such as Sharpe-style ranking, depending on the selected strategy and current UI options.

Do not choose candidates from a single metric without reviewing validation evidence and warnings.

## Step 4: Run Optimization

Click the optimization action and wait for the run to finish.

After completion, review:

- best candidate title;
- candidate parameter summary;
- ranked result count;
- key metrics;
- engine label, such as CPU only, GPU kernel, GPU hybrid, or Auto;
- warnings or blocked-state messages.

If no ranked candidates appear, reduce constraints, verify data, or use a smaller first-run configuration.

## Step 5: Run Validation

Validation is optional but recommended before trusting a candidate.

Validation checks the selected optimization result against additional evidence. The Simple Mode recommendation should tell you whether validation is:

- blocked because no optimization result exists;
- recommended because optimization is complete but validation has not run;
- passed;
- in review;
- rejected or failed.

Validation can still pass on historical evidence and fail in live markets. Treat it as research evidence, not deployment approval.

## Step 6: Save Or Copy The Summary

Simple Mode can produce a Markdown summary containing:

- dataset label;
- timeframe;
- row count;
- date/range label when available;
- strategy;
- objective;
- search depth;
- engine;
- optimization status;
- best candidate;
- validation recommendation;
- evidence row count;
- validation engine label;
- warnings.

This summary is useful for notes, issue reports, research logs, and comparing runs. Review it before sharing because it may include ticker symbols, dataset names, parameters, timestamps, and local context.

## When To Switch To Advanced Mode

Switch to Advanced Mode when you need:

- a strategy not available in Simple Mode;
- explicit backend controls;
- heatmap or export-all controls;
- advanced validation modules;
- report exports;
- saved artifact history;
- cross-dataset validation;
- ML-signal workflows;
- options-expression workflows;
- live or paper execution workflows.

Simple Mode should remain the main public path even for advanced users because it is the fastest way to confirm the app works end to end.

