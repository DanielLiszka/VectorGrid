# Options Expression Density Direction — Final

Date: 2026-04-09  
Status: final Prompt 2 consolidation for `options_research/options_expression_density_direction_FINAL.md`

## Executive summary

The repo should **not ship another default search-shape change now**.

The April 8 overlap-quality pass already did the right thing: make overlap and adequacy explicit, keep the aggregate-lane defaults steady, rerun the fixed SPY/QQQ sign-off pair, and confirm that the live `auto` route still stands without retuning. The current evidence still does **not** justify a new default clamp.

The reason is structural. The two bounded provider-backed sign-off roots are weak in **different** ways:

- **SPY warm** is not an overlap problem. It has full dataset overlap, still clears the accelerated backend, but suffers from **holdout starvation**, with invalids dominated by `entry_past_cutoff`.
- **QQQ warm** is not mainly a late-entry or strike-grid problem. It is first a **dataset-overlap / as-of-eligibility** problem, with `weak_dataset_overlap` and `contract_not_eligible_as_of_event` dominating search invalidity.

Those are not the same failure class. A new default search-shape tweak now would mostly be optimizing one symptom while the real cross-root causes remain mixed.

The only follow-up experiment that still deserves life is a **non-default split-assignment ablation** using the already-supported `EventChronological` versus `SessionChronological` modes. That should be treated as a measurement-only lever, not as a shipped default change, and only promoted if it improves split-level executable coverage across the fixed SPY/QQQ pair without gaming density or hiding structural invalids.

## Current density, overlap, and invalid-slot evidence summary

### What the last completed pass already established

The last completed overlap-quality pass was intentionally scoped to improve classification and reporting first, and only consider narrow profile hardening if the benchmark pair proved it. Its sign-off result was to ship the new overlap/adequacy reporting, keep the current aggregate defaults steady, rerun the fixed SPY/QQQ pair cold and warm, and confirm that the live `auto` route still stands without retuning.

That framing still matters, because Prompt 2 should not silently reopen a default clamp that the sign-off pass already declined to adopt.

### SPY warm

SPY warm is a **healthy-overlap / weak-holdout** slice.

Key facts from the warm sign-off artifact:

- `candidate_density = 0.5247`
- `invalid_slot_rate = 0.4753`
- overlap is perfect:
  - `search_overlap_rate = 1.0`
  - `holdout_overlap_rate = 1.0`
  - `sealed_test_overlap_rate = 1.0`
- adequacy verdict is still `holdout_coverage_weak`
- split-level candidate and baseline-outcome coverage:
  - search: `7 / 10 = 70.0%`
  - holdout: `0 / 3 = 0.0%`
  - sealed: `4 / 5 = 80.0%`
- dominant invalid reason is `entry_past_cutoff`
- overall invalid totals:
  - `entry_past_cutoff = 272`
  - `no_strike_in_tolerance = 36`

Axis evidence matters too:

- delay bucket valid rates:
  - `0m = 51.9%`
  - `1m = 51.9%`
  - `5m = 53.7%`
- DTE valid rates:
  - `1DTE = 55.6%`
  - `3DTE = 51.9%`
  - `5DTE = 50.0%`
- moneyness valid rates:
  - `ATM = 61.1%`
  - `OTM1 = 61.1%`
  - `OTM2 = 58.6%`
  - `ITM1 = 29.0%`

Interpretation: SPY does **not** need overlap repair. It needs better fold-level executable coverage, especially in holdout, and its main loss mode is late-entry cutoff rather than a universal strike-grid failure.

### QQQ warm

QQQ warm is a **weak-overlap / contract-eligibility** slice.

Key facts from the warm sign-off artifact:

- `candidate_density = 0.2513`
- `invalid_slot_rate = 0.7487`
- overlap is mixed:
  - `total_overlap_rate = 50.5%`
  - `search_overlap_rate = 23.0%`
  - `holdout_overlap_rate = 100.0%`
  - `sealed_test_overlap_rate = 81.8%`
- adequacy verdict is `weak_dataset_overlap`
- `non_promotable = true`
- total events: `103`
- events inside dataset window: `52`
- search events inside dataset window: `14` out of `61`
- split-level in-window candidate and baseline-outcome coverage:
  - search: `10 / 14 = 71.4%`
  - holdout: `11 / 20 = 55.0%`
  - sealed: `10 / 18 = 55.6%`
- dominant invalid reason is `contract_not_eligible_as_of_event`
- overall invalid totals:
  - `contract_not_eligible_as_of_event = 1692`
  - `entry_past_cutoff = 930`
  - `no_price_row_after_entry = 145`
  - `no_strike_in_tolerance = 9`

Axis evidence matters here too:

- delay bucket valid rates:
  - `0m = 24.6%`
  - `1m = 24.9%`
  - `5m = 25.9%`
- DTE valid rates:
  - `1DTE = 27.2%`
  - `3DTE = 24.8%`
  - `5DTE = 23.5%`
- moneyness valid rates:
  - `ATM = 28.8%`
  - `OTM1 = 26.3%`
  - `OTM2 = 23.5%`
  - `ITM1 = 21.9%`

Interpretation: QQQ is **not** mainly asking for a late-entry clamp. The principal issue is that a large share of search events never meaningfully overlap the provider-backed dataset window, and the dominant invalid class is as-of contract eligibility. Density alone hides that.

### Cross-root conclusion

The sign-off pair is still heterogeneous:

- **SPY**: healthy overlap, weak holdout coverage, dominant `entry_past_cutoff`
- **QQQ**: weak search overlap, dominant `contract_not_eligible_as_of_event`, still enough density to route GPU but not enough to trust the slice as generally healthy

That is why a one-size-fits-all default search-shape change is not justified.

## Final recommendation

### Decision

**Keep the current defaults unchanged. Do not ship another default search-shape change now.**

That means keeping the current default posture intact:

- `delay_minutes = [0, 1, 5]`
- `dte_buckets = [1, 3, 5]`
- `moneyness_buckets = [ATM, OTM1, OTM2, ITM1]`
- `max_entry_lag_minutes = 5`
- `split_assignment_mode = EventChronological`

### Why this is the right decision

#### 1. The evidence does not support a late-entry clamp

The most tempting narrow change would be to tighten `max_entry_lag_minutes` or drop the `5m` delay bucket because `entry_past_cutoff` appears on both roots.

That is not supported by the sign-off data. The `5m` delay bucket is actually the **strongest** delay bucket on both roots:

- SPY: `53.7%` valid at `5m` versus `51.9%` at `0m` and `1m`
- QQQ: `25.9%` valid at `5m` versus `24.6%` and `24.9%`

So a late-entry default clamp is not just unproven; on the current pair it is directionally suspicious.

#### 2. The evidence does not support a new moneyness or DTE default clamp

`ITM1` is clearly weaker than the other moneyness buckets on both roots, especially on SPY where it absorbs all `no_strike_in_tolerance` failures.

But that still does **not** justify a default clamp now, because the repo already has evidence that `no_strike_in_tolerance` can be a **slice-curation** problem rather than a runtime search-grid problem. The earlier SPY lessons showed that an under-centered provider strike band created artificial strike misses, while a near-spot band fixed them without changing the core search model.

That makes a default moneyness clamp too easy to overread as progress.

#### 3. The current pair is still telling two different stories

Prompt 2 has to be hostile to fake progress. Right now:

- SPY says: “coverage inside the dataset window is real, but fold allocation and late-entry realism are starving holdout.”
- QQQ says: “search overlap and contract-reference eligibility are the main issue before you even talk about a better tuple grid.”

When the two roots disagree that sharply, the correct default action is usually to **stop changing defaults** until one lever shows a true cross-root win.

## Explicitly retained current strengths

These should remain unchanged now:

- the current artifact-driven options-expression workflow
- the current bounded structure-family surface
- the current aggregate-lane default grid
- the current `auto` backend policy
- the compiled-tensor GPU hot path
- the current overlap/adequacy reporting posture from the last completed pass
- the current use of provider-backed sign-off roots as the decision envelope
- the current distinction between search-space defaults and provider-slice curation

The repo already has enough instrumentation to see the weakness. The next mistake would be to force the weakness to look cleaner by default rather than preserving truthful diagnostics.

## Explicitly deferred ideas

These are still alive, but only as bounded later work:

### 1. Split-assignment ablation as the one remaining serious experiment

The only experiment still worth running before full closure is:

- `EventChronological` versus `SessionChronological`
- with all other defaults held constant

Why this survives:

- SPY’s main weakness is fold starvation, especially in holdout
- split assignment is already a real owned surface in the code
- session-based grouping can change whether whole sessions end up starving holdout or sealed without pretending the underlying invalids vanished

But it stays **deferred and non-default** until it proves a cross-root win.

### 2. Slice curation fixes when the dataset itself is the problem

When the sign-off evidence shows provider-window or strike-window mismatch, treat that as a **data curation** problem first, not as a default kernel/search-shape problem.

That work belongs in bounded dataset selection and import/download policy, not in the tuple grid.

### 3. Any later lag or profile clamp

A lag clamp, DTE clamp, or moneyness clamp can be reopened only after a one-factor experiment proves it improves split-level executable coverage on the fixed SPY/QQQ pair without simply pruning harder cases.

## Explicitly rejected ideas

These should be treated as the wrong next move from the current evidence:

### 1. Tightening `max_entry_lag_minutes` now
Rejected because the current sign-off pair does not show that doing so improves useful coverage, and the `5m` bucket is actually strongest on both roots.

### 2. Dropping the `5m` delay bucket now
Rejected for the same reason. It would look like a realism-tightening change, but the measured bucket quality does not support it.

### 3. Shipping a new DTE or moneyness default clamp now
Rejected because it risks overfitting SPY-specific no-strike behavior that may actually come from slice curation, while QQQ’s dominant issue is overlap/eligibility.

### 4. Broad tuple-grid or axis redesign
Rejected because the hardening guide explicitly kept that out of scope for this pass, and the current pair does not prove that the existing axis set is the first-order problem.

### 5. Parent-event filtering to make ratios look better
Rejected because it would hide the real overlap problem in QQQ and would turn truthful invalidity into a cleaner but less honest density number.

### 6. Any “fix” that turns provider-slice overlap into a kernel/search default issue
Rejected because `contract_not_eligible_as_of_event` and `weak_dataset_overlap` are telling the truth about the bounded slice.

## Explicit `stop here` criteria

The repo should stop here on default search-shape changes unless a later one-factor ablation satisfies **all** of the following:

1. It improves split-level candidate coverage and split-level baseline-outcome coverage on **both** SPY and QQQ.
2. It does not merely improve `candidate_density` or `invalid_slot_rate` by dropping harder cases.
3. It does not leave QQQ structurally weak for the same overlap / eligibility reasons.
4. It does not require `auto` retuning or planner relaxation.
5. It does not meaningfully regress preview time.
6. It preserves the truthfulness of invalid-reason reporting.

If those conditions are not met, the correct action remains: **no new default search-shape change.**

## Concrete code touch points by file and symbol

Because the recommendation is “no shipped default change,” these are primarily **ownership surfaces** for the one remaining non-default ablation and for any future reopening.

### `crates/app_core/src/options_types.rs`

These are the current default-policy surfaces that should remain unchanged unless a later cross-root ablation proves otherwise:

- `default_delay_minutes()`
- `default_dte_buckets()`
- `default_moneyness_buckets()`
- `default_split_assignment_mode()`
- `OptionsStaticFilterBundle::default()`
- `OptionsSplitAssignmentMode`
- `OptionsExpressionRunRequest`

### `crates/app_core/src/options_expression.rs`

This is the main search-shape and split-policy ownership surface:

- `assign_splits(...)`
- `select_candidate(...)`
- `compile_structural_candidate_cache(...)`
- `split_coverage_counts(...)`
- `prepare_options_expression_run(...)`
- `analyze_options_expression_feasibility_blocking(...)`
- `run_options_expression_blocking(...)`

If the split-assignment ablation is reopened, it should be done here and kept one-factor.

### `src-tauri/src/commands_options_expression.rs`

This is the correct Tauri/reporting boundary for any ablation plumbing or artifact-summary exposure:

- `AnalyzeOptionsExpressionFeasibilityResponse`
- `build_options_expression_replay_inputs(...)`
- `analyze_options_expression_feasibility_blocking_impl(...)`
- `run_options_expression_blocking_impl(...)`
- `summarize_options_expression_artifact(...)`

If any later split-mode comparison is surfaced through artifacts or feasibility responses, it belongs here, not in a new workflow.

### `crates/app_core/src/options_data_store.rs`

This remains the correct ownership surface for dataset-window truth:

- `load_imported_options_dataset_from_csv(...)`
- manifest derivation of `coverage_start`
- manifest derivation of `coverage_end`
- persistence of `contract_reference_as_of_utc`

These fields are part of why QQQ’s weak-overlap / as-of-eligibility verdict should be treated as a slice truth, not a tuple-grid defect.

### `src-tauri/src/commands_options_data.rs`

This remains the correct ownership surface for provider-slice curation:

- `download_polygon_options_dataset_into_store(...)`
- `format_contract_reference_as_of_utc(...)`

If the repo later decides a sign-off slice is mis-bounded or under-centered, fix it here rather than mutating the core search defaults.

### `crates/app_core/src/options_signal_manifest.rs`

This remains the ownership surface for parent-event timing and session identity:

- `build_options_signal_manifest_from_double_like_curves(...)`
- `session_id_from_timestamp(...)`

That matters because split-assignment experiments depend on trustworthy session grouping.

## Ablation plan

The smallest justified plan is **one** one-factor experiment.

### Ablation A — split assignment only

Compare:

- `EventChronological`
- `SessionChronological`

Hold everything else constant:

- same dataset
- same parent artifact
- same delay buckets
- same DTE buckets
- same moneyness buckets
- same lag and filter bundle
- same backend preference
- same sealed finalist count

### Why this is the only ablation that survives now

It directly tests the one weakness that SPY actually shows — fold-level starvation — without pretending that QQQ’s overlap/eligibility problem is solved.

It is also already supported in the backend and backed by tests showing that `SessionChronological` keeps sessions intact.

### What not to ablate yet

Do **not** lead with:

- lag clamps
- dropping the `5m` delay bucket
- DTE clamps
- moneyness clamps
- new tuple axes
- parent-event filtering

Those all have a higher fake-win risk than the split-mode ablation.

## Benchmark and verification plan

Use the same fixed sign-off sequence that the overlap-quality guide already standardized:

1. cold SPY
2. cold QQQ
3. warm SPY
4. warm QQQ

### Compare at minimum

For each run, compare:

- `candidate_density`
- `invalid_slot_rate`
- split overlap rates
- split candidate coverage rates
- split baseline-outcome coverage rates
- dominant invalid reasons overall
- invalid reasons by split
- `best_available_backend`
- `auto_backend_decision`
- `cpu_ranking_us`
- `gpu_ranking_us`
- `gpu_speedup_vs_cpu`
- `replay_build_ms`
- `total_preview_ms`

### Verification order

If the repo actually touches code for the split-mode experiment or associated surfacing, use this order:

1. `cargo test -q -p ta_app_core options_ --lib`
2. `cargo test -q -p ta_desktop_demo_app commands_options_expression --lib`
3. `cargo check -q -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
4. rerun the fixed SPY/QQQ cold/warm sequence
5. only if frontend surfaces change:
   - `npm run typecheck`
   - `npm run test -- optionsExpressionService.test.ts optionsExpressionStore.test.ts OptionsExpressionPanel.test.ts`

Because the recommendation is not to ship a new default, there is no justification for a broader verification blast than that.

## Rejection criteria and stop conditions

Reject the split-assignment experiment — and keep the current defaults — if any of the following happen:

1. It improves density or invalid-slot rate but does **not** improve split-level candidate coverage and baseline-outcome coverage.
2. It helps only SPY while QQQ stays weak for the same overlap/eligibility reasons.
3. It makes SPY holdout look cleaner only by changing which sessions land in which fold, without improving executable opportunity across the pair.
4. It reduces truthfulness by masking provider-slice weakness behind a prettier coverage readout.
5. It regresses preview time meaningfully.
6. It changes backend-selection behavior in a way that would require `auto` retuning.

Reject any later lag or default-profile clamp if any of the following happen:

1. the gain is only in `candidate_density` or `invalid_slot_rate`
2. split-level outcome coverage regresses on either root
3. the improvement depends on dropping hard late-entry cases rather than increasing executable coverage
4. the result is root-specific rather than pair-stable

## Bottom line

Prompt 2 should end conservatively:

- **No new shipped default search-shape change now**
- **Keep the current defaults**
- **Withdraw late-entry/default-tightening as the next move**
- **If the repo insists on one more experiment, run split assignment only, keep it non-default, and promote it only on a cross-root coverage win**

That is the most defensible way to avoid fake progress while preserving the current GPU-first options lane.

## Sources reviewed

- `options_expression_gpt54_pro_prompt_pack_2026-04-08.md`
- `options_expression_overlap_quality_hardening_implementation_guide_2026-04-08.md`
- `spy_warm_phase05_summary.json`
- `qqq_warm_phase05_summary.json`
- `options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_expression_future_scope_roadmap_2026-04-07.md`
- `options_types.rs`
- `options_expression.rs`
- `commands_options_expression.rs`
- `options_data_store.rs`
- `commands_options_data.rs`
- `options_signal_manifest.rs`
