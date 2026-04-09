# Quote-Aware Realism Direction — Final

Date: 2026-04-09
Status: final consolidated direction for Prompt 4 (`Quote-Aware Realism Productization`)

## 1. Executive summary

The right repo decision is:

**keep quote-aware realism first-class in product posture, but import-first in ingestion.**

That means:

- keep **Polygon/Massive `1m` aggregate** as the canonical hot path,
- keep **derived-mid** and **top-of-book quote** as supported realism lanes,
- keep the current **artifact-driven options optimization + embedded validation** workflow intact,
- and **defer direct quote-aware Polygon/Massive download** unless the current aggregate lane plus import-based quote-aware lane clearly prove insufficient.

This is a **revise-but-keep** outcome relative to the earlier first-pass answer.
The direction stays the same, but one semantic point needs to be sharpened:

- **TopOfBookQuote** is the only lane that currently supports quote-side fill models (`quote_conservative`, `quote_mid`).
- **DerivedMidSnapshot** is already first-class as a **lane / provenance / UI** concept, but today it still executes through the aggregate-style runtime path and only supports `aggregate_research`.

So the adopted posture is not “quote-aware everywhere.” It is:

**first-class realism lane, import-first ingestion, explicit semantic asymmetry, no downloader symmetry.**

## 2. Current shipped realism surface

The attached prompt pack and roadmap make the scope question explicit: the repo already supports canonical Polygon/Massive `1m` aggregate download, quote-aware and derived-mid CSV import, and realism-lane selection/reporting, so the remaining question is not whether quote-aware realism exists, but how far it should be productized without breaking the performance-first shape of the system. (Evidence: `options_expression_gpt54_pro_prompt_pack_2026-04-08.md`, Prompt 4.)

### Aggregate lane

This is the canonical hot path today.

- The viable plan treats **`1m` minute aggregates** as the first-wave optimizer resolution and describes direct provider support as bounded to contract-reference plus `1m` minute aggregates. Quote-aware surfaces are explicitly secondary realism lanes rather than the default baseline.
- The direct provider download code builds manifests as:
  - `provider_source_surface = "minute_aggregate"`
  - `price_semantics = MinuteAggregate`
  - `aggregate_price_field = "close"`
- Minute-aggregate imports are also explicitly constrained to `aggregate_price_field='close'`.

### Quote-aware lane

This lane is already real and already productized at the UX/reporting level.

- The future roadmap marks quote-aware realism as already promoted from “supported import semantics” to a **clearly first-class product lane**.
- The options panel already exposes a **top-level realism-lane chooser**, dataset filtering, fill-model guidance, and aggregate-only provider-download gating.
- `TopOfBookQuote` supports all current fill models, including `quote_conservative` and `quote_mid`.
- In runtime execution, `TopOfBookQuote` is the only semantics family that actually uses quote-side entry/exit pricing.

### Derived-mid lane

Derived-mid is also already present and first-class as a lane selection and provenance surface.

- The frontend explicitly maps a `derived_mid` realism lane to `derived_mid_snapshot` semantics.
- The normalized CSV loader already supports optional quote-side fields (`bid`, `ask`, `mid`, `quote_age_ms`) that make import-first realism practical.

But the runtime contract is narrower than the lane label might suggest:

- `DerivedMidSnapshot` currently supports only `aggregate_research` in the backend fill-model matrix.
- In the scorer, `DerivedMidSnapshot` is still routed through the same aggregate-style entry/exit path used by `MinuteAggregate`, not the quote-side pricing path used by `TopOfBookQuote`.

That semantic asymmetry is real and should be stated clearly in any final direction.

## 3. Final recommendation

### Adopted direction

Adopt this as the stable repo posture:

**Quote-aware realism stays first-class in UX, artifacts, dataset contracts, and validation posture — but not yet first-class in provider-download symmetry.**

Concretely:

1. **Keep `1m` aggregate as the canonical fast lane.**
   It remains the default downloader path, the main bounded provider-backed research surface, and the canonical hot path for wide search.

2. **Keep import-first realism for higher-fidelity work.**
   Derived-mid and top-of-book quote should remain supported through normalized CSV import, where the current data contract already exists.

3. **Keep semantic asymmetry explicit.**
   Treat the lanes as intentionally different, not as equivalent:
   - `minute_aggregate`: aggregate-only execution semantics
   - `derived_mid_snapshot`: first-class realism/provenance lane, but still aggregate-style runtime semantics today
   - `top_of_book_quote`: true quote-aware execution semantics

4. **Do not add direct quote-aware Polygon/Massive download now.**
   The roadmap explicitly defers that work unless the current aggregate lane and import-based quote-aware lane prove insufficient.

### Why this wins now

This direction beats the main alternatives.

#### Versus a more conservative aggregate-first-only stance

That stance is now too conservative.

The repo has already shipped:

- first-class realism-lane selection,
- fill-model guidance,
- result labeling for aggregate vs derived-mid vs quote-aware runs,
- and artifact/report surfaces that preserve realism-lane provenance.

Rolling quote-aware realism back into a hidden advanced-import niche would reverse already-shipped productization rather than improve it.

#### Versus a more aggressive quote-aware-first / direct-download-now stance

That stance is too aggressive for the current repo shape.

The current downloader path is structurally aggregate-only. It emits a minute-aggregate dataset contract and relies on the aggregate `close` field. The current roadmap explicitly says not to prioritize direct quote-aware Polygon/Massive download unless the existing aggregate + import-based realism lanes prove insufficient.

The current repo therefore has **real quote-aware productization**, but it does **not** yet have evidence that downloader symmetry is the best next investment.

## 4. What remains canonical and unchanged

The following should remain unchanged:

- **Aggregate `1m` remains the canonical hot path.**
- **CPU fallback/oracle behavior remains unchanged.**
- **The artifact-driven workflow remains unchanged.**
- **The current bounded options host remains unchanged**; no new workflow stage or options mode should be created for this question.
- **Current structure-family and parent-family scope remain unchanged.**
- **Direct provider download remains aggregate-lane only** in the options panel.
- **The current fill-model compatibility matrix remains strict** at both backend and frontend boundaries.

## 5. Deferred ideas

These are worth keeping deferred rather than expanding now.

### A. Direct quote-aware Polygon/Massive download

Defer this, do not implement it now.

Reason:

- the roadmap explicitly defers it,
- the downloader code is currently minute-aggregate-specific,
- and the repo has not yet shown that import-first quote-aware realism is operationally insufficient.

### B. Distinct derived-mid runtime semantics

Also defer this.

The derived-mid lane already exists as a first-class user choice and provenance surface, but it is not yet a distinct quote-side execution model. That mismatch is acceptable for now as long as the UI and docs stay explicit.

Only reopen this if the team finds repeated, material cases where `derived_mid_snapshot` should behave differently from `minute_aggregate` at runtime while still not requiring full quote-side semantics.

### C. Broader quote-native workflow expansion

Defer any expansion that would move the system toward:

- generic quote replay,
- quote-native chain browsing,
- or a quote-simulator product branch.

That would widen the product in exactly the way the roadmap tries to avoid.

## 6. Rejected ideas

The repo should reject these directions for the current scope.

### A. Demote quote-aware realism back to a secondary / hidden lane

Rejected.

The product already shipped first-class realism-lane selection and labeling, so intentionally demoting it would move backward.

### B. Force downloader symmetry now

Rejected for now.

The right question is not whether quote-aware data exists in principle, but whether adding a new provider-ingestion surface is the best next repo-fit use of effort. The attached roadmap says no unless import-first realism proves insufficient.

### C. Treat derived-mid as equivalent to quote-side execution today

Rejected.

That is not how the current contracts behave. Derived-mid is first-class as a lane/provenance surface, but not yet as a distinct quote-side fill/execution semantics family.

### D. Turn the options workflow into a quote replay simulator

Rejected.

That would conflict with the repo’s core doctrine: compiled search first, bounded irregular replay later, and performance as a product feature.

## 7. Concrete code touch points by file

This recommendation does **not** justify a new backend/provider implementation. Most code surfaces should remain as they are, with only small clarification or test hardening allowed.

### Keep as-is

#### `src-tauri/src/commands_options_data.rs`

Keep direct provider download aggregate-only.

Current contract shape already hardcodes the downloader output as:

- `provider_source_surface = minute_aggregate`
- `price_semantics = MinuteAggregate`
- `aggregate_price_field = close`

That is the correct current boundary and should remain the boundary.

#### `crates/app_core/src/options_data_store.rs`

Keep import-first realism support here.

This file already accepts normalized CSV datasets with optional:

- `bid`
- `ask`
- `mid`
- `quote_age_ms`

That is the correct place for quote-aware and derived-mid realism ingestion.

#### `crates/app_core/src/options_types.rs`

Keep the current fill-model compatibility matrix.

This file correctly expresses today’s semantics:

- `MinuteAggregate` -> `aggregate_research` only
- `DerivedMidSnapshot` -> `aggregate_research` only
- `TopOfBookQuote` -> quote-aware fills allowed

#### `crates/app_core/src/options_expression.rs`

Keep the current runtime semantics split.

This file correctly distinguishes:

- aggregate-style execution for `MinuteAggregate` and `DerivedMidSnapshot`
- quote-side execution for `TopOfBookQuote`

That distinction should stay explicit.

#### `web/src/stores/optionsExpressionStore.ts`

Keep the current realism-lane chooser and aggregate-only provider-download gate.

This file already:

- maps user lanes to backend semantics,
- coerces incompatible fill models,
- and blocks direct provider download unless the lane is `aggregate`.

That is the right current UX boundary.

### Acceptable small hardening / clarity changes

These are optional and low-risk if the team wants to make the final direction more explicit.

#### `web/src/components/OptionsExpressionPanel.vue`

Clarify user-facing copy so that:

- `derived_mid` is described as a supported realism lane,
- but not described as equivalent to full quote-side fill semantics.

#### `web/src/stores/optionsExpressionStore.ts`

Tighten wording in guidance strings so users understand:

- quote-aware = top-of-book quote semantics,
- derived-mid = supported realism lane, aggregate-style runtime semantics today,
- aggregate = canonical hot path and downloader lane.

#### `crates/app_core/src/options_types.rs` and related tests

If any clarification work is done, add or tighten tests that lock in the current semantics matrix so the distinction between `DerivedMidSnapshot` and `TopOfBookQuote` stays explicit.

## 8. Validation / verification order

Because the adopted direction is mostly **hold steady, clarify semantics, preserve boundaries**, the required verification is regression-heavy rather than benchmark-heavy.

### 1. Backend semantics regression

Run or preserve tests that prove:

- `MinuteAggregate` rejects quote-side fill models
- `DerivedMidSnapshot` rejects quote-side fill models
- `TopOfBookQuote` accepts quote-aware fill models

### 2. Import/load regression

Verify the normalized CSV importer still correctly accepts quote-aware/derived-mid-supporting fields:

- `bid`
- `ask`
- `mid`
- `quote_age_ms`

### 3. UI gating regression

Verify the options store still:

- maps realism lanes correctly,
- coerces fill models correctly,
- and blocks provider download outside the aggregate lane.

### 4. End-to-end labeling regression

Run one end-to-end workflow for each lane category:

- one aggregate dataset run,
- one derived-mid imported dataset run,
- one top-of-book quote imported dataset run.

Confirm that saved artifacts and reopened summaries label:

- `price_semantics`
- fill model
- realism lane
- backend route

correctly and consistently.

### 5. Performance re-check only if scope reopens

Do **not** invent new performance benchmarking work for this decision alone.

Only add new provider-backed benchmarking if the repo reopens one of the deferred items, especially direct quote-aware provider download or distinct derived-mid runtime semantics.

## 9. Adoption gates

These are the gates that must all be met before the repo should reopen additional quote-aware productization.

### Gate 1 — proven insufficiency of the current posture

The team should be able to show that:

- aggregate `1m` + import-first quote-aware validation is operationally insufficient,
- not merely aesthetically incomplete.

Examples of acceptable evidence:

- repeated cases where aggregate and quote-aware finalists disagree in ways users cannot efficiently reproduce with the current import-first lane,
- repeated friction where manual dataset preparation is the main blocker to decision-grade validation,
- or repeated evidence that important decisions depend on quote-native realism that cannot be supported cleanly through import-first datasets.

### Gate 2 — bounded downloader shape

Any direct quote-aware provider implementation must remain bounded and artifact-driven.

It must **not** reopen the system into:

- general quote browsing,
- generic quote replay,
- or a new options platform branch.

### Gate 3 — quote-native dataset contract

Before adopting direct quote-aware download, the repo must define a real quote-native dataset contract, including explicit semantics for fields such as:

- bid
- ask
- mid
- quote age / freshness
- provider surface provenance

without silently treating quote data like another minute-aggregate file.

### Gate 4 — performance-fit gate

The new path must preserve the repo’s performance model.

That means it must not:

- weaken aggregate `1m` as the canonical hot path,
- force quote-native logic into the hot path for all runs,
- or materially erode compiled-tensor search economics.

### Gate 5 — economics / entitlement / supportability gate

The new provider surface must be supportable for the intended user base.

If the required quote entitlement, storage, download volume, or operational complexity makes the feature unreliable or narrow, the repo should keep the import-first posture.

## 10. Stop conditions

Stop and keep the current posture if any of the following become true.

1. **The work starts weakening the aggregate `1m` lane** as the canonical optimizer hot path.
2. **The work starts turning the options workflow into a quote replay simulator** instead of a compiled, artifact-driven optimizer with bounded irregular replay.
3. **The downloader path cannot stay bounded** to a small, deterministic artifact-driven contract.
4. **The provider/data economics remain too narrow or too operationally heavy** for the intended user base.
5. **The only justification is realism aesthetics**, rather than clear decision-quality or workflow-friction evidence.
6. **Derived-mid semantics cannot be clarified cleanly** without confusing users about the difference between midpoint-style imported realism and true quote-side fills.

## Final decision

**Keep the current scope narrow.**

The winning repo direction is:

- **aggregate `1m` remains canonical**,
- **quote-aware and derived-mid remain first-class lanes**,
- **realism stays import-first for higher-fidelity work**,
- **direct quote-aware Polygon/Massive download stays deferred**,
- and any near-term changes should be limited to **clarifying the semantic distinction between derived-mid and top-of-book quote**, not widening the ingestion/runtime surface.
