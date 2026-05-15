# Feature Expansion Implementation Plans

Date: 2026-04-30

Status: planning package for future implementation work. No source behavior is changed by this folder.

## Public Status Boundary

The root `README.md` remains authoritative for current public status: this repository is currently published as a backend/CLI preview, and the Tauri desktop app is not the public quick-start path unless a signed desktop release candidate passes the public release gate.

The desktop-oriented plans in this folder are therefore release-readiness plans, not claims that the desktop app is already public-supported.

## Purpose

This folder is a repo-grounded feature expansion planning package for VectorGrid. It is intended to answer:

- where each expansion should start;
- which current files own the behavior;
- which new files are reasonable to add;
- what external technical constraints matter;
- what must remain out of scope;
- which tests, gates, and release evidence are required before handoff.

This package complements, but does not replace:

- `docs/REPO_MAP.md` for canonical repo navigation;
- `docs/repo_wide_deep_dive_2026-04-30/` for broad audit coverage;
- `docs/long_running_feature_tasks/` for already-scoped long-running tasks;
- `docs/verification_benchmark_release_authority.md` for evidence authority;
- `docs/artifact_replay_report_evidence_contract.md` for artifact/report/replay semantics;
- `docs/PUBLIC_RELEASE.md` for signed Windows release policy.

## How To Use

1. Pick the feature facet.
2. Open the matching plan in this folder.
3. Open the listed current owner files before broader search.
4. Implement one phase at a time.
5. Run the feature-specific tests first.
6. Run the broader gate only after targeted evidence is clean.
7. Update the plan with completed evidence if implementation starts later.

Do not treat this as permission to make one repo-wide change. These plans are deliberately split into small implementation lanes.

## Folder Contents

| File | Facet |
|---|---|
| `00_research_basis_and_plan_index.md` | Web-research basis, source ledger, source-informed expansion rules, and plan index. |
| `01_data_polygon_market_data_expansion.md` | CSV/cache/timeframe lifecycle, Polygon historical/live data, options data, cache manifests, and market-time correctness. |
| `02_optimization_strategy_route_expansion.md` | Optimization request shaping, strategy capability routing, CPU/GPU route policy, and new strategy onboarding. |
| `03_validation_evidence_gpu_expansion.md` | Validation semantics, walk-forward/evidence, GPU validation route labeling, output evidence, and user-facing verdicts. |
| `04_options_expression_expansion.md` | Options expression optimization, validation replay, quote quality, CPU/GPU parity, and options data hardening. |
| `05_ml_meta_signal_expansion.md` | Meta selection, Stage 5 ML confidence, ML Signal artifacts, score-cache policy, and model evidence. |
| `06_portfolio_live_execution_expansion.md` | Portfolio feasibility, local paper execution, Alpaca integration, risk policy, broker event evidence, and live-feed safety. |
| `07_frontend_workbench_visualization_expansion.md` | Simple Mode, Advanced workbench, visualization, charts, QA selectors, accessibility, and frontend performance. |
| `08_artifacts_reports_history_expansion.md` | Artifact lifecycle, replay classes, report generation, output parity, export schemas, and durability. |
| `09_desktop_runtime_security_release_expansion.md` | Tauri command boundary, capabilities, local secrets, updater/signing, privacy, release gates, and public docs. |
| `10_gpu_cuda_performance_expansion.md` | CUDA residency, kernels, PTX/native assets, profiling, benchmark boundaries, and GPU expansion policy. |
| `11_verification_ci_quality_expansion.md` | Test strategy, CI reactivation, benchmark baselines, QA dashboards, and gate authority. |
| `12_prioritized_execution_program.md` | Cross-facet sequencing, dependency map, first implementation batches, kill criteria, and readiness checkpoints. |
| `13_manifest_toolchain_dependency_contract.md` | Cargo/npm/toolchain features, `vector-ta` version/patch policy, native/PTX resources, Windows resource manifests, and dependency adoption gates. |
| `14_second_pass_research_feature_candidates.md` | Additional research-backed feature candidates from the second pass, including market calendars, options snapshots/Greeks, portfolio risk dashboards, local model registry semantics, columnar analytics, and CUDA graphs/probes. |
| `15_frontend_qa_gate_wiring_matrix.md` | Exact frontend QA script matrix, fixture ownership, visual/responsive/regression lanes, accessibility coverage, and desktop smoke wiring. |
| `16_accessibility_observability_support_expansion.md` | Dense-table/chart accessibility, live-region policy, local-first diagnostics, support bundles, opt-in telemetry, and privacy-aware troubleshooting. |

## Global Expansion Rules

- Preserve CPU-only builds without CUDA, NVIDIA drivers, or native ML libraries.
- Keep existing command names stable unless a compatibility migration is explicitly planned.
- Add persisted fields additively and make them optional or backfilled for legacy artifacts.
- Keep Polygon.io as the product data-provider label unless product policy changes. Current Polygon documentation redirects to Massive-branded pages; repo/provider naming should not drift accidentally.
- Keep live execution out of Simple Mode and out of public promises unless release policy explicitly changes.
- Never change optimization, validation, execution, or broker defaults as a side effect of UI/reporting work.
- Treat GPU claims as profile-backed claims, not route labels alone.
- Treat artifacts and reports as product contracts. Add output parity when shape changes.
- Treat CSV, imported artifact metadata, deep-link payloads, and file-open paths as untrusted input even when they are local.
- Run one benchmark/profiler at a time.

## Recommended Starting Point

If implementation begins immediately, start with the low-risk, high-leverage tracks:

1. `09_desktop_runtime_security_release_expansion.md`: command risk/capability/release hygiene.
2. `08_artifacts_reports_history_expansion.md`: artifact health, report parity, atomic writes.
3. `01_data_polygon_market_data_expansion.md`: market-time/cache manifest hardening.
4. `07_frontend_workbench_visualization_expansion.md`: Simple Mode E2E and stable QA action IDs.
5. `03_validation_evidence_gpu_expansion.md`: validation evidence labels before route behavior changes.

The higher-risk lanes, especially CUDA residency, options GPU promotion, and live broker expansion, should wait until their CPU oracle and evidence gates are explicit.
