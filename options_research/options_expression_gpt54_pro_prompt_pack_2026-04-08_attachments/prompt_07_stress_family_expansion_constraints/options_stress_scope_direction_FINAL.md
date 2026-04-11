# Options Stress Scope Direction — Final

Date: 2026-04-09
Status: final consolidation for Prompt 7 (`Stress-Family Expansion Constraints`)

## Executive summary

The repo should **freeze the current options stress-family surface** at **`baseline`**, **`conservative`**, **`harsher`**, and **`extreme`**. The roadmap already names those four as the shipped robustness lanes and explicitly says future policy is to add lanes only when they add concrete decision value, not for symmetry alone. The same roadmap also points the next worthwhile work toward deeper validation clarity, robustness-frontier reporting, and realism-lane disagreement reporting inside the current workflow rather than broader stress-family proliferation. fileciteturn16file4

So the adopted direction is:

- **do not add a fifth generic stress family now**;
- keep **`baseline` / `conservative` / `harsher`** as the broad compiled robustness lanes;
- keep **`extreme`** as the retained-tuple / sealed-finalist exact stress lane rather than an always-on full-grid lane; and
- treat any later quote-aware spread/liquidity shock idea as a **realism-lane / embedded-validation** question, not as Prompt 7 stress-family expansion.

This stays aligned with the current repo shape, which is still an artifact-driven, GPU-first options workflow with hybrid GPU broad ranking and CPU retained-tuple exactification at the finalist boundary. fileciteturn16file12

## Current shipped stress-family surface

The current scope is already real and already broad enough to support meaningful robustness decisions:

- The roadmap describes the shipped robustness lanes as **`baseline`**, **`conservative`**, **`harsher`**, and **`extreme`**. fileciteturn16file4
- The core engine defines `AggregateStressMode` with exactly those four modes and compiles broad candidate/summary caches for **baseline**, **conservative**, and **harsher** during `prepare_options_expression_run`. fileciteturn16file3
- The exact post-ranking path uses `exactify_ranked_results` and `apply_sealed_finalist_stage` to attach **extreme** stress outputs only for retained tuples and sealed finalists rather than compiling an always-on fourth broad lane. fileciteturn16file3
- `OptionsExpressionTupleResult` already carries `stress_metrics`, `harsher_stress_metrics`, `extreme_stress_metrics`, plus the sealed-finalist counterparts, and the validation verdict logic already turns stressed deterioration into `StressFragile`. fileciteturn16file7
- `OptionsExpressionArtifactSummary` already exposes separate best-value and presence fields for stress, harsher-stress, and extreme-stress outputs, so each added lane widens persisted summary, compare, and reporting surfaces. fileciteturn16file1
- The prompt-pack framing for current repo evidence also says the shipped slice already includes holdout/sealed/stress reporting and that the GPU route is still hybrid at the finalist boundary. fileciteturn16file12

In other words, the repo already covers the practical stress story the current product needs:

1. normal search behavior,
2. conservative deterioration,
3. harsher deterioration,
4. worst-case retained-tuple deterioration,
5. verdict-level fragility detection, and
6. saved-artifact reporting for all of the above.

## Final recommendation

### Adopted direction

**Freeze the current stress-family set now.**

That means:

- **no fifth generic stress family**;
- **no “slightly harsher than harsher” lane**;
- **no symmetry-completion pressure**;
- **no promotion of `extreme` into a broad always-on compiled lane**.

### Why this wins now

The repo’s own roadmap already gives the answer: the default direction is to improve validation quality and clarity inside the current workflow, while keeping additional lanes only when they add concrete decision value. It explicitly rejects widening stress families for symmetry alone and instead prioritizes richer robustness-frontier reporting plus disagreement reporting between aggregate, derived-mid, and quote-aware lanes. fileciteturn16file4

The current code shape matches that policy:

- broad compile-time work already covers **baseline**, **conservative**, and **harsher**;
- **extreme** is intentionally left to exact retained-tuple / sealed-finalist evaluation;
- verdict logic already consumes stressed deterioration to identify fragility; and
- saved artifacts already expose separate stress/harsher/extreme reporting fields. fileciteturn16file3 fileciteturn16file7 fileciteturn16file1

Adding another generic lane would therefore increase cost in all the places that are already widened:

- candidate/summary cache surfaces,
- broad GPU/CPU lane preparation,
- tuple result contracts,
- robust-score inputs,
- artifact summaries,
- compare/report views,
- and likely frontend explanation burden.

That is too much surface area for a lane that, absent new evidence, would probably be highly correlated with the existing conservative/harsher/extreme picture.

## Deferred ideas

### 1. Realism-lane disagreement work

The strongest not-completely-dead idea is **not** another generic stress family. It is better disagreement surfacing between:

- aggregate,
- derived-mid,
- and quote-aware lanes,

especially when quote-aware data shows that aggregate winners are fragile. But the roadmap already classifies that as realism/validation work inside the current workflow, not as a reason to widen the generic stress-family set. fileciteturn16file4

### 2. Dataset-semantic-specific shock work

If the repo later sees repeated evidence that current lanes miss a distinct, decision-relevant failure mode on quote-aware datasets, a dataset-semantic-specific shock lane could be reconsidered. Even then, it should reopen under **realism-lane / validation-depth** scope, not as a default fifth stress family.

## Rejected ideas

### 1. A fifth generic lane for symmetry

Rejected because the roadmap explicitly says not to widen stress families for symmetry alone. fileciteturn16file4

### 2. Promoting `extreme` into the broad compiled hot path

Rejected because the current design already uses `extreme` as a retained-tuple / sealed-finalist exact lane, which fits the dense-GPU / branchy-CPU split better than another full broad compiled lane. fileciteturn16file3 fileciteturn16file4

### 3. Treating quote-aware spread or liquidity shock as Prompt 7 work

Rejected **as a current stress-family addition**. If it matters later, it should reopen through realism-lane disagreement or embedded-validation work, not by mutating the generic stress-family set.

## Concrete code touch points by file

### No implementation change recommended now

Because the adopted decision is a freeze, there is **no runtime expansion to implement now**.
For the current follow-through tracker, see [options_stress_scope_freeze_implementation_guide_2026-04-10.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_gpt54_pro_prompt_pack_2026-04-08_attachments/prompt_07_stress_family_expansion_constraints/options_stress_scope_freeze_implementation_guide_2026-04-10.md).

### If you want to encode the freeze explicitly in docs

Primary file:

- `options_research/options_expression_future_scope_roadmap_2026-04-07.md` — keep or sharpen the existing policy language that the current lanes are `baseline`, `conservative`, `harsher`, and `extreme`, and that future additions require concrete decision value. fileciteturn16file4

### If the question is reopened later

Primary backend touch points would be:

- `crates/app_core/src/options_expression.rs`
  - `AggregateStressMode`
  - `prepare_options_expression_run`
  - `exactify_ranked_results`
  - `apply_sealed_finalist_stage`
  - `robust_score` fileciteturn16file3
- `crates/app_core/src/options_types.rs`
  - `OptionsExpressionTupleResult`
  - `strongest_validation_stress_metrics`
  - `inferred_validation_verdict`
  - `is_stress_fragile` fileciteturn16file7
- `src-tauri/src/commands_options_expression.rs`
  - `OptionsExpressionArtifactSummary`
  - `summarize_options_expression_artifact` fileciteturn16file1
- `web/src/components/OptionsExpressionPanel.vue`
  - only after backend proof exists; presentation should not drive lane proliferation by itself. fileciteturn16file10
- `crates/app_core/src/options_expression_gpu.rs`
  - only if a reopened lane is promoted into the broad compiled path; that is not justified now. fileciteturn16file5

Repo-routing guidance for this workflow already points contributors first to the roadmap, types, engine, GPU scorer, and Tauri command boundary for options-expression work. fileciteturn16file0 fileciteturn16file10

## Verification order

### For the adopted freeze decision

No runtime verification is required, because this recommendation is **not** to add a new lane.

### If the question is reopened later

Use this verification order:

1. **Backend unit/build checks**
   - `cargo test -q -p ta_app_core options_ --lib`
   - `cargo check -q -p ta_desktop_demo_app`
2. **CUDA build check** if any broad-lane or scorer contract changes occur
   - `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
3. **Options proof harness / feasibility rerun** if planner/scorer behavior changes
   - rerun the relevant feasibility or promotion-gate harness and capture before/after output under `output/options_expression_phase05` or the GPU-promotion wrapper surfaces already used by the current workflow. fileciteturn16file4
4. **Artifact-summary parity review**
   - confirm that tuple results, verdict derivation, and artifact summaries remain coherent across search, holdout, sealed, and stress lanes. fileciteturn16file7 fileciteturn16file1
5. **Frontend validation review** only after backend proof exists
   - make sure compare/reopen surfaces still label validation and stress outputs clearly. fileciteturn16file4

## Reopen criteria

Do **not** reopen Prompt 7 unless **all** of the following become true:

1. **Distinct failure mode**
   - repeated real-data usage shows a failure mode that `baseline` / `conservative` / `harsher` / `extreme` plus current `StressFragile` logic do not already expose. fileciteturn16file7
2. **Decision relevance**
   - the proposed lane materially changes:
     - robust-winner ordering,
     - retained `top_k` composition,
     - sealed-finalist promotion/demotion,
     - or final validation verdicts.
3. **Deterministic semantics**
   - the lane has clear semantics across supported price-semantics / fill-model combinations rather than being an ad hoc provider-specific tweak. fileciteturn16file7
4. **Tensor-economics fit**
   - it fits the current compiled-tensor model or a bounded retained-tuple exact path without disproportionate cache, planner, artifact, or UI sprawl. The roadmap’s standing rule is that scope expands only when tensor regularity survives. fileciteturn16file4
5. **Beats simpler alternatives**
   - it is more useful than improving:
     - validation clarity,
     - robustness-frontier reporting,
     - or aggregate-vs-derived-mid-vs-quote-aware disagreement surfacing. fileciteturn16file4

If those conditions are not met, the correct decision remains:

> **keep the current stress-family surface frozen and spend the next decision-value dollar elsewhere.**
