# Options Expression Optimizer — Comprehensive Future Implementation Roadmap

Date: 2026-04-07
Status: comprehensive future-scope implementation roadmap for turning the current options-expression feature into a world-class, performance-first options backtest optimizer focused on deeper optimization and embedded validation within the current options lane

## Purpose

This document is the **forward implementation roadmap** for the options-expression feature after the current shipped slice.

Use this together with:

- [options_expression_optimizer_viable_plan_2026-04-02.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_optimizer_viable_plan_2026-04-02.md)

The split between the documents is:

- `options_expression_optimizer_viable_plan_2026-04-02.md`
  - what is already implemented or tightly coupled to the current shipped slice
- `options_expression_future_scope_roadmap_2026-04-07.md`
  - how to extend the feature into a fuller, world-class options optimizer without losing the repo’s performance edge

---

## 1. Strategic Target

### What the long-term product should become

The target is **not**:

- a miniature LEAN
- a generic event-scripting engine
- a broker simulator
- a CPU-first research tool with optional GPU acceleration

The target **is**:

**a world-class, artifact-driven, GPU-first options backtest optimizer that keeps as much useful work as possible inside dense compiled passes while preserving robust post-search validation, stress testing, and decision-grade reporting within the current options workflow.**

### Product philosophy

The right comparison point is:

- LEAN-like in **research breadth where the work can still be compiled and accelerated**
- VectorGrid-like in **performance model, constrained regularity, and optimization throughput**

That means:

- keep the parts of LEAN/options research that materially improve decision quality
- trim the parts that turn the product into a slow general simulator
- maximize useful computation per pass
- treat performance as a first-class product feature, not an implementation detail

### Practical shorthand

The feature should evolve toward:

**“LEAN’s useful options research subset, redesigned around compiled tensors, GPU scoring, and multi-stage robustness analysis.”**

---

## 2. Design Doctrine

These principles should govern all future work.

1. **The parent signal stays fixed.**
   The options engine optimizes the expression of a saved directional strategy artifact. It does not rediscover the underlying signal logic.

2. **Compilation first, simulation second.**
   Irregular chain work belongs in preprocessing. The GPU should score compiled outcomes, not scan raw chains.

3. **Do as much useful work as possible in one pass.**
   If more metrics, score lanes, or stress lanes can be computed with little additional hot-path cost, keep them.

4. **Separate dense work from branchy work.**
   Dense work belongs in the GPU path. Branchy or infrequent work belongs in CPU post-passes.

5. **Validation is mandatory, not optional polish.**
   The options layer is a second optimizer. Every serious run must include holdout/finalist robustness stages.

6. **Scope expands only when tensor regularity survives.**
   If a new feature destroys density, cache reuse, or planner-fit economics, it should be rejected or redesigned.

---

## 3. Current Baseline

The current repo already has a real options-expression system.

Current shipped baseline:

- artifact-driven options expression optimization is implemented
- the current frontend host is a unified History follow-up panel
- supported parent families are:
  - `DoubleMa`
  - `CompositeTrend`
  - `RegimeHybrid`
  - `BreakoutMomentum`
- direct Polygon/Massive provider download exists for bounded `1m` minute-aggregate workflows
- quote-aware and derived-mid realism are supported through imported normalized CSV datasets
- structure families currently shipped:
  - `long_single`
  - `debit_vertical`
  - `long_pair`
- CPU reference scoring exists
- CUDA compiled-tensor scoring exists
- holdout, sealed-finalist, and multi-lane robustness metrics exist
- artifact persistence, compare, event drilldown, and path replay exist

This roadmap assumes that baseline and does not propose rebuilding it from scratch.

---

## 4. GPU-Compatible LEAN Parity Envelope

### The right parity target

The goal is **not** full LEAN parity.

The goal is:

**near parity with the subset of LEAN-style options research that can be expressed as fixed-shape contract selection, deterministic structure compilation, template exits, and multi-stage rescoring.**

### High-value LEAN-like surfaces worth emulating

These are worth targeting because they fit the current architecture:

- contract selection by:
  - side
  - DTE
  - moneyness / strike distance
  - delta, IV, OI, volume, spread where data supports it
- fixed expression families:
  - long calls / puts
  - same-expiry debit verticals
  - bounded fixed-ratio same-side ladders / long-pair bundles
- template exits:
  - fixed hold
  - target / stop
  - end-of-day cap
  - base-artifact-exit cap
  - expiry cutoff
- explicit fill lanes:
  - aggregate research
  - derived mid
  - quote conservative
  - quote mid
- first-class robustness:
  - holdout
  - finalist-only sealed evaluation
  - harsher fee/spread/liquidity/stress lanes
- artifact persistence and comparative analysis

### LEAN-like surfaces that should stay out of scope

These are strategically misaligned with a GPU-first optimizer:

- assignment / exercise modeling
- short premium margin modeling
- portfolio margin
- arbitrary event scripting
- arbitrary leg counts
- mixed-expiry calendar/diagonal engines
- rolling logic
- generic live-trading semantics

### Working definition of “world-class”

For this product, “world-class” should mean:

- very fast search over meaningful options-expression spaces
- strong visibility into robustness, invalidity, and data coverage
- realistic enough execution semantics to be decision-useful
- broad enough structure/filter support that the user does not feel trapped in a toy system
- integrated validation, comparison, and only limited secondary analysis when it directly improves decisions

---

## 5. Performance-First Product Model

### The core performance promise

The product should aim to do the options equivalent of what the conventional optimizer already does well:

- compute a rich set of useful metrics for every candidate
- do it with minimal incremental hot-path cost
- avoid forcing the user to rerun slow post-processing for every new view

### The “one pass if possible” rule

If a metric or score lane can be collected during the same compiled pass with near-zero or modest extra cost, prefer collecting it then.

Examples of work that should ideally be carried together:

- baseline selection metrics
- holdout metrics
- candidate invalid-rate context
- per-side and per-split counters
- summary coverage counts
- cached stress summaries when they share compiled structure

Examples of work that can stay outside the main hot loop:

- full path replay for inspection
- heavyweight portfolio overlay
- optional cross-artifact comparison that helps validation decisions
- any downstream analysis that materially disrupts the scorer should remain secondary

### Optimization and validation should feel integrated

The feature should trend toward:

1. compile once
2. score many tuples
3. validate finalists immediately
4. stress finalists immediately
5. store enough metrics that the user can inspect and compare without expensive recomputation

---

## 6. Data Surface Roadmap

### Provider assumption

The product should continue assuming:

- Polygon/Massive-origin options data only

### Lane A — canonical hot-path lane

This should remain the main optimization surface:

- Polygon/Massive contract reference
- Polygon/Massive option `1m` minute aggregates
- aligned underlying stock `1m` bars

Why:

- regular
- compact
- cacheable
- GPU-friendly
- sufficient for wide search

### Lane B — quote-aware realism lane

This should become a first-class realism lane:

- top-of-book quote datasets
- derived-mid snapshot datasets

Primary uses:

- higher-realism finalist evaluation
- spread-aware sensitivity
- fill-model comparison
- narrow quote-aware optimization when the data volume remains tractable

### Lane C — optional direct quote-aware provider lane

This is a larger future decision, not a default assumption.

It should only be implemented if:

- entitlements are reliable
- data economics are acceptable
- the workflow proves real demand

### Supplemental metadata surfaces

These should be progressively supported when data quality permits:

- open interest
- implied volatility
- delta and greeks
- chain snapshots

Admission rule:

- use them only when they can be aligned deterministically and do not introduce silent lookahead or survivorship mistakes

---

## 7. Execution Semantics Roadmap

### A. Aggregate-based realism

This remains the default fast lane:

- `1m` aggregates
- aggregate research fill model
- conservative/harsher/extreme stress modifiers on top

### B. Derived-mid realism

This should be a first-class intermediate lane:

- midpoint-style semantics
- more realistic than aggregates
- more regular than raw quotes

### C. Top-of-book quote realism

This should support:

- quote conservative
- quote mid

This lane is especially important for:

- finalist validation
- trust in high-spread names
- diagnosing when aggregate winners are fragile

### D. Clear semantics in artifacts and UI

Every run and saved artifact should clearly say:

- price semantics
- fill model
- source surface
- stress lane assumptions

This should never be implicit.

---

## 8. Scope Freeze For This Roadmap

This roadmap is no longer treating new structure families or new parent strategy families as active next steps.

Current shipped structures remain:

- `long_single`
- `debit_vertical`
- `long_pair`

Current supported parent families remain:

- `DoubleMa`
- `CompositeTrend`
- `RegimeHybrid`
- `BreakoutMomentum`

The default direction from here is:

- deepen optimization quality
- deepen embedded validation quality
- improve realism/robustness interpretation
- preserve performance

not:

- widen into more strategy families
- widen into more expression families
- create new workflow branches

---

## 9. Embedded Optimization and Validation Roadmap

This is the area most worth expanding if the goal is “not lacking in features.”

### Stage 1 — search metrics

Already present:

- selection metrics for search events
- robust score
- invalidity context

### Stage 2 — holdout validation

Already present in backend/artifacts, but it should become more prominent in the product.

Desired productization:

- clear holdout verdicts
- search-vs-holdout delta views
- hard visibility on coverage and trade-count adequacy

### Stage 3 — sealed-finalist evaluation

Already present in backend form.

Desired productization:

- explicit finalist budget
- finalist promotion / demotion visibility
- acceptance rules visible in UI/reporting

### Stage 4 — robustness lanes

Current lanes:

- baseline
- conservative
- harsher
- extreme

Future policy:

- keep adding lanes only if they add concrete decision value
- do not widen stress families for symmetry alone

### Stage 5 — optimization-depth within the current workflow

This is now the preferred future direction.

Desired capabilities:

- richer robustness-frontier reporting across selection, holdout, sealed, and stress lanes
- stronger acceptance/rejection cues for finalists
- clearer disagreement reporting between aggregate, derived-mid, and quote-aware lanes
- additional useful metrics only when they can be collected with little hot-path cost

This should stay inside the existing options optimization and embedded validation workflow rather than creating a separate research stack by default.

### Stage 6 — validation-depth within the current workflow

This should remain embedded, not broken into a separate product stage.

Good future uses:

- stricter finalist validation summaries
- stronger conservative-ranking and fragility views
- better saved-artifact comparison for validation outcomes
- better diagnostics when the search winner and the robust winner disagree

Bad use:

- turning options into a separate workflow tree before the current host is truly insufficient

The default direction should remain: improve validation quality and clarity inside the current follow-up workflow.

---

## 10. Workflow and UX Roadmap

### Current host

- History follow-up panel

This is still a sensible default.

### Next UX step

The most valuable UX expansion is **not** a new mode.

It is:

- a clearer first-class **options validation** presentation inside the current follow-up workflow

That should include:

- search vs holdout vs sealed metrics
- pass/fail or caution signals
- stronger finalist comparison
- clearer realism-lane labeling

---

## 11. GPU Architecture and Runtime Roadmap

### Core principle

The GPU should own:

- dense tuple scoring
- split-lane scoring where possible
- one-pass metric aggregation where economically justified

The CPU should own:

- irregular replay/projection
- dynamic file/provider I/O
- heavyweight inspection and rare exactification

### Near-term GPU work

1. Harden `auto` to make GPU the normal path whenever measured planner-fit and workload economics support it.
2. Keep improving shared-buffer reuse and planner telemetry.
3. Preserve exact CPU fallback and finalist exactification.

### Medium-term GPU work

1. Extend the GPU path to carry richer multi-lane metrics in one pass where possible.
2. Improve planner behavior for bounded but still worthwhile workloads.
3. Keep stress-lane compilation shaped so additional robustness work does not collapse performance.

### Key acceptance test

A new feature belongs in the hot path only if:

- it materially helps selection quality
- it fits the compiled tensor model
- it does not erase the performance edge

---

## 12. Metrics Philosophy

The goal is not just “more features.”

The goal is:

**more useful information per candidate, at little additional incremental cost.**

Future metric expansion should favor:

- selection expectancy
- expectancy per premium
- holdout delta
- sealed-finalist delta
- stress fragility
- per-side coverage
- invalid-reason concentration
- spread / realism sensitivity

This mirrors the philosophy already used in the conventional optimizer: if extra rows or lanes can be computed cheaply enough, keep them.

---

## 13. Phased Future Roadmap

### Phase F1 — First-Class Options Validation

Deliverables:

- explicit embedded options validation presentation inside the existing follow-up workflow
- clearer search / holdout / sealed / stress breakdowns
- finalist comparison view
- acceptance/rejection cues

Exit criterion:

- a user can tell whether the best tuple is merely a search winner or actually robust enough to trust

### Phase F2 — Quote-Aware Productization

Deliverables:

- stronger UI and artifact support for quote-aware vs aggregate-based runs
- fill-model guidance by price semantics
- clearer dataset-lane labeling
- better realism comparison views

Exit criterion:

- quote-aware lanes feel like supported product paths, not hidden advanced imports

### Phase F3 — GPU Auto Promotion Hardening

Deliverables:

- canonical provider-backed benchmark set
- revised `auto` thresholds where justified
- clearer backend-selection reasoning in UI/artifacts

Exit criterion:

- GPU is the normal path for materially GPU-worthy workloads without sacrificing trust

### Phase F4 — Parent Adapter Hardening

Deliverables:

- explicit family-local adapters for current supported parent families
- clearer tests and ownership boundaries

Exit criterion:

- the current parent-support surface is explicit and maintainable without treating additional parent families as roadmap work

### Phase F5 — Optimization-Depth Expansion

Deliverables:

- richer robustness-frontier reporting
- stronger validation acceptance dashboards
- better disagreement reporting across realism lanes where data exists
- additional low-cost score/context rows only when they preserve the performance model

Exit criterion:

- the current optimization + validation workflow becomes materially more decision-useful without widening into new product branches

### Phase F6 — Validation-Depth Expansion

Deliverables:

- stronger saved-artifact validation comparison
- better robust-winner vs search-winner surfacing
- more explicit validation failure/fallback narratives

Exit criterion:

- users can understand whether to trust, reject, or further inspect an options run without resorting to raw artifact JSON or ad hoc comparison

---

## 14. Comprehensive Future Checklist

Status key:

- `[ ]` not implemented yet
- `[~]` partially implemented / present in backend form but not fully productized
- `[-]` intentionally deferred / not a roadmap priority

### A. Finish The Current Intended Product

- `[x]` Turn options post-search validation into a first-class product surface
- `[x]` Add explicit options validation verdicts and acceptance cues
- `[x]` Add stronger finalist validation comparison views
- `[~]` Promote quote-aware realism from supported import semantics to a clearly first-class product lane
- `[x]` Add stronger fill-model guidance and guardrails by dataset price semantics
- `[x]` Add clearer result labeling for aggregate vs derived-mid vs quote-aware runs
- `[x]` Finish GPU-first productization of `auto`
- `[x]` Revisit `auto` thresholds after a cleaner canonical provider-backed benchmark set
- `[x]` Make backend-selection reasons even more explicit in saved artifacts and comparison views
- `[x]` Replace the implicit shared double-like replay path with explicit family-local adapters for the currently supported parent families

### B. Optimization and Validation Depth

- `[x]` Add stronger artifact comparison / provenance diffing for options-expression runs
- `[x]` Add first-wave robustness-frontier reporting for live run result review
- `[ ]` Add deeper robustness-frontier reporting for reopened artifacts and saved-artifact comparison
- `[x]` Add clearer search-winner vs robust-winner disagreement reporting
- `[x]` Add aggregate vs derived-mid vs quote-aware disagreement summaries where matching datasets/artifacts exist
- `[ ]` Add additional useful metrics only when they can be gathered with little incremental hot-path cost

### C. Already-Shipped Secondary Context

- `[x]` First-wave options meta-analysis over saved options-expression artifacts exists
- `[x]` First-wave cross-root and cross-parent-family comparison surfaces already exist
- `[ ]` Deepen cross-artifact comparison only when it directly improves optimization/validation decisions

### D. Intentionally Deferred Expansion

- `[-]` Do not prioritize additional parent strategy families
- `[-]` Do not prioritize a dedicated options mode
- `[-]` Do not prioritize optional ML/AI ranking stages
- `[-]` Do not prioritize direct quote-aware Polygon/Massive download unless the aggregate lane and import-based quote-aware lane prove insufficient

### E. Keep Out Of Scope

- `[-]` Do not add assignment / exercise modeling
- `[-]` Do not add rolling
- `[-]` Do not add arbitrary multi-leg scripting
- `[-]` Do not add live options execution
- `[-]` Do not add generic broker/margin simulation

Current shipped validation-surface scope:

- live run results now expose an explicit tuple validation verdict derived from backend-owned holdout / sealed / stress metrics
- reopened artifact summaries and comparison tables now surface the same validation verdict language instead of forcing users to infer it from EV columns alone
- the current result table and selected-tuple inspector now expose validation-frontier comparisons across selection, holdout, sealed, and stress lanes
- the current run results now expose a validation-winners frontier table so users can see which tuple wins each major selection / validation / stress lane without manually clicking every row
- reopened artifact summaries now carry explicit backend route, fill-model, structure-family, split-mode, and realism-lane labeling instead of leaving compare views to infer those facts from sparse raw fields
- the current options panel now includes a first-wave cross-artifact meta-analysis layer grouped by root, parent family, structure family, realism lane, and top validation verdict across all saved options-expression artifacts
- the current options panel now exposes an explicit top-level realism-lane chooser that drives dataset filtering, import semantics, fill-model guidance, and aggregate-only provider-download gating
- the current options panel now labels the active dataset lane and disables or coerces incompatible fill models before preview/run, while the backend still enforces the same compatibility matrix at the command/runtime boundary
- supported parent families now flow through explicit family-local adapter wrappers before entering the shared double-like replay helper, so the current support surface is structurally visible instead of implicit
- the current gate-driven `auto` policy has now been rechecked against a clean provider-backed SPY DTE7 promotion-gate run, and on that canonical slice it recommends the GPU path while clearing density/cache/scorer/wall-speed gates
- larger finalist-comparison workflows and broader acceptance dashboards remain future work, but the default direction is still to improve them inside the current unified options host

---

## 15. Explicit Non-Goals For Future Expansion

These should remain rejected unless the product strategy changes materially.

- becoming a general options platform
- matching LEAN on slow, CPU-heavy broker realism
- arbitrary event-driven options scripting
- margin/accounting parity with full broker simulators
- feature breadth that destroys the optimizer’s performance edge

---

## 16. Research Anchors

The roadmap above is intentionally aligned to the current official surfaces that matter most to this product direction, as checked on **April 7, 2026**:

- QuantConnect LEAN option strategy/reference surface:
  - https://www.lean.io/docs/v2/lean-engine/class-reference/cs/classQuantConnect_1_1Securities_1_1Option_1_1OptionStrategies.html
- QuantConnect / Polygon local dataset support:
  - https://www.quantconnect.com/docs/v2/local-platform/datasets/polygon
- Massive options overview:
  - https://massive.com/docs/rest/options/overview
- Massive all-contracts reference:
  - https://massive.com/docs/rest/options/contracts/all-contracts
- Massive option minute aggregates:
  - https://massive.com/docs/flat-files/options/minute-aggregates/2025
- Massive option quotes:
  - https://massive.com/docs/flat-files/options/quotes
- Massive option chain snapshot:
  - https://massive.com/docs/rest/options/snapshots/option-chain-snapshot

These sources support the practical product stance:

- copy the useful research/filter/structure ideas
- keep the provider/data assumptions concrete
- do not copy the slow general-simulator parts

---

## 17. Short Verdict

The current options-expression feature is already real.

The next step is not “add everything.”

The next step is:

1. make validation more explicit
2. make realism clearer
3. make GPU the normal path where the data supports it
4. deepen the current optimization and embedded validation surfaces before considering any breadth expansion

That is how VectorGrid can become a world-class options backtest optimizer **without sacrificing the performance edge that makes it worth building.**

---

## 18. Implementation Surface Map

This section is intentionally practical.

Use it to avoid broad repo search when implementing future phases.

### Backend touchpoints

- parent replay / artifact adaptation:
  - [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - [options_signal_manifest.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_signal_manifest.rs)
- options types / artifacts / request contracts:
  - [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- candidate compile / exit summary / scoring / robustness:
  - [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- CUDA scorer / kernel:
  - [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
  - [options_expression.cu](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/options_expression.cu)
- options dataset import / provider download:
  - [commands_options_data.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_data.rs)
  - [options_data_store.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_data_store.rs)
- headless proof / compare / smoke:
  - [ta_options_expression_feasibility.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_expression_feasibility.rs)
  - [ta_options_expression_compare.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_expression_compare.rs)
  - [ta_options_expression_smoke.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_expression_smoke.rs)
  - [ta_options_dataset_import.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_dataset_import.rs)
  - [ta_options_dataset_download.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_dataset_download.rs)

### Frontend touchpoints

- panel host and user-facing options workflow:
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)
- frontend workflow state:
  - [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- Tauri bindings and normalization:
  - [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- high-signal tests:
  - [OptionsExpressionPanel.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.test.ts)
  - [optionsExpressionStore.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.test.ts)
  - [optionsExpressionService.test.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.test.ts)

### Proof and benchmark touchpoints

- phase-0.5 wrapper:
  - [run_options_expression_phase05.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_phase05.ps1)
- GPU promotion wrapper:
  - [run_options_expression_gpu_promotion_gate.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_gpu_promotion_gate.ps1)
- criterion benches:
  - [options_expression_candidate_compile.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/benches/options_expression_candidate_compile.rs)
  - [options_expression_summary_compile.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/benches/options_expression_summary_compile.rs)
  - [options_expression_cpu_score.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/benches/options_expression_cpu_score.rs)
  - [options_expression_gpu_score.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/benches/options_expression_gpu_score.rs)

---

## 19. Phase-by-Phase Engineering Detail

This section answers:

- where each phase should primarily change code
- what each phase is allowed to widen
- what each phase must not disturb

### Phase F1 — First-Class Options Validation

Primary goal:

- make options validation visible and decisive, not just present in backend payloads

Primary files:

- [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
- [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

Allowed changes:

- artifact-summary additions for clearer validation verdicts
- richer finalist comparison rows
- more explicit validation labels, badges, and warnings

Do not change:

- tuple compile model
- hot-path scorer contracts
- dataset ingestion contracts

Phase exit criteria:

- a user can distinguish search winner vs holdout-resilient winner vs sealed-finalist winner without opening raw JSON
- validation warnings are visible in both live run results and reopened artifacts

### Phase F2 — Quote-Aware Productization

Primary goal:

- make quote-aware and derived-mid lanes feel first-class rather than advanced side channels

Primary files:

- [commands_options_data.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_data.rs)
- [options_data_store.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_data_store.rs)
- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [optionsExpressionStore.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/stores/optionsExpressionStore.ts)
- [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

Allowed changes:

- dataset manifest clarity
- fill-model gating by semantics
- quote-aware import and reporting polish
- optional direct quote-aware provider ingestion if data economics justify it

Do not change:

- aggregate hot-path assumptions for the canonical `1m` lane
- GPU-first scorer shape just to accommodate raw quote irregularity

Phase exit criteria:

- the user can intentionally choose aggregate, derived-mid, or quote-aware lanes and understand the consequences
- incompatible fill-model / data-semantic combinations are impossible or loudly rejected

### Phase F3 — GPU Auto Promotion Hardening

Primary goal:

- make `auto` aggressively useful without turning it into blind GPU forcing

Primary files:

- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [options_expression_gpu.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression_gpu.rs)
- [options_expression.cu](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/kernels/options_expression.cu)
- [ta_options_expression_feasibility.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_expression_feasibility.rs)
- [run_options_expression_gpu_promotion_gate.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_gpu_promotion_gate.ps1)

Allowed changes:

- planner thresholds
- fit heuristics
- cache reuse strategy
- benchmark harness/reporting improvements

Do not change:

- CPU oracle correctness contract
- artifact semantics
- backend fallback clarity

Phase exit criteria:

- representative provider-backed workloads choose GPU under `auto` when they should
- CPU remains available as explicit oracle/debug/fallback
- the benchmark set and thresholds are documented enough to avoid ad hoc policy drift

### Phase F4 — Parent Adapter Hardening

Primary goal:

- replace implicit shared double-like replay assumptions with explicit supported-family adapters

Primary files:

- [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
- [options_signal_manifest.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_signal_manifest.rs)

Allowed changes:

- add small family-local adapter functions
- add parent-family capability summaries
- add parent-family-specific tests

Do not change:

- downstream options tuple/scoring/result contracts unless truly necessary

Phase exit criteria:

- supported parent families are obvious in code ownership
- the current supported parent families remain explicit and maintainable without encouraging breadth expansion as a roadmap priority

### Phase F5 — Optimization-Depth Expansion

Primary goal:

- deepen the usefulness of the current optimization surface without widening the product into new strategy or workflow branches

Possible work types:

- stronger cross-artifact comparison
- richer robustness-frontier reporting
- clearer realism-lane disagreement reporting
- additional low-cost score/context rows

Primary files vary by lane, but likely involve:

- [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
- [options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_expression.rs)
- [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

Do not change:

- one-root-per-job rule
- fixed-geometry requirement
- deterministic compile model

Phase exit criteria:

- the current optimization stage becomes more decision-useful without eroding speed or trust

### Phase F6 — Validation-Depth Expansion

Primary goal:

- deepen trust and clarity in the existing validation-heavy options workflow

Primary files:

- likely the existing options validation/reporting surfaces:
  - [options_types.rs](/C:/Rust%20Projects/ta_desktop_demo/crates/app_core/src/options_types.rs)
  - [commands_options_expression.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs)
  - [optionsExpressionService.ts](/C:/Rust%20Projects/ta_desktop_demo/web/src/services/optionsExpressionService.ts)
  - [OptionsExpressionPanel.vue](/C:/Rust%20Projects/ta_desktop_demo/web/src/components/OptionsExpressionPanel.vue)

Allowed changes:

- artifact comparison that helps validation decisions
- stricter finalist comparison and acceptance cues
- stronger disagreement and fragility reporting

Do not change:

- deterministic search/validation as the first-class decision engine

Phase exit criteria:

- users can understand whether to trust, reject, or further inspect an options run without needing a separate validation workflow or speculative downstream ranking stage

---

## 20. Verification Matrix

This section is meant to keep future work concrete.

### Minimum verification for any backend options change

- `cargo test -q -p ta_app_core options_ --lib`
- `cargo check -q -p ta_desktop_demo_app`

### Minimum verification for any CUDA options change

- `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
- rerun the narrow relevant bench or proof harness:
  - [ta_options_expression_feasibility.rs](/C:/Rust%20Projects/ta_desktop_demo/src-tauri/src/bin/ta_options_expression_feasibility.rs)
  - [run_options_expression_gpu_promotion_gate.ps1](/C:/Rust%20Projects/ta_desktop_demo/scripts/run_options_expression_gpu_promotion_gate.ps1)
- if the change materially touches planner/scorer behavior, capture before/after proof output under `output/options_expression_phase05`

### Minimum verification for any frontend options change

- `npm run typecheck`
- `npm run test`
- `npm run build`

### Minimum verification for any supported-parent adapter maintenance

- targeted replay/projection tests
- one saved-artifact replay path for the affected supported parent family
- one end-to-end run against an imported options dataset

### Minimum verification for any quote-aware expansion

- import/load tests for the new semantics
- fill-model compatibility tests
- at least one end-to-end artifact proving semantics are labeled correctly in results

---

## 21. Artifact and Reporting Expectations

Future work should preserve these invariants.

### Every saved options artifact should remain able to answer:

- what parent artifact produced this run
- what dataset and semantics were used
- what backend actually ran
- whether fallback occurred
- what validation stages ran
- what stress lanes ran
- what final verdict context the user should care about

### Every future reporting expansion should preserve:

- explicit search / holdout / sealed separation
- explicit realism-lane labeling
- explicit backend-selection labeling
- explicit stress-lane labeling

### Every future comparison surface should prioritize:

- robustness differences
- semantics differences
- backend differences
- dataset/provenance differences

not just:

- raw best-score ordering

---

## 22. Decision Rules For Scope Requests

This section is for future triage.

When considering a new feature, ask:

1. Does it materially improve options research quality?
2. Can it be compiled into dense enough tensors or isolated to a small CPU post-pass?
3. Does it preserve the current trust/validation model?
4. Does it fit the product framing better than it fits a general simulator?

If the answer to any of these is “no,” the default decision should be:

- defer it
- redesign it
- or reject it

That is how the feature stays fast and still useful instead of becoming a slow, unfocused options sandbox.
