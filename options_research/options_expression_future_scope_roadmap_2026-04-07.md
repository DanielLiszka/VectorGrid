# Options Expression Optimizer — Comprehensive Future Implementation Roadmap

Date: 2026-04-07
Status: comprehensive future-scope implementation roadmap for turning the current options-expression feature into a world-class, performance-first options backtest optimizer

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

**a world-class, artifact-driven, GPU-first options backtest optimizer that keeps as much useful work as possible inside dense compiled passes while preserving robust post-search validation, stress testing, meta-analysis, and optional ML ranking stages.**

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
- integrated validation, comparison, and optional downstream meta/ML ranking

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
- meta-analysis across many saved option artifacts
- ML feature extraction if it materially disrupts the scorer

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

## 8. Expression Search-Space Expansion

### Current shipped structures

- `long_single`
- `debit_vertical`
- `long_pair`

### Short-term admissible expansion

The next structure expansions should be limited to:

- additional same-expiry fixed debit families
- additional same-side fixed-ratio bundles

Examples:

- slightly richer debit call/put spread families
- controlled ladder variants beyond the current `long_pair`

### Hard admission rules

Any new structure family must:

- compile into dense fixed-width candidate slots
- keep deterministic leg selection
- avoid dynamic chain search in the scorer
- preserve cache reuse economics
- preserve acceptable invalid-slot rates

### Not admissible

- arbitrary user-defined leg geometry
- rolling structures
- mixed-expiry calendars / diagonals
- broker-style assignment-sensitive spreads

---

## 9. Parent Strategy Expansion

### Current supported parent families

- `DoubleMa`
- `CompositeTrend`
- `RegimeHybrid`
- `BreakoutMomentum`

### Near-term hardening

Even though support exists, the next improvement should be:

- explicit family-local projector/adapter wrappers for the currently supported parents

Reason:

- clearer ownership
- cleaner future maintenance
- fewer hidden assumptions around “double-like” replay

### Later expansion

New parent families should only be added when:

- replay is deterministic
- event timing is recoverable without lookahead
- the resulting signal manifest still fits the current options workflow cleanly

This is an adapter-expansion task, not an optimizer rewrite.

---

## 10. Validation, Meta-Analysis, and ML/AI Roadmap

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

### Stage 5 — portfolio overlay

Current posture:

- appropriate as a finalist/post-pass lane

Future target:

- preserve it as a post-pass
- never let portfolio path dependence dominate the main tuple scorer

### Stage 6 — options meta-analysis

This is one of the best future differentiators.

Desired capabilities:

- compare many options-expression artifacts across roots and parent strategies
- study robustness by root, side, DTE family, and structure family
- rank recurring option-expression motifs

This should mirror the spirit of existing meta-analysis in the conventional optimizer, but only after the options artifact family is mature enough.

### Stage 7 — options ML/AI stage

This should be optional and downstream.

Good future uses:

- rank finalists by survivability probability
- identify robustness signatures
- classify fragile vs durable options-expression tuples
- learn when quote-aware realism disagrees with aggregate-based winners

Bad use:

- replacing the compiled optimizer with a black-box model

ML should sit after deterministic search and validation, not replace them.

---

## 11. Workflow and UX Roadmap

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

### Dedicated mode decision

Create a dedicated options mode only if all of these start becoming true:

- the options workflow has its own heavy dataset lifecycle
- the current follow-up panel becomes cramped
- the user mental model is clearly no longer “saved run -> options follow-up”

Until then, a unified shell remains the cleaner choice.

---

## 12. GPU Architecture and Runtime Roadmap

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

## 13. Metrics Philosophy

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

## 14. Phased Future Roadmap

### Phase F1 — First-Class Options Validation

Deliverables:

- explicit options validation presentation stage
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

- the parent-support surface is explicit and maintainable

### Phase F5 — Conservative Breadth Expansion

Deliverables:

- one additional admissible parent family or one additional admissible structure family
- only after density/performance proof

Exit criterion:

- breadth increases without compromising the performance model

### Phase F6 — Options Meta / ML Layer

Deliverables:

- cross-artifact comparison and meta-analysis
- optional ML ranking / survivability stages

Exit criterion:

- the options workflow gains higher-level research value beyond one-off tuple search

---

## 15. Comprehensive Future Checklist

Status key:

- `[ ]` not implemented yet
- `[~]` partially implemented / present in backend form but not fully productized

### A. Finish The Current Intended Product

- `[~]` Turn options post-search validation into a first-class product surface
- `[ ]` Add explicit options validation verdicts and acceptance cues
- `[ ]` Add stronger finalist validation comparison views
- `[~]` Promote quote-aware realism from supported import semantics to a clearly first-class product lane
- `[ ]` Add stronger fill-model guidance and guardrails by dataset price semantics
- `[ ]` Add clearer result labeling for aggregate vs derived-mid vs quote-aware runs
- `[~]` Finish GPU-first productization of `auto`
- `[ ]` Revisit `auto` thresholds after a cleaner canonical provider-backed benchmark set
- `[ ]` Make backend-selection reasons even more explicit in saved artifacts and comparison views
- `[~]` Replace the implicit shared double-like replay path with explicit family-local adapters for the currently supported parent families

### B. Conservative Scope Expansion

- `[ ]` Add one additional admissible non-double-like parent artifact family only after deterministic replay proof
- `[ ]` Add one additional bounded same-expiry fixed-geometry structure family only after density/cache proof
- `[ ]` Add stronger artifact comparison / provenance diffing for options-expression runs
- `[ ]` Add richer robustness-frontier reporting only if real usage proves it is decision-useful

### C. Higher-Level Research Stack

- `[ ]` Add options meta-analysis over saved options-expression artifacts
- `[ ]` Add optional ML/AI ranking stages for finalists after deterministic search and validation
- `[ ]` Add cross-root and cross-parent-family analysis surfaces for options artifacts

### D. Larger Product Decisions

- `[ ]` Decide whether the current unified History follow-up host remains sufficient
- `[ ]` Promote options into a dedicated mode only if workflow pressure clearly justifies it
- `[ ]` Decide whether direct quote-aware Polygon/Massive download is worth the operational and storage cost

### E. Keep Out Of Scope

- `[ ]` Do not add assignment / exercise modeling
- `[ ]` Do not add rolling
- `[ ]` Do not add arbitrary multi-leg scripting
- `[ ]` Do not add live options execution
- `[ ]` Do not add generic broker/margin simulation

---

## 16. Explicit Non-Goals For Future Expansion

These should remain rejected unless the product strategy changes materially.

- becoming a general options platform
- matching LEAN on slow, CPU-heavy broker realism
- arbitrary event-driven options scripting
- margin/accounting parity with full broker simulators
- feature breadth that destroys the optimizer’s performance edge

---

## 17. Research Anchors

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

## 18. Short Verdict

The current options-expression feature is already real.

The next step is not “add everything.”

The next step is:

1. make validation more explicit
2. make realism clearer
3. make GPU the normal path where the data supports it
4. only then widen breadth

That is how VectorGrid can become a world-class options backtest optimizer **without sacrificing the performance edge that makes it worth building.**
