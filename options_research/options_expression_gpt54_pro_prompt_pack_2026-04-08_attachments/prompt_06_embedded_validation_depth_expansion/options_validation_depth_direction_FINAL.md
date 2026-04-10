# Options Validation Depth Direction — Final

Date: 2026-04-10
Status: final third-pass consolidation for Prompt 6 (`Embedded Validation-Depth Expansion`)

Implementation follow-through:

- [options_expression_support_aware_validation_semantics_implementation_guide_2026-04-10.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_support_aware_validation_semantics_implementation_guide_2026-04-10.md)

## 1. Executive summary

The current options-expression lane already ships real embedded validation. The repo is not missing holdout, sealed-finalist, or stress evaluation, and it is not missing row-level validation inspection. The best remaining Prompt 6 move is therefore **not** another validation stage, not a broader artifact dashboard, and not a budget-policy rewrite.

The highest-ROI next step is narrower:

**make embedded validation semantics support-aware, using the counts the repo already stores, and surface that adequacy directly in the current UI.**

Today the repo can still overstate validation because verdict derivation is mostly **lane-presence based**. A tuple can be labeled `holdout_supported` or `sealed_finalist` when the relevant metric struct exists, even if that lane has zero valid trades. The same issue can spill into `stress_fragile` when a stress lane exists structurally but has no supported trades. That is the main remaining trust gap.

The winning direction is therefore:

1. keep the current verdict enum and current workflow shape,
2. change verdict derivation to require **actual supported trades** in the relevant validation lane,
3. recompute verdicts for reopened artifacts instead of blindly trusting persisted legacy verdicts,
4. show raw support counts and support ratios inside the existing result and artifact surfaces,
5. stop there unless later evidence proves that shortlist-budget or frontier math is the real next blocker.

## 2. Current shipped validation surface

The current repo already has a real embedded-validation lane inside the artifact-driven options-expression workflow.

Shipped baseline:

- artifact-driven options expression optimization is already live
- current follow-up host is the existing History follow-up surface
- holdout, sealed-finalist, and multi-lane stress reporting already exist
- explicit tuple-level validation verdicts already exist
- validation-frontier and validation-winner review surfaces already exist
- saved-artifact comparison and reopened-artifact validation review already exist

On the backend, each retained tuple row already carries the major validation lanes and related reporting state:

- `selection_metrics`
- `holdout_metrics`
- `sealed_evaluation_metrics`
- `stress_metrics`
- `sealed_stress_metrics`
- `harsher_stress_metrics`
- `sealed_harsher_stress_metrics`
- `extreme_stress_metrics`
- `sealed_extreme_stress_metrics`
- `validation_verdict`
- `robust_score`
- `pre_sealed_rank`
- `sealed_final_rank`

Just as important: the repo already stores the adequacy ingredients needed for a better verdict. `OptionsExpressionMetrics` already includes:

- `valid_trades`
- `invalid_events`
- `invalid_rate`

That means the repo can already derive per-lane support counts and support ratios without widening GPU aggregate state or adding replay.

Current backend semantics are still too presence-oriented:

- `inferred_validation_verdict()` promotes a tuple beyond `search_only` when `holdout_metrics.is_some()` or `sealed_finalist || sealed_evaluation_metrics.is_some()`.
- `is_stress_fragile()` compares the strongest validation lane against the strongest stress lane when those structs exist.
- `exactify_ranked_results()` populates holdout and stress metrics for retained rows whenever holdout events exist.
- `apply_sealed_finalist_stage()` populates sealed metrics for finalist-budget rows whenever sealed events exist.

That means the repo currently answers “does this lane exist?” more reliably than “is this tuple actually supported by that lane?”

The attached SPY provider-backed artifact makes that distinction concrete. The preview reports:

- `18` total events
- `10` search events
- `3` holdout events
- `5` sealed-test events
- `0` holdout events with candidates
- `0` holdout events with baseline outcomes
- `4` sealed-test events with candidates/outcomes

So the current system already has at least one realistic case where a validation split exists but real tuple-level support inside that split is materially thin or absent.

The frontend is already close enough to expose this cleanly, but it currently mirrors the backend’s presence-based logic. The current panel resolves tuple and artifact verdicts primarily from:

- stored `validationVerdict` / `topValidationVerdict` when present
- otherwise, boolean presence of `holdoutMetrics`, `sealedEvaluationMetrics`, `sealedFinalist`, and stress-lane values

The artifact summary layer is also still mostly maxima/count oriented. It is useful for compact review, but it should not be the main place where Prompt 6 tries to deepen validation logic.

## 3. Final recommendation

### Adopt now: support-aware embedded validation semantics

The adopted direction is:

**correct the meaning of validation before adding more validation math.**

That means four implementation rules.

### 3.1 Reinterpret validation support from existing metrics

Do not add a new heavyweight support payload first.

Instead, use the data the repo already has:

- lane event count = `valid_trades + invalid_events`
- lane support ratio = `valid_trades / (valid_trades + invalid_events)`
- equivalently, support ratio = `1 - invalid_rate`

This is the key third-pass refinement: the repo does **not** need a new backend data plane to answer adequacy. It mostly needs to use the state it already persists.

### 3.2 Make tuple verdicts require supported validation, not merely present lanes

Keep the current enum:

- `search_only`
- `holdout_supported`
- `sealed_finalist`
- `stress_fragile`

But change the semantics.

Recommended rule:

- `holdout_supported` requires `holdout_metrics.valid_trades > 0`
- `sealed_finalist` requires `sealed_evaluation_metrics.valid_trades > 0`
- `sealed_finalist` status as a finalist-budget flag stays separate from whether the tuple has sealed validation support
- `search_only` remains the fallback when no supported holdout or sealed lane exists
- `stress_fragile` should only be possible when both the base validation lane and the stressed comparison lane have supported trades

This fixes the core overclaim without adding a new taxonomy.

### 3.3 Recompute verdicts for reopened artifacts and artifact summaries

This is mandatory.

If semantics change but reopened artifacts still trust old persisted verdict fields, the repo will keep showing stale validation labels.

So the repo should not blindly trust legacy persisted verdicts during:

- artifact summary derivation
- artifact list/load flows
- frontend fallback rendering

Instead, it should recompute current verdicts from current metrics.

### 3.4 Surface raw adequacy in the existing UI

Do not create a new validation mode.

Use the current surfaces and add compact support facts such as:

- holdout support `0 / 3`
- sealed support `4 / 5`
- support ratio or support-rate chip

Important UI rule:

- use raw counts and ratios first
- do **not** introduce a new opaque “confidence score” or a hard-coded adequacy threshold in this pass

This keeps the product decision-grade without hiding thin support behind invented categories.

## 4. What remains unchanged

The following should stay unchanged in this Prompt 6 pass.

### Workflow shape

- stay inside the current options optimization + embedded validation workflow
- keep the current History follow-up host
- do not add a separate validation mode or workflow tree

### Validation families and ranking shape

- keep current holdout / sealed / stress family structure
- keep current baseline / conservative / harsher / extreme lane set
- keep current `robust_score` function unchanged in this pass
- keep sealed metrics out of broad first-pass tuple ranking

### Budget policy

- keep `top_k = 25` as the current default
- keep `sealed_finalist_count = 5` as the current default
- do not change shortlist/finalist budgets without a targeted ablation proving distortion is a recurring real problem

### Artifact-summary posture

- do not add more `best_*` maxima as the answer to validation depth
- do not turn artifact summaries into another frontier dashboard
- keep artifact-level changes narrow: corrected verdict semantics and limited adequacy cues only

### Runtime / performance posture

- no new replay pass
- no new bootstrap pass
- no new GPU aggregate widening just for Prompt 6
- no CPU/GPU boundary redesign inside this pass

## 5. Deferred ideas

These ideas may become worthwhile later, but they are not the adopted Prompt 6 direction now.

### 5.1 Monotone stress-path summary

A compact monotone-deterioration or “first break” stress summary is reasonable, but it is secondary to support-aware semantics.

If adopted later, it should be derived from already-computed lanes and should remain compact. It should not require extra replay or extra artifact-dashboard breadth.

### 5.2 Shortlist/finalist budget pressure instrumentation

A retained-boundary or finalist-boundary analysis may become useful later, especially if support-aware semantics reveal many cases where the current retained/finalist budgets are likely constraining trustworthy validation.

But there is not enough attached evidence here to justify changing `top_k` or `sealed_finalist_count` now.

### 5.3 Frontier / regret / dominance math

Broader frontier-style math may become useful later, but it should only be attempted after support semantics are corrected.

Until then, the repo risks adding mathematically interesting but operationally misleading summaries on top of already-unclear adequacy.

### 5.4 Additional artifact-summary coherence work

If Prompt 5 summary-coherence work is still incomplete in the code snapshot, that should be handled as Prompt 5 residue, not as the main Prompt 6 answer.

Prompt 6 should not solve validation depth by turning artifact summaries into larger mixed-summary products.

## 6. Rejected ideas

These ideas should be explicitly rejected for this pass.

### 6.1 A new validation stage or validation mode

Rejected.

The roadmap already says validation depth should stay embedded in the current workflow. Prompt 6 should improve clarity and adequacy inside the current host, not create a second product lane.

### 6.2 A new verdict taxonomy right now

Rejected.

The current four-way enum is enough if the semantics become support-aware and the UI shows raw support counts. Adding more labels now risks complexity without first fixing the underlying truthfulness problem.

### 6.3 More artifact-level scalar maxima or summary dashboards

Rejected.

That does not solve the main trust problem and would duplicate existing row-level validation/frontier surfaces.

### 6.4 Budget rewrites without ablation

Rejected.

Do not change `top_k` or `sealed_finalist_count` until targeted evidence shows that retained/finalist containment is materially degrading trustworthy decisions.

### 6.5 Replay-heavy confidence or bootstrap additions

Rejected.

Prompt 6 should not add slow, low-value reporting work when the current repo already stores the adequacy counts needed for a better first correction.

## 7. Concrete code touch points by file

## 7.1 `crates/app_core/src/options_types.rs`

Primary ownership for the semantic fix.

Target symbols:

- `OptionsExpressionMetrics`
  - add tiny helper methods such as:
    - `total_events()`
    - `has_supported_trades()`
    - `support_rate()`
- `OptionsExpressionTupleResult::strongest_validation_metrics`
  - revise to prefer only supported validation lanes
- `OptionsExpressionTupleResult::strongest_validation_stress_metrics`
  - revise to prefer only supported stress lanes
- `OptionsExpressionTupleResult::inferred_validation_verdict`
  - change from lane-presence semantics to supported-trade semantics
- `OptionsExpressionTupleResult::is_stress_fragile`
  - require supported base and stressed lanes before fragility is asserted

Also update the current unit tests around default verdicts and stress-fragility classification.

## 7.2 `crates/app_core/src/options_expression.rs`

Primary ownership for runtime refresh behavior.

Target symbols:

- `refresh_validation_verdicts`
  - continue to centralize verdict refresh after exactification/finalist staging, but now with support-aware semantics
- `exactify_ranked_results`
  - no major logic rewrite required, but verify that its populated metrics flow into the updated support-aware verdict rules cleanly
- `apply_sealed_finalist_stage`
  - keep finalist-budget logic unchanged, but ensure finalist rows with zero sealed support are not mislabeled as sealed-validated

Recommended additional tests here:

- retained row receives `holdout_metrics` with `valid_trades = 0` and remains `search_only`
- sealed finalist with zero sealed support but nonzero holdout support resolves to `holdout_supported`
- zero-support stress lanes do not trigger `stress_fragile`

## 7.3 `src-tauri/src/commands_options_expression.rs`

Primary ownership for reopened-artifact consistency.

Target symbols:

- `summarize_options_expression_artifact`
  - recompute validation verdict counts from current tuple metrics instead of trusting stored verdicts
- `load_options_expression_artifact_blocking_headless`
  - normalize/recompute tuple verdicts on artifact load so reopened artifacts reflect current semantics
- any artifact-summary helper introduced for verdict normalization
  - keep it compact and metrics-derived

Important rule here:

- do not widen `OptionsExpressionArtifactSummary` into a larger validation product
- only carry corrected verdicts/counts and minimal adequacy cues if truly needed

## 7.4 `web/src/services/optionsExpressionService.ts`

Primary ownership for backward-compatible transport normalization.

Target areas:

- tuple normalization
- artifact-summary normalization
- any helper that currently treats persisted verdict strings as authoritative

Preferred posture:

- if the backend recomputes verdicts on artifact load and summary generation, keep frontend logic thin
- if older artifacts can still slip through unchanged, add a safe fallback that derives current verdicts from metrics rather than trusting stale stored strings

## 7.5 `web/src/components/OptionsExpressionPanel.vue`

Primary ownership for current-surface adequacy presentation.

Target symbols/patterns:

- `resolveTupleValidationVerdict`
- `resolveArtifactValidationVerdict`
- `describeTupleValidation`
- `describeArtifactValidation`
- current validation chips / tuple table / reopened artifact summary / selected tuple detail

Required changes:

- stop presence-only fallback logic from overclaiming support
- show raw holdout/sealed support counts and/or support ratios
- change explanatory copy from “metrics are present” language to “supported trades exist” language

Important UI rule:

- reuse current surfaces
- do not add another validation dashboard

## 7.6 `web/src/stores/optionsExpressionStore.ts`

This file likely needs little or no structural change.

Only touch it if:

- updated normalized fields need to flow through state cleanly
- or status/help copy should explain the new support-aware semantics

The store should not become the main home of Prompt 6 logic.

## 8. Verification order

### Step 1 — backend semantic-unit tests

In `options_types.rs`:

- verify `holdout_metrics` with `valid_trades = 0` does **not** yield `holdout_supported`
- verify `sealed_finalist = true` plus zero-support sealed metrics does **not** automatically yield `sealed_finalist`
- verify supported holdout or sealed lanes still promote correctly
- verify stress fragility only fires when both base and stressed lanes have supported trades

### Step 2 — backend artifact-summary and load tests

In `commands_options_expression.rs`:

- verify `summarize_options_expression_artifact(...)` recomputes verdict counts from metrics
- verify summary counts do not overstate validated tuples when holdout/sealed lanes have zero supported trades
- verify a legacy artifact payload with stale stored verdicts is normalized to current semantics on load or summary

### Step 3 — runtime regression tests in `options_expression.rs`

Verify:

- ranking order remains unchanged when verdict semantics are corrected
- finalist selection/budget logic remains unchanged
- only verdict labeling and support presentation change

### Step 4 — frontend service/panel tests

Verify:

- reopened artifacts with old stored verdicts render the current verdict semantics
- tuple/detail surfaces show support counts or ratios from existing metrics
- artifact rows do not describe the top tuple as validated when actual holdout/sealed support is zero

### Step 5 — manual artifact smoke

Use the current provider-backed SPY DTE7-style artifact set and a reopened saved artifact with thin support.

Manual checks:

- no row with `0` supported holdout trades is labeled `holdout_supported`
- no row with `0` supported sealed trades is labeled `sealed_finalist`
- `stress_fragile` appears only when the stressed lane actually has supported trades
- UI copy shows raw support facts rather than inventing a new confidence score

## 9. Stop boundary

Stop the Prompt 6 workstream once all of the following are true:

1. verdicts are support-aware rather than lane-presence-based,
2. reopened artifacts are normalized to current verdict semantics,
3. current tuple/artifact surfaces show raw support counts or ratios,
4. current workflow users can distinguish:
   - search-only,
   - validated with actual support,
   - and validated-but-fragile,
   without a new workflow surface.

Do **not** continue Prompt 6 past that point into:

- a new validation mode,
- a bigger artifact-summary dashboard,
- budget-policy rewrites,
- replay-heavy confidence layers,
- or broader frontier math.

Those are separate decisions and should only reopen if the support-aware correction proves insufficient.

## 10. Adoption gates

Proceed with this direction only if these gates hold.

### Gate A — the trust problem is real

Adopt if a quick audit confirms that current live or reopened artifacts can overclaim validation because metric presence is being treated as validation support.

The attached SPY phase-0.5 evidence already strongly suggests this gate is real, because holdout splits can exist with zero holdout-supported events.

### Gate B — the fix is cheap

Adopt only if support-aware adequacy can be derived from existing metric fields or from tiny helper methods.

If implementation unexpectedly requires:

- widened GPU aggregate state,
- extra replay,
- or artifact schema bloat,

stop and do not continue in Prompt 6.

### Gate C — the fix is interpretable

Adopt only if the user-visible result stays simpler, not more opaque:

- raw support counts and ratios are acceptable
- a new opaque confidence scalar is not

### Gate D — the fix is semantically contained

Adopt only if ranking order, finalist budgets, and workflow shape remain unchanged.

If the work begins to force:

- `top_k` policy redesign,
- `sealed_finalist_count` redesign,
- or broader frontier math,

defer those and keep this pass narrow.

## Closing decision

**Adopt now:** support-aware verdict semantics and adequacy surfacing from existing metrics.

**Defer:** stress-path summarization, shortlist-pressure instrumentation, broader frontier/regret math, and any broader artifact-summary redesign.

**Reject for this pass:** a new validation stage, new verdict taxonomy, budget rewrites without ablation, replay-heavy confidence additions, and more artifact-level scalar maxima.

That is the narrowest change that materially improves trust in the repo’s current embedded-validation surface without widening the workflow or the runtime shape.
