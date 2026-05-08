# Known Limitations And Release Status

This page keeps public expectations conservative. A release can be useful before every internal surface is promoted as a supported public feature.

## Current Public Posture

The first signed desktop release should be treated as a Windows x64 research application. If that release promotes Simple Mode as the main public path, Simple Mode must be implemented in the shipped UI, covered by frontend tests, and verified by clean-machine smoke evidence first. The current checked-in frontend defaults to the Advanced workbench and ignores legacy guided/simple mode requests.

The recommended public default is:

- CPU-only installer first;
- first-run workflow matching the shipped UI;
- bundled synthetic sample or local CSV first;
- optional Polygon.io setup after the local workflow is verified;
- Advanced Mode only when users need deeper controls.

GPU, ML, options, live execution, and advanced strategy surfaces should be described as release-specific or research-oriented unless a release explicitly promotes them.

## Release Artifact Status

Public docs describe the intended signed desktop release. They are not proof that a release artifact exists.

A publishable Windows release should have:

- clean release build;
- generated installer bundle;
- Authenticode signature;
- hash file;
- release manifest;
- packaged smoke evidence;
- clean-machine smoke evidence;
- public release notes;
- privacy and disclaimer links.

Unsigned artifacts should be labeled as private beta or test-only.

## Platform Limitations

Current public target:

- Windows x64.

Not currently a public target:

- macOS;
- Linux;
- web-hosted browser app;
- mobile app;
- cloud service.

The app is local-first desktop software, not a hosted trading platform.

## Data Limitations

Local CSV support depends on clean numeric timestamp and price columns.

Polygon.io historical support currently covers:

- stocks;
- forex;
- crypto.

Polygon.io live streaming currently covers:

- stocks only.

Polygon options workflows are separate from stock/forex/crypto aggregate workflows and should be treated as research-oriented unless the release notes say otherwise.

## Strategy Limitations

Not every strategy supports:

- every backend;
- every validation module;
- heatmaps;
- export-all;
- include-all;
- exact-kernel shortlist controls;
- GPU validation;
- live execution.

The app's route metadata and warnings should be treated as part of the result.

## Advanced Workflow Limitations

Several surfaces are advanced or research-oriented rather than part of the Simple Mode public path:

- candidate drilldown and local-stability diagnostics;
- aggregate/meta selection;
- research campaigns and batch summaries;
- portfolio feasibility analysis;
- ML confidence and policy evaluation;
- ML Signal;
- options-expression workflows;
- live execution and local paper workflows;
- system/GPU diagnostics.

These can be useful, but release notes should not imply that every advanced surface is equally mature or equally supported.

## GPU Limitations

GPU acceleration is route-specific.

Known expectation boundaries:

- CPU-only remains supported.
- GPU variants require matching release notes and compatible NVIDIA drivers.
- Some strategy and validation workloads are CPU-preferred.
- Small workloads may not benefit from GPU execution.
- Other GPU-heavy applications can distort timings.
- Do not treat `Auto` as proof that a GPU route was used.

## Validation Limitations

Validation adds evidence, but it cannot prove live profitability or safety.

Validation can still be limited by:

- bad data;
- short sample length;
- market regime changes;
- overfit parameter spaces;
- unsupported module/route combinations;
- stale or incomplete saved artifacts;
- score-cache provenance gaps in ML workflows.

## ML And Options Limitations

ML Signal is a CPU research workflow built around exported features, external scores, score caches, and validation policies. It is not a built-in forecasting guarantee.

Options-expression workflows are research workflows bound to supported parent artifacts and provider/data semantics. They are not a full options trading platform.

## Live Execution Limitations

Live execution, if enabled, is not the default public workflow.

It is limited by:

- broker credentials;
- market status;
- data freshness;
- risk policy;
- account state;
- order handling;
- user supervision;
- release-specific support boundaries.

Dry-run and local paper modes are not equivalent to live broker execution.

## Documentation Versioning

Public docs must match the release they describe. If the app changes, update:

- release page;
- known limitations;
- first-run guide;
- data setup;
- GPU/performance page;
- live execution page if broker workflows changed;
- privacy/security page if storage or network behavior changed.
