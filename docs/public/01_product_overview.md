# Product Overview

VectorGrid is a local Windows desktop application for trading-strategy research. It focuses on fast strategy optimization, validation evidence, and exportable local artifacts.

The app is designed for users who want to ask questions such as:

- Which parameter sets look strongest on a dataset?
- Do the best optimization candidates survive additional validation checks?
- How do candidate rankings change under different strategy families, objectives, or validation settings?
- Can the same workflow run on CPU-only machines and, where supported, on NVIDIA GPU-backed paths?

VectorGrid does not decide whether you should trade. It produces research evidence that still requires independent review.

Simple Mode language on this page applies only to a signed desktop release that implements, tests, and verifies Simple Mode as the shipped first-run surface. The current checked-in frontend defaults to the Advanced workbench, so these Simple Mode sections are release-candidate public copy rather than current repository run instructions.

## Core Workflow

The public first-run workflow is intentionally simple:

1. Load a bundled synthetic sample, a local OHLCV CSV, or a cached Polygon.io dataset.
2. Choose a strategy and objective.
3. Run optimization.
4. Review the best-ranked candidate and supporting metrics.
5. Optionally run validation.
6. Save or copy a Markdown summary, or use Advanced Mode for detailed exports.

For a release that promotes Simple Mode, Simple Mode focuses on that path. Advanced Mode exposes deeper controls, additional strategy surfaces, validation modules, saved-run history, reports, data tooling, ML-signal workflows, options-expression workflows, and live execution workspaces where available.

## What VectorGrid Is For

VectorGrid is useful for:

- Researching parameter sensitivity across exact-grid and strategy-specific search spaces.
- Comparing CPU and GPU execution routes where the selected strategy supports both.
- Running validation modules after optimization rather than relying on a single in-sample result.
- Keeping local evidence through saved artifacts, reports, and summaries.
- Replaying or reopening prior saved runs when the saved artifact and dataset context are still available.
- Inspecting route metadata so users can tell whether a run used CPU, GPU, or hybrid execution.

## What VectorGrid Is Not

VectorGrid is not:

- Financial advice.
- A recommendation engine.
- A broker compliance system.
- A guarantee that backtest results will repeat in live markets.
- A substitute for data-quality checks.
- A guarantee that GPU acceleration helps every strategy or every validation mode.
- A production autonomous trading system.

Broker-connected workflows, if enabled in a release, require user supervision, provider credentials, live-market awareness, and separate risk controls.

## Local-First Design

VectorGrid is built as a local desktop app. User-selected CSV files, downloaded provider caches, run artifacts, reports, settings, and logs stay local unless the user chooses to share them.

Network access is used for user-invoked workflows such as Polygon.io data downloads, Polygon live streams, Alpaca broker calls, update checks if a release enables them, and links opened by the operating system.

## Simple Mode And Advanced Mode

Simple Mode is the intended public-facing mode for a release that ships it. It keeps the main task sequence visible:

- Data.
- Strategy.
- Optimization.
- Optional validation.
- Summary/export.

Advanced Mode is for users who need more control:

- broader strategy selection;
- backend and route controls;
- exact-kernel shortlist controls where supported;
- validation modules such as walk-forward, reoptimization walk-forward, CSCV/PBO, SPA, perturbation, trade bootstrap, and cross-dataset checks;
- saved artifacts and report exports;
- ML-signal and options-expression research workflows;
- live/paper execution workspaces where enabled.

## Strategy Families At A Glance

The app has several execution surfaces. Public docs should describe them by workflow and support boundary rather than by internal module count.

| Surface | Public meaning |
|---|---|
| Double MA | Primary moving-average crossover optimization and validation family. |
| Composite signal | Streamlined public Simple Mode strategy family built from combined signal logic. |
| Composite-profile routes | Trend, breakout, and regime-aware profiles that route through the Double-MA product path when supported. |
| Advanced standalone strategies | Research-oriented strategy modules with strategy-specific CPU/GPU behavior. |
| Composite MO | CPU-side multi-objective composite search with Pareto-style output. |
| ML Signal | CPU research path for exported features, external scores, and score-policy validation. |
| Options expression | Experimental follow-up workflow for evaluating option-expression candidates from supported parent artifacts. |

## Results Are Evidence, Not Answers

Optimization finds strong candidates under the configured historical assumptions. Validation adds additional evidence, but it does not make a strategy deployable by itself.

Users should review:

- data source and range;
- timeframe;
- strategy family;
- objective;
- selected candidate parameters;
- validation recommendation;
- warnings;
- CPU/GPU route;
- report and artifact provenance;
- known limitations for the selected workflow.
