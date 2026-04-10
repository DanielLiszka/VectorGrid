# VectorGrid Deep Research Synthesis

Date: 2026-04-09

Scope: repo-fit synthesis for reviewed response documents currently saved in `responses/`.

## Batch status

This document is an in-progress synthesis for the current reviewed response set.

- It records the current repo-fit review, implementation judgment, and cautions for:
  - `DR-01` through `DR-05`
  - `DR-06`
  - `DR-07` through `DR-15`
- It is intended to be appended to after additional deep-research responses are added to `responses/`.
- Later additions should preserve the judgments already recorded here unless new evidence from later responses or repo inspection materially changes them.

## Snapshot

| ID | Question | Response File | Status | Repo Fit | Change Shape | Outcome |
| --- | --- | --- | --- | --- | --- | --- |
| `DR-01` | selective parity with VectorBT | `responses/deep-research-report.md` | reviewed | high | mostly additive | `Adopt Later` |
| `DR-02` | experiment comparison and lineage | `responses/deep-research-report (1).md` | reviewed | high | additive | `Adopt` |
| `DR-03` | portfolio and basket research parity | `responses/deep-research-report (2).md` | reviewed | medium | mostly additive | `Adopt Later` |
| `DR-04` | validation stack and overfitting defense | `responses/deep-research-report (4).md` | reviewed | medium-high | additive | `Doc Only` |
| `DR-05` | route provenance, fallback honesty, and artifact metadata | `responses/deep-research-report (3).md` | reviewed | high | additive | `Adopt` |
| `DR-06` | GPU runtime architecture for repeated optimization and validation | `responses/deep-research-report (14).md` | reviewed | medium-high | mostly additive | `Adopt Later` |
| `DR-07` | benchmarking and public comparison discipline | `responses/deep-research-report (5).md` | reviewed | high | additive | `Doc Only` |
| `DR-08` | live execution operator safety | `responses/deep-research-report (6).md` | reviewed | medium-high | additive | `Adopt Later` |
| `DR-09` | minimum useful options parity envelope | `responses/deep-research-report (7).md` | reviewed | medium-high | additive | `Doc Only` |
| `DR-10` | options realism lanes and semantic clarity | `responses/deep-research-report (8).md` | reviewed | high | additive | `Doc Only` |
| `DR-11` | dataset catalogs, universes, and data-workflow parity | `responses/deep-research-report (9).md` | reviewed | medium | mostly additive | `Adopt Later` |
| `DR-12` | packaging, supportability, and release posture | `responses/deep-research-report (10).md` | reviewed | high | additive | `Doc Only` |
| `DR-13` | capability modeling to reduce frontend/backend drift | `responses/deep-research-report (11).md` | reviewed | medium | mostly additive | `Adopt Later` |
| `DR-14` | multi-objective workflow productization | `responses/deep-research-report (12).md` | reviewed | medium-high | additive | `Adopt` |
| `DR-15` | analysis and visualization parity | `responses/deep-research-report (13).md` | reviewed | high | mostly additive | `Adopt Later` |

## Per-question review

### DR-01 - Selective parity with VectorBT

#### Research claim summary

- The response argues that the highest-value parity gaps are analysis/visualization depth and experiment comparison/lineage.
- It also identifies dataset catalog, constrained portfolio research, constrained feature engineering, and resumability as lower-priority or backlog items.
- It explicitly rejects unrestricted callback-based portfolio simulation and arbitrary expression evaluation as poor fits for VectorGrid.

#### What seems valid

- The ranking is directionally consistent with the repo's own parity and improvement docs.
- The rejection of unrestricted Python-like openness is correct and aligned with repo constraints.
- The recommendation to prioritize inspectability and comparison over broad new engines is a good fit.

#### What conflicts with VectorGrid constraints

- Nothing material, but the “Implement Now” label is too broad if treated as one parity program.
- Parts of the answer overlap with existing repo strategy docs rather than adding new decisions.

#### Change shape

- `Mostly Additive`
- Smaller additive fallback:
  - do not create a general “VectorBT parity” program
  - split this into two bounded tracks:
    - experiment comparison/lineage
    - analysis/inspection UX

#### What the repo already has

- Existing parity and competitive-planning docs already cover much of this framing:
  - `research/VECTORGRID_FEATURE_PARITY_ROADMAP_VS_VECTORBT_PRO_2026-03-19.md`
  - `research/VECTORGRID_COMPETITIVE_IMPROVEMENT_PLAN_VS_VECTORBT_PRO_2026-03-19.md`
- The current repo already has candidate compare surfaces inside result workflows, but not a broader artifact-level compare and lineage layer.

#### Decision

- `Adopt Later`

#### Smallest useful repo action

- Treat `DR-01` as prioritization guidance, not as a standalone implementation plan.
- Use it to justify `DR-02` and a future analysis/inspection slice rather than opening a broad parity initiative.

#### Candidate touchpoints

- `research/VECTORGRID_FEATURE_PARITY_ROADMAP_VS_VECTORBT_PRO_2026-03-19.md`
- `IMPROVEMENT_MASTER_PLAN_2026-04-08.md`

#### Verification needed

- none beyond repo-fit review unless translated into a real implementation slice

### DR-02 - Experiment comparison and lineage

#### Research claim summary

- The response recommends a thin metadata/index layer over existing artifacts:
  - canonical request and dataset digests
  - explicit parent-child links between related artifacts
  - diff-first run comparison
  - candidate-to-validation lineage

#### What seems valid

- This is a real gap in the main run-artifact/history flow.
- The recommendation is additive and consistent with local-first artifacts.
- The “thin index over files” approach fits VectorGrid better than a heavier experiment platform.

#### What conflicts with VectorGrid constraints

- The response slightly overstates how much is missing:
  - the repo already persists `config_fingerprint`
  - normalized `dataset_meta` already includes source-content fingerprinting and resample identity
  - the options-expression lane already uses parent-artifact links and replay provenance
- Candidate identity is still a repo-local question and should be solved before broad lineage work.

#### Change shape

- `Additive`
- Smaller additive fallback:
  - start with parent pointers + diffable run headers in general run artifacts
  - defer any heavier “artifact graph” UI beyond a two-run compare view

#### What the repo already has

- `src-tauri/src/run_artifacts.rs`
  - `config_fingerprint`
  - `dataset_meta`
  - `source_content_fingerprint`
- `web/src/services/optimizeService.ts`
  - run artifact payload already includes request, result, fingerprint, and dataset metadata
- `src-tauri/src/commands_options_expression.rs`
  - explicit `parent_artifact_id` support in the options-expression lane
- `web/src/components/OverviewWorkspacePanel.vue`
  - current candidate comparison surface

#### Decision

- `Adopt`

#### Smallest useful repo action

1. Add parent pointers for main derived artifact families where the parent is already obvious:
   - validation artifact -> originating run artifact
   - meta/stage artifacts -> originating run or source artifact set
2. Expose a two-run artifact compare view in the history/library flow that shows:
   - changed request fields only
   - dataset identity/fingerprint differences
   - route/fallback differences
3. Standardize a compact run header schema instead of inventing a separate index first.

#### Candidate touchpoints

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/stores/workflowArtifactsStore.ts`

#### Verification needed

- artifact schema tests
- output parity checks
- history/library UI tests
- regression checks that old artifacts still load

### DR-03 - Portfolio and basket research parity

#### Research claim summary

- The response recommends a bounded portfolio/basket layer:
  - saved universes
  - basket validation
  - constrained portfolio-of-candidates with simple weighting and rebalance rules
- It explicitly rejects open-ended portfolio engines.

#### What seems valid

- The product framing is sensible.
- Saved universes and basket validation are credible intermediate steps.
- The warning about semantic ambiguity around score-weighting and capital model is correct.

#### What conflicts with VectorGrid constraints

- This is not a low-risk additive feature in the same way as lineage/provenance.
- Even the “smallest useful slice” still requires new request/result types, new artifacts, and multi-layer orchestration.
- It is larger than the current “additive-only, low regression” target for near-term research adoption.

#### Change shape

- `Mostly Additive`
- Smaller additive fallback:
  - extract only `saved universes`
  - consider `basket validation summaries` later, without portfolio capital simulation

#### What the repo already has

- Cross-ticker validation already exists.
- Improvement and parity docs already identify constrained portfolio research as a strategic gap, but also as high-effort/high-risk.

#### Decision

- `Adopt Later`

#### Smallest useful repo action

- If anything is promoted from this research now, it should be `saved universes`, not portfolio simulation.

#### Candidate touchpoints

- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/services/validationService.ts`

#### Verification needed

- design spike first
- data/workflow schema review
- no implementation should start until the domain model is narrowed

### DR-04 - Validation stack and overfitting defense

#### Research claim summary

- The response recommends a tiered validation stack:
  - fast default stack with walk-forward, holdout, PSR/DSR, CSCV/PBO
  - stricter optional stack with SPA and time-series-aware bootstrap
- It emphasizes conservative confidence language and evidence-panel style presentation.

#### What seems valid

- The statistical framing is sound.
- The confidence-language guidance is good.
- The emphasis on selection-aware diagnostics is appropriate.

#### What conflicts with VectorGrid constraints

- The response is not a major new feature roadmap for this repo because most of the recommended machinery already exists.
- Some stricter methods would need local policy decisions, not just literature support.

#### Change shape

- `Additive`
- Smaller additive fallback:
  - treat this as validation wording/policy hardening rather than validation-method expansion

#### What the repo already has

- `web/src/services/validationRequestBuilder.ts`
  - walk-forward, reopt walk-forward, SPA, trade bootstrap, CSCV/PBO, cross-ticker, `min_psr`, `min_dsr`
- `web/src/components/ValidationWorkspacePanel.vue`
  - UI controls for the same validation stack
- `web/src/services/validationService.ts`
  - PSR/DSR and CSCV already surfaced in shortlist and summaries
- `src-tauri/tests/validation_xlsx_report_tests.rs`
  - workbook/report coverage for these modules

#### Decision

- `Doc Only`

#### Smallest useful repo action

- Use this research to tighten validation language and evidence presentation:
  - emphasize “evidence” and “selection-adjusted” wording
  - explicitly label holdout results as untouched by optimization
- Do not open a new backend validation-method project from this response.

#### Candidate touchpoints

- `web/src/services/validationService.ts`
- `web/src/components/ValidationWorkspacePanel.vue`
- validation report wording in `src-tauri/src/commands_reports_validation_common.rs`

#### Verification needed

- UI copy review
- report text parity / workbook tests if wording changes

### DR-05 - Route provenance, fallback honesty, and artifact metadata

#### Research claim summary

- The response recommends a structured `execution_provenance` block with requested/resolved/realized route, stage map, and fallback events, surfaced as a compact “Planned -> Realized” UI badge.

#### What seems valid

- This fits VectorGrid very well.
- The additive slice is narrow.
- The distinction between requested, resolved, and realized route is genuinely useful.

#### What conflicts with VectorGrid constraints

- The response overstates the size of the current gap:
  - the repo already records `backend_route` and `backend_fallback_reason`
  - the frontend already surfaces route summary and fallback summary in multiple places
- A detailed per-stage provenance system would risk over-instrumentation if done too early.

#### Change shape

- `Additive`
- Smaller additive fallback:
  - keep the first slice to planned/resolved/realized route plus compact fallback metadata
  - defer detailed stage-map expansion unless a real ambiguity remains

#### What the repo already has

- `web/src/services/optimizeService.ts`
  - route and fallback status text
  - artifact payload with request/result/fingerprint metadata
- `src-tauri/src/run_artifacts.rs`
  - strong run artifact and dataset metadata normalization
- `web/src/services/validationService.ts`
  - route/category/aux-tail summaries

#### Decision

- `Adopt`

#### Smallest useful repo action

1. Add a compact execution provenance block to run artifacts:
   - requested route
   - resolved route
   - realized route
   - fallback reason if route changed
2. Surface a single compare-friendly UI badge or header summary using that data.
3. Reuse existing `backend_route` and `backend_fallback_reason` instead of designing a new large provenance model from scratch.

#### Candidate touchpoints

- `web/src/services/optimizeService.ts`
- `src-tauri/src/run_artifacts.rs`
- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`

#### Verification needed

- artifact compatibility tests
- output parity checks
- route/fallback UI tests

### DR-06 - GPU runtime architecture for repeated optimization and validation

#### Research claim summary

- The response recommends a steady-state GPU runtime posture for repeated runs on the same dataset:
  - selective CUDA Graphs for repeated multi-kernel pipelines
  - caller-owned/reused device buffers
  - stream-ordered allocation and memory-pool reuse
  - fusion when it materially reduces memory traffic
  - profiling-led decisions rather than broad architecture rewrites

#### What seems valid

- The architectural guidance is sound and based on real CUDA best-practice sources.
- The response correctly distinguishes mature transferable patterns from ideas that are too repo-specific without profiling.
- It fits the repo’s existing direction around VRAM-resident execution, caller-owned buffers, resident caches, and plan/executable reuse.

#### What conflicts with VectorGrid constraints

- This is not a near-term additive feature slice; it is architecture guidance for future GPU work.
- The biggest decision points it highlights are explicitly profiling-sensitive, which means the response should not be treated as permission for broad refactors.
- Several recommended patterns are already partly present in the repo, so the immediate gap is selective extension rather than missing architecture from scratch.

#### Change shape

- `Mostly Additive`
- Smaller additive fallback:
  - keep this as a GPU architecture guardrail
  - only promote narrow profiling-backed slices such as:
    - borrowing already-resident buffers in generic indicator paths
    - selectively expanding CUDA Graph use where many fine-grained kernels repeat

#### What the repo already has

- `REPO_MAP.md`
  - already documents the Double-MA kernel route as the intended pure VRAM path and explicitly notes that the generic dynamic-indicator path still uploads from `Candles` instead of borrowing resident backtest buffers
- `crates/app_core/src/gpu_backtest_kernel.rs`
  - hot kernel path, runtime timing, VRAM-aware execution, and graph-timing fields
- `crates/app_core/src/double_ma.rs`
  - GPU-resident route policy, graph-capture timing fields, and VRAM prefetch/staging support
- `crates/app_core/src/strategy_plan.rs`
  - backend-owned compiled GPU plan structure
- `crates/app_core/src/strategy_gpu_runtime.rs`
  - compiled-plan and executable cache reuse
- strategy-local GPU kernels such as:
  - `crates/app_core/src/asr_gpu_kernel.rs`
  - `crates/app_core/src/carm_gpu_kernel.rs`
  - `crates/app_core/src/vcc_gpu_kernel.rs`
  - these already use resident-series cache patterns
- `crates/app_core/src/options_expression_gpu.rs`
  - already expects reusable GPU output buffers in parts of the options lane
- profiling harnesses already expose graph-related experimentation:
  - `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

#### Decision

- `Adopt Later`

#### Smallest useful repo action

- Do not open a broad “GPU runtime redesign” project from this response.
- Use it as a future architecture checklist for profiling-backed GPU work, with the most plausible later target being the documented gap where generic indicator GPU paths do not yet borrow already-resident runtime buffers.

#### Candidate touchpoints

- `REPO_MAP.md`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`

#### Verification needed

- local profiling first
- no architecture changes should be promoted without perf evidence on the current hot paths

### DR-07 - Benchmarking and public comparison discipline

#### Research claim summary

- The response recommends a benchmark charter plus disclosure package rather than headline speedup claims.
- It emphasizes semantic equivalence, warm vs cold boundaries, explicit CUDA synchronization, multi-trial statistics, and frozen public scenarios.

#### What seems valid

- This is directionally strong and aligned with how benchmark claims should be handled.
- The insistence on semantic alignment and explicit timing boundaries is especially useful for this repo.
- The recommendation is additive and does not try to distort the product around a benchmark.

#### What conflicts with VectorGrid constraints

- This is mostly a documentation/process improvement, not a product feature.
- The repo already has benchmark runbooks, faceoff docs, perf gates, and app-path benchmarking utilities, so the gap is discipline standardization rather than missing measurement capability.

#### Change shape

- `Additive`
- Smaller additive fallback:
  - keep it to a benchmark charter plus disclosure template
  - do not open a new benchmark harness rewrite unless an existing tool is proven inadequate

#### What the repo already has

- `README.md`
  - product-facing benchmark posture
- `research/VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_FACEOFF_PLAN_2026-03-11.md`
- `research/VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_FACEOFF_RESULTS_2026-03-12.md`
- `research/VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_CAPTURE_RUNBOOK_2026-03-12.md`
- `scripts/`
  - perf gates, faceoff runners, and workflow benchmarking surfaces already exist

#### Decision

- `Doc Only`

#### Smallest useful repo action

- Standardize a reusable benchmark charter and README-style disclosure block for future public comparisons.

#### Candidate touchpoints

- `README.md`
- `research/VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_FACEOFF_PLAN_2026-03-11.md`
- `research/VECTORBT_VS_TA_DESKTOP_DEMO_DOUBLE_MA_CAPTURE_RUNBOOK_2026-03-12.md`

#### Verification needed

- doc review only

### DR-08 - Live execution operator safety

#### Research claim summary

- The response recommends a live-safety envelope built around environment separation, feed freshness gating, explicit confirmation, kill switches, append-only audit logs, and fail-safe runner transitions.

#### What seems valid

- The safety framing is sound.
- The recommendation to treat delayed vs real-time feed state as an operator-facing constraint is a real fit for the current Polygon + Alpaca workflow.
- The emphasis on hard separation between dry-run, paper, and live is correct.

#### What conflicts with VectorGrid constraints

- The response is broader than the current low-regression target if interpreted as a large live-execution redesign.
- Broker-facing changes are inherently higher-risk than artifact/UI hardening.
- The repo already has some of the recommended surface:
  - explicit paper/live mode
  - dry-run support
  - delayed-feed toggle and status text
  - confirmation prompts for broker-armed actions
  - broker-order refresh and runner start/stop controls

#### Change shape

- `Additive`
- Smaller additive fallback:
  - start with stronger stale/delayed feed posture and explicit emergency controls
  - defer audit-log expansion and any broader runner-state redesign

#### What the repo already has

- `web/src/components/ExecutionWorkspacePanel.vue`
  - broker mode, delayed-feed settings, posture chips, and confirmation prompts
- `web/src/services/executionService.ts`
  - live-feed status handling and broker order refresh
- `web/src/stores/executionStore.ts`
  - action gating, blocked-reason summaries, and paper/live credential handling
- `src-tauri/src/commands_execution.rs`
  - current Alpaca REST integration, snapshot/status fetches, one runner slot, and stop-runner control

#### Decision

- `Adopt Later`

#### Smallest useful repo action

1. If this moves forward, start with:
   - explicit stale/delayed warning hardening
   - a broker-facing emergency control such as cancel-open-orders
2. Do not widen live automation breadth until those operator-safety basics are clearer.

#### Candidate touchpoints

- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- `src-tauri/src/commands_execution.rs`

#### Verification needed

- paper-trading integration tests
- manual broker-safety review
- no live-broker testing should happen casually from this research alone

### DR-09 - Minimum useful options parity envelope

#### Research claim summary

- The response recommends a bounded options parity target centered on deterministic contract selection, explicit execution assumptions, validation credibility, and assignment/exercise only when the product meaningfully supports short-premium semantics.

#### What seems valid

- The scope discipline is good.
- The recommendation to avoid a simulator rewrite and stay inside fixed-shape options semantics is correct for this repo.
- The assignment/exercise warning is correct in principle.

#### What conflicts with VectorGrid constraints

- The response is mostly confirming choices already present in the repo's options roadmap.
- It does not reveal an obvious missing immediate feature slice because the current options lane already intentionally keeps assignment/exercise out of scope.

#### Change shape

- `Additive`
- Smaller additive fallback:
  - keep using this as a scope guardrail and documentation aid
  - do not expand into slow LEAN-style brokerage realism

#### What the repo already has

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
  - bounded artifact-driven options posture
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`
  - explicit rejection of assignment/exercise modeling for the near-term product
- `src-tauri/src/commands_options_expression.rs`
  - parent-artifact replay and provenance enforcement

#### Decision

- `Doc Only`

#### Smallest useful repo action

- Use this response as a parity-boundary reference when future options scope questions come up.

#### Candidate touchpoints

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`

#### Verification needed

- none unless repo docs are updated

### DR-10 - Options realism lanes and semantic clarity

#### Research claim summary

- The response recommends a clean separation between aggregate, derived-mid, and quote-aware lanes, with truth-in-labeling and conservative ambiguity policies.

#### What seems valid

- The semantics are sound.
- The recommended lane posture matches the repo's current direction very closely.
- The emphasis on quote-aware as the only quote-side execution claim is correct.

#### What conflicts with VectorGrid constraints

- Very little. The main issue is that this is largely already landed rather than a new implementation target.

#### Change shape

- `Additive`
- Smaller additive fallback:
  - preserve lane wording consistency across artifacts and compare surfaces
  - do not reopen broader realism/provider expansion from this response

#### What the repo already has

- `web/src/components/OptionsExpressionPanel.vue`
  - explicit aggregate / derived-mid / quote-aware lane copy
- `options_research/options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md`
  - current realism-lane lock-in guidance
- `options_research/options_expression_artifact_summary_coherence_implementation_guide_2026-04-09.md`
  - artifact/compare coherence work that assumes the current lane posture

#### Decision

- `Doc Only`

#### Smallest useful repo action

- Keep this as a semantic consistency check, not as a new feature project.

#### Candidate touchpoints

- `web/src/components/OptionsExpressionPanel.vue`
- `options_research/options_expression_quote_aware_realism_semantic_hardening_implementation_guide_2026-04-09.md`

#### Verification needed

- existing options panel/report tests if wording changes

### DR-11 - Dataset catalogs, universes, and data-workflow parity

#### Research claim summary

- The response recommends a local dataset catalog plus first-class saved universes, with durable IDs, refresh history, and derived-series lineage.

#### What seems valid

- The general direction is useful.
- Saved universes and stronger dataset reuse would help cross-workflow consistency.
- The answer correctly prefers a bounded local-first catalog over a broad data-platform buildout.

#### What conflicts with VectorGrid constraints

- This is a larger domain addition than the current additive-only near-term threshold.
- The repo already has a lot of underlying dataset identity machinery:
  - `.vta` caching
  - source identity and content fingerprinting
  - Polygon manifests and cache metadata
  - dataset metadata inside artifacts
  - in-memory derived dataset reuse/invalidation
- The gap is first-class catalog/universe UX, not raw dataset identity.

#### Change shape

- `Mostly Additive`
- Smaller additive fallback:
  - start with saved universes and benchmark baskets only
  - defer a full dataset catalog domain with refresh-history UI

#### What the repo already has

- `src-tauri/src/data_cache.rs`
  - source identity and content fingerprinting for dataset reproducibility
- `src-tauri/src/polygon.rs`
  - Polygon manifest and cache metadata
- `web/src/stores/dataStore.ts`
  - current dataset loading and cache-facing workflow surface
- `src-tauri/src/run_artifacts.rs`
  - artifact-level dataset metadata enrichment
- options dataset manifests already exist through `src-tauri/src/commands_options_data.rs`

#### Decision

- `Adopt Later`

#### Smallest useful repo action

- If promoted later, start with saved universes that reference existing dataset IDs and tags rather than a broad catalog rewrite.

#### Candidate touchpoints

- `web/src/stores/dataStore.ts`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/run_artifacts.rs`

#### Verification needed

- domain model review first
- schema design before implementation

### DR-12 - Packaging, supportability, and release posture

#### Research claim summary

- The response recommends supportable Windows packaging with explicit WebView2 policy, prebuilt PTX posture, capability detection, local-first logs, and clear CPU fallback messaging.

#### What seems valid

- The release-policy framing is solid.
- The focus on user-visible supportability rather than CUDA implementation detail is appropriate.
- The response correctly treats CPU-only reliability as a first-class support goal.

#### What conflicts with VectorGrid constraints

- This is mostly already covered in current repo docs and runtime posture.
- It does not justify a major code project by itself.

#### Change shape

- `Additive`
- Smaller additive fallback:
  - tighten support/release docs
  - optionally standardize a support-bundle/log-collection path later

#### What the repo already has

- `RELEASE.md`
  - CPU-only / GPU build variants and prebuilt PTX posture
- `REPO_MAP.md`
  - packaging, PTX, WebView2 runtime notes, diagnostics, and logging surfaces
- `src-tauri/Cargo.toml`
  - `tauri-plugin-log`

#### Decision

- `Doc Only`

#### Smallest useful repo action

- Tighten release/support docs around supported GPU posture, WebView2 expectations, and where to collect logs before asking for help.

#### Candidate touchpoints

- `RELEASE.md`
- `README.md`
- `REPO_MAP.md`

#### Verification needed

- doc review only

### DR-13 - Capability modeling to reduce frontend/backend drift

#### Research claim summary

- The response recommends a backend-owned capability catalog plus plan/compile endpoint, with generated shared types and reason codes consumed by the frontend.

#### What seems valid

- Drift between frontend capability text and backend reality is a real class of problem in this repo.
- The direction of moving toward stronger shared contracts and backend authority is good.
- The recommendation to encode fallback honesty as structured reason codes is especially useful.

#### What conflicts with VectorGrid constraints

- A full backend-owned capability catalog + planner API would be a bigger architectural move than the current low-risk threshold.
- The repo already has some meaningful pieces:
  - route-aware capability summaries in `web/src/utils/strategyOptimizationCapabilities.ts`
  - catalog services for MAs and dynamic indicators
  - backend strategy compile/plan machinery in `crates/app_core/src/strategy_plan.rs`
- Replacing current request-builder surfaces too quickly would risk regressions.

#### Change shape

- `Mostly Additive`
- Smaller additive fallback:
  - start with narrower shared reason codes or a route-preview command
  - do not attempt a whole-form schema/planner replacement first

#### What the repo already has

- `web/src/utils/strategyOptimizationCapabilities.ts`
  - current product-facing capability summaries
- `web/src/services/movingAverageCatalogService.ts`
- `web/src/services/dynamicIndicatorCatalogService.ts`
- `crates/app_core/src/strategy_plan.rs`
  - current backend strategy compile/plan machinery

#### Decision

- `Adopt Later`

#### Smallest useful repo action

- If this is promoted, begin with a narrow backend preview surface for route eligibility and fallback reason codes rather than a full capability catalog migration.

#### Candidate touchpoints

- `web/src/utils/strategyOptimizationCapabilities.ts`
- `web/src/services/movingAverageCatalogService.ts`
- `web/src/services/dynamicIndicatorCatalogService.ts`
- `crates/app_core/src/strategy_plan.rs`

#### Verification needed

- local drift audit first
- incremental adoption plan before implementation

### DR-14 - Multi-objective workflow productization

#### Research claim summary

- The response recommends clarifying and constraining the current multi-objective flow rather than expanding it, using better Pareto presentation, representative shortlist construction, and conservative confidence language.

#### What seems valid

- This is a strong fit for the current repo.
- It aligns with the fact that `CompositeMo` is already a CPU-special Pareto workflow rather than the main GPU route.
- The recommended “clarify, do not overclaim” posture is exactly what this surface needs.

#### What conflicts with VectorGrid constraints

- The response would be a bad fit if interpreted as “build a richer recommendation engine.”
- The useful slice is small UI/wording work, not more algorithmic complexity.

#### Change shape

- `Additive`
- Smaller additive fallback:
  - add clearer representative-frontier labels and confidence wording only
  - defer any more opinionated candidate recommendation logic

#### What the repo already has

- `web/src/components/ParetoWorkspacePanel.vue`
  - current Pareto workspace shell
- `web/src/services/validationRequestBuilder.ts`
  - pareto-frontier admission logic
- `web/src/services/validationService.ts`
  - robust/cross Pareto summary wording
- `REPO_MAP.md`
  - current CPU-preferred `CompositeMo` posture is already documented

#### Decision

- `Adopt`

#### Smallest useful repo action

1. Improve the Pareto surface so it explicitly frames:
   - frontier trade-offs
   - representative candidate types
   - non-certainty confidence language
2. Keep the work inside current Pareto/validation UI instead of expanding `CompositeMo` scope.

#### Candidate touchpoints

- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/services/analysisService.ts`
- `web/src/services/validationService.ts`
- `web/src/components/OptimizeWorkspacePanel.vue`

#### Verification needed

- UI tests
- wording review across Pareto and validation summaries

### DR-15 - Analysis and visualization parity

#### Research claim summary

- The response recommends a richer candidate analysis dashboard with trade-history inspection, benchmark-relative views, MAE/MFE-style diagnostics, parameter-surface exploration, and cross-run visual comparison.

#### What seems valid

- This is a real gap and a high-value parity area.
- The emphasis on inspectability and trust fits VectorGrid well.
- The response correctly prefers additive analysis surfaces over changing strategy logic.

#### What conflicts with VectorGrid constraints

- This can easily turn into a large product program.
- The repo already has partial comparison and review surfaces:
  - Overview baseline compare
  - History selected-run review
  - Top results
  - Pareto
  - meta compare
- There is no clear current trade-history/MAE/MFE pipeline in the main research UI, so the full response is larger than a near-term additive slice.

#### Change shape

- `Mostly Additive`
- Smaller additive fallback:
  - start with run-to-run visual diff and candidate inspection improvements
  - defer MAE/MFE and heavier analysis dashboards until after lineage/compare work is stronger

#### What the repo already has

- `web/src/components/OverviewWorkspacePanel.vue`
  - baseline comparison and winner review
- `web/src/components/HistoryWorkspacePanel.vue`
  - selected-run inspection surface
- `web/src/components/ParetoWorkspacePanel.vue`
  - Pareto trade-off view
- `web/src/services/analysisService.ts`
  - current top/pareto/benchmark-related summary logic

#### Decision

- `Adopt Later`

#### Smallest useful repo action

- Treat this as a follow-on to `DR-02` compare/lineage and any future bounded inspection slice, not as a first-wave dashboard build.

#### Candidate touchpoints

- `web/src/services/analysisService.ts`
- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`

#### Verification needed

- design slice first
- avoid opening a large UI program without narrowing the first panel set

## Cross-cutting conclusions

### Strong candidates for implementation

- `DR-02`: main artifact comparison + lineage MVP
- `DR-05`: compact execution provenance / planned-vs-realized route summary
- `DR-14`: bounded Pareto clarify/constrain UX hardening

### Good doc-only conclusions

- `DR-04`: current validation stack is directionally strong; focus on wording and evidence framing, not new validation machinery
- `DR-01`: good prioritization guidance, but not its own implementation plan
- `DR-07`: benchmark discipline should become a reusable charter/disclosure package
- `DR-09`: options parity envelope mostly confirms current scope boundaries
- `DR-10`: options realism-lane semantics are directionally already landed
- `DR-12`: release/support posture mostly needs clearer docs, not new compute work

### Explicit rejections worth preserving

- unrestricted callback-driven portfolio engines
- arbitrary expression-evaluation mechanisms for research ergonomics parity

### Questions that still need local evidence

- candidate identity schema for lineage and compare
- whether any route/provenance ambiguity remains after adding a compact planned/resolved/realized summary
- whether saved universes can be introduced cleanly without dragging in a larger basket domain
- whether the generic indicator GPU path should borrow resident runtime buffers rather than uploading from `Candles`
- where CUDA Graphs actually help on current multi-kernel steady-state paths versus adding overhead
- what the smallest safe live-execution safety slice is without increasing broker-integration risk
- whether a narrow backend route-preview contract can reduce capability drift without replacing current request builders
- what the first analysis/inspection panel should be after compare/lineage, if any

## Translation targets

| Research ID | Next Doc Or Area | Why |
| --- | --- | --- |
| `DR-02` | `REPO_Improvement/` implementation slice for artifacts/history compare | high fit, additive, clearly missing in the main run-artifact flow |
| `DR-05` | `REPO_Improvement/` implementation slice for compact route provenance | high fit, additive, small schema/UI hardening |
| `DR-14` | bounded Pareto/validation wording and representative-candidate UX slice | additive and low-risk if kept inside current Pareto surface |
| `DR-04` | existing validation docs / report wording | best used as policy and wording guidance |
| `DR-06` | future GPU architecture / profiling backlog | valid direction, but only worth acting on through narrow profiling-backed runtime slices |
| `DR-07` | benchmark charter / disclosure doc | process and public-comparison discipline, not product change |
| `DR-10` | existing options docs / compare-copy consistency | mostly a semantic consistency checkpoint |
| `DR-03` | backlog only | too large for immediate additive work |
| `DR-08` | later live-execution safety backlog | valid, but high-stakes and should be narrowed before coding |
| `DR-11` | backlog only | useful domain direction, but larger than current additive target |
| `DR-13` | later architecture/design spike | promising direction, but too broad for immediate adoption |
| `DR-15` | future analysis/inspection roadmap | real gap, but should follow compare/lineage work rather than precede it |

## Current Implementation-Plan Docs

The currently accepted bounded implementation-plan documents created from this synthesis live under:

- `research/vectorgrid_deep_research_program_2026-04-09/implementation_plans/README.md`
- `research/vectorgrid_deep_research_program_2026-04-09/implementation_plans/DR-02_artifact_compare_lineage_mvp_implementation_plan_2026-04-09.md`
- `research/vectorgrid_deep_research_program_2026-04-09/implementation_plans/DR-05_execution_provenance_route_summary_implementation_plan_2026-04-09.md`
- `research/vectorgrid_deep_research_program_2026-04-09/implementation_plans/DR-14_pareto_clarity_guardrails_implementation_plan_2026-04-09.md`

## Final program verdict

The reviewed responses are mostly valid, but only a small subset justify near-term additive implementation work.

- `DR-02` is still the strongest implementation candidate: parent pointers, compare-friendly run headers, and a two-run artifact compare surface.
- `DR-05` is still worth implementing in a bounded form: compact execution provenance that distinguishes requested/resolved/realized route.
- `DR-14` is the best additional candidate from the second batch, but only as clarify/constrain UI hardening inside the current Pareto flow.
- `DR-06` is valid, but it should stay in the profiling-led GPU architecture backlog rather than becoming an immediate implementation project.
- `DR-07`, `DR-09`, `DR-10`, and `DR-12` are useful mainly as documentation/policy confirmations, not as new feature projects.
- `DR-08`, `DR-11`, `DR-13`, and `DR-15` are strategically valid, but they should remain deferred until they are reduced to smaller additive slices with clearer local evidence.
