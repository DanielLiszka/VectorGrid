# CompositeMo Bundle 6 Validated Tracking Checklist

Validated against the current repo on 2026-03-31 and tightened again at implementation start.

Use this file as the authoritative Bundle 6 scope and progress tracker.

Rules:

- Only unchecked `[ ]` items are implementation work for this pass.
- Checked `[x]` items under "Already present" and "Out of scope" are not to be re-implemented.
- If older Bundle 6 docs disagree with this tracker, this tracker wins.
- Mark items complete only after the code and the directly relevant tests/verification for that item land.

## Validation status

- [x] Review `gpt54_feature_review_map.md` Bundle 6 scope against current repo state.
- [x] Review `composite_mo_bundle6_implementation_plan.md`.
- [x] Review `composite_mo_bundle6_addendum.md`.
- [x] Validate current backend/runtime state in `crates/app_core/src/composite_mo.rs`.
- [x] Validate current optimizer support in `crates/optimizer/src/mo.rs`.
- [x] Validate current frontend/product wiring in `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`, and `web/src/utils/strategyOptimizationCapabilities.ts`.
- [x] Remove stale or already-implemented items from required scope.
- [x] Promote this file to the authoritative Bundle 6 tracker.

## Already present in the repo

- [x] `CompositeMo` is product-wired and command-wired.
- [x] Sampler support exists for `Nsga2`, `Random`, `Tpe`, and `Qmc`.
- [x] GPU prescan exists.
- [x] Predictive rescue exists in prescan.
- [x] Multiple prescan MA-pair support exists.
- [x] Phase-based throttled progress reporting exists.
- [x] Generic optimizer grid enumeration exists in `crates/optimizer/src/mo.rs`.
- [x] TPE already dedups suggested points.

## Active implementation checklist

### 1. Tracker and scope hygiene

- [x] Keep this checklist aligned with the final implemented scope and mark completed items `[x]` as work lands.
- [x] Keep optional follow-up work separated from required work.

### 2. Optimizer crate: `crates/optimizer/src/mo.rs`

- [x] Reject duplicate param names in `SearchSpace::validate()` using case-insensitive comparison.
- [x] Expose a public reusable `param_values_hash(...)` helper and switch internal hash users to it.
- [x] Add a bounded exact-grid multi-objective helper using existing `SearchSpace::grid(...)` support and chunked evaluation.
- [x] Dedup repaired values in `random_mo_search(...)`.
- [x] Dedup repaired values in `qmc_mo_search(...)`.
- [x] Dedup repaired values in `nsga2_optimize(...)` initial and offspring evaluation paths.
- [x] Keep actual NSGA-II evaluation accounting honest after dedup.
- [x] Replace the final NSGA-II full `non_dominated_sort(&all_seen)` tail with a cheaper constrained-frontier extraction on deduped evaluated points.
- [x] Reduce TPE model-building payload cloning by ranking from lightweight objective/violation data instead of cloning full generic payloads.
- [x] Count TPE duplicate-avoidance honestly in `MoSearchStats`.
- [x] Add or expand focused optimizer tests for duplicate-name rejection, exact-grid helper correctness, and sampler dedup behavior.

### 3. Backend correctness and contract: `crates/app_core/src/composite_mo.rs`

- [x] Validate `req.search_space` explicitly before execution starts.
- [x] Add a backend allowlist for supported `CompositeMo` search-space param names.
- [x] Reject unsupported param names with a clear backend error.
- [x] Keep rejecting `strategy_trade_on_next_bar` as disallowed on this path.
- [x] Add typed categorical decoders for numeric categorical params.
- [x] Add typed categorical decoders for bool categorical params.
- [x] Switch non-period numeric decode call sites to typed categorical-aware helpers.
- [x] Switch non-period bool decode call sites to typed categorical-aware helpers.
- [x] Keep MA-length categorical decode on the existing period-choice path.
- [x] Keep MA-type categorical decode on the existing categorical-string path.
- [x] Make `CompositeMoResult.total_evaluations` report actual evaluator invocations, not requested sampler budget.
- [x] Add `CompositeMo` telemetry for requested-vs-actual evaluation accounting.
- [x] Add `CompositeMo` telemetry for whether exact-grid auto-switch was used.

### 4. Backend prescan, repair, and bank shaping: `crates/app_core/src/composite_mo.rs`

- [x] Honor optional `gpu_prescan.device_id` end-to-end instead of always resolving the preferred device.
- [x] Record truthful prescan requested device id and resolved device id telemetry.
- [x] Record truthful prescan backend/runtime telemetry from the actual Double-MA prescan runs.
- [x] Stop hardcoding the appended dynamic-strategy artifact backend label to `"Cpu"` when prescan meaningfully changes the route metadata.
- [x] Replace global `allowed_pairs` with type-aware allowed-pair constraints keyed by `(fast_ma_type, slow_ma_type, fast_len, slow_len)`.
- [x] Rebind prescan candidate domains into type-aware allowed-pair data without collapsing distinct MA-type pairs together.
- [x] Add a per-type prescan candidate-pair cap with a clear error when exceeded.
- [x] Add prescan candidate pair-count telemetry.
- [x] Add prescan pair-density telemetry.
- [x] Make repair prescan-aware so repaired MA lengths snap to a valid allowed pair for the selected MA-type pair.
- [x] Add ordered repair for `macz_fast_length < macz_slow_length`.
- [x] Keep repair bounded and deterministic when no prescan map is present.
- [x] Make MA-bank shaping type-aware after prescan so each MA type builds only the periods it actually needs.
- [x] Reduce sparse/non-uniform bank rebuild cost with arithmetic-run batching instead of one-row-at-a-time fallback for every period.

### 5. Backend exact-grid integration and sampler dispatch: `crates/app_core/src/composite_mo.rs`

- [x] Detect tiny fully discrete spaces that are eligible for bounded exact-grid search.
- [x] Auto-switch to the exact-grid helper before stochastic sampler dispatch when the space is eligible.
- [x] Keep the normal sampler dispatch unchanged when exact-grid is not eligible.
- [x] Keep progress reporting truthful for exact-grid runs.
- [x] Keep evaluation accounting truthful for exact-grid runs.

### 6. Frontend request shaping: `web/src/services/optimizeRequestBuilder.ts`

- [x] Reject unsupported custom `CompositeMo` param names instead of silently dropping them.
- [x] Reject duplicate custom `CompositeMo` param names instead of silently dropping them.
- [x] Reject attempts to provide custom `fast_len`/`slow_len` params in the custom search space, since those are builder-owned.
- [x] Stringify numeric categorical choices emitted from `parseMoRangeFloat(...)`.
- [x] Stringify bool categorical choices emitted from `parseMoBoolDomain(...)`.
- [x] Keep categorical OTT MA-type choices string-based.
- [x] Stop forcing `gpu_prescan.device_id = 0` by default; omit it unless an explicit value is available.
- [x] Auto-expand prescan MA-type pairs from optimized `fast_ma_type` / `slow_ma_type` search-space axes when prescan is enabled and explicit pairs were not supplied.
- [x] Fail early when prescan auto-expansion would exceed the backend MA-pair cap.
- [x] Add tiny-space / fully-discrete default budget shaping only when the user did not explicitly supply sampler-budget fields.
- [x] Keep explicit user sampler budgets untouched.
- [x] Remove the redundant `strategy` field from the CompositeMo request payload and keep `base_strategy` as the only strategy config field.
- [x] Keep request shaping aligned with the backend allowlist and categorical decode rules.

### 7. Frontend capability text: `web/src/utils/strategyOptimizationCapabilities.ts`

- [x] Update `CompositeMo` execution text to state that the main evaluator is CPU-side.
- [x] Update `CompositeMo` execution text to state that GPU prescan is optional and only narrows MA domains.
- [x] Update `CompositeMo` capability text to state that the path does not participate in the global backend selector.
- [x] Keep the text aligned with current validation limitations for `CompositeMo`.

### 8. Benchmarks

- [x] Prefer `crates/app_core/benches/composite_mo_opt.rs` for focused CompositeMo search-overhead benchmarks.
- [x] Add or expand a focused `composite_mo_opt.rs` benchmark for tiny exact-grid-eligible spaces only if that hotspot is still not covered well enough.
- [x] Add or expand a focused `composite_mo_opt.rs` benchmark for duplicate-heavy discrete spaces that stress sampler dedup/accounting only if that hotspot is still not covered well enough.
- [x] Add or expand a focused `composite_mo_opt.rs` benchmark for NSGA-II tail-cost reduction only if that hotspot is still not covered well enough.
- [x] Use `crates/app_core/benches/dynamic_strategy_cpu.rs` only for sparse/type-aware MA-bank prep or dynamic-slot-prep hotspots if those are still not covered well enough.

### 9. Rust tests

- [x] Add or expand `CompositeMo` tests for categorical numeric round-trips.
- [x] Add or expand `CompositeMo` tests for categorical bool round-trips.
- [x] Add or expand `CompositeMo` tests for backend unknown-param rejection.
- [x] Add or expand `CompositeMo` tests for actual-vs-requested evaluation accounting.
- [x] Add or expand `CompositeMo` tests for prescan device-id honoring.
- [x] Add or expand `CompositeMo` tests for type-aware prescan candidate narrowing.
- [x] Add or expand `CompositeMo` tests for prescan pair-density / candidate-pair telemetry.
- [x] Add or expand `CompositeMo` tests for pair-aware repair.
- [x] Add or expand `CompositeMo` tests for MACZ fast/slow ordered repair.
- [x] Add or expand `CompositeMo` tests for exact-grid eligibility and exact-grid dispatch behavior.

### 10. Frontend tests

- [x] Add or expand request-builder tests for duplicate custom param rejection.
- [x] Add or expand request-builder tests for unknown custom param rejection.
- [x] Add or expand request-builder tests for numeric categorical stringification.
- [x] Add or expand request-builder tests for bool categorical stringification.
- [x] Add or expand request-builder tests for preserving string-valued `ott_ma_type` categorical choices.
- [x] Add or expand request-builder tests for prescan device-id omission by default.
- [x] Add or expand request-builder tests for MA-pair auto-expansion.
- [x] Add or expand request-builder tests for tiny-space default budget shaping without overriding explicit user budgets.
- [x] Add or expand request-builder tests for removal of the redundant CompositeMo `strategy` payload field.
- [x] Add or expand capability-text tests for the updated `CompositeMo` wording.
- [x] Add or expand optimize-service tests where request auto-building changed shape.

## Verification checklist

- [x] Targeted Rust tests for `ta_optimizer` pass.
- [x] Targeted Rust tests for `ta_app_core` `composite_mo` pass.
- [x] Targeted frontend unit tests for CompositeMo request shaping and capability text pass.
- [x] Any touched benchmark file still compiles.
- [x] CPU perf gate is run once after the implementation settles. Note: `scripts/run_cpu_perf_gate.ps1 -Mode candidate` failed the unrelated `criterion_mean_not_regressed_15pct` check on `double_ma/cpu/sma_sma/200k/small`; see `target/cpu_perf_summary.json`.
- [x] This checklist is updated to reflect the final landed work before handoff.

## Optional valid follow-up only

- [ ] Add backward-compatible sampler stop/cancel support only if it stays small and avoids broad optimizer API redesign.
- [ ] Add `PeriodIndex` or similar bank row-lookup micro-optimization only if profiling shows it is still worthwhile after the core fixes land.

## Explicitly out of scope for this validated pass

- [x] Re-adding existing sampler support or existing prescan features.
- [x] Re-adding existing predictive rescue or multi-MA-pair support.
- [x] Re-adding existing progress phases/throttling.
- [x] Re-adding existing generic optimizer grid support.
- [x] Broad new UI surfaces or user-facing knobs.
- [x] Broad optimizer API redesign.
- [x] New typed discrete optimizer-domain redesign.
- [x] Prescanned pair-index axis redesign.
- [x] Broad perf-gate infrastructure expansion.
