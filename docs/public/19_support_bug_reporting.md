# Support And Bug Reporting

Use GitHub Issues for normal support, bug reports, scoped feature requests, and usage questions.

Do not use public issues for security vulnerabilities.

## Before Opening An Issue

Check:

- release notes;
- [First Run And Simple Mode](04_first_run_simple_mode.md);
- [Data And Polygon.io Setup](05_data_and_polygon_setup.md);
- [Troubleshooting](14_troubleshooting.md);
- [FAQ](15_faq.md);
- existing open and closed issues.

If the problem involves installation, verify the installer hash and publisher first.

## What To Include

Include:

- VectorGrid version;
- release variant: CPU-only, GPU MA, or GPU kernel;
- Windows version;
- whether the app was installed from a signed public release or built locally;
- CPU model if performance is relevant;
- NVIDIA GPU model and driver version for GPU issues;
- data source: bundled sample, local CSV, Polygon historical, Polygon live, options dataset, or ML score cache;
- strategy/workflow;
- backend route shown by the app;
- exact error text;
- screenshots if useful;
- copied Simple Markdown summary when relevant;
- logs or reports with secrets redacted.

## Good Bug Report Shape

Use this structure:

```markdown
## Environment

- VectorGrid version:
- Release variant:
- Windows version:
- GPU/driver if relevant:

## Workflow

1. 
2. 
3. 

## Expected


## Actual


## Evidence

- Error text:
- Summary/report path:
- Screenshot:
- Logs:
```

## Data Issues

For local CSV issues, include:

- header row;
- first 5 non-sensitive rows if possible;
- timestamp unit;
- timeframe;
- whether timestamps are sorted;
- whether duplicates exist;
- whether the bundled synthetic sample works.

Do not upload private datasets unless you are comfortable sharing them publicly.

## Polygon Issues

For Polygon.io issues, include:

- market: stocks, forex, crypto, or options;
- symbol;
- date range;
- timeframe;
- whether API key is saved;
- whether a cached dataset already exists;
- rate-limit or entitlement error text;
- cache validation output if available.

Do not include the API key.

## GPU Issues

For GPU issues, include:

- release variant;
- NVIDIA GPU model;
- driver version;
- selected strategy;
- requested backend;
- resolved backend/route;
- route notes;
- whether CPU-only works;
- whether another GPU-heavy task was running.

## Report Or Artifact Issues

For reports, history, or replay issues, include:

- report type;
- artifact replay class;
- replay/reopen reason;
- app version that created the artifact;
- whether the original dataset is still available;
- whether paths were moved or deleted.

## UI Or Accessibility Issues

For UI, navigation, or accessibility issues, include:

- screen or workspace name;
- Simple Mode or Advanced Mode;
- window size or display scaling;
- keyboard-only steps if relevant;
- screenshot if useful;
- whether the command palette or workspace rail was involved;
- whether the issue blocks the first-run workflow.

## Security Reports

Do not report vulnerabilities in public issues.

Use the repository's private vulnerability reporting path when available. Include:

- affected release or commit;
- reproduction steps;
- impact;
- proof-of-concept material if safe to share privately;
- whether credentials, local files, broker actions, updater behavior, or command permissions are involved.

## Support Boundary

Support is best-effort. There is no guaranteed response time.

The narrowest supported public path is:

- latest signed Windows x64 release;
- CPU-only variant;
- Simple Mode only if that release implements and verifies it, otherwise the release-documented first-run workspace;
- bundled synthetic sample or clean local CSV.

Advanced, GPU, ML, options, and live execution issues may require additional evidence and may be treated as research or release-specific surfaces.
