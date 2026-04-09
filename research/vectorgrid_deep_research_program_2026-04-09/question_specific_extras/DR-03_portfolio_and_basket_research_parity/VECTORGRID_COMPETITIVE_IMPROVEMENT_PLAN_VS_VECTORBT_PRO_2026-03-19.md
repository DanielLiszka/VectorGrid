# VectorGrid Competitive Improvement Plan vs VectorBT Pro

Date: 2026-03-19

## Related Documents

- `research/VECTORGRID_FEATURE_PARITY_ROADMAP_VS_VECTORBT_PRO_2026-03-19.md`
  - operational roadmap companion to this document
  - includes gap matrix, phased plan, effort/risk notes, repo touchpoints, and explicit non-goals

## Purpose

This document captures where VectorGrid should improve to be more competitive with VectorBT Pro, while respecting the actual product constraints of this repo:

- core stack is Rust + CUDA, not Python
- product is a Tauri desktop application, not a notebook-first library
- arbitrary unrestricted user code execution is not a target
- GPU-backed exact-grid and validation workflows remain core differentiators

The goal is not to clone VectorBT Pro mechanically. The goal is to reach feature parity where it makes product sense, and to close the most important workflow and research gaps without weakening VectorGrid's reliability, guardrails, or GPU-first identity.

## Current Product Position

VectorGrid is already stronger than a simple "fast Double-MA demo."

Current strengths in the repo:

- guided desktop workflow shell with staged `Data -> Optimize -> Validate -> Analyze -> Finalize`
- run library, history, meta-analysis, holdout, ML-confidence, artifacts, and report export
- strong validation stack: walk-forward, reopt walk-forward, perturbation, bootstrap, CSCV/PBO, SPA, cross-ticker
- strong artifact and report plumbing
- dynamic indicator catalog and constrained dynamic strategy IR
- paper/live execution surface with Alpaca and Polygon integration
- serious engineering rigor: parity checks, workflow regression gate, perf gates, output parity checks

Representative current files:

- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands_validation.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_execution.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/strategy_dynamic.rs`

## Strategic Framing

VectorBT Pro is ahead primarily in breadth and openness:

- broader data ecosystem
- richer portfolio-level research primitives
- more open-ended research ergonomics
- stronger code-first composability
- broader visualization and exploratory analysis surface

VectorGrid is ahead or competitive in different ways:

- guided desktop workflow
- exact-grid product focus
- guarded execution model
- GPU-resident optimization and validation roadmap
- artifact discipline and reproducibility

The best path is not "become a Python scripting platform." The best path is:

1. make the existing desktop workflows much more informative and visual
2. broaden the supported research surface in constrained, productized ways
3. add portfolio and multi-asset features where they can be expressed safely
4. improve experiment management and product maturity

## High-Level Weaknesses

VectorGrid is currently weak against VectorBT Pro in these areas.

### 1. Visualization and exploratory analysis

This is one of the most obvious gaps.

Current UI has workflow panels, tables, heatmaps, drilldown, and reports, but it still feels operator-oriented and form-heavy. It is weaker at:

- linked multi-panel exploration
- parameter-surface exploration beyond static heatmap/table views
- validation diagnostics dashboards
- cross-run comparison visuals
- multi-asset comparative visuals
- interactive route/fallback visibility
- chart overlays that explain why a candidate passed or failed

This is the most realistic area to improve quickly without violating the Rust + CUDA product model.

### 2. Research ergonomics

VectorGrid has a strong guided workflow, but it is not yet a strong exploratory research environment.

Weak points:

- dense forms instead of fluid exploratory tooling
- limited ad hoc feature engineering workflow
- no reusable visual "analysis notebook" equivalent inside the product
- dynamic strategy support exists, but the user experience is still narrow and constrained
- advanced surfaces still show migration debt and some legacy bridging behavior

This is not a call for arbitrary scripting. It is a call for better constrained composition and inspection.

### 3. Portfolio and multi-asset support

This is a major gap.

Current product is mostly:

- single-dataset optimization
- cross-ticker validation
- cross-dataset robustness ranking via meta-selection

That is useful, but it is not the same as first-class portfolio research.

Weak points:

- no true portfolio construction and rebalancing workflow
- no basket-level simulation surface
- no allocation model layer
- no portfolio-level walk-forward / holdout / robustness tooling
- no asset-universe visual management beyond batch/meta-style selection

This is one of the biggest capability gaps versus VectorBT Pro.

### 4. Data ecosystem and dataset workflow

Current product has good CSV + Polygon flows, but the connector and data-management story is still narrow.

Weak points:

- limited external data connectors
- limited built-in local dataset discovery/catalog capabilities compared with broader research tools
- weak feature-oriented dataset workflow
- limited provenance, lineage, and data transformation visibility
- weak reusable dataset-universe management

### 5. Experiment comparison and run intelligence

VectorGrid stores runs and artifacts well, but cross-run intelligence is still weaker than it should be.

Weak points:

- comparing runs still feels like artifact browsing, not experiment intelligence
- limited lineage views
- weak "why did this run differ?" tooling
- weak diffing for requests, route decisions, validation outcomes, and meta outcomes
- limited baseline/candidate comparison UX

### 6. Product maturity and polish

The repo is rigorous, but product maturity is behind engineering maturity.

Weak points:

- end-user documentation is weaker than contributor-facing documentation
- CI is manual-only and not broad enough
- packaging and release flow are still fairly manual
- app still carries some migration debt
- current UX is powerful but dense

## What Parity Should Mean for VectorGrid

Parity should be selective.

### Good parity targets

- richer data visualization
- richer validation visualization
- richer multi-run comparison
- broader curated data connectors
- constrained strategy composition
- portfolio-level research primitives with guardrails
- experiment lineage and reproducibility
- better local data management

### Bad parity targets

- unrestricted arbitrary Python-like strategy code execution
- arbitrary callback-based portfolio engines with no product guardrails
- full user-programmable simulation runtime without constrained IR
- trying to reproduce the entire Python package ecosystem inside the desktop app

VectorGrid should prefer:

- declarative requests
- constrained IR
- approved operators and indicators
- explicit route previews and fallback reasons
- reproducible artifacts

## Priority Improvement Areas

## Priority 1: Visualization and analysis UX

This should be the top product priority.

Why:

- large user-visible impact
- realistic in current architecture
- closes a real competitive gap
- does not require arbitrary scripting
- compounds the value of the backend work already done

Recommended improvements:

### 1. Candidate analysis dashboard

Add a richer per-candidate analysis surface that combines:

- price chart with entry/exit markers
- indicator overlays used by the strategy
- equity curve
- drawdown curve
- trade distribution
- rolling Sharpe / rolling returns
- validation pass/fail badges by module

This should make one candidate feel inspectable, not just rankable.

### 2. Validation diagnostics dashboard

Current validation is strong but too report-like.

Add visual modules for:

- walk-forward windows and outcomes
- perturbation result distributions
- bootstrap outcome distributions
- CSCV/PBO summary visuals
- cross-ticker pass map
- route/backend used for each stage
- final verdict decomposition

The user should be able to see why a candidate is fragile.

### 3. Run-to-run comparison views

Add a direct compare mode for two or more runs:

- request diff
- backend/route diff
- top candidate overlap
- validation metric diff
- holdout/meta outcome diff
- report/artifact diff summary

This should live above raw artifact browsing.

### 4. Better parameter-space visuals

Improve optimization result exploration:

- richer heatmaps
- 3-way projections where feasible
- stability plateau views
- frontier views
- MA-type family comparison views
- explicit "robust region vs sharp peak" visuals

### 5. Better dataset visuals

Improve data inspection:

- data quality diagnostics
- missing/duplicate gap views
- resample previews
- market-hours/session overlays
- symbol comparison views
- feature preview panes

## Priority 2: Portfolio and multi-asset research

This is the largest strategic gap and should become a major roadmap track.

This does not need arbitrary portfolio scripting at first.

Recommended first-pass scope:

### 1. Portfolio-of-candidates workflow

Start with a constrained model:

- select N approved candidates from saved runs/meta
- assign static or rule-based weights
- simulate combined portfolio equity
- show correlation/diversification statistics
- run portfolio-level validation summaries

This is a realistic bridge from the current run/meta system.

### 2. Basket validation

Support applying a candidate set or strategy family across a basket and summarizing:

- basket pass rate
- dispersion of outcomes
- concentration risk
- worst-asset contribution
- stability by regime and market type

### 3. Rebalancing primitives

Add a constrained set of rebalancing modes:

- fixed weights
- equal weight
- top-k by score
- periodic rebalance
- threshold rebalance

Avoid arbitrary order callbacks. Keep it declarative.

### 4. Portfolio visualization

Once portfolio primitives exist, visualize:

- portfolio equity and drawdown
- asset contribution
- turnover
- allocation over time
- correlation and clustering
- portfolio stress windows

## Priority 3: Constrained research ergonomics

VectorGrid should not become an unrestricted scripting platform, but it should become much easier to explore ideas.

Recommended improvements:

### 1. Stronger dynamic strategy builder

Build on the current dynamic IR work in `crates/app_core/src/strategy_dynamic.rs`.

Improve:

- compositional UI
- reusable templates
- guided validation of unsupported constructs
- better preview of resolved backend
- better explanation of why GPU is or is not available
- saved reusable strategy templates

### 2. Derived-series workbench

Add a constrained feature engineering surface where users can define:

- approved transforms
- approved rolling statistics
- indicator chains
- thresholded boolean signals
- normalized derived series

This should compile into safe declarative requests, not arbitrary code.

### 3. Strategy explainability

For any strategy form or dynamic strategy:

- show active components
- show resolved constraints
- show fallback reasons
- show expected output surface
- show unsupported features clearly before execution

## Priority 4: Data ecosystem and data management

Recommended improvements:

### 1. Broaden connectors selectively

Add only connectors that fit the product and can be supported well.

Likely good candidates:

- more local dataset import conventions
- better folder/catalog ingestion
- additional market data providers where licensing and implementation are realistic
- stronger local cache management

### 2. Dataset catalog

Turn current data handling into a stronger dataset-management product:

- searchable local catalog
- dataset tags
- provenance metadata
- timeframe coverage
- session/calendar metadata
- quality status
- last refresh time

### 3. Universe management

Add first-class saved universes:

- symbol lists
- taggable groups
- benchmark baskets
- holdout baskets
- sector/theme baskets

This helps both validation and future portfolio workflows.

## Priority 5: Experiment intelligence and artifact UX

Recommended improvements:

### 1. Request and artifact lineage

Each run should make lineage obvious:

- parent run
- derived validation run
- derived meta artifact
- derived Stage 4/5 artifact
- source datasets
- source config fingerprint

### 2. Better diff tooling

Support direct diff for:

- optimization requests
- validation requests
- backend route resolutions
- report summaries
- artifact metadata

### 3. Baseline workflows

Let users mark runs or artifacts as:

- baseline
- candidate
- promoted
- archived
- rejected

Then support compare-against-baseline everywhere.

## Priority 6: Live execution and monitoring

Execution is not the main competitive gap, but the current surface is still narrow.

Recommended improvements:

- stronger monitoring dashboard
- better strategy-state visualization
- explicit pre-trade and post-trade risk diagnostics
- better live vs paper comparison
- richer event timeline
- better visibility into broker/account/execution state

This should follow, not precede, better research and validation UX.

## Priority 7: Product maturity

Recommended improvements:

### 1. End-user documentation

Add product-facing docs for:

- workflow concepts
- validation concepts
- strategy family explanations
- route policy explanations
- portfolio features once added

### 2. CI and release automation

Current local discipline is better than CI enforcement.

Improve:

- automatic CI triggers
- frontend gates in CI
- broader test coverage in CI
- packaging checks
- release automation

### 3. Finish migration debt

Reduce mixed legacy/Vue surfaces so the product feels coherent and stays maintainable.

## Suggested Roadmap Order

Recommended order of execution:

1. visualization and analysis UX
2. experiment comparison and artifact intelligence
3. constrained research ergonomics
4. dataset catalog and universe management
5. first constrained portfolio workflow
6. portfolio visualization and validation expansion
7. product maturity and release automation

This order preserves momentum and keeps the work aligned with the current architecture.

## Concrete Weakness Summary

If reduced to the most important weak points, VectorGrid is currently weak on:

- visualization depth
- portfolio and multi-asset research
- research ergonomics
- breadth of curated data workflows
- run comparison and experiment intelligence
- end-user polish and distribution maturity

It is less weak on:

- optimization
- validation depth
- artifacts and reports
- desktop workflow structure
- engineering rigor

## Recommended Competitive Thesis

VectorGrid should not try to win by copying VectorBT Pro's openness.

VectorGrid should try to win by being:

- the best guarded desktop product for GPU-accelerated strategy research
- the clearest optimization-to-validation-to-meta workflow
- the strongest visual analysis environment for exact-grid and robustness workflows
- a constrained but powerful multi-asset and portfolio research tool
- a more reproducible, more productized system than ad hoc notebook stacks

## Bottom Line

The most realistic parity path is:

- improve visualization first
- improve research ergonomics through constrained composition
- add first-class portfolio and basket research
- strengthen data and experiment management

The least realistic path is:

- trying to replicate unrestricted Python-style arbitrary strategy execution

VectorGrid does not need to become a scripting language runtime to become much more competitive with VectorBT Pro. It does need to become much better at showing, comparing, organizing, and validating research results across datasets, strategies, and eventually portfolios.
