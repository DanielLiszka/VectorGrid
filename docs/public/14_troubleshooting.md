# Troubleshooting

This page covers common public-release issues.

## App Will Not Install

Check:

- Windows version is Windows 10/11 x64.
- Installer is from the official release page.
- SHA-256 hash matches the release notes.
- Publisher/signature matches the release notes.
- Microsoft Edge WebView2 Runtime is installed or bootstrapper installation is allowed.
- Antivirus or enterprise application-control policy is not blocking the installer.

Try the CPU-only installer if a GPU variant fails.

## App Opens Blank Or Closes Immediately

Try:

- rebooting Windows;
- repairing or reinstalling WebView2 Runtime;
- launching the CPU-only variant;
- checking release notes for known GPU requirements;
- clearing only app runtime cache if directed by support;
- reinstalling the signed build.

When reporting, include Windows version, release version, variant, and whether WebView2 is installed.

## CSV Will Not Load

Check:

- file extension is `.csv`;
- file is not locked by another program;
- file has a header row;
- timestamp and close columns exist;
- close values are finite numbers;
- timestamps are numeric and sorted ascending;
- rows are not empty or malformed;
- optional OHLCV fields are valid if the selected strategy needs them.

Recommended header:

```csv
timestamp,open,high,low,close,volume
```

## Polygon Download Fails

Check:

- API key is saved;
- API key has not been mistyped;
- subscription plan includes the requested data;
- symbol format is valid for the selected market;
- date range is allowed by your plan;
- rate limit has not been reached;
- the selected interval had eligible trades;
- Polygon.io service status.

Rate limit errors usually require waiting, narrowing the date range, or using an already cached dataset.

## Polygon Cache Shows Integrity Warnings

Use cache validation and repair where available.

Warnings can include:

- parse errors;
- duplicates;
- out-of-order rows;
- large same-day gaps.

If repair cannot fix the dataset, delete and redownload the affected cache range.

## Optimization Produces No Results

Try:

- load a known-good dataset;
- use the bundled synthetic sample to confirm the app works;
- reduce constraints;
- use a smaller grid first;
- choose Double MA crossover;
- switch backend to CPU-only;
- check warnings for unsupported controls or invalid strategy settings.

## GPU Run Falls Back To CPU

Fallback can be expected for unsupported strategy/module combinations.

Check:

- you installed a GPU release variant;
- NVIDIA driver is current;
- GPU diagnostics detect the device;
- selected strategy supports the requested GPU route;
- validation modules support that route;
- another GPU-heavy process is not occupying the device.

Include route notes and warnings in bug reports.

## Validation Is Blocked

Validation usually needs:

- loaded dataset;
- completed optimization;
- selected candidate or valid shortlist;
- matching dataset context;
- supported strategy/route combination.

If validating a saved artifact, confirm the original dataset or matching cached dataset is available.

## Reports Will Not Export Or Open

Check:

- destination folder exists and is writable;
- security software is not blocking file writes;
- the workflow supports that report type;
- the app has a valid saved result or validation result to export;
- the path is not on a disconnected network drive.

## Live Execution Is Blocked

Check:

- release supports the execution workflow;
- dry-run vs paper vs live mode is selected correctly;
- credentials are saved when required;
- a matching Polygon live stock stream is active when runner mode needs it;
- market is open for live broker actions;
- risk policy allows the order;
- confirmation dialog has not expired.

Do not bypass blocked live-execution warnings.

## What To Include In A Bug Report

Include:

- VectorGrid version and release variant;
- Windows version;
- CPU/GPU model;
- NVIDIA driver version for GPU issues;
- data source type: sample, local CSV, Polygon historical, or Polygon live;
- strategy and backend;
- exact error text;
- screenshots if useful;
- copied Simple Markdown summary if relevant;
- logs with secrets redacted.

