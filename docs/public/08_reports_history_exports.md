# Reports, History, And Exports

VectorGrid can create local summaries, saved run artifacts, and detailed reports.

## Simple Markdown Summary

For a release that implements Simple Mode, Simple Mode can produce a compact Markdown summary. It is the recommended first sharing format for that release because it is readable and includes the main workflow context.

It can include:

- generated timestamp;
- dataset label;
- timeframe;
- row count;
- dataset range;
- strategy;
- objective;
- search depth;
- engine label;
- optimization status;
- best candidate;
- selected metrics;
- validation recommendation;
- evidence rows;
- validation engine;
- warnings.

The summary is research evidence only, not trading advice.

## Saved Run Artifacts

Saved run artifacts preserve structured workflow output for later review.

Artifacts can help with:

- reopening a prior run;
- comparing current and saved results;
- replay or repair workflows where supported;
- report generation;
- provenance review.

Saved artifacts may include dataset identity, strategy parameters, metrics, route metadata, validation output, report paths, meta-stage output, ML policy output, options-expression output, live-session summaries, batch summaries, and campaign summaries where those workflows are used.

## Artifact Replay Classes

VectorGrid classifies saved artifacts by replay/reopen capability. A saved result may be:

- directly reusable;
- reopenable with warnings;
- blocked because required data or provenance is missing;
- partially useful for inspection but not replay.

If an artifact cannot be replayed, use the displayed reason rather than assuming the run is broken. Common reasons include missing dataset context, changed files, unsupported legacy output, or route mismatch.

## Reports

Advanced Mode can export detailed reports depending on the workflow:

- JSON summaries;
- CSV shortlist exports;
- report JSON;
- validation report XLSX;
- meta report XLSX;
- options or ML artifacts where supported.

Report availability depends on the strategy and workflow. Not every workflow emits every report type.

## History And Artifact Library

The History and artifact-library surfaces are useful for:

- finding recent runs;
- applying a saved run back into the current session;
- comparing saved and current results;
- opening reports;
- updating artifact tags;
- repairing supported saved-run metadata;
- deleting old artifacts.

If the app reports that a saved artifact is inspectable but not replayable, keep that limitation visible when sharing the result.

## Output Parity

Maintainer release gates include output-parity checks for deterministic report/artifact shape. For users, the important point is simpler: report files should remain structurally consistent within a release line, but users should still keep the app version with any shared artifact.

When sharing a report, include:

- VectorGrid version;
- release variant;
- dataset source;
- run date;
- strategy;
- validation state;
- warnings.

## Local Storage

Reports and artifacts are local files. They may contain:

- symbols;
- date ranges;
- strategy parameters;
- objective metrics;
- candidate details;
- validation warnings;
- local timestamps;
- dataset provenance;
- report paths.

Review generated files before sharing them publicly.

## Cleaning Up

Users can delete old reports and artifacts when they are no longer needed. Do not delete files that are needed to reproduce, reopen, or audit a result you still care about.

For provider-backed datasets, deleting cache files can force later redownloads.
