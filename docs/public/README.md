# VectorGrid Public Documentation

This folder contains public-facing documentation for VectorGrid website pages, release pages, and end-user help. It is intentionally separate from maintainer runbooks under `docs/` so website copy can stay clear, user-centered, and conservative.

These pages describe the intended signed Windows desktop release surface. Until a signed installer is published, treat them as release-candidate public docs rather than proof that a public desktop artifact already exists.

## Start Here

| Page | Audience | Purpose |
|---|---|---|
| [Product Overview](01_product_overview.md) | Everyone | What VectorGrid is, who it is for, and what it is not. |
| [System Requirements](02_system_requirements.md) | Prospective users | Windows, CPU, GPU, WebView2, storage, and account requirements. |
| [Windows Install Guide](03_windows_install_guide.md) | End users | Download verification, install, first launch, updates, and uninstall. |
| [First Run And Simple Mode](04_first_run_simple_mode.md) | New users | The streamlined path: load data, optimize, validate, and export a summary. |
| [Data And Polygon.io Setup](05_data_and_polygon_setup.md) | Users with market data | CSV format, bundled sample, Polygon API key setup, cache validation, repair, and live-stream limits. |
| [Optimization Workflows](06_optimization_workflows.md) | Active users | Strategy families, CPU/GPU routing, objectives, Top-K, heatmaps, and export behavior. |
| [Validation And Evidence](07_validation_and_evidence.md) | Research users | Holdout-style checks, walk-forward, robustness modules, cross-dataset checks, and how to read evidence. |
| [Reports, History, And Exports](08_reports_history_exports.md) | Users sharing results | Saved artifacts, Markdown summaries, JSON/CSV/XLSX exports, and replay/reopen expectations. |
| [GPU And Performance](09_gpu_and_performance.md) | Performance-sensitive users | GPU variants, backend labels, benchmark interpretation, and safe expectations. |
| [Advanced Mode Reference](10_advanced_mode_reference.md) | Power users | Advanced workspaces and when to leave Simple Mode. |
| [Options, ML, And Experimental Workflows](11_options_ml_experimental.md) | Research users | Current boundaries for options-expression and ML-signal workflows. |
| [Live Execution And Risk](12_live_execution_and_risk.md) | Broker-connected users | Local paper, Alpaca, live stream dependency, dry-run behavior, and safety boundaries. |
| [Privacy, Security, And Local Data](13_privacy_security_local_data.md) | Everyone | Local storage, credentials, generated files, network access, and safe sharing. |
| [Troubleshooting](14_troubleshooting.md) | Users and support | Common install, data, Polygon, GPU, validation, and export issues. |
| [FAQ](15_faq.md) | Everyone | Short answers to recurring questions. |
| [Website Content Guide](16_website_content_guide.md) | Website builders | Page structure, messaging, copy blocks, claims policy, and launch checklist. |
| [Release Page Template](17_release_page_template.md) | Maintainers | Public release notes template for a signed Windows build. |
| [Known Limitations And Release Status](18_known_limitations_release_status.md) | Everyone | Current support boundary, release readiness, and feature limits. |
| [Support And Bug Reporting](19_support_bug_reporting.md) | Users and maintainers | What to include in support requests, bug reports, and security reports. |
| [Legal, License, And Disclaimers](20_legal_license_disclaimers.md) | Everyone | License, no-warranty posture, provider terms, and financial-risk language. |
| [Public Docs Coverage Matrix](21_public_docs_coverage_matrix.md) | Maintainers and website builders | Checklist mapping docs to app surfaces before publishing. |
| [Glossary](22_glossary.md) | Everyone | Plain-English definitions for common VectorGrid terms. |
| [Roadmap And Future Scope](23_roadmap_future_scope.md) | Prospective users | Conservative future direction without release promises. |

## Public Positioning

VectorGrid is local-first research software for strategy optimization and validation. It helps users compare candidate trading-strategy parameter sets, validate those candidates against additional evidence, and export local research artifacts.

The public message should stay grounded in four facts:

- The app is research and analysis software, not financial advice.
- Simple Mode is the recommended first-run workflow.
- CPU-only builds are the broadest compatibility path.
- GPU builds are variant-specific and require clear driver/runtime expectations.

## Public Support Boundary

The first public desktop release should be documented as:

- Windows x64 only.
- CPU-only as the default broad-compatibility release.
- Optional NVIDIA GPU variants only when released with variant-specific evidence.
- Local CSV and bundled synthetic sample support.
- Polygon.io historical data support for stocks, forex, and crypto.
- Polygon.io live streaming support for stocks only.
- Alpaca-backed execution workflows only where the release explicitly enables and documents them.

Do not present benchmark numbers, GPU support, live execution, options workflows, or ML workflows as blanket production guarantees. Each claim needs the release variant and evidence boundary named in the relevant page.

## Coverage Checklist

This public docs pack covers the major user-facing surfaces:

- Installation and release integrity: requirements, hashes, signatures, SmartScreen, WebView2, updates, uninstall.
- First-run path: Simple Mode, bundled synthetic sample, local CSV, optimization, optional validation, Markdown summary.
- Data: CSV format, binary cache, Polygon.io historical data, Polygon cache validation/repair/top-off, live stock streams.
- Research workflow: strategy families, CPU/GPU route labels, Top-K, heatmaps, include-all/export-all, validation modules, evidence interpretation.
- Outputs: saved artifacts, replay classes, reports, JSON/CSV/XLSX/Markdown exports, local cleanup.
- Advanced surfaces: Advanced Mode, ML Signal, options-expression, portfolio feasibility, campaign/meta references, live execution boundaries.
- Trust and support: privacy, local data, credentials, logs, troubleshooting, support reports, security reports, license, disclaimers.
- Website readiness: website structure, copy guidance, screenshot rules, benchmark-claim policy, release-page template, glossary, future-scope language.
