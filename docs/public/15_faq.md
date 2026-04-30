# FAQ

## Is VectorGrid financial advice?

No. VectorGrid is research and analysis software. It is not financial advice, investment advice, tax advice, legal advice, or a recommendation to trade.

## What should I try first?

Install the CPU-only release, open Simple Mode, load the bundled synthetic sample, run Double MA optimization, then run optional validation.

## Do I need a GPU?

No. CPU-only is the broadest compatibility path. GPU variants are optional and useful only for supported workloads.

## Do I need CUDA Toolkit?

Normal users should not need CUDA Toolkit when the release uses bundled prebuilt GPU resources. Developers and maintainers may need CUDA tools for profiling or building GPU code.

## What data can I use?

You can use the bundled synthetic sample, local CSV files, and Polygon.io cached datasets. Polygon historical support includes stocks, forex, and crypto. Polygon live streaming currently supports stocks only.

## What CSV format should I use?

Use:

```csv
timestamp,open,high,low,close,volume
```

At minimum, provide timestamp and close columns. Use a header row.

## Is the bundled sample market data?

No. It is synthetic workflow-verification data.

## Why did my GPU run use CPU?

The selected strategy, validation module, or route may not support GPU execution. The app's route label and warnings are the source of truth.

## Why is validation recommended?

Optimization alone can overfit. Validation adds additional evidence before you trust a candidate.

## Does passing validation mean a strategy is safe to trade?

No. Passing validation means the candidate passed the configured historical checks. It does not prove live performance, execution quality, broker compatibility, or risk suitability.

## Where are reports saved?

Reports and artifacts are saved locally under the app's data/report locations unless the UI lets you choose a different path.

## Can I share reports?

Yes, but review them first. They may include symbols, date ranges, parameters, timestamps, route metadata, and local context.

## Does VectorGrid send my data to a server?

The local CSV workflow runs locally. Network access is used for provider downloads, live streams, broker workflows, update checks if enabled, and external links.

## Is Polygon.io required?

No. Polygon.io is needed only for provider-backed workflows. Local CSV and bundled sample workflows do not require it.

## Can I use Alpaca live trading?

Only if the release enables and documents broker-connected workflows. Live broker usage requires user supervision and independent risk controls.

## What platform is supported?

The public desktop target is Windows x64.

## How do I report a bug?

Use GitHub Issues for normal bugs and support questions. Include version, release variant, Windows version, workflow, error text, and logs/screenshots with secrets redacted. Use private vulnerability reporting for security issues.

