# VectorGrid Feature Parity Roadmap vs VectorBT Pro

Date: 2026-03-19

## Purpose

This document is the operational companion to:

- `research/VECTORGRID_COMPETITIVE_IMPROVEMENT_PLAN_VS_VECTORBT_PRO_2026-03-19.md`

It is intended to be more execution-oriented. It adds:

- a parity gap matrix
- a phased roadmap
- effort and risk guidance
- repo-grounded implementation touchpoints
- explicit non-goals
- suggested acceptance criteria

This document still assumes the core product constraints of VectorGrid:

- Rust + CUDA backend
- Tauri desktop product
- constrained, guardrailed strategy composition
- no unrestricted arbitrary user code execution

## Executive Summary

The most important parity gaps with VectorBT Pro are not raw optimization features. They are:

1. visualization depth
2. portfolio and multi-asset research
3. research ergonomics
4. dataset and universe management
5. experiment comparison and lineage
6. product maturity and distribution polish

The most realistic roadmap is:

1. improve visualization and analysis UX first
2. improve artifact intelligence and compare flows
3. strengthen constrained research ergonomics
4. add dataset catalog and saved universes
5. add first-class constrained portfolio research
6. extend reporting, validation, and monitoring around those capabilities

## Parity Model

Parity should mean:

- matching the user-visible value of key workflows
- not reproducing every code-first capability of a Python research platform
- preserving safety, reproducibility, and GPU-aware route visibility

Parity should not mean:

- unrestricted callback-based strategies
- arbitrary imperative simulation logic from end users
- recreating the whole Python package ecosystem in desktop form

## Competitive Gap Matrix

| Area | VectorBT Pro Strength | VectorGrid Today | Target Parity Stance |
| --- | --- | --- | --- |
| Optimization workflow | Broad research tooling around optimization, plus Python-side flexibility | Already strong in guided exact-grid and validation-centric product workflow | Maintain advantage; improve UX rather than chase Python-style openness |
| Validation depth | Broad research workflows, flexible composition | Already strong: walk-forward, reopt, perturb, bootstrap, CSCV/PBO, SPA, cross-ticker | Keep as a flagship area and improve visualization |
| Visualization | Rich exploratory analysis patterns and code-first chart composition | Useful but still form-heavy, static, and panel-bounded | High-priority parity target |
| Portfolio research | Strong portfolio-level abstractions and rebalancing | Mostly cross-dataset robustness and meta-selection; no first-class portfolio domain | Major parity target |
| Data ecosystem | Broad integrations and data workflows | CSV + Polygon are solid, but ecosystem is narrow | Selective parity target |
| Research ergonomics | Highly composable and open-ended | Guardrailed and reproducible, but narrow and dense | Improve through constrained composition, not scripting |
| Experiment management | Strong code-native reproducibility and reuse | Strong artifacts, but lineage/diff UX is weaker | High-priority parity target |
| Live execution surface | Available through ecosystem flexibility | Present, but narrow and single-strategy/single-symbol oriented | Secondary target |
| Product polish | Mature ecosystem and user-facing education | Engineering maturity is strong; product maturity less so | Medium-priority parity target |

## Initiative Scoring

Scale:

- effort: `S`, `M`, `L`, `XL`
- risk: `Low`, `Medium`, `High`
- parity value: `Low`, `Medium`, `High`, `Very High`

| Initiative | Effort | Risk | Parity Value | Notes |
| --- | --- | --- | --- | --- |
| Visualization and analysis UX | M | Low | Very High | Highest product leverage with current architecture |
| Run comparison and artifact intelligence | M | Low | High | Strong fit with current artifact model |
| Validation diagnostics dashboard | M | Low | High | Backend data already exists; mostly product surfacing |
| Stronger dynamic strategy builder and explainability | M | Medium | High | Must preserve guardrails |
| Dataset catalog and saved universes | L | Medium | High | Strong foundation exists, but needs new domain layer |
| First constrained portfolio workflow | L | High | Very High | Biggest strategic gap; needs new domain model |
| Portfolio visualization and reporting | M | Medium | High | Depends on first portfolio domain model |
| Broader curated data connectors | M | Medium | Medium | Worth doing selectively, not indiscriminately |
| Basket-oriented live monitoring | L | High | Medium | Defer until portfolio research primitives exist |
| CI/release/documentation maturity | M | Low | Medium | Important for product credibility, but not core parity alone |

## Phased Roadmap

## Phase 1: Visualization, Compare, and Validation UX

Objective:

- make existing backend depth much more visible and explorable

Why first:

- highest user-visible payoff
- lowest architectural risk
- closes a real parity gap without changing product philosophy

Primary deliverables:

- richer candidate dashboard
- richer validation diagnostics dashboard
- stronger run comparison UX
- better parameter-space exploration
- stronger dataset inspection views

Expected outcomes:

- one candidate becomes inspectable instead of just rankable
- one validation run becomes explainable instead of just exportable
- one saved run becomes comparable instead of just reloadable

Key frontend touchpoints:

- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/services/analysisService.ts`
- `web/src/services/validationService.ts`
- `web/src/services/historyService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/stores/historyStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/composables/useLightweightChart.ts`
- `web/src/composables/useVirtualTableRows.ts`

Key backend/report touchpoints:

- `src-tauri/src/commands_analysis.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/commands_reports_api.rs`

Acceptance criteria:

- user can inspect price, signals, equity, drawdown, and trade distribution for a selected candidate from one screen
- user can see visual summaries of walk-forward, perturb, bootstrap, CSCV/PBO, and cross-ticker outcomes without leaving the app
- user can compare two runs and see request diffs, top-row overlap, and validation deltas
- exported reports remain compatible with current artifact/report checks

## Phase 2: Artifact Intelligence, Lineage, and Research Ergonomics

Objective:

- make the product feel more like a real research system than a collection of workflows

Primary deliverables:

- explicit artifact lineage
- baseline/candidate/promoted states
- diff-friendly request snapshots
- stronger dynamic strategy builder UX
- stronger route/fallback explainability
- constrained derived-series workbench

Key backend touchpoints:

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_meta.rs`
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `src-tauri/src/commands_dynamic.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/indicator_catalog.rs`
- `src-tauri/src/data_cache.rs`

Key frontend touchpoints:

- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/services/historyService.ts`
- `web/src/services/analysisService.ts`
- `web/src/services/dynamicIndicatorCatalogService.ts`
- `web/src/strategies/dynamicDoubleMaBuilder.ts`
- `web/src/stores/workflowArtifactsStore.ts`

Acceptance criteria:

- every saved artifact can show its parent artifact or source run when applicable
- compare workflows no longer require full artifact reloads to show basic route or request differences
- dynamic strategy builder can explain unsupported constructs before run time
- capability preview remains explicit for CPU vs GPU vs hybrid behavior

## Phase 3: Dataset Catalog and Saved Universes

Objective:

- turn data handling from cache-oriented workflow state into a reusable research asset system

Primary deliverables:

- dataset catalog
- dataset tags and quality state
- provenance and derived-series metadata
- saved symbol universes
- holdout baskets and benchmark baskets

Key backend touchpoints:

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/polygon.rs`

Key frontend touchpoints:

- `web/src/components/DataWorkspacePanel.vue`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`
- `web/src/services/validationService.ts`
- `web/src/components/MetaWorkspacePanel.vue`

Acceptance criteria:

- user can browse a local dataset catalog instead of working from paths only
- user can save and reuse a universe across validation and meta flows
- derived datasets preserve provenance and refresh history
- cross-ticker and future portfolio workflows can target saved universes directly

## Phase 4: First Constrained Portfolio Research Workflow

Objective:

- close the single biggest parity gap without abandoning product guardrails

Recommended scope:

- portfolio-of-candidates workflow
- constrained allocation models
- constrained rebalance models
- portfolio-level simulation outputs
- portfolio-level validation summaries

Suggested v1 allocation models:

- equal weight
- fixed weights
- score-weighted
- top-k by score

Suggested v1 rebalance modes:

- none / buy-and-hold
- periodic
- threshold-based

Key backend touchpoints:

- `crates/app_core/src/meta_select.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`

Recommended new backend shape:

- add a dedicated portfolio module under `crates/app_core/src/`
- define request/result types near current meta and validation request families
- keep portfolio simulation separate from broker execution
- feed initial portfolio selection from meta outputs rather than arbitrary user code

Acceptance criteria:

- user can select approved candidates from saved runs or meta outputs and simulate a basket
- user can visualize basket equity, drawdown, allocation, and contribution
- user can export portfolio-level summary artifacts and reports
- user can evaluate portfolio outcomes across saved universes or holdout baskets

## Phase 5: Portfolio Visualization, Basket Monitoring, and Data Expansion

Objective:

- deepen the portfolio workflow once the domain model exists

Primary deliverables:

- allocation-over-time visuals
- asset contribution and turnover visuals
- correlation and clustering views
- stress-window views
- basket-level execution monitoring
- selectively broader data connectors

Key backend touchpoints:

- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- reporting modules under `src-tauri/src/commands_reports_*.rs`

Key frontend touchpoints:

- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- portfolio-specific panels to be added under `web/src/components/`

Acceptance criteria:

- basket/portfolio state is visible in-app without relying on external tooling
- provider expansion does not create connector sprawl or unsupported product surface
- execution UX still consumes constrained approved portfolio outputs rather than open-ended code

## Phase 6: Product Maturity and Distribution

Objective:

- make the product easier to trust and easier to adopt

Primary deliverables:

- stronger end-user docs
- broader CI coverage
- automatic CI triggers
- frontend gates in CI
- release automation
- reduced migration debt

Key touchpoints:

- `.github/workflows/ci.yml`
- `README.md`
- `RELEASE.md`
- `web/README.md`
- remaining mixed legacy/Vue UI surfaces

Acceptance criteria:

- CI runs automatically on normal development flow
- frontend typecheck, test, lint, and parity gates are enforced in CI
- release packaging is documented and partially automated
- product-facing documentation explains workflows, not just engineering mechanics

## Repo Touchpoint Map by Initiative

## A. Visualization and exploratory analysis

Primary leverage points:

- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`
- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`
- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/composables/useLightweightChart.ts`
- `web/src/composables/useVirtualTableRows.ts`

Why:

- these files already own chart lifecycle, top-table shaping, compare summary generation, drilldown rendering, and heatmap behavior

## B. Workflow shell and interaction model

Primary leverage points:

- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/WorkflowQuickSwitch.vue`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/components/RunFooterPanel.vue`
- `web/src/stores/workflowStore.ts`
- `web/src/stores/taskStore.ts`
- `web/src/tauri/instrumentation.ts`

Why:

- these files already own layout, navigation, command actions, task feedback, and run context

## C. Validation diagnostics and export-linked UX

Primary leverage points:

- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/services/validationService.ts`
- `web/src/stores/validationStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`

Why:

- these are the current seams for validation request building, previewing, artifact persistence, and exporting

## D. Run comparison and lineage

Primary leverage points:

- `web/src/services/historyService.ts`
- `web/src/stores/historyStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/data_cache.rs`
- `crates/app_core/src/strategy_plan.rs`

Why:

- current artifact persistence is already strong, but lineage and diff workflows are still implicit

## E. Constrained research ergonomics

Primary leverage points:

- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `src-tauri/src/commands_dynamic.rs`
- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/indicator_catalog.rs`
- `web/src/strategies/dynamicDoubleMaBuilder.ts`
- `web/src/services/dynamicIndicatorCatalogService.ts`

Why:

- this is the existing declarative strategy system and capability-preview boundary

## F. Dataset catalog and universes

Primary leverage points:

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/polygon.rs`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`

Why:

- current data workflow already has cache identity, provider integration, and loaded/derived dataset state

## G. Portfolio and basket research

Primary leverage points:

- `crates/app_core/src/meta_select.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/commands_reports_validation_xlsx.rs`
- `src-tauri/src/run_artifacts.rs`

Why:

- these are the closest current surfaces to multi-asset aggregation, cross-dataset evaluation, and future report integration

## What Not to Build

These should remain explicit non-goals unless the product thesis changes materially.

### 1. Unrestricted arbitrary strategy code execution

Do not add:

- raw user scripting hooks
- arbitrary callback-based simulation APIs
- eval-like execution surfaces
- plugin systems that bypass IR validation

Reason:

- breaks guardrails
- undermines reproducibility
- complicates GPU routing and capability preview
- turns the product into a different category

### 2. Portfolio callbacks before a constrained portfolio model exists

Do not start with:

- arbitrary rebalance callbacks
- arbitrary order-generation hooks
- user-defined imperative portfolio policies

Reason:

- the right first step is a declarative portfolio request model

### 3. Connector sprawl

Do not chase every data provider.

Prefer:

- a small number of well-supported connectors
- strong local catalog and universe handling
- good provenance and cache behavior

### 4. Frontend-first feature invention without backend identity and artifact design

Do not build:

- new research workflows that cannot persist reproducible artifacts
- visual compare surfaces with no normalized compare payloads
- portfolio views before portfolio request/result types exist

## Suggested Success Metrics

Use metrics that reflect product value, not just implementation completion.

### Visualization and analysis

- user can inspect top candidate and validation context without exporting reports
- reduced reliance on XLSX for basic diagnosis
- compare workflow can answer "why did run B beat run A?" inside the app

### Dataset and universe management

- saved universes become reusable inputs across validation, meta, and future portfolio workflows
- dataset provenance remains visible after reload and export

### Portfolio workflow

- user can construct and evaluate a basket from approved candidates without external tools
- portfolio-level outputs are reproducible and persistable

### Research ergonomics

- dynamic strategy features remain explicit about support and fallback behavior
- new constrained composition features reduce the need for out-of-band experimentation

### Product maturity

- CI covers normal frontend and backend gates
- release and onboarding docs become product-facing, not only contributor-facing

## Recommended First Three Concrete Projects

If work needs to start immediately, the best first three projects are:

### Project 1: Validation diagnostics dashboard

Reason:

- highest immediate product value
- leverages existing backend depth
- closes a clear UX gap

Likely files:

- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/services/validationService.ts`
- `web/src/stores/validationStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `src-tauri/src/commands_validation.rs`

### Project 2: Run comparison and lineage v1

Reason:

- turns artifact storage into actual experiment intelligence
- relatively low risk

Likely files:

- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/services/historyService.ts`
- `web/src/stores/historyStore.ts`
- `web/src/stores/workflowArtifactsStore.ts`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`

### Project 3: Dataset catalog and saved universes v1

Reason:

- foundational for both better validation and future portfolio features

Likely files:

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/components/DataWorkspacePanel.vue`
- `web/src/services/dataService.ts`
- `web/src/stores/dataStore.ts`

## Bottom Line

If VectorGrid wants to approach feature parity with VectorBT Pro where it makes sense, the roadmap should not begin with arbitrary execution features.

It should begin with:

- richer visualization
- stronger experiment comparison
- stronger dataset and universe management
- then a constrained portfolio domain model

That path is both realistic for this repo and aligned with what VectorGrid can plausibly become better than notebook-centric tools at: a guardrailed, GPU-aware, reproducible desktop research product.
