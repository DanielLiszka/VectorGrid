# Data And Polygon.io Setup

VectorGrid can work with bundled sample data, local CSV files, and Polygon.io-backed cached datasets.

## Bundled Synthetic Sample

The bundled synthetic sample is included for first-run verification. It is small, redistributable, and designed to exercise the workflow.

Use it to confirm:

- the app launches;
- data loading works;
- optimization can run;
- validation can run;
- summaries and reports can be created.

Do not treat the bundled sample as real market data.

## Local CSV Files

CSV files should contain timestamp and close columns at minimum. OHLCV files are preferred.

Recommended header:

```csv
timestamp,open,high,low,close,volume
```

Accepted named aliases include:

| Field | Accepted names |
|---|---|
| timestamp | `timestamp`, `t`, `time`, `ts` |
| open | `open`, `o` |
| high | `high`, `h` |
| low | `low`, `l` |
| close | `close`, `c` |
| volume | `volume`, `v`, `vol` |

The timestamp should be a numeric integer-like value. Millisecond timestamps are the normal app convention. Float-form timestamps may be rounded for compatibility, but clean integer timestamps are strongly preferred.

The close column is required and must contain finite numeric values. Optional OHLCV fields may be blank, but strategies that depend on missing fields may be unavailable or less meaningful.

## Header Requirement

Use a header row. Headerless files can be misread because the first non-empty row is treated as a header/mapping row.

## CSV Data Quality Checklist

Before relying on a CSV:

- confirm timestamps are sorted ascending;
- confirm there are no duplicate timestamp rows;
- confirm the timeframe matches the strategy assumptions;
- confirm the market/session hours are what you expect;
- confirm split/dividend adjustments are what you expect;
- confirm missing bars are understood;
- confirm close values are finite and positive where appropriate;
- confirm high/low/open/volume are present if the selected strategy uses them.

## Binary Cache

VectorGrid may create a `.vta` binary cache for faster reloads. The cache is tied to the source file identity and content fingerprint. If the CSV changes, the app should rebuild or invalidate stale cache content.

Users can delete `.vta` cache files when troubleshooting. The app can rebuild them from the CSV.

## Polygon.io Historical Data

Polygon.io historical downloads currently support:

- stocks;
- forex;
- crypto.

The app caches 1-minute aggregate data first, then derives higher timeframes locally when requested. This avoids repeated downloads for every higher-timeframe view.

Provider access depends on your Polygon.io plan. If a request fails, check:

- API key;
- subscription entitlements;
- symbol format;
- market;
- date range;
- plan recency limits;
- rate limit status;
- whether eligible trades existed during the selected interval.

## Polygon API Key Storage

VectorGrid prefers operating-system credential storage where supported. If secure keyring storage is unavailable, the Polygon implementation can fall back to a local JSON settings file.

Do not share logs, screenshots, or config files that might expose provider state. Public release artifacts must never include maintainer API keys.

## Polygon Cache Location

Polygon cache files live under the effective market-data root in a `polygon_cache` folder. The app can show default, configured, and effective market-data roots.

Users may choose a custom market-data root. If the app offers to move the existing Polygon cache, use an empty destination and avoid moving a cache into itself.

## Polygon Cache Validation And Repair

Cached datasets can be inspected, validated, and repaired.

Validation can detect:

- parse errors;
- duplicate timestamps;
- out-of-order rows;
- large same-day gaps.

Repair can fix some duplicate/order issues in place. When an API key is available, repair may refetch gappy days where supported. Very large datasets may need deletion and redownload instead of in-place repair.

## Polygon Top-Off

Top-off extends a cached dataset from its last cached date toward the current date when the request is allowed by the plan and cache state.

Top-off is useful when:

- the existing dataset is mostly correct;
- you need recent bars;
- you want to avoid redownloading the full range.

If top-off is blocked, check that the last cached date is within the allowed lookback window and that the cache integrity state is clean.

## Polygon Live Streaming

Polygon live streaming currently supports stocks only.

Live stock streaming defaults to delayed mode unless explicitly disabled. Delayed streams are not equivalent to real-time broker execution input.

Only one active live stream slot is available at a time. Starting a new live stream may require stopping the previous one.

## Data Privacy

Local CSV names, ticker symbols, dates, downloaded provider data, cache metadata, and report outputs can reveal sensitive research activity. Review files before sharing them.

