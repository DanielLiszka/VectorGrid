# REPO_MAP

Last updated: 2026-07-24

## Purpose

This file is a routing document for this repository.

Use it to answer:

- what part of the repo owns a workflow
- which files to open first
- which large files to avoid reading until needed
- what strategies and backend routes currently exist
- what verification commands matter for a given change

Do not treat this as the full implementation spec. Use it together with:

- local `AGENTS.md` if present for agent-only rules and invariants
- `README.md` for product framing and benchmark posture
- `docs/ARCHITECTURE_NOTES.md` for backend/runtime notes
- `docs/RELEASE.md` for build/release/PTX packaging notes
- `docs/PUBLIC_RELEASE.md` for the signed Windows desktop public-release gate, staged artifact evidence, signing hook, updater prerequisites, and clean-machine checklist
- `docs/WINDOWS_INSTALL.md` for end-user Windows install notes
- `docs/public/README.md` for the website-ready public documentation index, first-run guide, data setup, validation guide, troubleshooting, FAQ, and release-page template
- `web/README.md` for frontend runtime notes
- `docs/archive/README.md` for retired planning/review/tracker material
- `docs/options_research/options_expression_optimizer_viable_plan_2026-04-02.md` for the current shipped-slice options-expression tracker, current checklist, and repo-fit implementation status
- `docs/options_research/options_expression_future_scope_roadmap_2026-04-07.md` for the refocused future roadmap that is now limited to improving the existing options optimization + embedded validation workflow
- `docs/gpt55_pro_hard_problem_meta_research_plan_2026-04-27.md` for the active meta-analysis plan used to identify which application-wide hard problems deserve external web research and GPT-5.5 Pro dossier treatment
- `docs/gpt55_pro_hard_problem_atlas_2026-04-27.md` for the first repo-wide and web-research-backed hard-problem atlas, ranked candidate list, and recommended GPT-5.5 Pro dossier order
- `docs/gpt55_pro_final_prompt_dossiers_2026-04-27.md` for ready-to-use GPT-5.5 Pro prompt dossiers with exact required and optional file packs for each hard-problem prompt
- `docs/gpt55_pro_high_roi_future_prompt_ideas_2026-04-27.md` for researched high-ROI prompt ideas beyond the active prompt pack, especially data lifecycle, native binary integrity, and prompt sequencing governance
- `docs/verification_benchmark_release_authority.md` for the claim-level verification authority ledger, benchmark timing boundaries, baseline policy, release evidence hierarchy, and GPT-5.5 Pro recommendation promotion checklist
- `docs/release_authority_matrix.json` for the canonical machine-readable claim/evidence/variant/gap index; validate it with `node scripts/check_release_authority_matrix.mjs`
- `docs/NUMERICAL_REPRODUCIBILITY.md` for NumericPolicyV1, OrderingPolicyV1, reproducibility tiers, semantic parity scope, benchmark-noise posture, and blocked CUDA/PTX authority work
- `docs/artifact_replay_report_evidence_contract.md` for the current artifact/replay/report evidence contract, report mutation boundaries, replay classes, and output parity authority
- `docs/tauri_desktop_technology_adoption_implementation_plan_2026-04-27.md` for the researched implementation plan covering native Tauri desktop plugins, IPC type generation, runtime validation, async cached reads, and low-regression advanced npm adoption
- `docs/frontend_trading_technology_additions_implementation_plan_2026-04-27.md` for the follow-on frontend/npm technology plan focused on live trading backpressure, chart annotations, market-time correctness, and optional large-artifact analytics
- `docs/feature_expansion_implementation_plans_2026-04-30/README.md` for the web-researched, facet-by-facet feature expansion implementation package covering data, optimization, validation, options, ML/meta, portfolio/live execution, frontend, artifacts/reports, desktop security/release, CUDA, verification/CI, and prioritized sequencing

Archived planning and implementation notes under `docs/archive/` plus dated notes under `docs/options_research/` are optional background only. Do not open them by default unless this map routes you there for the exact task you are doing.

## How To Use This Map

1. Identify the workflow or subsystem you are touching.
2. Open the small routing files listed for that workflow first.
3. Only then open the large implementation files.
4. Treat `docs/research/`, `docs/archive/`, and similar folders as optional background, not default reading unless this map explicitly routes you there.

When a high-level doc disagrees with the code, trust the code-facing routing files:

- `src-tauri/src/lib.rs` for the current Tauri command surface
- `web/src/services/*RequestBuilder.ts` for current request rules
- `web/src/utils/strategyOptimizationCapabilities.ts` and `web/src/strategies/compositeProfileGpuPolicy.ts` for current product-facing capability summaries

## Core Mental Model

- Product: `VectorGrid`, a Tauri desktop app for GPU-accelerated trading-strategy optimization and validation.
- `web/src`: Vue 3 + Pinia frontend, request shaping, workflow state, Tauri command invocation.
- `src-tauri/src`: Tauri command boundary, runtime state, OS integration, data/cache/live execution orchestration.
- `crates/app_core/src`: main engine for optimization, validation, CUDA/runtime plumbing, meta/ML workflows.
- `crates/strategies/src`: shared strategy definitions and shared strategy logic.
- `crates/optimizer/src`: optimizer/search/support primitives.
- `scripts/`: authoritative gates, specialized/manual gates, parity checks, profiling helpers, benchmark faceoff runners, telemetry/sample exporters, and maintenance helpers.

The repo is large, but the real execution families are fewer than the file count suggests:

- Double-MA family
- Composite/advanced family
- CompositeMo
- MlSignal
- Options expression follow-up workflow

## Stable Entry Points

Open these first before broad search:

- local `AGENTS.md` if present
- `README.md`
- `docs/ARCHITECTURE_NOTES.md`
- `docs/RELEASE.md`
- `web/README.md`
- `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/main.json`
- `src-tauri/src/commands.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/ma_registry.rs`
- `crates/app_core/src/lib.rs`
- `crates/strategies/src/lib.rs`
- `crates/optimizer/src/lib.rs`
- `web/src/App.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/workbench/WorkbenchShell.vue`
- `web/src/services/desktopProductization.ts`
  - desktop plugin bridge for logging, window-state persistence, and background-aware OS-notification routing
- `web/src/preflight.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`

## Options Expression Workflow

If the task is about the current options backtest optimization / embedded validation lane, open these first before broad search:

- current planning / scope
  - `docs/options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
  - `docs/options_research/options_expression_future_scope_roadmap_2026-04-07.md`
  - `docs/options_research/options_expression_polygon_validation_hardening_implementation_plan_2026-04-25.md`
- frontend host / request shaping
  - `web/src/components/OptionsExpressionPanel.vue`
  - `web/src/stores/optionsExpressionStore.ts`
  - `web/src/services/optionsExpressionService.ts`
- Tauri command boundary
  - `src-tauri/src/commands_options_expression.rs`
  - `src-tauri/src/commands_options_data.rs`
  - `src-tauri/src/options_parent_adapters.rs`
- app-core data / evaluation
  - `crates/app_core/src/options_types.rs`
  - `crates/app_core/src/options_signal_manifest.rs`
  - `crates/app_core/src/options_data_store.rs`
  - `crates/app_core/src/options_expression.rs`
  - `crates/app_core/src/options_expression_gpu.rs`
  - `crates/app_core/src/kernels/options_expression.cu`
- representative tests
  - `web/src/components/OptionsExpressionPanel.test.ts`
  - `web/src/stores/optionsExpressionStore.test.ts`
  - `web/src/services/optionsExpressionService.test.ts`

Use dated options implementation guides under `docs/options_research/` only as secondary context when you are touching one of those exact hardening lanes and the current code/tests are not enough.

Current shipped-slice guardrails to remember:

- `src-tauri/src/options_parent_adapters.rs` owns the supported parent-family adaptation layer for the current options workflow; do not spread new parent-artifact logic across unrelated command modules.
- saved options artifacts are now replay-provenance guarded against drift in parent artifact identity/hash, signal manifest identity, dataset id/hash, price semantics, and recorded dataset provenance.
- saved options-expression artifacts can be revalidated through the first-class `validate_options_expression_artifact` and `analyze_options_expression_validation_feasibility` commands, surfaced in the History options follow-up panel.
- subsequent options validation supports retained-tuple original-split replay, session-chronological replay, date-range holdout, and anchored/rolling walk-forward union-holdout plans; per-window walk-forward result tables remain future scope.
- options validation artifacts include Polygon.io dataset provenance and quote-quality summaries when the imported dataset carries quote/mid fields.
- options compare/reopen behavior is provenance-scoped to the saved artifact and currently-shipped dataset semantics, not to a broader future-options product lane.
- Polygon.io is the sole supported options data provider; legacy `massive` identifiers are compatibility aliases only where the code explicitly canonicalizes them.
- direct provider download remains minute-aggregate only in the current shipped slice; quote-lane expansion remains future-roadmap scope.

## Top-Level Workspace Map

### Workspace crates

- `Cargo.toml`
  - Rust workspace with 4 members.
- `src-tauri/`
  - Package: `ta_desktop_demo_app`
  - Role: desktop app crate and Tauri command surface
  - Features: `cuda`, `cuda-backtest-kernel`
- `crates/app_core/`
  - Package: `ta_app_core`
  - Role: core optimization/validation engine and GPU/runtime hub
  - Features: `cuda`, `cuda-backtest-kernel`, optional `cli`
- `crates/strategies/`
  - Package: `ta_strategies`
  - Role: shared strategy definitions and logic
- `crates/optimizer/`
  - Package: `ta_optimizer`
  - Role: optimizer support crate

### Top-level directories to care about first

- `src-tauri/`
- `crates/app_core/`
- `crates/strategies/`
- `crates/optimizer/`
- `web/`
- `scripts/`
- `.github/`

### Usually ignore unless the task explicitly needs them

- `target/`
- `dist/`
- `node_modules/`
- `.codex_tmp/`
- `.codex_tmp*/`
- `.playwright-cli/`
- generated benchmark/result dumps

### Local generated artifacts

- `output/`
  - gitignored preserved local artifacts from benchmark, profiling, faceoff, and report runs
  - preferred home for any local evidence you want to keep across routine `target/` cleanups
- `target/`
  - Cargo build cache, generated binaries, Criterion internals, and other disposable build scratch
  - safe to delete when you do not need the current build products

### Secondary reference, not canonical routing

- `docs/archive/`
  - retired implementation plans, review packets, prompt packs, and historical trackers
- `docs/research/`
  - mixed background notes, benchmark writeups, and helper harnesses rather than a primary routing surface
- `docs/options_research/`
  - current options scope docs plus older prompt-pack and implementation-guide background material
- `docs/`
  - current and archived documentation, supporting assets, and background material

### Support crate map

These crates are smaller than `app_core`, but they are part of the real execution surface and are worth opening before digging into giant app-core files when the task lives there.

- `crates/strategies/src/double_ma/mod.rs`
  - shared Double-MA params, metrics, CPU batch backtest, and curve/trade structures
- `crates/strategies/src/composite_cpu/mod.rs`
  - composite signal logic, `CompositeSignalMode`, execution overlay behavior
- `crates/strategies/src/records.rs`
  - vectorbt-style trade record arrays and trade-stat helpers
- `crates/optimizer/src/grid.rs`
  - deterministic grid search helpers
- `crates/optimizer/src/lib.rs`
  - shared optimizer concepts such as `ObjectiveKind`, `OptimizationMode`, `OptimizationResult`, and `OptimizationHeatmap`
- `crates/optimizer/src/mo.rs`
  - multi-objective search-space, domain, and sampler primitives used by `CompositeMo`
- `crates/optimizer/src/stream.rs`
  - streaming top-k and heatmap aggregation helpers

### Dependency and feature wiring

- dependency name in `Cargo.toml`: `vector-ta`
- Rust import path: `vector_ta`
- keep the `vector-ta` version aligned across workspace crates
- feature wiring:
  - `src-tauri/Cargo.toml`
    - `cuda` and `cuda-backtest-kernel` forward into `ta_app_core`
  - `crates/app_core/Cargo.toml`
    - `cuda` forwards into `vector-ta/cuda`
- current VectorTA CUDA pointer model in this repo:
  - `crates/app_core/src/gpu_backtest_kernel.rs`
    - the Double-MA kernel path is the intended pure VRAM route: existing resident device price buffers feed `VramMaInputs`, and VectorTA MA kernels write into caller-owned device buffers via `compute_period_ma*_into`
  - `crates/app_core/src/dynamic_indicator_adapter.rs`
    - the generic indicator path uses VectorTA device-pointer input / device-pointer output dispatch through `compute_cuda_device(..., CudaOutputTarget::DeviceF32)`
    - it currently prepares its own cached device uploads from `Candles` instead of borrowing the already-resident backtest runtime buffers
  - if a downstream kernel consumes a `CudaDeviceSliceF32Ref` row derived from a returned `DeviceMatrixF32`, keep the owning `DeviceMatrixF32` alive for the whole downstream use site
    - PIT already follows this keepalive pattern; other strategy-local device-regime builders should be checked against the same rule before assuming a pure pointer-out handoff

### Build / Dev / Release Workflow

- `web/package.json`
  - Node toolchain requirements, frontend scripts, generated-command checks, parity scripts, and request-builder benchmark entrypoints
- `scripts/run_repo_quality_gate.ps1`
  - canonical local-first static-analysis lane for broad non-options work: Rust fmt/check/clippy plus web lint/typecheck, with optional desktop typecheck, browser E2E QA, desktop smoke, and workspace tests
- `scripts/run_backend_release_gate.ps1`
  - backend/CLI-preview release lane: Rust-only checks, workspace tests, backend/headless CLI release builds, optional benchmark compilation, and optional perf gates
  - intentionally does not call npm, require `web/`, build `web/dist`, or run Tauri packaging; use this lane for backend-only checkouts where frontend sources are omitted or ignored
- `scripts/run_frontend_e2e_gate.ps1`
  - build-once frontend browser QA lane for workbench smoke, action coverage, core workflow E2E, and optional responsive/regression/a11y/baseline/exploratory slices; `-IncludeFullWorkflow` runs the full browser spec set instead of the sliced lane
  - `-IncludeInteractionLatency` opts into the separately serialized scoped warm mocked-browser latency lane; that lane performs its own attested production build under the benchmark lock
- `scripts/run_frontend_interaction_latency.ps1`
  - opt-in, benchmark-lock-serialized scoped warm mocked-browser measurement for trusted workspace-navigation and saved-result-detail clicks; it is not full application/native latency authority
  - attests exact source inputs and the built distribution, seals a fresh no-hardlink per-run distribution copy, preloads and serves only those attested bytes, hashes the effective fixture and measurement-harness file set, and requires a wrapper-held child-verifiable lease bound to the live named-mutex owner, process ancestry/start identities, token, and context
  - rejects reparse/ADS/hardlink output aliases, repeatedly validates parent/publication identity, uses fixed warmups/samples with retained readiness/quiet-window evidence and DOM-quiet plus post-paint boundaries, then writes fresh no-replace machine-readable evidence to a unique `output/playwright/frontend-interaction-latency/<run>/summary.json`
- `web/scripts/check-a11y-smoke.mjs`
  - Playwright + axe smoke coverage for the shipped desktop shell surfaces
- `docs/frontend_end_to_end_qa_automation_plan.md`
  - implementation plan for comprehensive click-and-use frontend QA automation, workflow matrix coverage, action coverage accounting, Playwright browser workflow QA, and WDIO desktop smoke boundaries
- `web/e2e/accessibility-review-matrix.md`
  - representative browser-first manual accessibility slices, browser-vs-desktop review boundary, and the review-log template for high-risk non-options UI changes
- `src-tauri/tauri.conf.json`
  - current Tauri app config and the main build handoff
  - `beforeDevCommand`: `npm --prefix ./web run dev -- --host 127.0.0.1 --port 1420`
  - `beforeBuildCommand`: `npm --prefix ./web run build`
  - `frontendDist`: `../web/dist`
  - `devUrl`: `http://127.0.0.1:1420`
- `tauri.conf.json`
  - compatibility mirror of the canonical `src-tauri/tauri.conf.json`
  - expected single path difference: root mirror `build.frontendDist` is `./web/dist` while the canonical config keeps `../web/dist`
  - keep it aligned via `scripts/check_tauri_config_sync.mjs`
- `docs/RELEASE.md`
  - Windows build variants, `cargo tauri build`, PTX packaging notes, public release checklist, asset naming, rollback posture, and compatibility/release validation commands

Current local quality gate to know:

- `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1`
  - use for broad non-options code-quality validation before handoff
  - add `-IncludeDesktopTypecheck` when desktop harness files move
  - add `-IncludeBrowserWorkflowQa` for the build-once browser QA bundle after frontend workflow or shell changes
  - add `-IncludeBrowserResponsiveE2E` and `-IncludeBrowserRegressionE2E` when responsive layout, rollback behavior, or regression specs move
  - add `-IncludeDesktopSmoke` only on machines with the Tauri/WebDriver desktop smoke setup
  - add `-IncludeWorkspaceTests` for higher-risk backend or cross-layer changes

Current verification order to know:

- run one workflow/perf/profile script at a time; do not overlap benchmark, faceoff, Nsight, or CUPTI sessions
- run the repo quality gate first for broad non-options changes
- run `scripts/run_workflow_gate.ps1` before CPU/GPU perf gates when the change could affect staged workflow timing rather than only a single benchmark surface
- treat perf/profile scripts as local-authority evidence and keep CI mirroring downstream from the local command contract

Current build/release variants to know:

- CPU-only
- GPU MA-sweep: `--features cuda`
- GPU kernel: `--features cuda-backtest-kernel`

Current PTX env vars documented in `docs/RELEASE.md`:

- `VECTORBT_USE_PREBUILT_PTX=1`
- `VECTORBT_REQUIRE_NVCC=1`

Current PTX artifacts to know:

- `crates/app_core/src/kernels/double_crossover.prebuilt.ptx`
- `crates/app_core/src/kernels/options_expression.prebuilt.ptx`

Important release/build caveat:

- GPU builds do not require a local CUDA Toolkit when the prebuilt PTX path is used

## Backend Semantics

Current backend labels matter more than many strategy names:

- `CpuOnly`
  - Indicators and backtest on CPU.
- `GpuOnly`
  - Indicators on GPU, copied to host, backtest on CPU.
- `GpuKernel`
  - Full GPU/VRAM path for supported routes.
- `GpuHybridKernel`
  - Hybrid or partially staged GPU route. In some current paths it is still close to the `GpuKernel` runtime, but product policy treats it as a distinct route.
- `Auto`
  - Request-time chooser, not a real execution backend.

## Task Routers

### If the task is optimization

Open in this order:

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `src-tauri/src/commands_optimize.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_strategy.rs`
- `crates/app_core/src/composite_profile_route.rs`

Only then open the large execution hubs:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`

### If the task is validation

Open in this order:

- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/validation_common.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/validation_core.rs`
- `crates/app_core/src/validation_pipeline.rs`

Only then open:

- `crates/app_core/src/composite_validate.rs`

### If the task is reports or shortlist export

Open in this order:

- `web/src/services/validationService.ts`
- `web/src/services/historyService.ts`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`

Only then open:

- `src-tauri/src/commands_reports.rs`

### If the task is artifacts, batch summaries, or run history

Use `docs/artifact_replay_report_evidence_contract.md` when changing report mutation semantics, evidence labels, replay/reopen labels, or output parity claims.

Open in this order:

- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/historyService.ts`
- `web/src/services/metaService.ts`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`

### If the task is meta selection or ML confidence

Open in this order:

- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.ts`
- `web/src/services/metaMlScopeBuilder.ts`
- `web/src/services/metaService.ts`
- `src-tauri/src/commands_meta.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`

Research-campaign orchestration truth anchors:

- frontend orchestration:
  - `web/src/services/metaService.ts`
  - `web/src/services/metaServiceCampaignStrategy.test.ts`
  - `web/src/services/metaServiceCampaignResume.test.ts`
  - `web/src/stores/taskStoreCampaignProgress.test.ts`
- backend orchestration:
  - `src-tauri/src/commands_campaign.rs`
- process-restart QA:
  - `src-tauri/src/bin/ta_campaign_restart_qa.rs`
  - `scripts/run_campaign_restart_qa.ps1`
  - the script drives nine campaign phases in separate CPU-only OS processes against marker-owned isolated app data: interrupted, resume, no-op, stale-resume, stale-no-op, injected partial failure, repaired retry, bounded 12-dataset catalog, and large-catalog no-op
  - it verifies strict identity/hash reuse, portfolio-feasibility synthesis, fail-closed corrupt or tampered summaries, partial-failure recovery, unique item/artifact/CSV paths, ordered durable-checkpoint progress, and bounded cleanup without provider credentials; it does not exercise packaged Wry IPC or make a performance claim
- keep these campaign contract layers explicit:
  - request contract owns dataset list, execution mode (`full`, `optimize_only`, `validate_only`), preload policy, validation payload, and optional auto-meta / auto-stage5 follow-ups
  - progress payload owns stage labels, counts, and summary-path disclosure during long-running work
  - saved campaign summary artifacts are the source of truth for retry, resume, and reveal behavior
  - supported campaign strategy surface is narrower than the full strategy catalog; `DynamicDoubleMa` and `MlSignal` remain explicitly rejected

### If the task is ML Signal

Open in this order:

- `web/src/services/mlSignalService.ts`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/ml_signal.rs`

### If the task is data loading, resampling, or Polygon cache/download

Open in this order:

- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/polygon.rs`

### If the task is live execution or broker integration

Open in this order:

- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/state.rs`

Use the lightweight safety gate before handoff for execution/broker-contract changes that do not need benchmarks or real credentials:

- `powershell -File scripts/run_execution_safety_gate.ps1`

### If the task is Candidate Detail, local stability, or follow-up analysis

Open in this order:

- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- `src-tauri/src/commands_analysis.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`

Current candidate-detail/local-stability coverage matrix:

- Double-MA family:
  - `DoubleMa`, `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct`, `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` use the Double-MA candidate-detail and local-stability path
- Composite-family analysis:
  - `CompositeCpu` supports both candidate detail and local stability on its own analysis path
  - `Asr` supports both candidate detail and local stability on the composite-family analysis path
  - `Vemr` supports both candidate detail and local stability on the composite-family analysis path
  - `Vcc` and `Rocb` support both candidate detail and local stability on the composite-family analysis path
  - `Vcet`, `Mkcs`, `Kqsr`, `Carm`, `Kvdf`, and `Cdmc` support both candidate detail and local stability on the composite-family analysis path
- `MlSignal` does not participate in optimizer candidate-detail/local-stability views; loading an `MlSignal` artifact should leave analysis views in the explicit non-optimizer state

Truth anchors for this coverage:

- `web/src/services/analysisService.ts`
- `web/src/services/analysisService.test.ts`
- `web/src/components/TopWorkspacePanel.test.ts`
- `web/src/components/CandidateDetailWorkspacePanel.test.ts`
- `src-tauri/src/commands_analysis.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`

### If the task is dynamic strategy IR or builder preview

Open in this order:

- `web/src/strategies/dynamicDoubleMaBuilder.ts`
- `src-tauri/src/commands_dynamic.rs`
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`

### If the task is moving-average or dynamic-indicator catalog support

Open in this order:

- `src-tauri/src/ma_registry.rs`
- `crates/app_core/src/indicator_catalog.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/movingAverageCatalogService.ts`
- `web/src/services/dynamicIndicatorCatalogService.ts`

### If the task is candle-field gating, MA whitelists, or frontend request-builder support

Open in this order:

- `web/src/utils/core/candleFields.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`

### If the task is frontend command wiring or capability text

Open in this order:

- `web/src/tauri/commands.ts`
- `web/src/tauri/invoke.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/strategies/compositeProfileGpuPolicy.ts`
- the relevant `web/src/services/*Service.ts`

Strategy-surface matrix truth anchors:

- hard legality first:
  - backend/request-builder truth in `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/validationRequestBuilder.ts`, `src-tauri/src/commands_campaign.rs`, and the backend route owners
- shared matrix/copy second:
  - `web/src/utils/strategyOptimizationCapabilities.ts`
  - `web/src/strategies/compositeProfileGpuPolicy.ts`
- UI gating third:
  - `web/src/components/OptimizeWorkspacePanel.vue`
  - `web/src/components/AdvancedWorkspacePanel.vue`
  - `web/src/components/MetaWorkspacePanel.vue`

If one layer disagrees with another, prefer:

1. backend/request-builder legality
2. capability-summary and route-preview truth
3. component copy or visibility logic

### If the task is GPU route policy or kernel eligibility

Open in this order:

- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/composite_profile_route.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

Only then open:

- `crates/app_core/src/kernels/double_crossover.cu`

The point is to understand route policy and runtime orchestration first, not jump straight to CUDA.

### If the task is GPU status, settings, or window/runtime environment

Open in this order:

- `src-tauri/src/commands_system.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/state.rs`
- `web/src/services/settingsService.ts`
- `web/src/stores/settingsStore.ts`

### If the task is workflow restoration, settings persistence, or app-shell behavior

Open in this order:

- `web/src/utils/core/workflowModel.ts`
- `web/src/App.vue`
- `web/src/stores/workflowStore.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/services/settingsBootstrap.ts`
- `web/src/utils/storage.ts`
- `web/src/stores/settingsStore.ts`
- `web/src/services/settingsService.ts`

### If the task is support-crate search mode, objective semantics, or stream aggregation

Open in this order:

- `crates/optimizer/src/lib.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/mo.rs`
- `crates/optimizer/src/stream.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `crates/strategies/src/composite_cpu/mod.rs`
- `crates/strategies/src/records.rs`

### If the task is headless tooling, route matrices, regression gates, or diagnostic binaries

Open in this order:

- `docs/performance_audit/README.md`
- `scripts/performance_audit_case_registry.json`
- `scripts/run_performance_audit_baseline_pack.ps1`
- `scripts/performance_audit_preflight.ps1`
- `scripts/performance_child_process.ps1`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_refactor_snapshots.rs`
- `crates/app_core/src/bin/ta_cli.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`

## Tauri Command Surface

The Tauri app currently exposes `132` commands from `src-tauri/src/lib.rs`.

Use `src-tauri/src/lib.rs` to see the public command registry and `src-tauri/src/commands.rs` to see the grouped surface. The actual command modules already form a good workflow map.

For strict performance-audit coverage, use `scripts/run_performance_audit_baseline_pack.ps1 -FailOnCoverageGap -FailOnActivePerfProcess -FailOnMissingReleaseBaseline`; it extracts the live handler list from `src-tauri/src/lib.rs` and the command-module list from `src-tauri/src/commands.rs`, rejects active performance processes, and reports every missing registry-declared release baseline without creating one.

### Command modules and ownership

- `src-tauri/src/commands_data.rs`
  - CSV load/resample, repo/sample CSV discovery, market-data root config, MA/indicator catalog queries, Polygon cache/download/repair/live stream, native file pickers
- `src-tauri/src/commands_options_data.rs`
  - options dataset download/import/list/manifest operations
- `src-tauri/src/commands_optimize.rs`
  - optimize entrypoints, run cancellation, progress sinks, product mode normalization
- `src-tauri/src/commands_validation.rs`
  - validation entrypoints, cross-ticker orchestration, price-bar projection
- `src-tauri/src/commands_campaign.rs`
  - research campaign execution and headless dataset-pack orchestration
- `src-tauri/src/commands_options_expression.rs`
  - options-expression run, feasibility, validation replay, artifact list/load, tuple/event-path inspection, and CSV export
- `src-tauri/src/commands_reports_api.rs`
  - public report/open/reveal/save wrappers
- `src-tauri/src/commands_reports.rs`
  - internal report router and bulky workbook builders
- `src-tauri/src/commands_simple_summary.rs`
  - simple Markdown summary export
- `src-tauri/src/commands_artifacts.rs`
  - thin facade for artifact CRUD/list/open operations
- `src-tauri/src/commands_meta.rs`
  - meta selection, Stage 4/5, ML-confidence model lifecycle and policy evaluation
- `src-tauri/src/commands_ml_signal.rs`
  - ML-signal export, score cache, validation, policy sweep, bundle status
- `src-tauri/src/commands_execution.rs`
  - one-shot strategy execution, live runner, Alpaca connectivity/snapshots
- `src-tauri/src/commands_portfolio.rs`
  - portfolio feasibility analysis
- `src-tauri/src/commands_analysis.rs`
  - candidate-detail and local-stability follow-up computations
- `src-tauri/src/commands_dynamic.rs`
  - dynamic strategy IR preview/formatting and constant-bit tooling
- `src-tauri/src/commands_system.rs`
  - GPU status and window effect control

### Tauri support modules

- `src-tauri/src/state.rs`
  - loaded datasets, derived dataset cache, cancellation state, live stream state, runner state
- `src-tauri/src/validation_common.rs`
  - shared cross-ticker validation helpers, candidate key normalization, and gate checks
- `src-tauri/src/run_artifacts.rs`
  - artifact persistence, summary extraction, tagging, and retention logic
- `src-tauri/src/commands_reports_shared.rs`
  - report path suggestion, sanitize/open/reveal helpers
- `src-tauri/src/settings.rs`
  - app-local settings persistence and market-data-root migration helpers
- `src-tauri/src/ma_registry.rs`
  - normalized moving-average descriptors for `list_moving_averages`
- `src-tauri/src/data_cache.rs`
  - binary candle cache helpers, source identity, sidecar reuse, and cache directory ownership
- `src-tauri/src/polygon.rs`
  - Polygon API-key storage, manifest, historical download, validate/repair, and live-stream internals

### Tauri runtime state

- `AppState`
  - owns loaded datasets, source identity, dataset versioning, derived-dataset invalidation, live stream state, live runner state, and the single-run guard used by optimization/validation flows
- `BackgroundTaskState`
  - owns generic background-task cancellation and is separate from run-level cancellation in `AppState`

Important consequence:

- `cancel_current_run` and `cancel_double_ma_optimization` are run-level controls
- `cancel_background_task` is a different generic cancellation path

### Runtime events

The current named event channels worth knowing are:

- `double_ma_progress`
  - optimization progress events from `commands_optimize.rs`
- `polygon_live_status`
  - Polygon live-stream lifecycle/status events from `commands_data.rs`
- `polygon_live_bar`
  - accepted live-bar events from `commands_data.rs`
- `live_execution_runner_event`
  - live-runner status and per-bar execution events from `commands_execution.rs`

### Boot and platform runtime setup

- `src-tauri/src/main.rs`
  - just the `run_tauri()` trampoline
- `src-tauri/src/lib.rs`
  - authoritative command registry and window/runtime setup
- current runtime explicitly uses a temp WebView2 data directory under `%TEMP%/vectorgrid/webview2`
- `src-tauri/src/commands_system.rs`
  - `set_window_effect` is platform-specific:
  - Windows: `mica`, `acrylic`, `blur`
  - macOS: `vibrancy`
  - other platforms: effectively no-op

## Frontend Runtime Map

The frontend is a Vue + Pinia workspace app, not a page-routed SPA.

- `web/src/main.ts`
  - bootstraps Pinia, drives the runtime boot stages, mounts the app early, and runs storage migrations, settings/workflow bootstrap, desktop startup wiring, and recommended optimization defaults
- `web/src/App.vue`
  - app shell, lazy panel router, and mount point for global overlays/bridges including the startup overlay
- `web/src/components/workbench/WorkbenchShell.vue`
  - production desktop shell for rail navigation, workflow stages, panel mount slots, inspector, and footer action-bar host
- `web/src/components/workbench/ResearchCockpit.vue`
  - flagged Research workbench overview, backed by `web/src/composables/useResearchCockpitModel.ts`
- `web/src/components/workbench/WorkbenchInspector.vue`
  - flagged right-side workbench inspector for selection, validation, history, tasks, and diagnostics context
- `web/src/stores/workflowStore.ts`
  - core workflow/tab state machine

### Frontend workflow model

`web/src/utils/core/workflowModel.ts` is the canonical source for tab/workspace vocabulary and allowed shell-state combinations, but it is not the only file that decides what the user actually sees.

- workflow tabs:
  - `data`, `optimize`, `validate`, `analyze`, `finalize`, `history`, `settings`
- workspace tabs:
  - `workflow`, `library`, `aggregate`, `pro`, `settings`
- aggregate tabs:
  - `meta`, `batch`, `saved`, `compare`
- controls tabs:
  - `data`, `opt`, `strategy`, `advanced`
- results tabs:
  - `overview`, `top`, `pareto`, `validate`, `drill`, `heatmap`, `history`, `meta`, `debug`

Visible-shell routing and restored state also depend on:

- `web/src/components/workbench/WorkbenchShell.vue`
  - decides which controls/results surfaces are visible and owns the shell mount slots such as `#metaVueShellMount`
- `web/src/stores/workflowStore.ts`
  - translates workspace/workflow changes into control/results selections, persists root shell attrs, and persists `aggregateTab`
- `web/src/services/workflowBootstrap.ts`
  - restores `workspace`, `workflowStep`, `controlsTab`, `resultsTab`, and `aggregateTab` from storage
- `web/src/components/MetaWorkspacePanel.vue`
  - owns the `meta` / `batch` / `saved` / `compare` aggregate switch once the shell has routed to the meta result surface
  - compare is explicitly provenance-scoped: working side = current Meta Selection output, baseline side = one Saved Meta artifact; Saved Stage 5 and Saved Policy Evaluation remain separate saved-library families instead of compare baselines
  - batch generation now exposes a compact operating-mode summary for execution mode, preload policy, automation mode, and selected dataset count without hiding retry/resume/reveal controls
  - the batch pane now also states the current holdout/skip posture, dataset-registry assumptions, and recovery contract directly in the UI: same-timeframe selection unless Data overrides, tags dedup on run, retry/resume depend on the saved batch summary, and both recovery flows force skip-completed behavior

If you are debugging navigation, visible panels, or why a tab is allowed or hidden, start with `workflowModel.ts`, then follow `WorkbenchShell.vue`, `workflowStore.ts`, and `workflowBootstrap.ts`.

### Global UI surfaces

`web/src/App.vue` does more than route lazy panels. It also mounts the cross-cutting app surfaces:

- `web/src/components/DesktopTitlebar.vue`
- `web/src/components/StartupAnimationOverlay.vue`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/components/GlobalShortcutBridge.vue`
- `web/src/components/AppRuntimeBanner.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/RunFooterPanel.vue`
- `web/src/components/WorkflowQuickSwitch.vue`

These are good first files when the task is startup flow, global shortcuts, quick actions, runtime banners, footer status, or dev-only workflow navigation.

### Shell and panel ownership

- `web/src/App.vue`
  - lazy async panel registry for workspace panels such as `DataWorkspacePanel`, `OptimizeWorkspacePanel`, `MetaWorkspacePanel`, `HistoryWorkspacePanel`, `ExecutionWorkspacePanel`, and `SettingsWorkspacePanel`
- `web/src/components/workbench/WorkbenchShell.vue`
  - owns the workbench rail, cockpit placement, inspector placement, Vue panel mount slots, and action-bar host
- `web/src/components/workbench/ResearchCockpit.vue`
  - owns the Research cockpit cards and workflow-step shortcuts; use `web/src/composables/useResearchCockpitModel.ts` for derived cockpit state
- `web/src/components/workbench/WorkbenchInspector.vue`
  - owns mode tabs and right-inspector actions; use `web/src/utils/inspectorModel.ts` for inspector state shape and action IDs
- `web/src/components/MetaWorkspacePanel.vue`
  - owns aggregate-subworkspace switching for `meta`, `batch`, `saved`, and `compare`
  - compare now surfaces selected baseline provenance directly in-panel while keeping compare summary store-driven
  - saved aggregate families keep family role and active selection visible while switching between Saved Meta, Saved Stage 5, and Saved Policy Eval so family identity does not collapse into generic “saved results”
  - the ML confidence section now separates model status, live Stage 5 output, and live policy-evaluation status into explicit summary cards and raises a visible warning callout when model health text is unhealthy or scan-error driven
- `web/src/components/ContextActionMenu.vue`
  - shared secondary-action menu primitive now used to collapse repeated saved-artifact action clusters without losing keyboard access or focus return
- `web/src/components/HistoryWorkspacePanel.vue`
  - owns saved-run inspection, rerun, reveal, apply, and the local Saved Runs fuzzy-search surface once the history/library surface is active
- `web/src/components/SettingsWorkspacePanel.vue`
  - owns workstation preferences, saved perspectives, diagnostics, desktop notification policy controls, and the local saved-perspective search surface

### `web/src` structure

- `components/`: `50` files
- `services/`: `49` files
- `stores/`: `30` files
- `strategies/`: `4` files

### Current frontend ownership model

- `components/`
  - workspace panels and UI shells
- `stores/`
  - long-lived domain and form state
- `services/`
  - request builders, Tauri invocation orchestration, artifact/report actions
- `tauri/`
  - generated command registry plus invoke/error wrappers
- `strategies/`
  - frontend-only strategy helpers such as composite GPU policy and dynamic Double-MA builder
- `utils/`
  - pure helpers and capability summaries

### Important stores

- `web/src/stores/dataStore.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/stores/strategyStore.ts`
- `web/src/stores/validationStore.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/advancedStore.ts`
- `web/src/stores/executionStore.ts`
- `web/src/stores/historyStore.ts`
  - owns saved-run search state, fuzzy ranking, selected-run detail hydration, and library actions
- `web/src/stores/settingsStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/stores/commandPaletteStore.ts`
- `web/src/stores/commandActionStore.ts`
- `web/src/stores/taskStore.ts`
- `web/src/stores/runtimeStore.ts`

### Important services

- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/optimizeService.ts`
- `web/src/services/validationRequestBuilder.ts`
- `web/src/services/validationService.ts`
- `web/src/services/analysisService.ts`
- `web/src/services/advancedService.ts`
- `web/src/services/metaService.ts`
- `web/src/services/mlSignalService.ts`
- `web/src/services/historyService.ts`
- `web/src/services/dataService.ts`
- `web/src/services/executionService.ts`
- `web/src/services/runControlService.ts`
- `web/src/services/recommendedOptimizationDefaults.ts`
  - boot-time and advanced-panel helper for recommended `topK` / `heatmapBins` defaults
- `web/src/services/gpuSpeedupsService.ts`
  - frontend GPU-speedup annotation service used by optimization and catalog-capability UI
- `web/src/services/settingsService.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/services/settingsBootstrap.ts`
- `web/src/services/movingAverageCatalogService.ts`
- `web/src/services/dynamicIndicatorCatalogService.ts`

### Composable infrastructure

- `web/src/composables/useQueuedInputWriter.ts`
  - shared queued/commit input writer used by panels such as optimize, advanced, settings, and meta
- `web/src/composables/useShellMount.ts`
  - shared shell mount helper for Vue-owned panel ownership over workbench mount slots
- `web/src/composables/useTaskLifecycleSummary.ts`
  - shared task/runtime summary model used by footer and diagnostics surfaces
- `web/src/composables/useHeadlessMatrixTable.ts`
  - shared TanStack-backed headless table model now used by Top, Validation, Meta, and execution-order review
- `web/src/composables/usePersistedMatrixColumnState.ts`
  - shared persisted column-visibility state currently adopted by Top Results, Validation, and Meta
- `web/src/composables/useUiReadiness.ts`
  - derives empty-state and readiness stages for history, meta, top, and validation surfaces
- `web/src/composables/useVirtualTableRows.ts`
  - shared virtualization helper for large table-style panels, especially Top, Validation, Meta, and execution-order review
- `web/src/composables/useAnchoredOverlay.ts`
  - shared anchored overlay positioning helper for floating UI surfaces
- `web/src/utils/perspectiveSearch.ts`
  - shared local fuzzy-ranking helper for saved perspectives; keeps Settings search scoped and subordinate to the global command palette
- `web/src/utils/iconography.ts`
  - shared minimal icon grammar for task state, compare/pin actions, artifact actions, and diagnostics badges

### Runtime events and task tracking

The frontend event path is:

- `web/src/tauri/events.ts`
  - event subscription wrapper
- `web/src/tauri/instrumentation.ts`
  - patches invoke, subscribes to current event channels, and normalizes payloads
- `web/src/stores/taskStore.ts`
  - task lifecycle, progress aggregation, live status capture, timeline, and toasts
- `web/src/utils/statusPresentation.ts`
  - shared lifecycle labels, severity labels, and timeline/alert summaries used by task/diagnostics UI
- `web/src/components/TaskCenterOverlay.vue`
  - task-center UI; now the primary high-signal task-state icon surface
- `web/src/components/RunFooterPanel.vue`
  - footer status summary
- `web/src/components/SettingsWorkspacePanel.vue`
  - current in-app diagnostics surface for runtime, task posture, timeline, recent alerts, desktop notification policy controls, and desktop-log filtering/copy/clear actions

Current named event channels handled on the frontend:

- `double_ma_progress`
- `research_campaign_progress`
- `live_execution_runner_event`
- `polygon_live_status`
- `polygon_live_bar`

Progress truth anchors:

- `double_ma_progress`
  - optimize/runtime progress from `src-tauri/src/commands_optimize.rs` and `crates/app_core/src/progress.rs`
- `research_campaign_progress`
  - staged dataset preparation, preload, optimization, validation, meta, and stage5 progress from `src-tauri/src/commands_campaign.rs`
- `load_price_data`
  - still a direct command in `src-tauri/src/commands_data.rs`; it does not currently emit incremental progress events and should be described as stage-based status only unless additive progress hooks are introduced
- current task/progress aggregation authority:
  - `web/src/stores/taskStore.ts`
  - `web/src/stores/taskStoreCampaignProgress.test.ts`
  - `web/src/stores/taskStoreEventThrottle.test.ts`
  - `web/src/services/optimizeService.test.ts`
  - `web/src/services/metaServiceCampaignStrategy.test.ts`

### Settings, bootstrapping, and persistence

- `web/src/utils/storage.ts`
  - localStorage helpers and schema migration
- `web/src/stores/workflowStore.ts`
  - persists shell attrs including `workspace`, `workflowStep`, `controlsTab`, `resultsTab`, and `aggregateTab`
- `web/src/services/workflowBootstrap.ts`
  - restores workspace/workflow/tab selection, including aggregate subworkspace restoration
- `web/src/services/settingsBootstrap.ts`
  - restores theme/effect/advanced-layout settings
- `web/src/stores/settingsStore.ts`
  - user-facing settings state
- `web/src/services/settingsService.ts`
  - window effect and settings orchestration
- `web/src/services/desktopProductization.ts`
  - desktop plugin bridge for logging, window-state persistence, and visibility-aware OS notification routing
- `web/src/utils/desktopNotificationPolicy.ts`
  - high-signal desktop-notification policy; success/failure routing now suppresses OS toasts while the desktop window remains visible
- `web/src/main.ts`
  - applies `applyRecommendedOptimizationDefaults()` during app boot
- `web/src/components/AdvancedWorkspacePanel.vue`
  - can reapply recommended optimization defaults from the advanced workspace

Important current-state note:

- the shell now exposes a discrete `live` workspace; `App.vue` mounts `SettingsWorkspacePanel.vue` only for `settings`, and `ExecutionWorkspacePanel.vue` only for `live`

### Saved run rehydration and history bridge

- `web/src/stores/workflowArtifactsStore.ts`
  - canonical active artifact handoff store
- `web/src/services/workflowArtifactsSync.ts`
  - rehydrates active workflow state from saved artifacts
- `web/src/services/historyService.ts`
  - library/history flows on top of saved artifacts

### Quick actions and command surfaces

The workbench exposes workflow-local action state through the inspector, action bar, command palette, and native menu bridge.

For command-palette and quick-action behavior, start with:

- `web/src/stores/commandPaletteStore.ts`
- `web/src/stores/commandActionStore.ts`
- `web/src/components/workbench/WorkbenchInspector.vue`
- `web/src/components/workbench/WorkbenchActionBar.vue`

### Frontend files that summarize behavior well

- `web/src/utils/strategyOptimizationCapabilities.ts`
  - best high-level file for strategy support, backend summary, heatmap/export behavior, and special-case workflows
- `web/src/utils/statusPresentation.ts`
  - best high-level file for current task/runtime/alert presentation vocabulary across footer, task center, and diagnostics
- `web/src/strategies/compositeProfileGpuPolicy.ts`
  - best high-level file for current composite route policy
- `web/src/strategies/dynamicDoubleMaBuilder.ts`
  - best high-level file for frontend dynamic-strategy IR building and preview input shaping
- `web/src/composables/useHeadlessMatrixTable.ts`
  - best high-level file for the current shared dense-table model
- `web/src/composables/usePersistedMatrixColumnState.ts`
  - best high-level file for current persisted dense-table column posture across Top, Validation, and Meta
- `web/src/composables/usePersistedTableDensity.ts`
  - shared persistent compact/comfortable/spacious density state for dense table and saved-list surfaces
- `web/src/components/ui/AppDataTableShell.vue`
  - shared dense-table chrome, toolbar contract, empty state, density classing, and keyboard-hint surface
- `web/src/components/ui/AppTableDensityControl.vue`
  - segmented density control used by Top, Validation, and History saved-run table surfaces
- `web/src/components/OptimizeWorkspacePanel.vue`
  - dense control-surface summary of what optimization UI currently exposes

### Usually skip on first pass

- `web/src/legacy/`
- most `*.test.ts`
- panel-template details unless the task is UI behavior

## Core Engine Map

### Optimization center of gravity

- `crates/app_core/src/double_ma.rs`
  - main Double-MA family execution hub
  - do not start here unless you already know the route policy question

Read these smaller files first:

- `crates/app_core/src/double_ma_mode.rs`
  - `Auto` mode resolution
- `crates/app_core/src/double_ma_backend.rs`
  - kernel blockers, route planning, backend summaries
- `crates/app_core/src/double_ma_ma_kind.rs`
  - Rust-side MA acceptance, period-based constraints, and single-output checks
- `crates/app_core/src/double_ma_vdps.rs`
  - `GpuVdps` rung sizing, decimation, anchor windows, candidate ranking, and seed-pair injection
- `crates/app_core/src/double_ma_result.rs`
  - result shaping, backend metadata, heatmap/grid output shaping, and follow-up cache metadata
- `crates/app_core/src/double_ma_strategy.rs`
  - strategy validation and gating
- `crates/app_core/src/composite_profile_route.rs`
  - `CompositeTrend` / `BreakoutMomentum` / `RegimeHybrid` route policy

### Validation layers

- `crates/app_core/src/double_ma_validate.rs`
  - Double-MA family validation entry, fixed-candidate GPU gating, and TrendQuality ADX-off CPU-fallback policy for parity safety
- `crates/app_core/src/composite_validate.rs`
  - strategy-local validation adapter/orchestrator for advanced strategies
- `crates/app_core/src/validation_core.rs`
  - shared outer validation loop
- `crates/app_core/src/validation_pipeline.rs`
  - PSR/DSR, CSCV/PBO, hard gates, robust ranking, run verdict

### GPU/runtime surface

- `crates/app_core/src/gpu_backtest_kernel.rs`
  - Rust-side CUDA runtime orchestration, scratch buffers, uploads, kernel dispatch
  - CUDA Graph execution has opt-in host-boundary instrumentation (`VECTORBT_KERNEL_CUDA_GRAPH_TIMING=1`) for capture, instantiate, immutable-topology update non-applicability, and replay submission; default execution remains uninstrumented
- `crates/app_core/src/staged_output.rs`
  - CPU-testable staged/synchronized atomic file publisher used by GPU export-all CSV so cancellation or failure cannot truncate an existing output
- `crates/app_core/src/vram_ma.rs`
  - VRAM-resident moving-average support and upload/compute helpers
- `crates/app_core/src/composite_gpu_metric_decode.rs`
  - non-Double-MA GPU metric-row decoding helpers
- `crates/app_core/src/strategy_gpu_runtime.rs`
  - GPU strategy executable/runtime cache
- `crates/app_core/src/strategy_gpu_program.rs`
  - compiled GPU predicate program representation
- `crates/app_core/src/kernels/double_crossover.cu`
  - low-level CUDA kernel implementation, not the first file to open

### Support modules worth knowing

These are smaller support modules that often answer high-level questions faster than the big execution hubs:

- `crates/app_core/src/indicator_catalog.rs`
  - normalized dynamic-indicator capability descriptors
- `crates/app_core/src/dynamic_indicator_adapter.rs`
  - CPU/CUDA compute bridge for dynamic indicators
- `crates/app_core/src/indicator_factory.rs`
  - indicator/moving-average factory helpers and batch request shaping
- `crates/app_core/src/timeframe.rs`
  - timeframe parsing and resample helpers
- `crates/app_core/src/csv_compat.rs`
  - CSV candle compatibility loader
- `crates/app_core/src/cuda_device.rs`
  - preferred CUDA device discovery and reporting
- `crates/app_core/src/request_normalization.rs`
  - normalization helpers for request defaults and invariants
- `crates/app_core/src/progress.rs`
  - progress payloads and sink traits used by Tauri orchestration
- `crates/app_core/src/iterative_sim.rs`
  - iterative validation simulation engine and intrabar rules
- `crates/app_core/src/validation_stats.rs`
  - PSR/DSR and probability gate helpers
- `crates/app_core/src/stats_common.rs`
  - shared statistical primitives reused across validation paths
- `crates/app_core/src/sharpe_stats.rs`
  - Sharpe-family utility functions
- `crates/app_core/src/cscv_pbo.rs`
  - CSCV/PBO request/result helpers
- `crates/app_core/src/prescan_predictive.rs`
  - predictive prescan helpers for search-space reduction
- `crates/app_core/src/matrix_index.rs`
  - matrix indexing helpers for batched row/column work
- `crates/app_core/src/alloc_policy.rs`
  - allocation policy helpers for hot-path buffers

### Allocation policy truth anchors

Treat `crates/app_core/src/alloc_policy.rs` as the only shared helper surface for repo-approved uninitialized / prefix-initialized allocation patterns.

Current hot consumer anchors:

- `crates/app_core/src/double_ma.rs`
  - overwrite-heavy MA/regime/value buffers
- `crates/app_core/src/gpu_backtest_kernel.rs`
  - host staging, heatmap, shortlist, and scratch buffers
- `crates/app_core/src/standalone_gpu_runtime.rs`
  - reusable runtime staging buffers

Current policy boundaries:

- new unsafe allocation tricks should go through `alloc_policy.rs`, not ad hoc local helpers
- warm-prefix semantics stay observable behavior and must remain explicit at the call site
- debug-poison behavior in debug/test builds is intentional and part of the current safety story

### Dynamic strategy surface

- `crates/app_core/src/strategy_ir.rs`
  - static IR profiles and indicator kinds
- `crates/app_core/src/strategy_plan.rs`
  - plan compilation and backend routing
- `crates/app_core/src/strategy_dynamic.rs`
  - dynamic plan compilation helpers
- `src-tauri/src/commands_dynamic.rs`
  - IR preview/formatting tooling

## Strategy Surface

### Counts

- Exposed optimization kinds: `24` real kinds (`25` including `Unknown`)
- Double-MA family kinds: `11`
- Composite/advanced family kinds: `11`
- Special flows: `CompositeMo`, `MlSignal`
- Static strategy IR profiles: `5`
- Static IR indicator kinds: `7`
- Kernel-capable MA ids for Double-MA GPU-kernel path: `33`

### Strategy families

#### Double-MA family

| Kind               | High-level indicator/features                                                    | Optimize route                                                                              | Validation notes                                                                                                                                                                                                              |
| ------------------ | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `DoubleMa`         | fast/slow MA pairs, optional `halftrend_filter`                                  | `Auto`, `CpuOnly`, `GpuOnly`, `GpuHybridKernel`, `GpuKernel`                                | fixed-candidate validation can use CPU or GPU main route; `GpuOnly` falls back to CPU, and TrendQuality requests without ADX confirmation currently fall back to CPU on `GpuKernel`/`Auto` fixed-candidate validation         |
| `DynamicDoubleMa`  | constrained `strategy_ir` builder over static profiles/indicator kinds           | previewed against strategy-plan backend, then runs through normal Double-MA path            | no separate validator; resolves into normal runtime flow                                                                                                                                                                      |
| `Pit`              | EFI, VOSC, CFO                                                                   | Double-MA family backends                                                                   | double-like GPU main route; requires OHLCV, and the native device-regime path is still under investigation during long CUDA sweeps even though validation coverage is inherited through the shared Double-MA engine           |
| `Set`              | linear-reg angle, efficiency ratio, chop                                         | Double-MA family backends                                                                   | double-like GPU main route; requires high/low                                                                                                                                                                                 |
| `Mts`              | ROC, momentum, PPO                                                               | Double-MA family backends                                                                   | double-like GPU main route                                                                                                                                                                                                    |
| `Orc`              | RSI, stochastic fast K/D, Williams %R                                            | Double-MA family backends                                                                   | double-like GPU main route; requires high/low                                                                                                                                                                                 |
| `Marrt`            | thin Double-MA variant, mostly `long_only=false` preset                          | Double-MA family backends                                                                   | double-like GPU main route; current MA whitelist is a product/request-builder constraint (`sma`, `ema`, `alma`, `wma`, `zlema`, `vwma`) rather than a separate validator feature gap                                          |
| `Vfct`             | OBV smoothing + MFI confirmation                                                 | Double-MA family backends                                                                   | double-like GPU main route; requires high/low/volume, and current trend-MA `vwma`-only posture is a product/request-builder constraint rather than a separate validator feature gap                                           |
| `CompositeTrend`   | trend-quality regime with linear-reg angle, BB width, optional ROC, optional ADX | routed through Double-MA; `GpuKernel` without ADX, `GpuHybridKernel` with ADX, CPU fallback | treated as double-like fixed-candidate validation; TrendQuality-without-ADX fixed-candidate validation currently falls back to CPU on `GpuKernel`/`Auto` because the exact-combo GPU validation path is not parity-safe there |
| `RegimeHybrid`     | trend-quality regime plus ADX enter/exit and mean-reversion thresholds           | routed through Double-MA; current target `GpuHybridKernel`, CPU fallback                    | treated as double-like fixed-candidate validation                                                                                                                                                                             |
| `BreakoutMomentum` | Bollinger breakout, MACD, CMO, ADOSC, optional ADX                               | routed through Double-MA; `GpuKernel` without ADX, `GpuHybridKernel` with ADX, CPU fallback | treated as double-like fixed-candidate validation; requires high/low/volume                                                                                                                                                   |

#### Double-MA family truth anchors

- exact-grid Double-MA remains the truth anchor for optimization fidelity and any cheaper route claims
- `DynamicDoubleMa` is a constrained builder + preview surface:
  - preview happens first against the strategy-plan backend
  - execution then re-enters the normal Double-MA optimizer path
  - fixed-candidate validation replay is still intentionally unsupported
- `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` are routed Double-MA family members, not `CompositeCpu` members
- generic Double-MA TrendQuality requests without ADX confirmation, including `CompositeTrend`, currently fall back to `CpuOnly` on `GpuKernel`/`Auto` fixed-candidate validation until the exact-combo GPU parity issue is repaired
- `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, and `Vfct` inherit the full Double-MA validation stack; the real caveats on current repo posture are generic Double-MA route limits plus candle-field or product-surface restrictions, not missing validation modules
- the canonical code surfaces for this family are:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/double_ma_validate.rs`
  - `web/src/services/optimizeRequestBuilder.ts`
  - `web/src/services/validationRequestBuilder.ts`
  - `web/src/utils/strategyOptimizationCapabilities.ts`
- the bounded exact-vs-cheaper comparison surface lives in:
  - `crates/app_core/src/bin/real_data_fidelity_audit.rs`

#### Composite/advanced family

These currently use the `CompositeCpuRequest` product surface and `Grid` mode in product routes.

| Kind           | High-level indicator/features                                                         | Optimize route                  | Validation notes                                                                                                                                                                                                                                                                                                      |
| -------------- | ------------------------------------------------------------------------------------- | ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CompositeCpu` | fast/slow MA core plus HalfTrend, MACZ, OTT, `CompositeSignalMode`                    | CPU and `GpuKernel` route exist | CPU-side validation coverage is broad; GPU fixed-candidate validation is only eligible for plain base-composite `HardGate` finalists with uniform HalfTrend/MACZ/OTT/strategy/MA contracts, vectorized engine, and no inline `cross_ticker`                                                                           |
| `Asr`          | TTM squeeze, squeeze momentum, linear-reg angle, ER filter, z-score, Kaufman overlays | CPU and `GpuKernel`             | GPU route exists, but `Auto` currently prefers CPU on measured workloads                                                                                                                                                                                                                                              |
| `Vcc`          | TTM squeeze, squeeze momentum, TTM trend, optional momentum-acceleration gating       | CPU and `GpuKernel`             | mixed validation route; CPU-reference signal prep still matters, but `GpuKernel` now uses dedicated GPU walk-forward and reopt walk-forward helpers                                                                                                                                                                   |
| `Vemr`         | z-score, Bollinger bands, ATR stop                                                    | CPU and `GpuKernel`             | GPU route exists, but `Auto` currently prefers CPU                                                                                                                                                                                                                                                                    |
| `Rocb`         | Donchian/channel breakout plus ATR risk overlay                                       | CPU and `GpuKernel`             | GPU route exists, but `Auto` currently prefers CPU                                                                                                                                                                                                                                                                    |
| `Vcet`         | VIDYA trend-state plus Chandelier Exit                                                | CPU and `GpuKernel`             | main fixed-candidate GPU route exists; current runtime tests cover walk-forward, reopt walk-forward, CSCV/PBO, SPA, perturb, and trade bootstrap on `GpuKernel`                                                                                                                                                       |
| `Mkcs`         | standard-deviation compression plus Donchian breakout                                 | CPU and `GpuKernel`             | full current validation module coverage exists on the active route; `Auto` still prefers CPU                                                                                                                                                                                                                          |
| `Kqsr`         | KDJ, stochastic RSI, IFT-RSI, QQE, PRB                                                | CPU and `GpuKernel`             | full current validation module coverage exists on the active route; `Auto` still prefers CPU and the strategy requires OHLC                                                                                                                                                                                           |
| `Carm`         | Coppock, AO, ROCR, APO, OTTO HOTT/LOTT                                                | CPU and `GpuKernel`             | full current validation module coverage exists on the active route; mixed validation route, `Auto` still prefers CPU, and fixed-candidate `GpuKernel` walk-forward parity is now locked through exact desired-series window metrics while reopt stays on the generic exact-series path                                |
| `Kvdf`         | Keltner, VWAP, DTI, DevStop, FVG trailing-stop logic                                  | CPU and `GpuKernel`             | full current validation module coverage exists on the active route; `Auto` still prefers CPU and the strategy requires high/low/close/volume                                                                                                                                                                          |
| `Cdmc`         | DPO, correlation cycle, MAMA/FAMA, decycler, phase threshold                          | CPU and `GpuKernel`             | full current validation module coverage exists on the active route; mixed validation route, current measured policy is more GPU-favorable than CARM, and fixed-candidate `GpuKernel` walk-forward parity is now locked through exact desired-series window metrics while reopt stays on the generic exact-series path |

#### Standalone profile cluster truth anchors

- Cluster A membership is fixed today:
  - `Asr`
  - `Vcc`
  - `Vemr`
  - `Rocb`
  - `Vcet`
- Cluster A is the stronger bench-backed standalone-profile tier:
  - strategy-local owners live in `crates/app_core/src/{asr,vcc,vemr,rocb,vcet}.rs`
  - GPU companions live in the corresponding `*_gpu_kernel.rs` files
  - frontend request/capability truth anchors live in:
    - `web/src/services/optimizeRequestBuilder.test.ts`
    - `web/src/utils/strategyOptimizationCapabilities.test.ts`
    - `web/src/services/validationRequestBuilder.test.ts`
- Cluster A validation posture to keep truthful across preview, post-run summaries, and report export:
  - `Asr`, `Vemr`, and `Rocb` currently keep `Auto` CPU-preferred on measured validation workloads
  - `Vcc` keeps a mixed validation route with CPU-reference signal prep, but `GpuKernel` now has dedicated GPU walk-forward and reopt walk-forward coverage
  - `Vcet` now has runtime-tested `GpuKernel` coverage for walk-forward, reopt walk-forward, CSCV/PBO, SPA, perturb, and trade bootstrap; keep preview/report surfaces aligned to that broader support

- Cluster B membership is fixed today:
  - `Mkcs`
  - `Kqsr`
  - `Carm`
  - `Kvdf`
  - `Cdmc`
- Cluster B is the staged unified-GPU standalone-profile tier:
  - strategy-local owners live in `crates/app_core/src/{mkcs,kqsr,carm,kvdf,cdmc}.rs`
  - GPU companions live in the corresponding `*_gpu_kernel.rs` files
  - frontend request/capability truth anchors live in:
    - `web/src/services/optimizeRequestBuilder.test.ts`
    - `web/src/utils/strategyOptimizationCapabilities.test.ts`
    - `web/src/services/validationRequestBuilder.test.ts`
- Cluster B validation posture to keep truthful across preview, post-run summaries, and report export:
  - `Mkcs`, `Kqsr`, `Carm`, and `Kvdf` currently keep `Auto` CPU-preferred or CPU-fallback-sensitive validation policies even though the main validation module stack is already present on the active route
  - `Cdmc` keeps the mixed validation route where current measured policy is more GPU-favorable than `Carm`, while still using CPU-reference signal prep for exact CPU-series validation metrics
  - current `composite_validation_workflow` evidence on the shared 50k/8 `wf_reopt_cscv_spa` surface still measures `Carm` materially cheaper than `Cdmc`, so route policy and runtime cost ranking are not the same thing
  - Cluster B parity is now mostly about composite `cross_ticker` scope, narrower aux analytics versus the `DoubleMa` baseline, and measured mixed-route policy truth, not about missing core validation modules
- Validation result route provenance now includes `requested_backend`, `backend_used`, `backend_route`, `backend_fallback_reason`, and `validation_residency_tier`; Double-MA GPU validation can also include `execution_profile.cpu_tail_wall_ms` and GPU materialization counters.

#### CompositeCpu / CompositeMo truth anchors

- `CompositeCpu` is the broad strategy-local composite request/optimization family. It owns:
  - request/result shapes in `crates/app_core/src/composite_cpu.rs`
  - product request shaping in `web/src/services/optimizeRequestBuilder.ts`
  - analysis follow-ups in `web/src/services/analysisService.ts`
  - Tauri candidate-detail/local-stability bridging in `src-tauri/src/commands_analysis.rs`
- `CompositeCpu` is not a routed Double-MA family member. It keeps its own:
  - family-specific route diagnostics
  - candidate-detail path
  - local-stability path
  - native Top-K/include-all result surface
- `CompositeMo` is a distinct Pareto/sampler/search-space product surface, not a normal Top-K/grid optimizer. It owns:
  - sampler/search-space/prescan runtime in `crates/app_core/src/composite_mo.rs`
  - optimizer-core multi-objective primitives in `crates/optimizer/src/mo.rs`
  - frontend request normalization in `web/src/services/optimizeRequestBuilder.ts`
  - user-facing capability summaries in `web/src/utils/strategyOptimizationCapabilities.ts`
- `CompositeMo` keeps:
  - explicit sampler families (`Nsga2`, `Random`, `Tpe`, `Qmc`)
  - optional stage-0 GPU prescan narrowing
  - bounded exact-grid collapse for tiny discrete spaces
  - CPU-preferred validation, with current GPU fixed-candidate eligibility tied to the same plain base-composite `HardGate` contract as `CompositeCpu`, not to a strict `CompositeMo`-specific provenance blocker
  - an explicit prescan contract owned by `web/src/services/optimizeRequestBuilder.ts` and `crates/app_core/src/composite_mo.rs`:
    - `top_k`
    - `window`
    - `predictive_top_k`
    - `predictive_stride`
    - `device_id`
    - `require_gpu`
    - `objectives`
    - `mode`
    - `ma_pairs`
- Composite validation matrix to keep in sync across preview builders, post-run summaries, and workbook export:
  - `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` use the fixed-candidate Double-MA validation path in `crates/app_core/src/double_ma_validate.rs`, but TrendQuality-without-ADX `GpuKernel`/`Auto` requests currently preserve parity by falling back to CPU for that fixed-candidate path
  - plain `CompositeCpu` can keep the GPU fixed-candidate validation route only when finalist sets remain compatible with the current plain-base eligibility contract: `HardGate`, uniform strategy/indicator settings, vectorized engine, and no inline `cross_ticker`
  - `CompositeMo` remains CPU-preferred for validation, but the current GPU limit is candidate-shape-based plain-base eligibility rather than a separate `CompositeMo`-only blocker
  - standalone profile Cluster A and Cluster B families validate through `run_composite_validation` with family-specific route notes that must stay aligned with `web/src/services/validationRequestBuilder.ts`, `web/src/services/validationService.ts`, and `src-tauri/tests/validation_xlsx_report_tests.rs`
  - composite-family `cross_ticker` reruns already preserve CSCV/PBO and hard-gate thresholds for preserved finalists, but they do not yet rerun the full walk-forward, reopt walk-forward, SPA, perturb, or bootstrap stack
  - composite-family aux analytics currently expose `regime_slice_robustness` and `friction_stress`; `entry_margin_fragility`, `underwater_duration`, `checkpoint_robustness`, and `benchmark_slice_summary` remain Double-MA-only parity gaps

### Strategy-local owners and GPU companions

If the question is “how this specific strategy works,” start with the strategy-local file before broader orchestration layers:

- `Asr`: `crates/app_core/src/asr.rs` and `crates/app_core/src/asr_gpu_kernel.rs`
- `Vcc`: `crates/app_core/src/vcc.rs` and `crates/app_core/src/vcc_gpu_kernel.rs`
- `Vemr`: `crates/app_core/src/vemr.rs` and `crates/app_core/src/vemr_gpu_kernel.rs`
- `Rocb`: `crates/app_core/src/rocb.rs` and `crates/app_core/src/rocb_gpu_kernel.rs`
- `Vcet`: `crates/app_core/src/vcet.rs` and `crates/app_core/src/vcet_gpu_kernel.rs`
- `Mkcs`: `crates/app_core/src/mkcs.rs` and `crates/app_core/src/mkcs_gpu_kernel.rs`
- `Kqsr`: `crates/app_core/src/kqsr.rs` and `crates/app_core/src/kqsr_gpu_kernel.rs`
- `Carm`: `crates/app_core/src/carm.rs` and `crates/app_core/src/carm_gpu_kernel.rs`
- `Kvdf`: `crates/app_core/src/kvdf.rs` and `crates/app_core/src/kvdf_gpu_kernel.rs`
- `Cdmc`: `crates/app_core/src/cdmc.rs` and `crates/app_core/src/cdmc_gpu_kernel.rs`

### Strategy presets and kernel-ready cases

For high-level canned strategy configs and representative cases, start with:

- `crates/app_core/src/strategy_presets.rs`
- `crates/app_core/src/double_ma_test_support.rs`

These are the fastest high-level reads for kernel-ready or representative configurations across `CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`, `Pit`, `Set`, `Mts`, `Orc`, plus `VFCT` and `MARRT`.

#### Special flows

- `CompositeMo`
  - separate multi-objective sampler/search-space flow
  - optional GPU prescan via Double-MA backend seeding
  - not a normal Top-K/grid optimizer
  - validation is CPU-preferred; current GPU fixed-candidate eligibility is candidate-shape-based plain-base support rather than a distinct `CompositeMo`-only provenance gate
- `MlSignal`
  - separate research workflow for dataset export, external score import, validation, and policy sweep
  - dedicated CPU research path; score-cache validation is a separate iterative-replay boundary and policy sweep is a separate timing surface over axis expansion plus the same replay core
  - artifact-oriented outputs rather than heatmaps/export-all grids

### Important caveats

- `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` are not optimized through the old `CompositeCpu` route. They route through the Double-MA family.
- `DynamicDoubleMa` is mainly a builder/preview surface, not a separate execution engine.
- Validation policy is more nuanced than optimization policy. For advanced strategies, do not infer validation support only from optimize-route UI text.
- for composite-profile routes, `export-all` is disabled on CPU and hybrid routes and enabled only on the pure `GpuKernel` route

## Indicator And MA Surfaces

### Dynamic indicator surface

If you need high-level indicator capability metadata without reading `vector-ta` internals, start with:

- `crates/app_core/src/indicator_catalog.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/indicator_factory.rs`
- `src-tauri/src/commands_data.rs`
  - `list_dynamic_indicators`
- `web/src/services/dynamicIndicatorCatalogService.ts`

Important current-state note:

- dynamic-indicator catalogs include `dynamic_strategy_eligible`, which is the main bridge between the general indicator inventory and the dynamic Double-MA builder surface

### Moving-average registry surface

If you need moving-average capability metadata without reading the whole backend:

- `src-tauri/src/ma_registry.rs`
- `src-tauri/src/commands_data.rs`
  - `list_moving_averages`
- `web/src/services/movingAverageCatalogService.ts`

The normalized MA catalog already summarizes:

- CPU support
- CUDA support
- CUDA MA-sweep support
- CUDA-kernel support
- kernel preference by speedup table
- whether candles are required
- whether the MA is period-based
- whether it is single-output
- parameter schema and notes

The normalized indicator descriptors already summarize:

- input kind
- whether candles are required
- CPU single/batch support
- CUDA single/sweep/VRAM support
- GPU-kernel support
- GPU preference by speedup table
- output count and parameter schema

### Discovery catalog truth anchors

Treat the pre-run discovery layer as one family with different responsibility types:

- plain inventory:
  - repo/sample CSV discovery in `web/src/services/dataService.ts` and `src-tauri/src/commands_data.rs`
  - Polygon-cache dataset discovery via the same data-service/command family
- hard capability:
  - moving-average registry export in `src-tauri/src/ma_registry.rs` and `web/src/services/movingAverageCatalogService.ts`
  - dynamic-indicator registry export in `crates/app_core/src/indicator_catalog.rs` and `web/src/services/dynamicIndicatorCatalogService.ts`
- advisory telemetry:
  - GPU speedup hints in `src-tauri/src/commands_data.rs` and `web/src/services/gpuSpeedupsService.ts`

Keep the boundary explicit:

- hard capability governs what request builders may legally send
- advisory telemetry is guidance for copy/ranking/hints, not legality
- plain inventory tells the app what data exists, not which strategies are supported

Cross-consumer truth anchors:

- `web/src/services/dataService.test.ts`
- `web/src/services/movingAverageCatalogService.test.ts`
- `web/src/services/dynamicIndicatorCatalogService.test.ts`
- `web/src/services/gpuSpeedupsService.test.ts`
- `web/src/services/metaService.test.ts`

### Request-builder and candle-field authority

If you need to know which candle fields or MA ids are accepted before a request is even sent to Rust, start with:

- `web/src/utils/core/candleFields.ts`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/services/validationRequestBuilder.ts`

Important current-state notes:

- `VWMA` and `VPWMA` require `volume`
- `FRAMA` requires `high` and `low`
- `maSource` can require `open`, `high`, `low`, and/or `close`
- `web/src/services/recommendedOptimizationDefaults.ts`
  - owns frontend-recommended output defaults such as `Top K` and `heatmap bins`
- `optimizeRequestBuilder.ts` enforces strategy-specific MA restrictions such as:
  - `MARRT`: `sma`, `ema`, `alma`, `wma`, `zlema`, `vwma`
  - `VFCT`: `vwma` fast/slow trend lines only
- `optimizeRequestBuilder.ts` also enforces route-dependent feature gates before Rust sees the request:
  - extra shortlist objectives, shortlist confidence/union/slice modes, `include_trade_metrics`, and `exportAllCsvPath` are currently `GpuKernel`-only
  - `exit_mode = cross` currently requires `GpuKernel`
- `validationRequestBuilder.ts` is not only request construction; it is also the frontend authority for validation route summaries and warning text, including:
  - `GpuOnly` validation fallback behavior
  - composite lightweight cross-rerun behavior
  - `CompositeMo` fixed-candidate GPU-validation limits
  - plain `CompositeCpu` HalfTrend-uniformity warning text
  - `VCET` GPU main-route caveats

### Static strategy IR surface

`crates/app_core/src/strategy_ir.rs` currently exposes `5` static profiles:

- `DoubleMa`
- `CompositeCpu`
- `CompositeTrend`
- `RegimeHybrid`
- `BreakoutMomentum`

It currently exposes `7` static indicator kinds:

- `FastMa`
- `SlowMa`
- `HalfTrendTrend`
- `MaczHist`
- `OttLine`
- `TrendQualityRegime`
- `BreakoutRegime`

### Kernel-capable MA ids

`crates/app_core/src/ma_policy.rs` currently marks `33` MA ids as kernel-capable:

`sma`, `ema`, `wma`, `alma`, `dema`, `tema`, `jsa`, `smma`, `sqwma`, `highpass`, `swma`, `trima`, `sinwma`, `epma`, `wilders`, `maaq`, `mwdx`, `cwma`, `fwma`, `pwma`, `srwma`, `supersmoother`, `supersmoother_3_pole`, `zlema`, `nma`, `hma`, `jma`, `edcf`, `ehlers_itrend`, `cora_wave`, `vwma`, `vpwma`, `frama`

## Reports, Artifacts, Meta, And ML Signal

### App-local storage layout

Several workflows write to different app-local directories. This is easy to miss if you only read command names.

- `app_local_data_dir/app_settings.json`
  - persisted app settings, especially market-data root override
- `app_local_data_dir/data_cache`
  - binary candle cache for CSV loads and validation/meta rereads
- `app_local_data_dir/reports`
  - saved XLSX reports
- `app_local_data_dir/runs`
  - run artifacts, meta artifacts, Stage 4/5 artifacts, ML-confidence policy-eval artifacts, batch summaries
- `app_local_data_dir/models/confidence`
  - meta ML-confidence models
- `app_local_data_dir/models/ml_signal`
  - ML-signal model bundles
- `effective_market_data_root/polygon_cache`
  - Polygon historical cache and manifest
- `effective_market_data_root/polygon_cache/polygon_settings.json`
  - persisted Polygon API-key fallback config when keyring is unavailable
- `effective_market_data_root/polygon_cache/polygon_manifest.json`
  - Polygon dataset manifest and integrity metadata

### Reporting

- active export posture is validation-first
- current product flow builds validation XLSX workbooks through validation report builders
- older analysis-style workbook code still exists, but is legacy/reference

Start here:

- `web/src/services/validationService.ts`
- `web/src/services/historyService.ts`
- `src-tauri/src/commands_reports_api.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/commands_reports_csv.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/commands_reports_shared.rs`

Important current-state notes:

- saved XLSX files go to `app_local_data_dir/reports`
- `commands_reports_shared.rs` owns report-path suggestion plus `open_path` and `reveal_path` helpers
- validation workbook generation still has legacy compatibility mode for older validation artifacts without the full verdict contract
- validation shortlist CSV generation is a separate path from XLSX generation

### Artifacts

Start here:

- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/historyService.ts`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`

- persistence is centralized in `src-tauri/src/run_artifacts.rs`
- Tauri wrappers live in `src-tauri/src/commands_artifacts.rs`
- artifact families include:
  - run artifacts
  - batch summaries
  - meta-selection artifacts
  - Stage 4 artifacts
  - Stage 5 artifacts
  - ML-confidence policy-eval artifacts
- app-local artifact root is `app_local_data_dir/runs`
- retention is explicit:
  - regular run artifacts prune to newest `200`
  - meta and Stage 4/5 artifacts prune to newest `100`
- tagged artifacts are retained even when they are older than the prune threshold
- legacy `vectorbt_*` artifact kinds and filename prefixes are still accepted alongside current `vectorgrid_*` names
- run artifact save enriches artifacts with environment metadata plus dataset identity/content-fingerprint metadata when possible
- run artifact list, latest-run hydration, and run-detail reopen surfaces now carry canonical `report_xlsx_path` / `report_xlsx_exists` metadata; run detail still exposes `report_path` / `report_path_exists` as compatibility aliases for older callers
- load/delete/tag-update operations are constrained to paths inside the app runs directory
- ML-confidence policy-eval artifacts have list/save support and a dedicated prune helper; current retention intent is symmetric with the other saved aggregate artifact families unless explicitly tagged
- frontend treats artifacts as canonical cross-workflow handoff objects through `web/src/stores/workflowArtifactsStore.ts`

### Meta selection and ML confidence

Start here:

- `web/src/stores/analysisStore.ts`
- `src-tauri/src/commands_meta.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `web/src/services/metaService.ts`
- `web/src/services/metaRequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.ts`
- `web/src/services/metaBatchRequestBuilder.ts`
- `web/src/services/metaMlScopeBuilder.ts`

Important current-state notes:

- Stage 4 and Stage 5 currently share the same backend implementation path
- Stage 4 and Stage 5 currently write the same artifact path, and that path uses the Stage-5-style `vectorgrid_stage5_*` prefix; `stage4_*` response fields remain compatibility aliases around the canonical Stage-5 file contract
- default meta strategy in `analysisStore` is currently `CompositeMo`
- model families currently include `Auto`, `Logistic`, and `GbdtStump`
- saved meta-selection reports go to `app_local_data_dir/reports` and saved meta-selection JSON artifacts go to `app_local_data_dir/runs`
- meta ML-confidence models live under `app_local_data_dir/models/confidence` as `ml_confidence_<strategy>_<timeframe>.json`
- meta selection operates over saved run artifacts and filters by strategy/timeframe/tags and presence of validation results
- frontend builder ownership is intentionally split:
  - `metaRequestBuilder.ts` owns Stage 3 meta-selection request shaping
  - `metaStage4RequestBuilder.ts` owns the current Stage 5 compatibility request
  - `metaBatchRequestBuilder.ts` owns campaign request shaping
  - `metaMlScopeBuilder.ts` owns the lightweight ML scope object only
  - `metaService.ts` is the orchestration layer that composes those builders into command calls
- the Stage 3 meta panel now surfaces request-contract summaries for scope, holdout, grouping, shortlist posture, Stage 3 scoring ownership, and evidence-flag meaning directly in the UI instead of leaving them implicit in builder code
- the Stage 3 holdout summary now explicitly distinguishes `light` versus `heavy` mode semantics: `heavy` emits nested validation config, `light` omits it, and backend rank fallback/warnings still own the final holdout interpretation
- Stage 3 grouping remains backend-owned: clustered mode is the default, representative count stays clamped to `1..=3`, and `strict_shortlist` remains hard-enabled with shortlist fallback off
- evidence flags remain warning signals rather than hard gates, and the current UI now says so explicitly while the JSON/artifact payload remains the strongest evidence-detail surface
- the ML-confidence panel now surfaces contract summaries for:
  - scope identity and model-file naming
  - available rows versus trained rows
  - label-threshold provenance
  - backend-owned family selection plus resolved `model_kind`
  - health/stale/retrain state
  - apply-path separation between in-memory apply, saved Stage 5, and saved policy eval
- the ML-confidence panel now also states the Stage 5 / policy-eval contract split explicitly:
  - Stage 5 request scope resolves from the latest Meta artifact first and then Saved Meta fallback
  - Stage 5 selection order is weighted rank -> `min_ml_confidence` filter -> optional meta fallback -> decision buckets
  - `run_meta_stage5_ml_confidence` is the authoritative command while `runStage4()` remains a compatibility alias
  - Stage 5 saved replay is JSON-only today; `stage5_json_path` is the user-facing durable path while `stage4_json_path` remains a backend compatibility alias around the same artifact
  - policy evaluation is an offline saved-Meta history comparison over `meta_only`, `ml_only`, and `stage5_blended`, not the same thing as in-memory Apply or the current live Stage 5 rerank
- `analysisStore.ts` remains the canonical Meta workspace state surface, and its `stage4Saved*` / `metaStage4*` families are still internal compatibility names even though user-facing copy now speaks in Stage 5 terms
- current ML UI copy also states that auto target prefers per-dataset holdout rows when present, falls back to aggregate holdout gates otherwise, and treats explicit unhealthy models differently from stale/retrain freshness signals

### ML Signal

Start here:

- `web/src/stores/optimizeStore.ts`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/ml_signal.rs`
- `web/src/services/mlSignalService.ts`

Important current-state notes:

- workflow is separate from normal optimizer analysis
- `mlSignalService.ts` is the dedicated frontend owner for export, score-cache, validation, sweep, artifact persistence, and ML-specific busy/status text; it is not part of the Meta campaign/service family
- high-level stages are dataset export, score-cache generation, validation, and policy sweep
- shared shell primary actions are step-aware in ML mode:
  - `data` -> Export dataset
  - `optimize` -> Build score cache
  - `validate` -> Run ML validation
  - `analyze` -> Run ML policy sweep
  - `finalize` -> inspect-only; no shared primary dispatch
- results still persist through the run-artifact channel, but use an `ml_signal` sub-payload
- dataset export now surfaces split-plan activation, ticker-holdout normalization, and attached sidecars directly in the ML panel so users can see when they are doing a plain export vs a manifest-producing export
- split-plan gap/step semantics are now described in-panel, including the current blank-field contract of `gap=0` and `step=1`
- dataset-export and current-artifact views now distinguish the always-generated export manifest from the conditional split manifest, and keep score-cache manifests separate as stage-2 artifacts
- ML-signal model bundles are bundle-based, not meta-model-based, and live under `app_local_data_dir/models/ml_signal/<bundle_id>/manifest.json`
- packaged Windows rehearsal builds can ship the reviewed app-level CPU
  CatBoost 1.2.10 fallback at `$RESOURCE/native/catboostmodel.dll`;
  `commands_ml_signal.rs` resolves it when no bundle-local or explicit override
  path is present
- `scripts/baselines/catboost-native-v1.2.10.json` and
  `scripts/acquire_catboost_native.ps1` pin/reproduce the official Windows
  x86-64 asset, release/tag, 16,438,272-byte size, SHA-256, and Apache-2.0
  license; readiness, manifest generation, and packaged smoke enforce that
  technical provenance. Organizational redistribution approval and product
  signing remain external public-release requirements.
- preferred CPU-native bundle packaging is `<bundle>/native/catboostmodel.dll` next to `manifest.json`, with `native_library_path` kept relative when declared
- production CatBoost native discovery is restricted to explicit trusted
  overrides, reviewed bundle/resource locations, and contained manifest paths;
  current-working-directory and bare-name discovery are excluded from the
  production candidate set
- the normal desktop `MlSignal` panel now exposes `native_cpu` as the only normal selectable backend; legacy `auto` / `native_cuda` values remain replay-compatible for older saved runs and internal testing
- stage-2 ML copy now makes the score-CSV ingest contract explicit: `timestamp` is required, at least one of `p_long` / `p_short` / `raw_long` / `raw_short` must exist, and calibrated probability columns are preferred when present
- the ML artifact panel now treats feature schema path/version as an operational contract, not just metadata: schema version must stay aligned across exported datasets, score caches, and reopened ML runs
- score-cache bundle truth is the cache JSON plus manifest plus feature-schema metadata, with `score_space`, `score_scope`, `matched_rows`, `duplicate_timestamps`, and `coverage_ratio` carried into artifacts and UI summaries
- when validation context is present, ML-signal validation and policy sweep enforce strategy kind and dataset/timeframe compatibility against the loaded data
- stage-3 and stage-4 ML panel summaries now surface the active validation context, current policy form, probability requirements, and sweep budget note; the UI keeps its explicit `top_k` contract even though Rust still defaults omitted sweep `top_k` to `25`
- the current ML artifact section now exposes stage, dataset export path, export manifest, split manifest, score manifest, and score-summary metadata so replay state is inspectable without opening the saved artifact JSON
- ML-signal runs commonly leave `report_xlsx_path` empty even when they persist a run artifact
- saved-run load/apply/rerun restores ML shell mode and the matching workflow step from the persisted `ml_signal.stage` when available

## Data, Polygon, And Live Execution

### Data and resampling

Start here:

- `web/src/stores/dataStore.ts`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/settings.rs`
- `web/src/services/dataService.ts`

Important current-state notes:

- CSV load is deduplicated by source identity
- cached binary candle format is `.vta`
- the binary cache lives under `app_local_data_dir/data_cache` and can also reuse a sidecar cache next to the CSV when available
- sample CSV quick-loads come from `list_sample_csvs`
- repo-local CSV discovery for batch/meta flows comes from `list_repo_data_csvs`
- derived datasets are cached in memory as `base_id@timeframe`
- when a base dataset changes, derived descendants are invalidated
- market-data root override is persisted in `app_settings.json` via `src-tauri/src/settings.rs`
- `get_market_data_dir_info` exposes default, configured, and effective market-data roots plus the resolved Polygon cache dir
- `set_market_data_dir` requires an absolute writable directory and can best-effort move the existing Polygon cache when requested
- base and derived in-memory dataset retention are bounded separately inside `state.rs`

### Polygon historical and live

Start here:

- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`

Important current-state notes:

- historical Polygon support: `stocks`, `fx`, `crypto`
- live Polygon streaming: `stocks` only
- live stock streaming defaults to delayed mode unless explicitly disabled
- Polygon cache lives under `effective_market_data_root/polygon_cache`
- historical download always caches `1m` data first, then resamples upward in memory if needed
- dataset validation/repair covers parse errors, duplicates, out-of-order rows, and large same-day gaps; repair can optionally refetch gappy days when an API key is available
- live stream emits `polygon_live_status` and `polygon_live_bar`
- only one active live stream slot exists at a time
- Polygon API keys prefer OS keyring storage, with JSON-file fallback and migration logic
- live bars mutate the loaded dataset by `data_id`, not raw CSV path, and can invalidate derived resamples

### Live execution

Start here:

- `web/src/stores/executionStore.ts`
- `src-tauri/src/commands_execution.rs`
- `web/src/services/executionService.ts`

Important current-state notes:

- current live execution strategy coverage is limited to `DoubleMa` and `CompositeCpu`
- live execution reuses candidate-detail compute paths
- the frontend shell now exposes live execution as a dedicated `live` workspace instead of a settings-mounted execution pane
- broker integration is Alpaca over REST
- `dry_run=true` requires no broker credentials; non-dry-run requires Alpaca credentials
- Alpaca commands can now resolve credentials from secure backend storage when typed request credentials are omitted
- local paper execution persists app-local broker state and simulates bracket parent/child order groups when both stop-loss and take-profit are present; protective child orders are all-or-none, use a documented stop-loss-first same-bar tie break, cancel the sibling on fill, and remain local-paper only
- execution risk-policy settings persist in app settings; request-level risk overrides can tighten but not loosen saved max-notional, shorting, or bracket requirements
- Alpaca submissions persist credential-free intent records, use stable `client_order_id` lookup before submit, and append post-submit broker reconciliation events to live-session evidence
- execution strategy support is code-backed by `StrategyExecutionAdapter` in `src-tauri/src/commands_execution.rs`; currently supported live execution adapters are `DoubleMa` and `CompositeCpu`, surfaced through `get_execution_strategy_support_matrix`
- deterministic execution test harness primitives live in `src-tauri/src/execution_mocks.rs` for broker fills/partials/rejects/timeouts/cancel failures and feed duplicate/out-of-order/close/error events
- the runner emits `live_execution_runner_event` status messages as bars are processed
- runner cancellation remains intentionally silent on the event bus; the shipped runner event vocabulary is `started`, `stopped`, `triggered`, `queued_latest`, `signal_processed`, `order_submitted`, and `error`
- execution targets the delta between desired and current position; bracket orders are only attached when both stop-loss and take-profit are present, and local paper snapshots expose parent/group/leg/trigger fields for inspection
- the live runner depends on an active Polygon live stream, and runner start now enforces that the active stream symbol matches the requested execution symbol on both the frontend and Rust command boundary
- feed interruption now disarms the live runner and finalizes active session evidence instead of leaving automation armed implicitly
- only one active live execution runner slot exists at a time

### System and desktop runtime

Start here:

- `src-tauri/src/commands_system.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/main.rs`

Important current-state notes:

- `get_gpu_status` reports whether the build includes `cuda` and `cuda-backtest-kernel`, plus current device discovery status
- `set_window_effect` controls platform-specific vibrancy or blur behavior
- startup builds the Tauri window and uses a temp WebView data directory

## Verification Map

Verification in this repo is layered:

- module/unit/integration tests for behavior
- frontend parity checks for command drift
- output-parity checks for artifact/report shape
- perf gates for benchmark-backed regression control

For claim-level authority, benchmark boundaries, baseline policy, and release evidence hierarchy, see `docs/verification_benchmark_release_authority.md`. This map routes contributors to verification surfaces; the authority document defines which evidence can approve which claim.

### High-signal commands by area

| Area                                  | Commands                                                                                                                                                     |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| backend-only release rehearsal        | `powershell -File scripts/run_backend_release_gate.ps1`                                                                                                      |
| broad backend confidence              | `cargo test --workspace`, `cargo test -p ta_app_core`                                                                                                        |
| frontend confidence                   | `npm --prefix web run test`, `npm --prefix web run typecheck`, `npm --prefix web run lint`, `npm --prefix web run parity`, `npm --prefix web run a11y:smoke`, `npm --prefix web run test:browser:ux`, `npm --prefix web run test:e2e:workflow`, `npm --prefix web run test:e2e:workflow:full` |
| artifact/report shape                 | `node scripts/run_output_parity_check.mjs`                                                                                                                   |
| live execution / paper broker safety  | `powershell -File scripts/run_execution_safety_gate.ps1`                                                                                                      |
| workflow regression                   | `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`                                                                                             |
| CPU perf-sensitive backend change     | `powershell -File scripts/run_cpu_perf_gate.ps1 -Mode candidate`                                                                                             |
| GPU/CUDA path                         | `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`                                                                                             |
| GPU validation performance/residency  | `powershell -File scripts/run_gpu_validation_perf_gate.ps1 -Mode candidate`; use `-WorkloadProfile validation-heavy` only for validation-heavy CUDA evidence |
| GPU/CUDA ALMA contract rows           | `powershell -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`                                                                          |
| GPU profiling                         | `powershell -File scripts/profile_gpu_backtest_ncu.ps1`, `powershell -File scripts/profile_gpu_backtest_nsys.ps1`                                            |
| scoped warm mocked-browser latency    | `powershell -File scripts/run_frontend_interaction_latency.ps1`                                                                                              |
| frontend request-builder benchmarking | `npm --prefix web run bench:request-builder`                                                                                                                 |

### Important scripts

Authoritative gates:

- `scripts/run_workflow_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`

Specialized/manual gates:

- `scripts/run_frontend_interaction_latency.ps1`
  - fixed-policy, headless Chromium scoped warm mocked-browser click-to-painted-stable authority for representative workspace navigation and saved optimization-result detail; records the exact v2 manifest-attested build, flat content-addressed immutable snapshot, in-memory served-distribution identity, child-verifiable lock lease proof, canonical scoped Git-status identity, trusted clicks, path-matched detail hydration, retained per-sample readiness/quiet-window evidence, recomputed report statistics, browser/fixture identity, p95/max budgets, and explicit full-app/backend/native/cold/device exclusions
  - exact dirty snapshots remain usable as explicitly labeled local candidate rehearsal evidence; only clean-source reports set `release_evidence_eligible=true`, and the public release gate remains the final publication boundary
- `scripts/run_execution_safety_gate.ps1`
  - lightweight execution and paper-broker safety gate; runs targeted Rust execution, paper-broker, risk-policy, live-stream state tests plus frontend execution tests and command contract check without benchmarks or live credentials
- `scripts/run_provider_sandbox_qa.ps1`
  - separate provider lane: always runs offline Polygon/Alpaca paper contract mocks, performs no external request unless explicitly opted in, and can fail closed unless credentialed read-only evidence passes for every selected provider
- `scripts/run_gpu_alma_contract_rows_perf_gate.ps1`
  - specialized ALMA contract-row gate, not the default PR perf authority
- `scripts/run_cuda_graph_timing_probe.ps1`
  - specialized serialized CUDA Graph diagnostic that writes machine-readable capture/instantiate/update-not-applicable/replay-submit evidence
  - `-PlanOnly` is CPU-safe, defaults to the separate `cuda_graph_timing_plan.json` path, and does not build or run a benchmark; a real probe requires the `cuda,cuda-backtest-kernel` runner and a CUDA device
  - invalidates a bounded stale immediate-child JSON file in `output/benchmarks` with a non-passing `running` marker before launching work; any other existing destination must be replaced by a fresh path
  - graph reuse is keyed by both launch topology and every tile-specific valid-pair-map device pointer captured into immutable kernel-node parameters; pointer changes recapture instead of updating
  - replay timing covers the asynchronous `cuGraphLaunch` host call, not device completion or broad GPU performance authority
- `scripts/run_cuda_graph_multitile_parity.ps1`
  - specialized serialized CUDA Graph correctness regression for the captured tile-pointer boundary
  - fixes one deterministic large Double MA case, constrains the VRAM budget, and fails unless runtime geometry proves at least two full tiles share the same coarse launch signature
  - runs the exact case with graphs off and then on, requires exact best-parameter and integer-metric parity plus bounded continuous-metric parity, and requires every capture, instantiation, and replay to succeed
  - requires pointer-aware recapture count to equal replay count, so the former coarse-shape graph reuse bug cannot pass merely because its stale tile happens not to change the winner
  - rejects any existing `OutPath`, atomically reserves a fresh path with an owned non-passing marker, and atomically promotes only that marker to passing evidence
  - `-PlanOnly` is CPU-safe; a real run is GPU-required and benchmark-lock serialized
- `scripts/run_gpu_validation_perf_gate.ps1`
  - dedicated CUDA validation perf/residency gate; use `-ListCases` or `-PlanOnly` to inspect the covered validation workloads without running benchmarks
  - process discovery uses a bounded fail-closed Win32 snapshot; launched Cargo, benchmark, and structured-probe children have hard timeouts and cleanup is restricted to the root plus creation-identity-verified descendants
  - default `-WorkloadProfile release-authority` preserves the existing release evidence case set
  - `-WorkloadProfile validation-heavy` selects the heavier validation-mode case set with a separate default baseline/output path
  - structured probe JSON is profile-specific: `output/benchmarks/gpu_validation_release_authority_perf_structured_probe.json` or `output/benchmarks/gpu_validation_validation_heavy_perf_structured_probe.json`
  - `-CaseId <case-id>` can focus the built-in case registry before launching benchmark work
  - `-Quick` is a partial smoke lane: it selects the sparse Double-MA validation case when no case is provided, skips the structured probe, and writes to a separate default quick summary path so full-profile evidence is not overwritten
  - `-PlanOnly` writes to a separate default plan path so dry-run inspection does not overwrite candidate summaries
- `scripts/run_performance_audit_baseline_pack.ps1`
  - strict inventory combines `-FailOnCoverageGap`, `-FailOnActivePerfProcess`, and `-FailOnMissingReleaseBaseline`; `release_baseline_health.json` names every required path and inventory never synthesizes timing evidence
  - `-IncludeFrontendInteractionLatency` opts the serialized pack into the scoped warm mocked-browser trusted-click authority and records its summary alongside the other raw gate evidence
- `scripts/performance_audit_preflight.ps1`
  - shared bounded process-snapshot primitive plus read-only active-performance and release-baseline health checks
- `scripts/performance_child_process.ps1`
  - bounded GPU-validation child runner and exact launched-tree cleanup helper; cleanup refuses PID-reused or ancestry-unverified targets

Profiling:

- `scripts/profile_gpu_backtest_ncu.ps1`
- `scripts/profile_gpu_backtest_nsys.ps1`

Supplemental verification and baseline helpers:

- `scripts/run_output_parity_check.mjs`
  - canonical repo entrypoint for deterministic headless JSON/CSV/XLSX regeneration plus structural and semantic parity checking against the tracked bundled synthetic dataset
- `scripts/check_output_parity.mjs`
  - structural parity checker only; not a semantic diff and not a perf gate
- `scripts/output_parity_pairs.json`
- `scripts/workflow_benchmark_pack.json`
- `scripts/baselines/workflow_regression_baseline.json`
- `scripts/baselines/cpu_perf_baseline.json`
- `scripts/baselines/gpu_perf_baseline.json`
- `scripts/baselines/gpu_validation_perf_baseline.json`
- `scripts/baselines/gpu_alma_contract_rows/`

Output parity remains separate artifact/report authority. The
release-candidate gate now runs the tracked structural and semantic
JSON/CSV/XLSX parity pack by default, while
`scripts/run_output_parity_check.mjs` remains its focused entrypoint.

GPU validation perf remains opt-in during release rehearsal. Use `powershell -File scripts/run_release_candidate_gate.ps1 -Variant gpu-kernel -IncludeGpuValidationPerf` only when the release claim depends on validation CUDA route, residency, or CPU-tail behavior.

When another GPU-heavy process is active, restrict validation-gate work to `-ListCases` or `-PlanOnly`; full candidate/baseline modes build and run CUDA validation benchmarks.

Primary benchmark-running gates use `scripts/benchmark_lock.ps1` to serialize workspace benchmark execution with a fixed write-exclusive `output/qa/benchmark-lock/active.json` lease; the named Windows mutex is secondary same-session compatibility evidence. Nested children inherit exact owner PID/start, token/hash, file identity, lease hash/path, and context markers, then independently require ancestor identity, sharing violation 32, stable lease bytes, and bounded Restart Manager confirmation that the claimed ancestor is the actual holder. Stale files, ACL-only denial, split-holder spoofing, and PID reuse fail closed without deadlocking legitimate wrapper/child nesting.

`scripts/baselines/` is currently eligible for tracking in this working tree.
Treat only intentionally reviewed and committed baseline files as clean-clone
evidence; untracked or generated baseline files remain machine-local evidence.

Benchmark and faceoff helpers:

- `scripts/run_vectorbt_vs_vectorgrid_faceoff.ps1`
- `scripts/run_vectorgrid_double_ma_faceoff.ps1`
- `scripts/generate_double_ma_faceoff_dataset.py`
- `scripts/vectorbt_double_ma_faceoff.py`
- `scripts/vectorbt_double_ma_faceoff_scenario.json`
- `scripts/vectorbt_double_ma_faceoff_scenario_250k.json`
- `scripts/requirements-vectorbt-faceoff.lock.txt`

Maintenance and review helpers:

- `scripts/check_migration_freeze_scope.mjs`
  - narrow opt-in helper for explicit migration or stabilization windows; not a general repo-quality gate
  - current allowlist is intentionally limited to `web/`, `scripts/baselines/`, and a short set of migration-support docs/scripts
- `scripts/post_release_telemetry_review.mjs`
  - supplemental post-release review, not an authority gate
- `scripts/capture_optimization_memory.ps1`
  - telemetry-only workflow-process memory sampling, not a perf gate
- `scripts/export_baseline_samples.mjs`
  - sample/export helper; not parity authority by itself

### CI and release surfaces

- `.github/workflows/cpu-quality.yml`
  - prepared working-tree Windows CPU-only pull-request/push/merge-queue/manual workflow; it is not active remotely until the uncommitted workflow and supporting files are landed
  - defines the future branch-protection check `Required CPU quality`
  - installs locked Node/npm dependencies plus headless Chromium, then delegates to `scripts/run_cpu_ci_gate.ps1`
- `.github/dependabot.yml`
  - weekly GitHub Actions update lane for reviewing and advancing the workflow's immutable full-commit action pins
- `scripts/run_cpu_ci_gate.ps1`
  - fail-fast, serialized CPU CI aggregator over the canonical repo quality, frontend, browser, security-contract, and output-parity gates
  - intentionally excludes CUDA/GPU, performance/profiling, native WebDriver/Computer Use, packaging/signing, credentialed providers, and clean-machine install evidence
- `scripts/build_support_evidence_bundle.ps1`
  - fixed-allowlist, text-only, redacting support archive builder; verifies payload/ZIP equality and excludes binary dumps, settings, datasets, credentials, and arbitrary files
- `scripts/test_support_evidence_bundle.ps1`
  - terminal-only privacy, traversal, malformed-input, and archive-integrity canaries for the support bundle
- `docs/CI.md`
  - prepared workflow scope, exact future branch-protection check name, toolchain/dependency contract, diagnostic artifact policy, and separate-lane boundaries
- `.github/workflows.disabled/ci.yml.disabled`
  - stale archived GitHub Actions definition; not part of the prepared workflow and not a reactivation source
- `.github/workflows.disabled/desktop-smoke.yml.disabled`
  - stale archived desktop-smoke definition; native desktop coverage remains a separate manual/dedicated-runner lane
- `docs/RELEASE.md`
  - Windows build variants, PTX/prebuilt-kernel packaging notes, public release checklist, frontend migration notes, rollback posture, and release-facing validation commands
  - useful for packaging and release posture, but use `src-tauri/src/lib.rs` and request builders as authority for current command counts and request rules
- `tauri.conf.json` and `src-tauri/tauri.conf.json`
  - Tauri build/runtime configuration, including the built frontend handoff into `web/dist`

### Tauri integration tests

- `src-tauri/tests/settings_tests.rs`
  - settings and app-local config coverage
- `src-tauri/tests/validation_xlsx_report_tests.rs`
  - validation-report workbook generation coverage

### App-core benches and binaries

These are useful when the task is benchmarking, profiling, or reproducing specific backend behavior without the full desktop app:

- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/benches/double_ma_metrics.rs`
- `crates/app_core/benches/double_ma_validation_workflow.rs`
- `crates/app_core/benches/double_ma_followup_workflow.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`
- `crates/app_core/benches/composite_cpu_opt.rs`
- `crates/app_core/benches/composite_mo_opt.rs`
- `crates/app_core/benches/dynamic_strategy_cpu.rs`
- `crates/app_core/benches/meta_select_workflow.rs`
- `crates/app_core/benches/ml_signal_workflow.rs`
- `crates/app_core/benches/ml_signal_validation.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/asr_opt.rs`
- `crates/app_core/benches/vcc_opt.rs`
- `crates/app_core/benches/vemr_opt.rs`
- `crates/app_core/benches/vcet_opt.rs`
- `crates/app_core/benches/rocb_opt.rs`
- `crates/app_core/benches/mkcs_opt.rs`
- `crates/app_core/src/bin/ta_cli.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`

Notes:

- `profile_gpu_dynamic_runner.rs`
  - gate-backed only through `scripts/run_gpu_perf_gate.ps1`; `--emit-json` cache-attribution output is currently supplemental evidence
- `ml_signal_workflow.rs` and `ml_signal_validation.rs`
  - current MlSignal timing evidence owners; use `ml_signal_validation_single_candidate` for score-cache validation and `ml_signal_policy_sweep_small` for sweep cost until a dedicated workflow gate exists
- `e2e_workflow_ml.rs`
  - Stage 5 ML confidence plus CompositeCpu/meta/holdout workflow evidence, not the primary MlSignal perf-authority surface
- `real_data_fidelity_audit.rs`
  - maintained evidence bin for exact-vs-cheaper optimization/validation comparisons
  - current CLI contract lives in `parse_args()` / `print_help()`
  - current artifact truth surface lives in `FidelityAuditArtifact`, `TimeframeAudit`, `CaseAudit`, `OptimizationFidelityAudit`, and `ValidationFidelityAudit`
- `profile_doublema_campaign_prefetch.rs`
  - supplemental/manual dataset-prefetch and staging diagnostic; useful for campaign/data-loading work, but not a primary product contract
- `real_data_optimizer_eval.rs`
  - supplemental/manual real-data optimizer comparison bin; useful for bounded backend measurements, not a desktop-facing contract
- `stage6b_kernel_report.rs`
  - exploratory kernel-report helper; treat output shape as diagnostic unless a script or doc explicitly freezes it
- `ta_cli.rs`
  - headless CLI parity surface for synthetic/data-path runs and script-backed faceoffs; preserve flags and output semantics when scripts depend on it

### Tauri-side headless and workflow tools

All non-product binaries under `src-tauri/src/bin/` are gated by the
non-default `maintainer-tools` feature so Tauri product bundles cannot absorb
test, profiling, or release-support executables. Direct Cargo invocations of
these binaries must include `--features maintainer-tools`, merged with any
required CUDA or NVTX features. The product binary remains gated by
`desktop-ui`.

- `src-tauri/src/bin/ta_workflow_regression.rs`
  - deterministic optimize -> validate -> meta -> holdout regression gate used by CI and scripts
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
  - supplemental/manual route-policy profiling for composite-profile-style cases
- `src-tauri/src/bin/ta_headless_validate.rs`
  - supplemental/manual headless report-generation and structural-parity helper, not a perf gate
- `src-tauri/src/bin/ta_options_expression_lifecycle.rs`
  - maintainer-only terminal harness for bounded options-expression run, list/load, tuple/event replay, validation, CSV export, and persisted-cache recovery checks
  - `scripts/run_options_expression_lifecycle_test.ps1` prepares an isolated fixture under `output/qa` and invokes the harness in two separate CPU-only processes; it does not open the desktop UI or mutate the source app-local data
- `src-tauri/src/bin/ta_campaign_restart_qa.rs`
  - maintainer-only production campaign-runner harness for durable checkpoint cancellation, separate-process restart/resume, idempotent no-op resume, stale-source recovery, portfolio feasibility, summary-integrity rejection, injected partial-failure repair/retry, bounded catalog identity/collision/progress checks, and referenced-artifact verification
  - `scripts/run_campaign_restart_qa.ps1` gives every run a unique Tauri app identifier, copies tracked synthetic fixtures into the run root, invokes nine separate CPU-only campaign phase processes plus bounded support/verification processes, and removes only marker-owned isolated app data unless explicitly retained for diagnosis
  - the 12-dataset lane is a bounded correctness case, not a benchmark or provider-sized scale claim; exact packaged Wry/foreground and true product aggregate-campaign journeys remain separate QA boundaries
- `src-tauri/src/bin/ta_cpu_reliability_soak.rs`
  - maintainer-only, synthetic-data CPU reliability worker that keeps one native process alive across repeated load, optimize, validate, artifact catalog/hydration, cleanup, and cancellation cycles
  - `scripts/run_cpu_reliability_soak.ps1` provides a four-cycle short smoke plus an opt-in six-hours-or-longer operator-authority mode; use `-CargoTargetDir <dedicated-path>` or a validated `CARGO_TARGET_DIR` to keep build scratch off the repository drive
  - the wrapper serializes build plus execution with the benchmark mutex, builds `--locked --offline` into an explicitly pinned target, clears compiler/profile overrides and credentials, traps external HTTP traffic, rejects filesystem-root/reparse/overlapping target paths, recovers only stopped marker-owned isolation trees, and attests the Cargo-emitted binary, launcher/toolchain environment, and deterministic source manifest (including the vendored `vector-ta` patch)
  - native schema-2 evidence verifies mutex contention and strict Tauri app-data containment, keeps retained samples bounded while calculating full-run private/working-set/handle and owned file/byte/directory trends, and proves positive-partial-progress optimization cancellation; short-mode inputs require at least 2,048 rows so the pinned 64 MiB cancellation workload spans multiple CPU tiles, and an operator result cannot claim release authority unless a fresh release build reaches at least six hours with at least 30 measured cycles and no sample gap above 15 minutes
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
  - supplemental/manual validation-cost measurement helper
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
  - app-path benchmark/faceoff utility
- `src-tauri/src/bin/ta_refactor_snapshots.rs`
  - parity snapshot helper for refactor work

Headless family role split:

- `crates/app_core/src/bin/ta_cli.rs`
  - bounded optimize-and-print utility for synthetic or CSV-backed Double-MA runs
- `src-tauri/src/bin/ta_headless_validate.rs`
  - optimize -> validate -> JSON/validation-shortlist-CSV/XLSX report builder for manual or script-driven report generation
- `src-tauri/src/bin/ta_workflow_regression.rs`
  - deterministic workflow-regression gate with timing and holdout summaries

When changing these surfaces, preserve:

- argument semantics that existing wrapper scripts depend on
- the current output-role split between stdout JSON, saved JSON summaries, and saved XLSX workbooks
- the app-core-only versus Tauri-linked boundary between the tools

## Large Files To Avoid Opening First

Avoid opening these first unless the task clearly lives there:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/commands_reports.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/polygon.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

For each of those, this map already lists the smaller routing files to read first.
