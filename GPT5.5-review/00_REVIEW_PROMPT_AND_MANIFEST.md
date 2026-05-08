# GPT-5.5 Pro Review Prompt And Manifest

Status/provenance: dated internal review prompt pack snapshot. Use the root `README.md`, `docs/REPO_MAP.md`, `docs/PUBLIC_RELEASE.md`, and `docs/public/README.md` as current public-status authorities when they differ from copied files in this folder.

Use this file as the main prompt. The other files in this folder are the capped context pack for the review. This pack is intentionally flat and limited to 18 files total.

## Prompt

You are evaluating a large Rust/Vue/Tauri trading-research repository called VectorGrid / `ta_desktop_demo`. Act as a senior product architect, quant-engineering reviewer, desktop systems engineer, and pragmatic implementation planner.

Your goal is to recommend future feature additions or expansions that are high-value, technically realistic, and aligned with the repository's current architecture, verification posture, and risk constraints.

Be direct and critical. Do not produce a marketing roadmap. Prefer fewer, stronger recommendations over broad speculative feature lists.

## Product Context

VectorGrid is a GPU-accelerated Rust backend and CLI for exact-grid trading-strategy optimization and validation. The repository also contains a substantial Vue 3 + Tauri desktop application, but the supported public surface today is primarily the Rust backend, CLI entry points, benchmark harnesses, and verification scripts. The desktop application is real and in-tree, but should be treated as an internal or future-facing frontend surface unless public-release docs say otherwise.

This is not a production trading system. Do not recommend broker-facing or live-trading expansion as if this project is production financial infrastructure.

## Architecture Summary

The workspace is organized around:

- `crates/app_core/`: Rust engine for optimization, validation, CUDA/runtime plumbing, options workflows, ML/meta workflows, portfolio feasibility, artifacts, and report contracts.
- `crates/strategies/`: shared strategy definitions and reusable strategy logic.
- `crates/optimizer/`: optimizer/search/grid/multi-objective primitives.
- `src-tauri/`: Tauri command boundary, orchestration, OS integration, data/cache/live execution commands, artifacts, and reports.
- `web/`: Vue 3 + Pinia + Vite frontend, workflow state, request shaping, command clients, workbench UI, charts, tests.
- `scripts/`: quality gates, benchmark scripts, profiling helpers, release gates, parity checks, and maintenance tooling.
- `docs/`: architecture notes, repo map, public-release docs, feature expansion plans, evidence contracts, numerical/reproducibility notes, and verification authority.

## Existing Feature Surface

The project already has broad feature surface. Do not assume the best next step is more surface area.

Existing capabilities include:

- Exact-grid strategy optimization.
- Mature GPU path for the Double-MA family.
- CPU fallback throughout the product.
- Warm in-memory and VRAM-resident execution paths for repeated optimization/validation on the same dataset.
- GPU-backed validation coverage for active supported strategy families.
- Major workflow families:
  - Double MA
  - Composite/advanced strategies
  - CompositeMo
  - ML Signal
  - Options-expression workflow
- CompositeTrend, BreakoutMomentum, and RegimeHybrid currently route through the Double-MA product path.
- Optimization and validation commands exposed through the Tauri command layer.
- Walk-forward and related validation flows.
- Cross-ticker validation orchestration.
- Artifact storage, artifact replay, artifact repair, run history, hydration, and report generation.
- XLSX report generation and validation shortlist CSV export.
- Polygon market-data integration, including API key management, download/load, cache info, repair, top-off, validation, and live candle stream controls.
- Local CSV/sample CSV loading and resampling.
- Options-expression workflow with dataset import/download, manifests, optimization, feasibility analysis, tuple/event inspection, artifact save/load/list, validation feasibility, and saved-artifact validation/replay.
- ML/meta workflows, including meta-selection, ML confidence model training/application/evaluation, score-cache generation, policy sweeps, validation, and dataset export.
- Portfolio feasibility analysis.
- Paper/live execution scaffolding, including Alpaca credential handling, local paper broker, live snapshots, risk policy, policy preview, runner controls, cancel/flatten, and emergency halt commands.
- Vue/Tauri desktop workbench with optimization, validation, options, artifacts, reports, charts, workflow state, and command clients.
- QA and verification surface across Rust tests, frontend tests, Playwright/browser workflows, desktop smoke paths, lint/typecheck, repo quality gates, benchmark gates, and release gates.

## Recent Local Implementation State

A recent local feature branch added a Validation Evidence Pack. Treat it as implemented if the provided code/docs show it:

- Additive `ValidationEvidencePackV1` DTOs.
- Validation evidence schema/version metadata.
- Evidence modules for run verdict, CSCV/PBO, SPA, PSR/DSR, and minimum-track-record signals.
- Pack generation for Double-MA and Composite validation.
- Artifact stage summaries enriched with evidence pack status/schema/version/modules.
- Report export parsing and XLSX summary support.
- Frontend evidence summary normalization that prefers pack-backed evidence while preserving legacy fallback.
- Focused Rust and frontend tests.

This feature is explanatory and non-behavior-changing. It should not alter scoring, ranking, route selection, or trading decisions.

## Engineering Constraints

Respect these constraints:

- CPU-only builds must keep working without CUDA installed.
- CUDA/GPU work must preserve CPU fallback.
- Additive schemas are preferred over migrations.
- Artifact replay compatibility matters.
- Evidence, provenance, route transparency, and reproducibility are first-class concerns.
- Performance claims require benchmark-boundary clarity and saved evidence.
- Avoid expanding live/paper execution without auditability and safety gates.
- Avoid ranking/scoring changes without explicit parity evidence.
- Prefer strategy-local architecture over large generic abstractions.
- For frontend-only proposals, treat backend contracts as fixed unless explicitly recommending backend contract evolution.
- Do not propose production broker automation as if this is a production trading product.

## Evaluation Task

Produce:

1. A concise executive summary.
2. A ranked list of the top 8-12 feature additions or expansions.
3. For each feature:
   - user value
   - technical value
   - risk-reduction value
   - implementation complexity
   - likely files/modules touched
   - backend/frontend/schema/report/artifact impact
   - testing and verification required
   - whether it should be implemented now, later, or rejected
4. A recommended 6-8 week roadmap.
5. A longer 6-12 month roadmap.
6. Features that sound attractive but should not be prioritized yet.
7. Unresolved architectural questions that should be answered before major expansion.
8. A concrete implementation sequence for the top 3 features.
9. A verification plan using the repo's existing gates.
10. Risks where a feature could create false confidence, data drift, replay breakage, or misleading performance claims.

## Feature Candidates To Consider

Give special attention to these candidates, but do not blindly accept them:

- Market-time canonicalization and versioned market-calendar service.
- Options-expression hardening:
  - per-window walk-forward result tables
  - quote-quality diagnostics
  - price semantics visibility
  - deterministic CPU/GPU parity gates
- GPU validation productization:
  - route metadata
  - deterministic fixed-candidate/top-k semantics
  - hidden CPU-tail visibility
  - route transparency in artifacts/reports/UI
- Artifact analytics lab for large optimization/validation artifacts.
- ML/meta evidence tiering and strict score-cache mode.
- Alpaca/local paper execution reconciliation timeline.
- CUDA residency diagnostics and CUDA graph experiments.
- Desktop productization hardening:
  - Tauri capability scopes
  - public-release gate
  - update/signing readiness
  - clean-machine install checks
- Data lifecycle hardening:
  - provider provenance
  - dataset identity
  - cache repair/top-off correctness
  - time-zone/session labeling
- Report/export parity and replay evidence improvements.

## Output Requirements

For every recommendation, distinguish between:

- evidence-backed findings from the provided files
- reasonable inference
- assumptions that need code inspection
- external research that would be needed before implementation

Prioritize features that improve trust, correctness, reproducibility, and maintainability before features that simply add more strategy surface area.

## Included File Manifest

- `01_README.md`: public product framing, benchmark posture, backend-preview status, quality gates.
- `02_REPO_MAP.md`: canonical high-level repo routing and workflow ownership map.
- `03_ARCHITECTURE_NOTES.md`: backend/runtime architecture notes.
- `04_SRC_TAURI_COMMAND_SURFACE.rs`: current Tauri command exports and broad desktop command surface.
- `05_FEATURE_EXPANSION_INDEX.md`: feature-expansion package index.
- `06_PRIORITIZED_EXECUTION_PROGRAM.md`: current internal sequencing logic for future work.
- `07_DATA_POLYGON_MARKET_DATA_EXPANSION.md`: data lifecycle, Polygon, market-time, and calendar expansion plan.
- `08_VALIDATION_EVIDENCE_GPU_EXPANSION.md`: validation evidence and GPU validation expansion plan.
- `09_OPTIONS_EXPRESSION_EXPANSION.md`: options-expression hardening and future scope plan.
- `10_ML_META_SIGNAL_EXPANSION.md`: ML/meta/signal expansion plan.
- `11_PORTFOLIO_LIVE_EXECUTION_EXPANSION.md`: portfolio and execution safety expansion plan.
- `12_FRONTEND_WORKBENCH_VISUALIZATION_EXPANSION.md`: frontend/workbench/visualization expansion plan.
- `13_ARTIFACTS_REPORTS_HISTORY_EXPANSION.md`: artifacts, reports, and history expansion plan.
- `14_DESKTOP_RUNTIME_SECURITY_RELEASE_EXPANSION.md`: Tauri runtime, security, and release expansion plan.
- `15_GPU_CUDA_PERFORMANCE_EXPANSION.md`: CUDA/GPU performance and diagnostics expansion plan.
- `16_ARTIFACT_REPLAY_REPORT_EVIDENCE_CONTRACT.md`: artifact/replay/report evidence contract.
- `17_VERIFICATION_BENCHMARK_RELEASE_AUTHORITY.md`: verification, benchmark, and release authority ledger.
