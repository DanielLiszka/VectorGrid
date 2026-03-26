# Double MA Single-Pass Valid Implementation Tracker

Date: 2026-03-24

## Purpose

This document consolidates the valid, additive implementation work from the following review documents:

- `ma_review/double_ma_single_pass_extra_work_evaluation_2026-03-23.md`
- `ma_review/double_ma_single_pass_math_addendum_2026-03-24.md`
- `ma_review/double_ma_single_pass_validation_pullforward_2026-03-24.md`

This tracker is grounded against the current repo state, not the older replay-backed product path. The current Double MA GPU product route is the 17-row single-pass path with exact `omega`, additive downside / side-summary fields, and optimization-stage underwater export, and the review items below are filtered against that reality.

## Current Repo State

- Default Sharpe/Volatility product route: 17-row single-pass GPU path.
- Current benchmark surface remains the contract-split `double_ma_opt` benchmark.
- The current optimization path already exposes the main product metrics from the widened 17-row contract and should remain the default path.
- Existing validation modules remain valid and in scope:
  - walk-forward
  - reopt walk-forward
  - perturb
  - trade bootstrap
  - SPA
  - CSCV / PBO
  - cross-ticker
- Optimization-stage additions must supplement those downstream validation modules, not replace them.

## Validity Filter

Only include items that satisfy all of the following:

1. Additive-only. No removal, narrowing, or weakening of the current single-pass product path, existing metrics, current benchmark variants, or CPU fallback behavior.
2. Compatible with the current shared GPU backend:
   - `crates/app_core/src/gpu_backtest_kernel.rs`
   - `crates/app_core/src/kernels/double_crossover.cu`
   - `crates/app_core/src/double_ma.rs`
3. Prefer exact row-derived or low-state additions before wider-state or approximate additions.
4. No replay / second-pass reintroduction for this workstream.
5. New optimization-stage robustness summaries must not be framed as replacements for the existing validation pipeline.
6. Prefer keeping the default exported contract stable unless a later item explicitly justifies an always-on widening or opt-in auxiliary buffer with measured acceptance evidence.
7. Every implementation phase must preserve or improve current performance on the serialized `double_ma_opt` benchmark surface.

## Consolidated Valid Work

### Phase 1: Immediate Exact Additions On Top Of The Existing Row / Reduction Path

These items are the strongest first-wave implementation targets because they mainly reuse the current row contract, current scoring path, or current device reduction path.

#### 1. Native GPU `Sortino` and `Omega` objective support

Why valid:

- The current backend already computes and decodes the necessary state for these objectives.
- The remaining gap is routing / scorer enablement, not a missing replay pass.

Primary touchpoints:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/double_ma.rs`

#### 2. Expanded device-side hard constraints

Why valid:

- Current device constraints are narrow.
- A broader device-side constraint surface reduces host filtering and keeps shortlist formation device-first.

Tier A first:

- `min_sharpe`
- `min_calmar`
- `max_volatility`
- `min_exposure`
- `max_exposure`
- `max_abs_net_exposure`

Tier B after Tier A:

- `min_omega`
- `max_ulcer`
- `max_downside_deviation`

Primary touchpoints:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

#### 3. Row-derived score heads and composite score families

Why valid:

- The bar walk is already paid for.
- Adding more score heads over the existing row is much cheaper than another backtest pass.

Recommended first-wave row-derived heads:

- `active_mean`
- `active_sortino`
- drawdown chronicity: `ulcer_index / max_drawdown`
- downside-share / downside-asymmetry proxy

Recommended first-wave composite families:

- constrained normalized additive utility
- adversarial / envelope-weight composite
- lower-confidence / shrinkage flavored score head, if kept additive and opt-in

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 4. Exact multi-head top-k and union-shortlist support

Why valid:

- The reduction machinery already exists.
- Multiple exact scalar heads can share the same optimization pass.

Scope:

- maintain exact top-k per scalar head
- build an opt-in union shortlist
- allow an optional host rerank only on the tiny deduplicated union shortlist
- keep host rerank explicitly separate from exact per-head top-k semantics

Primary touchpoints:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/benches/double_ma_opt.rs`

### Phase 2: Exact Low-State Kernel Additions

These items add useful diagnostics or stress proxies while keeping state growth controlled.

#### 5. Underwater duration diagnostics

Why valid:

- The kernel already tracks equity and drawdown state.
- Duration-style drawdown diagnostics are a natural additive extension.

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 6. Entry-margin fragility metrics

Why valid:

- The kernel already has the crossover decision context needed to accumulate margin-style fragility proxies.

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 7. Exact fee-stress / stressed final-PnL transform

Why valid:

- Useful and additive if it is made exact.

Required qualification:

- Do not assume exported `trades` is identical to commission-event count.
- First prove equivalence across all entry / exit / flip / stop / take-profit branches, or add a dedicated internal `cost_event_count`.

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma_validate.rs`

### Phase 3: Exact Robustness Pull-Forward Using Optional Auxiliary State

These items are valid, but they should be implemented only after the row/reduction work above is stable. They need optional auxiliary state or post-pass logic, not a premature widening beyond the current default product export contract.

#### 8. PSR / LCB-style shortlist features aligned with current validation semantics

Why valid:

- Useful as additive shortlist or ranking signals.

Required qualification:

- Current downstream PSR / DSR uses simplified skew / kurtosis assumptions.
- First-wave parity should target the current `validation_stats.rs` semantics.
- Treat richer `m3 / m4` probability metrics as a separate advanced mode, not implicit parity.

Primary touchpoints:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 9. Checkpoint slice robustness

Why valid:

- Half-split / quartile summaries can be accumulated from one pass if checkpoints are used.

Recommended first-wave slice summaries:

- worst-slice metric
- positive-slice fraction
- jackknife-like omission summary

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/double_ma_validate.rs`

#### 10. Volatility-bucket robustness

Why valid:

- Existing validation already computes volatility-slice style robustness downstream.
- Optimization can accumulate low / mid / high volatility bucket summaries with one precomputed dataset state array.

Primary touchpoints:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

#### 11. Matrix-neighborhood robustness / basin / plateau metrics

Why valid:

- Additive optimization-stage surrogate for downstream perturb robustness.

Required qualification:

- This is not "free row algebra."
- It needs either shortlist-surface retention or a dedicated small surface buffer.
- It supplements the current perturb validation module; it does not replace it.

Primary touchpoints:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/validation_pipeline.rs`

### Phase 4: Medium-Risk Exact Extensions

These items remain valid, but they should stay behind the earlier phases because they add more state, more semantics, or more exactness caveats.

#### 12. Hurdle partial moments

Candidate scope:

- `Omega_tau` via the existing hurdle-adjusted `omega` semantics
- `Kappa_tau` on the widened full-row/export path only

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 13. Downside frequency / severity decomposition

Candidate scope:

- `count_neg`
- `sum_neg_abs`
- derived severity / frequency diagnostics

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 14. Side-conditional long / short summaries

Why valid:

- Exact and useful, but more stateful than the first-wave row algebra additions.

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`

#### 15. Richer heatmap aggregates

Candidate scope:

- constrained pass rate
- mean score
- best constrained score
- robustness aggregate per cell

Primary touchpoints:

- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/benches/double_ma_opt.rs`

#### 16. Pareto / lexicographic / epsilon-front exploration on a union shortlist

Why valid:

- Additive ranking mode.

Required qualification:

- Keep the exact scalar-head top-k path intact.
- Introduce this only as an opt-in shortlist or rerank mode.

Primary touchpoints:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`

## Explicit Non-Goals For This Tracker

These ideas may still be useful research directions, but they should not be implemented as part of this optimization-stage tracker:

- full optimization-stage `reopt_walk_forward`
- full optimization-stage trade bootstrap
- full optimization-stage cross-ticker validation
- replacing downstream SPA with an optimization-stage proxy
- replacing downstream CSCV / PBO with an optimization-stage proxy
- reintroducing a generic replay pass for finalist completion
- shrinking or redefining the current default single-pass export contract

## Implementation Order

1. Phase 1: objectives, constraints, row-derived heads, composite scoring, multi-head top-k / union shortlist.
2. Phase 2: underwater duration, entry-margin fragility, cost-event semantics, exact stressed final-PnL.
3. Phase 3: PSR / LCB alignment, checkpoint slice summaries, volatility buckets, basin / plateau metrics.
4. Phase 4: hurdle partial moments, downside frequency / severity, side splits, richer heatmaps, Pareto / lexicographic shortlist modes.

## Rollout And Exposure Rules

- New capabilities must be opt-in unless the implementation is strictly invisible plumbing or a backend bug fix.
- Existing default ranking behavior must remain unchanged unless an explicit future decision changes product defaults.
- Existing request payloads must remain backward-compatible.
- User-facing exposure for new objectives, constraints, or ranking modes must include backend request parsing, Tauri command plumbing where needed, and frontend request-builder / capability updates where the feature is intended to be selectable from the app.
- If a capability is backend-only in the first step, the checklist should still mark the later API / UI exposure separately instead of leaving it implicit.
- Target end state for this tracker: no hidden env-gated capability should remain. By conclusion, each validated feature must be either:
  - always-on default backend work, if the semantics are broadly useful and the measured cost is worth it, or
  - a real request/API/UI opt-in, if the feature changes ranking semantics or adds optional diagnostics.
- Productized opt-ins are acceptable as long as the feature-off path remains effectively flat relative to a hardcoded feature-off implementation. Being selectable must not itself become a meaningful performance tax.

## Performance Acceptance Criteria

- Phase 1 items should target no material regression on the serialized `double_ma_opt` benchmark surface.
- For default-path behavior, treat any sustained regression in `end_to_end_product` or `search_only` as a blocker until explained.
- For opt-in features, benchmark both:
  - feature disabled, to prove no baseline regression
  - feature enabled, to measure marginal cost
- Record benchmark environment, command, and any relevant env vars for every captured result.
- Preserve the meaning of the current benchmark variants; do not let new features collapse `search_only` and `end_to_end_product` into indistinguishable harnesses.

## Phase 1A / 1B Result Log

### Exposure Decisions

- `Sortino` / `Omega` objective support:
  - Phase 1 implementation shape: backend bug-fix / enablement only
  - UI impact: existing objective selectors already exposed both objectives; no new product control was required
- Expanded Double MA constraints:
  - Phase 1 implementation shape: backend + API + UI
  - New app-reachable controls:
    - `min_sharpe`
    - `min_calmar`
    - `max_volatility`
    - `min_exposure`
    - `max_exposure`
    - `max_abs_net_exposure`
    - `min_omega`
    - `max_ulcer`
    - `max_downside_deviation`
- Phase 1D multi-head shortlist union:
  - Phase 1 implementation shape: backend-only exact kernel enhancement
  - Initial exposure: env-gated via `VECTORBT_KERNEL_EXTRA_SHORTLIST_OBJECTIVES`
  - Initial scope limits:
    - exact scalar objective heads only
    - default behavior unchanged when the env var is unset
    - no result-shape change
    - no frontend / Tauri control yet
    - no `include_all`, no `score_mode`, no export path
  - Current validated examples:
    - `VECTORBT_KERNEL_EXTRA_SHORTLIST_OBJECTIVES=pnl,omega`

### Commands And Evidence

- CPU safety / request validation:
  - `CARGO_TARGET_DIR=target/codex_phase1_cpu cargo test -p ta_app_core sanitize_double_ma_constraints_validates_new_fields -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_phase1_cpu cargo test -p ta_app_core metrics_pass_constraints_supports_extended_row_derived_fields -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_phase1_cpu cargo test -p ta_app_core capability_report_allows_omega_for_kernel_capable_request -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_phase1_cpu cargo test -p ta_app_core balanced_risk_adjusted_score_prefers_more_robust_profile -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_phase1_cpu cargo test -p ta_app_core effective_score_uses_score_mode_override -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_phase1c_adv_cpu cargo test -p ta_app_core adversarial -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_phase1c_support_cpu cargo test -p ta_app_core support_shrunk -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_active_sortino_cpu cargo test -p ta_app_core active_sortino -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_active_sortino_cpu cargo test -p ta_app_core derived_ -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_phase1_cpu cargo test -p ta_app_core --no-run`
- CUDA parity / route validation:
  - `CARGO_TARGET_DIR=target/codex_phase1_cuda cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_supports_sortino_and_omega_objectives_for_plain_strategy -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_phase1_cuda cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_topk_v2_matches_forced_legacy -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_phase1_cuda cargo test -p ta_app_core --features cuda,cuda-backtest-kernel search_metrics_count -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_phase1_cuda cargo test -p ta_app_core --features cuda,cuda-backtest-kernel search_metrics_count_promotes_to_full_rows_for_score_mode -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_active_sortino_cuda cargo test -p ta_app_core --features cuda,cuda-backtest-kernel search_metrics_count_promotes_to_full_rows_for_score_mode -- --nocapture`
  - `CARGO_TARGET_DIR=target/codex_active_sortino_cuda cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_export_all_csv_writes_one_row_per_pair -- --nocapture`
  - `$env:CARGO_TARGET_DIR='target/codex_union_shortlist_cuda'; cargo test -p ta_app_core --features cuda,cuda-backtest-kernel extra_shortlist -- --nocapture`
  - `$env:CARGO_TARGET_DIR='target/codex_union_shortlist_cuda'; cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_topk_v2_matches_forced_legacy -- --nocapture`
  - `cargo test -p ta_app_core --lib --features cuda,cuda-backtest-kernel gpu_kernel_exact_fee_stress_pnl_matches_rerun -- --nocapture`
  - `cargo test -p ta_app_core --lib --features cuda,cuda-backtest-kernel gpu_kernel_export_all_csv_writes_one_row_per_pair -- --nocapture`
- Frontend / request-path validation:
  - `npm test -- src/services/optimizeRequestBuilder.test.ts src/services/optimizeService.test.ts`
  - `npm test -- src/services/optimizeRequestBuilder.test.ts src/services/optimizeService.test.ts src/services/analysisService.test.ts`
  - `npm run typecheck` still fails on the pre-existing `String.prototype.replaceAll` lib-target issue in `web/src/services/validationRequestBuilder.ts`; that failure is outside this workstream
- Phase 2 proof / semantics validation:
  - `cargo test -p ta_app_core exact_stressed_pnl_from_components_matches_closed_form -- --nocapture`
  - `cargo test -p ta_app_core exact_fee_stress -- --nocapture`
  - `cargo check -p ta_app_core`
- Phase 2 validation-side low-state diagnostics:
  - `cargo test -p ta_app_core entry_margin_fragility -- --nocapture`
  - `cargo test -p ta_app_core underwater_duration -- --nocapture`
  - `cargo test -p ta_app_core double_ma_validation_populates_entry_margin_and_underwater_for_top_finalists -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel double_ma_validation_gpu_kernel_aux_diagnostics_match_cpu_plain -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_export_all_csv_writes_one_row_per_pair -- --nocapture`
- Phase 3A probability / confidence validation:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel shortlist_confidence -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel rerank_shortlist_by_confidence_matches_manual_probability_order -- --nocapture`
  - `cargo test -p ta_app_core --no-run`
- Serialized benchmark / perf gates:
  - `Get-Process cargo, double_ma_opt, ta_cli -ErrorAction SilentlyContinue`
  - `CARGO_TARGET_DIR=target/codex_phase1_bench cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs --noplot`
  - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
  - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
  - `$env:VECTORBT_KERNEL_EXTRA_SHORTLIST_OBJECTIVES='pnl,omega'; $env:VECTORBT_KERNEL_GLOBAL_TOPK='1'; Remove-Item Env:VECTORBT_KERNEL_FORCE_LEGACY_TOPK -ErrorAction SilentlyContinue; cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs"`

### Benchmark Log

| Phase | Baseline Source | search_only | search_plus_heatmap | search_plus_finalist_recompute | end_to_end_product | Notes |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Pre-Phase-1A/1B reference | `research/MA review/double_ma_validated_implementation_tracking_plan_2026-03-23.md` | `77.23 ms` | `77.24 ms` | `151.14 ms` | `77.13 ms` | Existing promoted 10-row single-pass baseline before the later always-on widened-row rollout |
| Phase 1A / 1B candidate | serialized `double_ma_opt` run on `2026-03-24` | `77.43 ms` | `77.79 ms` | `163.70 ms` | `78.65 ms` | Default-path deltas stayed small; replay-backed diagnostic path moved more and remains non-default |
| Phase 1C candidate | serialized `double_ma_opt` rerun on `2026-03-24` after closing background GPU load | `77.60 ms` | `77.59 ms` | `153.75 ms` | `77.54 ms` | Clean rerun after `score_mode` implementation; default path remains flat, replay-backed diagnostic path remains intentionally slower |
| Phase 1C adversarial-envelope candidate | serialized `double_ma_opt` rerun on `2026-03-24` after adding `adversarial_envelope_10` | `77.41 ms` | `77.51 ms` | `153.56 ms` | `77.51 ms` | No meaningful default-path regression after the second opt-in score family |
| Phase 1C support-shrunk candidate | serialized `double_ma_opt` rerun on `2026-03-24` after adding `support_shrunk_10` | `77.50 ms` | `77.53 ms` | `154.28 ms` | `77.47 ms` | No meaningful default-path regression after the third opt-in score family |
| Phase 1C active-sortino / derived-diagnostics candidate | serialized `double_ma_opt` rerun on `2026-03-24` after adding `active_sortino_10` and additive derived export columns | `77.43 ms` | `77.42 ms` | `154.23 ms` | `77.37 ms` | Derived heads remain row-derived on the host/export side; default path stayed flat and the replay-backed diagnostic path remained isolated |
| Phase 1D shortlist-union candidate (feature off) | serialized `double_ma_opt` rerun on `2026-03-24` after adding env-gated extra-head shortlist support, with feature disabled | `77.33 ms` | `77.32 ms` | `152.30 ms` | `77.32 ms` | Default-path semantics remain unchanged when `VECTORBT_KERNEL_EXTRA_SHORTLIST_OBJECTIVES` is unset |
| Phase 1D shortlist-union candidate (feature on: `pnl,omega`) | serialized `double_ma_opt` rerun on `2026-03-24` with `VECTORBT_KERNEL_EXTRA_SHORTLIST_OBJECTIVES=pnl,omega` | `77.59 ms` | `77.58 ms` | `154.46 ms` | `77.55 ms` | Opt-in union shortlist adds exact extra objective heads with about `0.2-0.5 ms` marginal cost on the default-path variants |
| Phase 1D shortlist-union productization candidate (feature off, request/UI surfaced) | serialized `double_ma_opt` rerun on `2026-03-24` after promoting `extra_shortlist_objectives` to a real advanced request/UI feature, with the default path still feature-off | `77.40 ms` | `77.42 ms` | `156.40 ms` | `77.21 ms` | The default product route stayed flat after request/API/UI exposure, request-owned semantics, fallback guards, and CPU-only compile fixes. |
| Phase 1D shortlist-union productization candidate (feature on via legacy env fallback: `pnl,omega`) | serialized `double_ma_opt` rerun on `2026-03-24` after request/UI productization, with `VECTORBT_KERNEL_EXTRA_SHORTLIST_OBJECTIVES=pnl,omega` and no request field set in the benchmark harness | `78.26 ms` | `77.70 ms` | `158.49 ms` | `78.01 ms` | The opt-in marginal cost remains about `0.3-1.1 ms` on the default-path variants. The benchmark still uses the legacy env knob as a feature-on proxy because the contract-split harness does not yet issue request-driven extra shortlist heads directly. |
| Phase 2 fee-stress export candidate | serialized `double_ma_opt` rerun on `2026-03-24` after adding exact fee-stress export derivation from existing `pnl + trades + effective_commission` | `77.37 ms` | `77.43 ms` | `153.02 ms` | `77.46 ms` | No kernel-row widening; default-path timings remained flat while export / validation surfaces gained `fee_stress_pnl` |
| Phase 2 validation-side entry-margin / underwater candidate | serialized `double_ma_opt` rerun on `2026-03-24` after landing validation-only finalist diagnostics and reverting the regressing kernel-side underwater export attempt | `77.61 ms` | `77.55 ms` | `155.47 ms` | `77.46 ms` | Validation gained exact finalist diagnostics while the default optimization hot path still remained on the earlier 10-row contract with no measurable regression |
| Phase 3A shortlist-confidence candidate (feature off) | serialized `double_ma_opt` rerun on `2026-03-24` after landing env-gated validation-aligned PSR / DSR shortlist rerank support, with feature disabled | `77.36 ms` | `77.43 ms` | `154.91 ms` | `77.37 ms` | Default-off path remained flat on the clean rerun. The current contract-split harness cannot directly benchmark feature-on confidence mode because its shared prewarm intentionally exercises legacy-row variants that the confidence mode rejects. |
| Phase 3A shortlist-confidence productization candidate (feature off, request/UI surfaced) | serialized filtered `double_ma_opt` rerun on `2026-03-24` after promoting `shortlist_confidence_mode` to a real advanced request/UI feature, with the default path still feature-off | `77.20 ms` | `77.26 ms` | `151.57 ms` | `77.24 ms` | Default-path timings stayed flat after request/API/UI exposure, request-owned fallback semantics, targeted Rust/web coverage, and clean CUDA/app compile sweeps. A broader full-suite rerun later hit an unrelated `gpu_hybrid_kernel/frama_sma` benchmark panic after these contract-split rows were already emitted, so this filtered rerun is the accepted Phase 3A benchmark evidence. |
| Phase 3C neighborhood-robustness candidate (feature off) | serialized `double_ma_opt` rerun on `2026-03-24` after landing env-gated additive neighborhood robustness over the existing shortlist-slice diagnostics path, with feature disabled | `77.99 ms` | `77.56 ms` | `160.11 ms` | `78.04 ms` | Default-off path remained effectively flat. Criterion reported a stale `sample.json` parse issue for historical comparison on `end_to_end_product`, but the fresh absolute timing was collected successfully. |
| Phase 4 shortlist-union-mode candidate (feature off) | serialized `double_ma_opt` rerun on `2026-03-24` after adding request-driven `shortlist_union_mode` support and backend parity coverage, with the default path still feature-off | `76.99 ms` | `77.21 ms` | `152.28 ms` | `77.24 ms` | Default-path timings remained flat after union-mode parser/rerank tests, exact exhaustive-host parity, and request/API/UI exposure. |
| Phase 4 shortlist-union-mode candidate (feature on via bench request env: `pnl,omega` + `epsilon_front`) | serialized `double_ma_opt` rerun on `2026-03-24` with `VECTORBT_BENCH_EXTRA_SHORTLIST_OBJECTIVES=pnl,omega` and `VECTORBT_BENCH_SHORTLIST_UNION_MODE=epsilon_front` | `77.09 ms` | `77.47 ms` | `151.56 ms` | `77.57 ms` | The opt-in exact union rerank stays within about `0.1-0.4 ms` of the feature-off default-path variants. The contract-split harness now uses dedicated bench-only env injection so feature-on measurement does not affect unrelated benchmark groups in the same Criterion binary. |
| Phase 4 downside / side-summary candidate | serialized `double_ma_opt` rerun on `2026-03-24` after finishing CPU parity and full-row export validation for `downside_frequency`, `downside_severity`, `long_mean_return`, and `short_mean_return` while still preserving the then-current 10-row default product row | `80.96 ms` | `81.06 ms` | `162.15 ms` | `81.29 ms` | The default-path variants stayed effectively flat relative to the current widened-row baseline. The new fields are now coherent on the widened full-row/export path and the shared CPU plain-strategy evaluators without yet widening the default product contract. |
| Phase 4 richer-heatmap-aggregate candidate | serialized `double_ma_opt` rerun on `2026-03-24` after adding additive per-cell `mean_values` and `sample_counts` on the existing opt-in heatmap branch | `80.93 ms` | `81.16 ms` | `165.66 ms` | `80.57 ms` | `search_only` and `end_to_end_product` stayed flat; the opt-in heatmap branch also stayed within noise. The legacy replay-backed diagnostic row moved upward again but remains non-representative of the product path. |
| Shortlist hidden-env removal candidate | serialized `double_ma_opt` rerun on `2026-03-24` after removing runtime shortlist env fallbacks and keeping benchmark feature injection request-driven only | `80.39 ms` | `80.46 ms` | `160.28 ms` | `80.65 ms` | Default-path timings stayed within noise. The runtime now treats shortlist extras / confidence / slice / neighborhood strictly as request-driven product features; only bench-local env injection remains for feature-on measurement. |
| Phase 4 hurdle-tau productization candidate (feature off, request/UI surfaced) | serialized `double_ma_opt` rerun on `2026-03-24` after fixing CUDA launch/decode alignment for `hurdle_tau` across both the normal and dynamic-preflight GPU routes, with the default path still feature-off | `80.66 ms` | `80.66 ms` | `162.30 ms` | `80.68 ms` | The accepted post-fix contract-split rerun stayed within noise on the default path. This rerun also confirms the benchmark binary exits cleanly again after the dynamic-preflight access-violation fix. |
| Phase 4 hurdle-tau productization candidate (feature on via bench request env: `0.01`) | serialized `double_ma_opt` rerun on `2026-03-24` with `VECTORBT_BENCH_HURDLE_TAU=0.01` | `80.60 ms` | `80.69 ms` | `162.69 ms` | `80.80 ms` | A nonzero hurdle stays within noise on the default-path variants while making Sortino / Omega / downside-family metrics threshold-aware in a single pass. The benchmark uses a dedicated bench-only request env so feature-on measurement does not introduce a hidden runtime toggle. |
| Phase 4 kappa-tau full-row/export candidate | serialized `double_ma_opt` rerun on `2026-03-25` after widening the full-row/export path to 15 stored metrics so `kappa_tau` exports alongside the existing downside / side-summary fields, while keeping the default product contract at 10 rows | `81.450 ms` | `81.497 ms` | `160.55 ms` | `81.533 ms` | Accepted clean rerun. Targeted CPU/GPU/export correctness passed, the contract-split benchmark stayed in the current `~81 ms` band on default-path variants, and the earlier `~200-300 ms` readings were not reproducible on the same benchmark surfaces. |
| Phase 4 kappa-tau post-release-fix rerun | serialized `double_ma_opt` rerun on `2026-03-25` after repairing the release-only dynamic-runner / uploaded-program compile drifts | `197.40 ms` | `202.96 ms` | `172.28 ms` | `81.639 ms` | The main `end_to_end_product` path stayed in the current `~81.6 ms` band, but the legacy-row `search_only` / `search_plus_heatmap` variants regressed sharply on this rerun. Treat that as a narrower search-only / legacy diagnostic issue rather than a default product-path regression. |
| Phase 4 trade-metrics productization candidate (feature off, request/UI surfaced) | serialized `double_ma_opt` rerun on `2026-03-25` after adding request-driven `include_trade_metrics` support plus widened trade-summary export/decode rows, with the default path still feature-off | `81.556 ms` | `81.614 ms` | `162.74 ms` | `81.561 ms` | The default product route stayed in the current `~81.6 ms` band after the trade-metrics rollout. The benchmark harness now has a dedicated bench-only request injector for `include_trade_metrics`, so the normal runtime stays request-driven with no hidden env fallback. |
| Phase 4 trade-metrics productization candidate (feature on via bench request env: `1`) | serialized `double_ma_opt` rerun on `2026-03-25` with `VECTORBT_BENCH_INCLUDE_TRADE_METRICS=1` | `81.365 ms` | `81.566 ms` | `163.74 ms` | `81.706 ms` | The opt-in widened trade-summary row stays within noise on the default-path variants. Feature-on measurement is now explicit and bench-local, and the exact `GpuKernel` request path remains default-off unless the user selects it. |
| Phase 4 widened-default-row + optimization-stage underwater acceptance | serialized `double_ma_opt` rerun on `2026-03-25` after promoting the full product row to the always-on 17-field default route and keeping underwater on that path | `81.986 ms` | `81.917 ms` | `166.67 ms` | `85.657 ms` | Accepted final revisit. `end_to_end_product` regressed by about `4.1 ms` / `4.9%` versus the prior `~81.6 ms` default-path band, which is within the explicitly accepted low-single-digit-to-small-single-digit regression budget, while the widened one-pass route now surfaces downside / side-summary fields plus optimization-stage underwater without a second pass. |

### Gate Results

- `scripts/run_gpu_perf_gate.ps1 -Mode candidate`: PASS
  - Latest clean rerun after accepting the always-on widened 17-row default route plus optimization-stage underwater export: `gpu_criterion_mean_not_regressed_10pct`: `85.300218 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `75.75 ms` on vs `83.25 ms` off
  - Latest clean rerun after the trade-metrics request/UI rollout and enabled-cost benchmark: `gpu_criterion_mean_not_regressed_10pct`: `80.27118 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `74 ms` on vs `75.25 ms` off
  - Latest clean rerun after the `kappa_tau` full-row/export rollout: `gpu_criterion_mean_not_regressed_10pct`: `82.256816 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `74 ms` on vs `80.5 ms` off
  - Latest rerun after repairing the release-only dynamic-runner / uploaded-program compile drifts: `gpu_criterion_mean_not_regressed_10pct`: `81.857497 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `74.75 ms` on vs `78.25 ms` off
  - Earlier Phase 1C pass: `gpu_criterion_mean_not_regressed_10pct`: `79.619534 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `72 ms` on vs `76.75 ms` off
  - Active-sortino / derived-diagnostics rerun: `gpu_criterion_mean_not_regressed_10pct`: `78.806105 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `71 ms` on vs `75 ms` off
  - Phase 1D shortlist-union rerun (feature off): `gpu_criterion_mean_not_regressed_10pct`: `77.732031 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `71.25 ms` on vs `76.75 ms` off
  - Phase 2 fee-stress export rerun: `gpu_criterion_mean_not_regressed_10pct`: `77.868256 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `71.75 ms` on vs `75.75 ms` off
  - Phase 3A shortlist-confidence productization rerun: `gpu_criterion_mean_not_regressed_10pct`: `77.311504 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `72 ms` on vs `75.5 ms` off
  - Phase 2 validation-side entry-margin / underwater rerun: `gpu_criterion_mean_not_regressed_10pct`: `80.065176 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `73 ms` on vs `78.25 ms` off
  - Direct clean benchmark on the same small surface after clearing concurrent benchmark load: `double_ma/gpu_kernel/sma_sma/200k/small/1590pairs`: `79.14 ms`
  - Latest clean Phase 3A rerun: `gpu_criterion_mean_not_regressed_10pct`: `78.625062 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `71 ms` on vs `72.25 ms` off
  - Latest rerun after the Phase 3B optimization-stage shortlist-slice diagnostics export: `gpu_criterion_mean_not_regressed_10pct`: `77.813492 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `164.25 ms` on vs `189.75 ms` off
  - Latest rerun after the Phase 3B optimization-stage benchmark-relative shortlist slice summary: `gpu_criterion_mean_not_regressed_10pct`: `77.605678 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `71 ms` on vs `75 ms` off
  - Latest rerun after the Phase 3C neighborhood-robustness slice: `gpu_criterion_mean_not_regressed_10pct`: `79.595132 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `71.25 ms` on vs `74.25 ms` off
  - Latest rerun after the Phase 1D request/UI productization and fallback-path compile fixes: `gpu_criterion_mean_not_regressed_10pct`: `82.362161 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `72.75 ms` on vs `78.25 ms` off
  - Phase 4 downside / side-summary rerun: `gpu_criterion_mean_not_regressed_10pct`: `82.177678 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `73.75 ms` on vs `77.75 ms` off
  - Phase 4 richer-heatmap-aggregate rerun: `gpu_criterion_mean_not_regressed_10pct`: `79.356705 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `72.25 ms` on vs `80.5 ms` off
  - Shortlist hidden-env removal rerun: `gpu_criterion_mean_not_regressed_10pct`: `79.853469 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `74.25 ms` on vs `76.5 ms` off
  - Phase 4 hurdle-tau rerun after fixing both the normal and dynamic-preflight GPU launch surfaces: `gpu_criterion_mean_not_regressed_10pct`: `80.121738 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct`: `73.5 ms` on vs `79.25 ms` off
  - One earlier `~208-330 ms` small-surface gate result observed during the `kappa_tau` verification pass was not reproducible: the same `double_ma/gpu_kernel/sma_sma/200k/small/1590pairs` benchmark reran at about `80.1 ms`, and the clean gate rerun passed. It is treated as transient contamination / stale-artifact evidence rather than accepted perf data.
- `scripts/run_workflow_gate.ps1 -Mode candidate`: PASS
  - Latest clean rerun after accepting the always-on widened 17-row default route plus optimization-stage underwater export: `runtime_not_regressed_50pct`: `1691 ms` vs baseline `1744 ms`; robustness gates all passed
  - Latest clean rerun after the trade-metrics request/UI rollout and enabled-cost benchmark: `runtime_not_regressed_50pct`: `1711 ms` vs baseline `1744 ms`; robustness gates all passed
  - Latest clean rerun after the `kappa_tau` full-row/export rollout: `runtime_not_regressed_50pct`: `1756 ms` vs baseline `1744 ms`; robustness gates all passed
  - `runtime_not_regressed_50pct`: `1695 ms` vs baseline `1744 ms`
  - robustness gates all passed
  - Latest rerun after the fee-stress export slice: `runtime_not_regressed_50pct`: `1772 ms` vs baseline `1744 ms`; robustness gates all passed
  - Latest rerun after the validation-side entry-margin / underwater slice: `runtime_not_regressed_50pct`: `1715 ms` vs baseline `1744 ms`; robustness gates all passed
  - Latest rerun after the Phase 3A shortlist-confidence slice: `runtime_not_regressed_50pct`: `1721 ms` vs baseline `1744 ms`; robustness gates all passed
  - Latest rerun after the Phase 3B validation-side checkpoint slice: `runtime_not_regressed_50pct`: `1730 ms` vs baseline `1744 ms`; robustness gates all passed
  - Latest rerun after the Phase 3B optimization-stage shortlist-slice diagnostics export: `runtime_not_regressed_50pct`: `1698 ms` vs baseline `1744 ms`; robustness gates all passed
  - Latest rerun after the Phase 3B optimization-stage benchmark-relative shortlist slice summary: `runtime_not_regressed_50pct`: `1683 ms` vs baseline `1744 ms`; robustness gates all passed
  - Latest rerun after the Phase 3B validation-side volatility-state reuse: `runtime_not_regressed_50pct`: `1665 ms` vs baseline `1744 ms`; robustness gates all passed
  - Latest rerun after the Phase 3C neighborhood-robustness slice: `runtime_not_regressed_50pct`: `1688 ms` vs baseline `1744 ms`; robustness gates all passed
  - Phase 3A shortlist-confidence productization rerun: `runtime_not_regressed_50pct`: `1698 ms` vs baseline `1744 ms`; robustness gates all passed
  - Latest rerun after the Phase 1D request/UI productization and fallback-path compile fixes: `runtime_not_regressed_50pct`: `1697 ms` vs baseline `1744 ms`; robustness gates all passed
  - Latest rerun after the Phase 4 shortlist-union-mode request/UI rollout: `runtime_not_regressed_50pct`: `1709 ms` vs baseline `1744 ms`; robustness gates all passed
  - Phase 4 downside / side-summary rerun: `runtime_not_regressed_50pct`: `1768 ms` vs baseline `1744 ms`; robustness gates all passed
  - Phase 4 richer-heatmap-aggregate rerun: `runtime_not_regressed_50pct`: `1749 ms` vs baseline `1744 ms`; robustness gates all passed
  - Shortlist hidden-env removal rerun: `runtime_not_regressed_50pct`: `1763 ms` vs baseline `1744 ms`; robustness gates all passed
  - Phase 4 hurdle-tau rerun after fixing both the normal and dynamic-preflight GPU launch surfaces: `runtime_not_regressed_50pct`: `1745 ms` vs baseline `1744 ms`; robustness gates all passed

### Current Assessment

- Phase 1A objective enablement is complete.
- Phase 1B device-constraint enablement is complete for the currently validated exact constraint set.
- Phase 1C normalized additive composite scoring is complete for the first-wave `balanced_risk_adjusted_10` mode, including backend routing, request parsing, advanced-panel exposure, and analysis-surface score semantics.
- Phase 1C adversarial / envelope-weight composite scoring is complete for the first-wave `adversarial_envelope_10` mode, again as an opt-in score-layer feature over the existing single-pass path.
- Phase 1C effective-support / lower-confidence scoring is complete for a first-wave `support_shrunk_10` mode that uses current-row trade/exposure support as an opt-in shrinkage proxy.
- Phase 1C active-risk score shaping is complete for a first-wave `active_sortino_10` mode that stays additive and opt-in over the existing single-pass path.
- Phase 1C row-derived helper coverage is complete for `active_mean`, `active_sortino`, drawdown chronicity, and downside share, with deterministic tests and additive analysis/export surfacing.
- Phase 1D exact multi-head shortlist support is now productized as an advanced opt-in request/UI feature via `extra_shortlist_objectives`, still default-off and still exact for the current supported path.
- Phase 1D request semantics are now strictly request-owned: the runtime no longer honors hidden shortlist env fallbacks, and the only remaining env injection for union-shortlist measurement lives in the contract-split benchmark harness under dedicated `VECTORBT_BENCH_*` knobs.
- Phase 1D now has explicit guardrails for unsupported shapes instead of silent ignore or backend fallback: it requires the exact resolved `GpuKernel` route, rejects `DynamicDoubleMa`, rejects multi-slice / multi-grid-expanded runs, and preserves the existing objective-only / bounded-top-k / no-export constraints.
- Phase 1D now has request-driven parser / parity coverage across Rust and frontend surfaces: request normalization, store/service forwarding, exhaustive-host-vs-GPU exact union parity, request-vs-env isolation, CPU-only compile, and app compile all pass.
- Full `vue-tsc` still fails on the pre-existing unrelated `validationRequestBuilder.ts` `replaceAll` lib-target issue; the new shortlist-union request/UI plumbing itself is covered by request-builder, optimize-service, and analysis-service tests.
- Phase 2 cost-event semantics are now validated for the currently covered execution paths: CPU and GPU single-pair audits show exported `trades` is sufficient for exact fee-stress transformation across covered plain, trade-on-next-bar, no-flip, and stop/take-risk branches.
- Phase 2 exact stressed final-PnL derivation is now exposed on the optimization-stage export surface as `fee_stress_pnl`, derived from existing `pnl + trades + effective_commission` without widening the kernel row contract.
- Phase 2 exact stressed final-PnL semantics are now validated against CPU reruns, GPU reruns, and the existing validation-stage fee-stress reference path.
- A dedicated internal `cost_event_count` is not required for the currently covered execution paths because the equivalence proof now exists for those branches.
- Phase 2 entry-margin fragility is now implemented as validation-side finalist analytics, derived from the existing `trade_entries + spread_ratio` series already computed in the validation helper path.
- Phase 2 underwater duration remains implemented as validation-side finalist analytics, derived from the existing per-bar step-return / equity path in validation for cached finalist diagnostics.
- The later Phase 4 revisit is now accepted: optimization-stage underwater export rides on the widened always-on 17-row single-pass product path, and the measured `~4.9%` `end_to_end_product` regression was accepted as worthwhile to maximize useful always-on one-pass work.
- Phase 3A validation-aligned shortlist-confidence is now productized as a default-off advanced request/UI feature via `shortlist_confidence_mode`, and the runtime path is request-driven only.
- Phase 3A reuses the current `validation_stats.rs` PSR / DSR semantics directly and remains additive: no kernel-row widening, no replay reintroduction, and no default behavior change when the request field is unset.
- Phase 3A also updates the `double_ma.rs` merge layer so the confidence-ranked shortlist is preserved through the final app-visible result for the supported single expanded GPU-kernel run shape, and the frontend summary now surfaces the selected confidence mode.
- Richer research-only probability families such as `m3 / m4` remain out of the product path for now and are explicitly rejected by the current env parser / tests.
- The current contract-split benchmark harness cannot directly time feature-on shortlist-confidence mode because its shared prewarm intentionally executes legacy-row variants that the confidence mode correctly refuses.
- An earlier failing Phase 3A perf-gate attempt was traced to benchmark-process contamination; the clean rerun passed and remains the accepted evidence for the default-off path.
- Validation-side regime-slice robustness already provides low / mid / high volatility, trend, and range bucket stats for cached top finalists.
- That validation-side regime robustness path now reuses rolling volatility tertiles keyed by shared `start_t` across finalists on the same dataset, eliminating redundant dataset-level volatility precompute without changing result semantics or adding a new optimizer payload.
- Phase 3B now has a first safe validation-only slice: cached top finalists receive half-split and quartile checkpoint robustness summaries plus quartile-omission sensitivity, all derived from the existing finalist-series cache in `double_ma_validate.rs`.
- The Phase 3B checkpoint slice is intentionally kept out of the optimization hot path for now; no kernel-row widening or optimization benchmark rerun was required for this validation-only addition.
- Phase 3B also has a second safe validation-only slice: cached top finalists receive benchmark-relative half / quartile slice summaries and omission sensitivity aligned to the configured SPA benchmark, again without widening the optimization-stage kernel contract.
- The optimization-stage shortlist slice path is now productized as a default-off advanced request/API/UI feature via `shortlist_slice_mode`, with request-owned semantics and no hidden runtime env fallback.
- The same shortlist-slice diagnostics path now also supports additive benchmark-relative half / quartile summaries against an opt-in `flat` or `buy_and_hold` benchmark through the request/API/UI surface, reusing the existing diagnostics payload rather than widening the product contract or adding a separate result surface.
- Phase 3C now has a first additive optimization-stage surface-robustness slice: a default-off advanced request/API/UI neighborhood robustness summary is attached to the existing shortlist-slice diagnostics payload for the returned best candidate, computed from the exact local-stability follow-up oracle rather than a new kernel row or replay path.
- The first-wave Phase 3C neighborhood summary stays additive and default-off: it requires the shortlist-slice diagnostics path, uses explicit request-owned neighborhood radius / step values, preserves all current default behavior when unset, and avoids widening the promoted default contract further.
- The exported Phase 3C neighborhood summary is now validated end to end: request parsing and guard behavior, deterministic summary invariants, backend result exposure, and parity against a direct exact local-stability recomputation for the returned best params all pass.
- Phase 4 exact union-shortlist rerank modes are now implemented as a default-off advanced request/UI feature via `shortlist_union_mode`, supporting `lexicographic`, `pareto`, and `epsilon_front` ordering over the existing exact union shortlist without changing the default scalar-head ranking path.
- Phase 4 union-mode validation is now complete for the current additive scope: parser coverage, deterministic rerank tests, exact exhaustive-host-vs-GPU parity for all three modes, targeted frontend request/UI tests, CUDA/app compile sweeps, serialized feature-off and feature-on benchmark runs, GPU perf gate, and workflow gate all pass.
- The contract-split benchmark harness now has dedicated bench-only request injection for `extra_shortlist_objectives` and `shortlist_union_mode`, so opt-in feature-on measurement can be captured without leaking those advanced request settings into unrelated benchmark groups in the same Criterion binary.
- The optimization-stage shortlist-slice / benchmark / neighborhood diagnostics are now surfaced as default-off advanced request/API/UI features; they still do not widen the current promoted default contract or change default behavior when unset.
- Accepted contract-split default-path evidence remains in the current `~81 ms` band through the clean `2026-03-25` `kappa_tau` and trade-metrics reruns. The earlier `2026-03-25` slowdown was not reproducible on the same benchmark surfaces, and the clean GPU perf-gate reruns now pass again.
- The legacy diagnostic contract-split variants remain slower / noisier on their separate harness; this does not block the current default-path rollout.
- Phase 4 downside frequency / severity and side-conditional long / short mean-return summaries are now complete for the current additive scope: the CUDA full-row/export path, shared CPU plain-strategy evaluators, CSV export, and GPU-vs-CPU parity all agree, and these fields now ride on the always-on widened 17-row default product route.
- Phase 4 richer heatmap aggregates are now complete for the current additive scope: the existing heatmap payload carries backward-compatible optional `mean_values` and `sample_counts` alongside the original max-score `values`, and the CPU/GPU heatmap merge path preserves all three.
- The remaining hidden env-only shortlist capability cleanup is now complete: shortlist extras, confidence, union, slice, slice benchmark, and neighborhood controls are all either always-off-by-default request/UI opt-ins or existing always-on backend work, with no runtime env fallback path left in `double_ma.rs` or `gpu_backtest_kernel.rs`.
- The rollout policy for this tracker is now explicit: hidden env-gates are not an acceptable final state. Additive capabilities must graduate either into always-on backend work or into real request/API/UI opt-ins, and feature-off selectability is acceptable only when it does not impose a meaningful performance tax relative to a hardcoded-off path.
- Phase 4 request-level `hurdle_tau` support is now complete for the current additive scope: Sortino, Omega, downside deviation, downside frequency, and downside severity are threshold-aware in the shared CPU path and the single-pass GPU path; the feature is exposed as a real advanced request/UI opt-in; there is no hidden runtime env fallback; and the normal plus dynamic-preflight GPU launch surfaces now agree on argument order and decode semantics.
- Phase 4 hurdle partial moments are now complete for the currently accepted additive semantics: the existing hurdle-adjusted `omega` surface already covers the `Omega_tau` scope, and `kappa_tau` is now exported coherently from the shared CPU path plus the widened GPU full-row/export path that is now also the default single-pass product route.
- The `kappa_tau` rollout fixed a real export gap in the GPU tile-stream CSV writer and now has explicit CPU/GPU parity, decode/export, and frontend analysis-surface coverage.
- Phase 4 opt-in trade-level metrics are now complete for the current accepted additive scope: the exact `GpuKernel` request path accepts `include_trade_metrics`, the widened 20-field trade-summary row exports `closed_trades`, `winning_trades`, `losing_trades`, `gross_profit`, and `gross_loss`, the frontend advanced panel exposes the feature as a real request/API/UI opt-in, and derived export/analysis metrics such as `win_rate`, `avg_win`, `avg_loss`, `profit_factor`, and `expectancy` are computed from that stored summary without changing the default-off product path.
- Trade-metrics validation is now complete for the scoped exact `GpuKernel` rollout: targeted CPU/GPU parity, decode/export coverage, targeted web request-path coverage, serialized feature-off and feature-on benchmark runs, GPU perf gate, and workflow gate all pass. Current scope note: the request is still intentionally rejected for `DynamicDoubleMa`, so this rollout does not widen unsupported hybrid/dynamic routes.
- The contract-split benchmark harness now also has a dedicated bench-only request injector for `include_trade_metrics`, so enabled-cost measurement is explicit and bench-local instead of relying on a hidden runtime env fallback.
- Phase 1C is now complete for the currently scoped exact row-derived heads and first-wave additive score families.
- Phase 3A is now complete for the currently valid validation-aligned shortlist-confidence slice.
- Optimization-stage benchmark-relative shortlist slice summaries are now complete for the current default-off advanced request/API/UI path.
- Phase 3B is now complete for the currently valid additive scope.
- Phase 3C is now complete for the current first-wave additive neighborhood / plateau robustness slice.
- The Pareto / lexicographic / epsilon-front shortlist-mode slice is now complete.
- The revisit work is now complete: the default single-pass product path is widened to the 17-row contract, optimization-stage underwater export is live on that path, and the acceptance evidence is the clean `2026-03-25` contract-split rerun plus passing GPU/workflow gates. No hidden env-gated runtime capability remains in scope for this tracker.

## Cross-Strategy Support Matrix

The completed Phase 1-5 work is not uniformly shared across every strategy backend in the repo. The matrix below separates:

- broad `include_all` / full-candidate-return support
- the current widened 17-row single-pass Double MA-family metric contract
- the newer advanced score / shortlist / neighborhood features from this tracker

| Strategy group | Current optimization owner | `include_all` / full candidate return | Inherits widened 17-row single-pass metrics | Inherits recent advanced single-pass features | Current caveats |
| --- | --- | --- | --- | --- | --- |
| `DoubleMa`, `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct` | shared `double_ma.rs` optimizer / `gpu_backtest_kernel.rs` exact `GpuKernel` path when selected | Yes | Yes on the exact `GpuKernel` route | Yes on the exact `GpuKernel` route | Advanced shortlist/confidence/slice features still require the exact global-top-k kernel path, so they are intentionally incompatible with `include_all=true` in the current implementation. |
| `CompositeTrend`, `BreakoutMomentum` with ADX off | composite-profile request shaping, then shared Double MA-family `GpuKernel` path | Yes | Yes when they resolve to exact `GpuKernel` | Yes when they resolve to exact `GpuKernel` | Turning ADX on moves these profiles to the hybrid route, which does not currently inherit the full Phase 1-5 feature set. |
| `CompositeTrend`, `BreakoutMomentum` with ADX on; `RegimeHybrid` current ADX-bearing form | Double MA-family request layer, but `GpuHybridKernel` / CPU fallback route policy | Yes | Not guaranteed / not validated as full parity | No, not as a full supported set | These routes are the main “not yet widened” part of the Double MA-family surface. |
| `DynamicDoubleMa` | dynamic builder preview + `double_ma.rs` execution | Yes | Partial base-path alignment only | No, not for the full feature set | `effective_sample_lcb_10`, extra shortlist objectives, shortlist confidence, shortlist union/slice, and `include_trade_metrics` are explicitly rejected for `DynamicDoubleMa` today. |
| `Asr`, `Vcc`, `Vemr`, `Rocb`, `Vcet`, `Kqsr`, `Carm`, `Kvdf`, `Mkcs`, `Cdmc` | strategy-local / `composite_cpu.rs` standalone CPU-unified-GPU routes | Yes | No | No | These strategies already support full candidate return, but they do not inherit the Double MA-family widened metric contract, score modes, or shortlist diagnostics from this tracker. |
| `CompositeCpu` | `composite_cpu.rs` CPU route | Yes | No | No | Separate route family; ranked-result return exists, but not the Double MA-family metric/diagnostic surface. |
| `CompositeMo`, `MlSignal` | separate non-Double-MA workflows | No | No | No | These strategies use dedicated search / research workflows rather than the normal `include_all` + top-k optimization surface. |

### Practical Interpretation

- The ability to return the full candidate matrix is already broader than this tracker; many strategies support `include_all`.
- The widened 17-row single-pass metric contract plus the newer score / shortlist / retained-surface diagnostics are still primarily a Double MA-family capability.
- The best current inheritance zone is the exact resolved Double MA-family `GpuKernel` route.
- The least-widened zones are:
  - `DynamicDoubleMa`
  - `GpuHybridKernel` Double MA-family profiles
  - the standalone `composite_cpu.rs` GPU-capable strategies

### Agreed Additive Widening Scope

The currently agreed widening plan should be interpreted as:

- keep the richer single-pass optimization feature surface scoped to strategies/routes that support the exact `GpuKernel` path
- separately preserve or widen the ability to return the full candidate result matrix across strategies via `include_all`
- define cross-strategy parity as "full native metric return for that route" rather than forcing every strategy to emit the exact Double MA 17-row/20-row metric schema

That means:

- exact `GpuKernel` routes are the right scope for widened single-pass metric/score/shortlist support
- CPU, hybrid, and standalone GPU routes should still expose full native per-candidate results where feasible, even if they do not inherit every Double MA-family advanced ranking/diagnostic feature
- widening `include_all` across strategies is a separate additive plan from widening the Double MA-family single-pass feature set
- this follow-on should not force non-Double-MA strategies to adopt the Double MA result contract when their native strategy-local metric surface is the better fit

## Follow-On Widening Feasibility

If the next goal is to widen this single-pass support across more strategies, the work should be staged rather than treated as one giant migration.

### Recommended widening order

1. Exact Double MA-family `GpuKernel` parity cleanup
   - Goal: make the remaining Double MA-family strategies that already route through the exact kernel path inherit the same widened metric / score / shortlist surface without profile-specific gaps.
   - First targets: `CompositeTrend` / `BreakoutMomentum` on ADX-off exact `GpuKernel` runs, then remaining exact-profile route guards.

2. Dynamic Double MA support
   - Goal: remove the current explicit rejections for advanced score / shortlist / trade-metric features where the dynamic builder can prove an exact compatible `GpuKernel(dynamic_strategy_v1)` route.
   - Main blockers today: explicit request rejections and route-shape assumptions around exact top-k / retained-surface follow-up behavior.

3. Standalone unified-GPU strategy surface parity
   - Goal: extend `Asr` / `Vcc` / `Vemr` / `Rocb` / `Vcet` / `Kqsr` / `Carm` / `Kvdf` / `Mkcs` / `Cdmc` so they expose a similar widened product metric surface and, later, compatible shortlist/score features where the row contract allows it.
   - This is a separate engineering track, because those strategies use dedicated signal-matrix GPU routes in `composite_cpu.rs`, not the shared Double MA kernel contract.

4. Hybrid-route parity
   - Goal: bring `GpuHybridKernel` routes into alignment where economically justified.
   - This should be last, because ADX-bearing/hybrid routes currently violate some of the exact-path assumptions that the newer score / shortlist / retained-surface diagnostics rely on.

### Follow-On Feasibility Notes

- Broadening `include_all` is mostly already solved; that is not the main gap.
- The additive cross-strategy target is therefore not "make every strategy pretend to be Double MA"; it is "preserve/widen full native candidate return everywhere, while widening the richer single-pass feature set only where exact `GpuKernel` assumptions hold."
- Broadening the widened metric contract is feasible for strategies that already have structured GPU decode rows, but it is not “free reuse” outside the Double MA-family shared kernel path.
- Broadening advanced shortlist / confidence / neighborhood features requires:
  - exact top-k support on the route
  - a retained shortlist/local-surface follow-up shape
  - compatible route guards for no-export / no-host-filter / no-`include_all` exact selection
- For the standalone GPU strategies, the realistic first widening target is metric-surface parity, not immediate parity on every advanced ranking/diagnostic feature.

### Suggested Next Tracker

If this follow-on is pursued, create a separate cross-strategy tracker rather than reopening the closed Phase 1-5 checklist in this document.

## Detailed Checklist

### Review Hygiene

- [x] Confirm the current benchmark baseline in `crates/app_core/benches/double_ma_opt.rs` before Phase 1 starts.
- [x] Capture the current default-path timings for:
  - `search_only`
  - `search_plus_heatmap`
  - `search_plus_finalist_recompute`
  - `end_to_end_product`
- [x] Keep all benchmark runs serialized and record the command / environment used for each captured number.
- [x] Create a tracker table or linked result log for:
  - feature / phase
  - baseline timing
  - candidate timing
  - delta
  - parity status
  - notes / env vars

### API / Exposure Planning

- [x] For every new user-selectable objective, constraint, or ranking mode, decide whether Phase 1 is:
  - backend-only
  - backend + API
  - backend + API + UI
  Files: `crates/app_core/src/double_ma.rs`, `src-tauri/src/commands_optimize.rs`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/utils/strategyOptimizationCapabilities.ts`
  Validation: explicit rollout decision recorded in this tracker.

- [x] Add request / config model tasks for new objectives or ranking modes that must be user-selectable.
  Files: `crates/app_core/src/double_ma.rs`, `src-tauri/src/commands_optimize.rs`
  Validation: request parsing tests and backward-compatibility tests.

- [x] Add frontend request-builder and capability-surface tasks where user-facing exposure is intended.
  Files: `web/src/services/optimizeRequestBuilder.ts`, `web/src/utils/strategyOptimizationCapabilities.ts`, relevant frontend tests
  Validation: frontend request-builder tests and capability tests.

- [x] Add documentation / product-surface notes for any newly exposed optimization controls.

- [x] Remove any remaining env-only hidden capabilities before closing this tracker.
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, relevant frontend request / UI files
  Validation: each surviving advanced capability is either always-on default work or exposed as a real request/API/UI option, with feature-off perf rechecked where applicable.
  Files: relevant research / product docs as needed
  Validation: docs updated when exposure is complete.

### Phase 1A: Native Objective Support

- [x] Remove the remaining GPU route blockers for `Sortino`.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `src-tauri/src/commands_optimize.rs`, optional frontend exposure files
  Validation: CPU/GPU parity tests for objective selection and ranking order.

- [x] Remove the remaining GPU route blockers for `Omega`.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `src-tauri/src/commands_optimize.rs`, optional frontend exposure files
  Validation: CPU/GPU parity tests for objective selection and ranking order.

- [x] Extend the device objective scorer so `Sortino` and `Omega` use the current single-pass row semantics directly.
  Files: `crates/app_core/src/kernels/double_crossover.cu`
  Validation: targeted scorer tests and end-to-end optimization parity on a fixed fixture.

### Phase 1B: Expanded Device Constraints

- [x] Extend `DoubleMaConstraints` with first-wave exact constraints:
  - `min_sharpe`
  - `min_calmar`
  - `max_volatility`
  - `min_exposure`
  - `max_exposure`
  - `max_abs_net_exposure`
  Files: `crates/app_core/src/double_ma.rs`
  Validation: request decode / defaulting tests and CPU-only compile safety.

- [x] Extend device-side constraint evaluation for the first-wave exact constraints.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`
  Validation: device-filter vs host-filter parity on fixed fixtures.

- [x] Add second-wave 10-row risk constraints:
  - `min_omega`
  - `max_ulcer`
  - `max_downside_deviation`
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`
  Validation: constrained top-k parity and no-regression benchmark rerun.

- [x] Ensure unsupported constraint combinations no longer force unnecessary host filtering when a device implementation exists.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: route-policy tests and log / instrumentation checks.

### Phase 1C: Row-Derived Heads And Composite Scoring

- [x] Add row-derived helpers for `active_mean`.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: compare against CPU-derived reference from the same candidate result set.

- [x] Add row-derived helpers for `active_sortino`.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: compare against CPU-derived reference with explicit exposure semantics checks.

- [x] Add row-derived helper for drawdown chronicity (`ulcer / max_drawdown`).
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: deterministic decode / derived-metric tests.

- [x] Add downside-share / downside-asymmetry style row-derived diagnostics.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: deterministic decode / derived-metric tests.

- [x] Add one normalized additive composite score mode over existing row fields.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/optimizer/src/stream.rs`, `web/src/stores/advancedStore.ts`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`, `web/src/services/analysisService.ts`, `web/src/components/AdvancedWorkspacePanel.vue`
  Validation: CPU/GPU parity on composite ranking order for a fixed fixture, request/UI reachability tests, and no-regression benchmark rerun.

- [x] Add one adversarial / envelope-weight composite score mode over existing row fields.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/optimizer/src/stream.rs`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`, `web/src/services/analysisService.ts`, `web/src/components/AdvancedWorkspacePanel.vue`
  Validation: deterministic score-order tests, request/UI reachability tests, and no-regression benchmark check.

- [x] Add one effective-support / lower-confidence opt-in score mode that penalizes low support without changing the default ranking modes.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/optimizer/src/stream.rs`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`, `web/src/services/analysisService.ts`, `web/src/components/AdvancedWorkspacePanel.vue`
  Validation: deterministic score-order tests, explicit opt-in behavior tests, and benchmark check.

- [x] Add one active-risk / exposure-aware opt-in score mode centered on active Sortino without changing the default ranking modes.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`, `web/src/services/analysisService.ts`, `web/src/components/AdvancedWorkspacePanel.vue`
  Validation: deterministic score-order tests, request/UI reachability tests, additive analysis/export coverage, and no-regression benchmark check.

### Phase 1D: Multi-Head Reduction / Shortlist Union

- [x] Add exact per-head top-k support for multiple scalar heads in one pass.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`
  Validation: each head exactly matches standalone per-head ranking on the same fixture.

- [x] Add opt-in union-shortlist materialization and deduplication.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: union membership tests and deterministic deduplication tests.

- [x] Add optional host rerank over the tiny union shortlist only.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: explicit tests that host rerank is opt-in and does not alter exact per-head top-k semantics.

- [x] Preserve current benchmark meaning after multi-head support:
  - `search_only` remains a first-pass benchmark
  - `end_to_end_product` remains the actual default product route
  Files: `crates/app_core/benches/double_ma_opt.rs`
  Validation: benchmark harness review and rerun.

- [x] Promote the exact union-shortlist feature from env-only to a real advanced request/API surface via `extra_shortlist_objectives`.
  Files: `crates/app_core/src/double_ma.rs`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`
  Validation: request decoding tests, request-driven exhaustive-host-vs-GPU parity, and no-regression benchmark rerun.

- [x] Add a default-off advanced UI control for `extra_shortlist_objectives`.
  Files: `web/src/stores/advancedStore.ts`, `web/src/components/AdvancedWorkspacePanel.vue`, `web/src/services/optimizeService.test.ts`
  Validation: store/service forwarding tests and manual guard-text review.

- [x] Make explicit request semantics request-owned and reject unsupported fallback / multi-slice shapes instead of silently ignoring the field.
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: request-vs-env isolation tests, unsupported-backend guard tests, CPU-only compile, app compile, GPU perf gate, and workflow gate.

### Phase 2: Low-State Exact Diagnostics

- [x] Add validation-side underwater-duration diagnostics for top finalists using the existing per-bar step-return / equity reconstruction.
  Files: `crates/app_core/src/double_ma_validate.rs`
  Validation: synthetic underwater-path tests, CPU/GPU validation parity, workflow gate, and serialized `double_ma_opt` rerun confirming no default-path regression.

- [x] Add validation-side entry-margin fragility diagnostics for top finalists using the existing `trade_entries + spread_ratio` series.
  Files: `crates/app_core/src/double_ma_validate.rs`
  Validation: synthetic fixture tests, finalist-population tests, CPU/GPU validation parity, and workflow gate.

- [x] Reevaluate direct optimization-stage underwater export after the later widened-default-row evidence clarified the real always-on cost.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/app_core/src/double_ma_validate.rs`
  Validation: targeted CPU/GPU parity for `underwater_fraction` / `max_underwater_duration_bars`, decode/export coverage, serialized `double_ma_opt` rerun, GPU perf gate, and workflow gate. Accepted outcome: optimization-stage underwater now rides on the widened always-on 17-row path with about `4.9%` `end_to_end_product` cost versus the prior `~81.6 ms` band.

- [x] Audit whether exported `trades` is identical to commission-event count for every trade path.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: targeted CPU/GPU path tests covering entry, exit, flip, stop-loss / take-profit, trade-on-next-bar, and Kaufman-stop branches.

- [x] If `trades != commission_event_count`, add a dedicated internal `cost_event_count`.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Resolution: not required for the current execution paths because the equivalence proof now exists.

- [x] Add exact stressed final-PnL derivation using the proven event-count semantics.
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma_validate.rs`
  Validation: exact helper parity against CPU reruns, GPU reruns, validation friction-stress replay, and current export / validation surfacing.

### Phase 3A: Probability / Confidence Features

- [x] Implement validation-aligned PSR / DSR shortlist-confidence features using semantics aligned with current `validation_stats.rs`.
  Files: `crates/app_core/src/validation_stats.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`
  Validation: exact helper parity tests, env-parser coverage, merge-layer preservation, exhaustive-host-vs-GPU shortlist rerank parity for both `psr` and `dsr`, clean GPU perf gate, and workflow gate.

- [x] Promote shortlist-confidence from env-only to a real advanced request/API surface via `shortlist_confidence_mode`.
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`
  Validation: request-owned fallback test coverage, CPU / CUDA `--no-run` compiles, request-driven exhaustive-host-vs-GPU parity, targeted web request-builder / service tests, clean GPU perf gate, and workflow gate.

- [x] Add a default-off advanced UI control for `shortlist_confidence_mode`.
  Files: `web/src/stores/advancedStore.ts`, `web/src/components/AdvancedWorkspacePanel.vue`, `web/src/services/analysisService.ts`
  Validation: panel visibility / stale-value tests, analysis summary surfacing tests, and no change to the default request when the control stays unset.

- [x] Keep richer `m3 / m4` probability metrics behind a separate research or advanced-mode gate.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/validation_stats.rs`
  Validation: explicit parser / mode-separation tests reject research-only tokens from the current product path.

### Phase 3B: Slice / Bucket Robustness

- [x] Add validation-side half-split and quartile checkpoint robustness for cached top finalists.
  Files: `crates/app_core/src/double_ma_validate.rs`
  Validation: synthetic slice-summary tests, finalist-population tests, GPU-vs-CPU validation aux parity, and workflow gate.

- [x] Add validation-side benchmark-relative slice summaries for cached top finalists.
  Files: `crates/app_core/src/double_ma_validate.rs`
  Validation: synthetic benchmark-slice tests, finalist-population tests, GPU-vs-CPU validation aux parity, and workflow gate.

- [x] Add optional half-split checkpoint snapshots for the default-off advanced optimization-stage shortlist slice path.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/app_core/src/double_ma_result.rs`, `crates/app_core/src/composite_cpu.rs`
  Validation: exhaustive-host-vs-GPU shortlist-slice parity, exported diagnostics assertions, CPU-only compile, GPU perf gate, and workflow gate.

- [x] Add optional quartile checkpoint snapshots for the default-off advanced optimization-stage shortlist slice path.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/app_core/src/double_ma_result.rs`, `crates/app_core/src/composite_cpu.rs`
  Validation: exhaustive-host-vs-GPU shortlist-slice parity, exported diagnostics assertions, CPU-only compile, GPU perf gate, and workflow gate.

- [x] Derive worst-slice, positive-slice fraction, and jackknife-like omission summaries from checkpoint state and export them with the default-off advanced shortlist slice diagnostics payload.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/app_core/src/double_ma_result.rs`, `crates/app_core/src/composite_cpu.rs`
  Validation: exhaustive-host-vs-GPU shortlist-slice parity, exported diagnostics assertions, CPU-only compile, GPU perf gate, and workflow gate.

- [x] Add dataset-wide volatility bucket precompute and accumulate low / mid / high volatility candidate stats.
  Files: `crates/app_core/src/double_ma_validate.rs`
  Validation: direct reuse-vs-direct regime-slice robustness test, existing finalist-population tests for low / mid / high volatility stats, and workflow gate.

- [x] Add opt-in benchmark-relative slice summaries such as beat-rate and worst-slice excess return, explicitly as additive optimization-stage summaries and not as replacements for downstream SPA.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`
  Validation: benchmark-env parser test, deterministic benchmark-slice summary test, exported diagnostics assertions, exhaustive shortlist-slice parity rerun, CPU-only compile, GPU perf gate, and workflow gate.

### Phase 3C: Surface Robustness

- [x] Define a first-wave basin / plateau robustness metric that can be computed from a retained shortlist surface or small auxiliary buffer.
  Files: `crates/app_core/src/double_ma.rs`
  Validation: deterministic neighborhood tests, request parser coverage, and explicit guard tests for the required shortlist-slice diagnostic mode.

- [x] Implement the first-wave basin / plateau metric as an additive optimization-stage robustness feature.
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/double_ma_result.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Validation: backend exposure parity against a direct exact local-stability recomputation, existing shortlist-slice parity rerun, serialized `double_ma_opt` rerun, GPU perf gate, workflow gate, and explicit separation from downstream `perturb` validation.

### Phase 4: Medium-Risk Exact Extensions

- [x] Add request-level `hurdle_tau` support for the existing Sortino / Omega / downside-family metrics without adding another pass.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/strategies/src/double_ma/mod.rs`, `crates/app_core/benches/double_ma_opt.rs`, `web/src/stores/advancedStore.ts`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`, `web/src/services/analysisService.ts`, `web/src/components/AdvancedWorkspacePanel.vue`
  Validation: targeted CPU/GPU parity for Sortino / Omega, dynamic-strategy GPU-path parity, decode coverage, targeted frontend request/UI tests, serialized `double_ma_opt` reruns with feature off and `VECTORBT_BENCH_HURDLE_TAU=0.01`, GPU perf gate, and workflow gate. Scope note: this is the currently shipped threshold parameter for the existing partial-moment family, not the still-open richer `Omega_tau` / `Kappa_tau` metric expansion.

- [x] Keep hurdle-adjusted `omega` as the current additive `Omega_tau` surface instead of introducing a second redundant ratio field.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/strategies/src/double_ma/mod.rs`, `crates/app_core/src/double_ma.rs`
  Validation: targeted CPU/GPU parity for Sortino / Omega under nonzero `hurdle_tau`, request/UI exposure tests, and existing threshold-aware decode coverage.

- [x] Add `kappa_tau` on the widened full-row/export path and later accept that widened path as the default single-pass product contract.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/strategies/src/double_ma/mod.rs`, `crates/app_core/src/double_ma.rs`, `web/src/services/analysisService.ts`
  Validation: targeted CPU/GPU parity, decode/export coverage, frontend analysis-service coverage, serialized `double_ma_opt` rerun, clean GPU perf gate rerun, and workflow gate rerun.

- [x] Add downside frequency / severity state:
  - `count_neg`
  - `sum_neg_abs`
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/strategies/src/double_ma/mod.rs`, `crates/app_core/src/double_ma.rs`
  Validation: strategies-crate rich-metric assertions, GPU-vs-CPU plain-strategy parity, serialized `double_ma_opt` rerun, GPU perf gate, and workflow gate. Scope note: the default product row stays 10-wide; the new fields are exposed on the widened full-row/export path and the shared CPU plain-strategy evaluators.

- [x] Add side-conditional long / short summary state.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/strategies/src/double_ma/mod.rs`, `crates/app_core/src/double_ma.rs`
  Validation: strategies-crate rich-metric assertions, GPU-vs-CPU plain-strategy parity, serialized `double_ma_opt` rerun, GPU perf gate, and workflow gate. Scope note: the default product row stays 10-wide; the new fields are exposed on the widened full-row/export path and the shared CPU plain-strategy evaluators.

- [x] Add richer heatmap aggregates only after the earlier phases are stable.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/optimizer/src/lib.rs`, `crates/optimizer/src/stream.rs`, `crates/app_core/src/double_ma.rs`, `crates/app_core/benches/double_ma_opt.rs`
  Validation: optimizer heatmap unit tests, GPU heatmap exposure test, serialized `double_ma_opt` rerun, GPU perf gate, and workflow gate. Current scope: additive per-cell `mean_values` and `sample_counts` alongside the existing max-score heatmap.

- [x] Investigate and resolve the transient default-path GPU benchmark regression observed during the first `kappa_tau` verification attempt.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/benches/double_ma_opt.rs`
  Resolution: the earlier `~208-330 ms` readings were not reproducible on the same benchmark surfaces; direct reruns of `double_ma/gpu_kernel/sma_sma/200k/small/1590pairs`, the serialized contract-split benchmark, and a clean GPU perf gate rerun all returned to the expected `~80-82 ms` band.

- [x] Promote the widened full 17-row metric contract to the default product path.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/strategies/src/double_ma/mod.rs`
  Validation: targeted CPU/GPU parity for the widened default path, serialized `double_ma_opt` rerun with `end_to_end_product` as the primary KPI, GPU perf gate, and workflow gate. Accepted outcome: `end_to_end_product` moved to `85.657 ms`, about `4.9%` above the prior `~81.6 ms` band, which is within the accepted budget for always-on one-pass useful work.

- [x] Accept optimization-stage underwater export on the default widened product path.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma_validate.rs`, `crates/app_core/src/double_ma.rs`, `crates/strategies/src/double_ma/mod.rs`
  Validation: targeted CPU/GPU parity for optimization-stage underwater metrics, decode/export coverage, serialized `double_ma_opt` rerun with `end_to_end_product` as the primary KPI, GPU perf gate, and workflow gate. Accepted outcome: underwater now ships on the normal one-pass route with the same accepted `~4.9%` `end_to_end_product` cost envelope as the widened default-row rollout.

- [x] Add opt-in trade-level metrics only after the earlier phases are stable and only if the benchmark budget remains intact.
  Files: `crates/app_core/src/kernels/double_crossover.cu`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/strategies/src/double_ma/mod.rs`, `web/src/stores/advancedStore.ts`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`, `web/src/components/AdvancedWorkspacePanel.vue`, `web/src/services/analysisService.ts`, `crates/app_core/benches/double_ma_opt.rs`
  Validation: targeted CPU/GPU trade-summary parity, decode/export coverage including `export_all_csv`, targeted web request/service tests, serialized `double_ma_opt` reruns with feature off and `VECTORBT_BENCH_INCLUDE_TRADE_METRICS=1`, GPU perf gate, and workflow gate. Scope note: current support is the exact `GpuKernel` route only; `DynamicDoubleMa` still rejects the request explicitly.

- [x] Add opt-in Pareto / lexicographic / epsilon-front union-shortlist modes.
  Files: `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/double_ma.rs`, `crates/app_core/benches/double_ma_opt.rs`, `web/src/stores/advancedStore.ts`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`, `web/src/services/analysisService.ts`, `web/src/components/AdvancedWorkspacePanel.vue`
  Validation: union-mode env/request parser tests, deterministic lexicographic / Pareto / epsilon-front rerank tests, exact exhaustive-host-vs-GPU parity for all three modes, targeted web request/UI tests, CUDA/app `--no-run` compiles, serialized `double_ma_opt` reruns with feature off and feature on, GPU perf gate, and workflow gate.

### Phase 5: Residual Valid Review-Doc Backlog

These items remained valid in the source review documents but were not part of the main completed rollout above. They should be treated as second-wave additions and held to the same no-severe-regression standard, with `end_to_end_product` as the primary KPI.

#### Phase 5 execution order

Implement in this order unless a later dependency changes the sequence:

1. Cheapest row-/host-derived additions first:
   - `min_sortino`
   - `active_downside_deviation`
   - `long_exposure`
   - `short_exposure`
   - `directional_bias`
2. Exact shortlist-local ranking modes next:
   - weighted-rank union-shortlist rerank
   - `omega_vs_sharpe` / hurdle-slack score families
3. Observation-count or retained-surface-dependent features after the low-risk wins are stable:
   - `n_obs`-aware shrinkage / lower-confidence scoring
   - curvature / roughness / basin-min summaries
   - multi-score surface-agreement diagnostics
4. Lowest-priority explanation layers last:
   - dominance-count / hypervolume-style shortlist explanation metrics

#### Phase 5 rollout policy

- `min_sortino`: backend + API first; UI only if product wants the constraint surfaced.
- Derived vocabulary (`active_downside_deviation`, `long_exposure`, `short_exposure`, `directional_bias`): always-on derived/backend-analysis additions if they remain effectively free.
- Weighted-rank rerank: advanced opt-in request/API/UI feature, not always-on.
- `n_obs`-aware shrinkage / lower-confidence scoring: backend-first; expose only after exact `n_obs` semantics are proven for the supported GPU route.
- `omega_vs_sharpe` / hurdle-slack score families: advanced opt-in request/API/UI score modes, not always-on.
- Curvature / roughness / basin-min summaries: default-off diagnostics unless they are later promoted into a score mode.
- Multi-score surface-agreement diagnostics: default-off diagnostics / analysis surface.
- Dominance-count / hypervolume-style explanation metrics: diagnostics only; do not let them alter default ranking semantics in first rollout.

#### Phase 5 performance acceptance

- Feature-off path: no measurable regression beyond normal benchmark noise.
- Cheap row-/host-derived additions: target `<= 2%` `end_to_end_product` regression if they are promoted to always-on.
- Shortlist-local opt-in ranking/diagnostic features: target `<= 5%` feature-on regression on `end_to_end_product`.
- Surface-dependent or observation-count-dependent features: target `<= 8%` feature-on regression unless the product value is clearly higher and explicitly accepted.
- Reject or redesign any Phase 5 feature that causes large structural regressions, unexpected host-filter fallback, or replay/pass reintroduction.

#### Phase 5 explicit non-goals

These remain out of scope for this backlog unless a future document reopens them:

- full bootstrap pull-forward
- cross-ticker pull-forward
- full CSCV / PBO pull-forward
- full-grid Pareto extraction as the primary device ranking architecture
- research-only `m3 / m4` probability families on the main product path
- exact active Sharpe / active volatility without a proven `n_obs` surface
- any replay-backed redesign of the current one-pass product route

- [x] Add `min_sortino` as a device-side hard constraint on the current single-pass row state.
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/app_core/src/kernels/double_crossover.cu`, `web/src/stores/strategyStore.ts`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`, `web/src/components/StrategyWorkspacePanel.vue`
  Rollout: completed as backend + request/UI support; the Double MA constraints panel now exposes an explicit Sortino floor control.
  Validation: `metrics_pass_constraints_supports_extended_row_derived_fields`, `sanitize_double_ma_constraints_validates_new_fields`, `search_metrics_count_tracks_objective_and_constraints`, `optimizeRequestBuilder.test.ts`, `optimizeService.test.ts`, and the serialized `double_ma_opt` contract-split benchmark were rerun successfully on 2026-03-25 after crash recovery.

- [x] Add the remaining zero-new-state derived vocabulary from the review docs:
  - `active_downside_deviation`
  - `long_exposure`
  - `short_exposure`
  - `directional_bias`
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, `web/src/services/analysisService.ts`, `web/src/services/analysisService.test.ts`
  Rollout: completed as always-on derived/export/analysis additions without widening the kernel row; surfaced in export CSV plus derived analysis columns/summary surfaces.
  Validation: `derived_active_downside_deviation_scales_by_sqrt_exposure`, `derived_side_exposures_split_exposure_and_directional_bias`, `gpu_kernel_export_all_csv_writes_one_row_per_pair`, `analysisService.test.ts`, `optimizeRequestBuilder.test.ts`, `optimizeService.test.ts`, and the serialized `double_ma_opt` contract-split benchmark were rerun successfully on 2026-03-25 after crash recovery.

  Recorded benchmark evidence after this Phase 5 slice:
  - `search_only`: `81.779 ms`
  - `search_plus_heatmap`: `81.825 ms`
  - `search_plus_finalist_recompute`: `165.478 ms`
  - `end_to_end_product`: `85.626 ms`
  - `profile_gpu_dynamic_runner` steady-state wall time (serial rerun): `79.000 ms` with dynamic preflight off vs `81.750 ms` with dynamic preflight on (`1.035x`, within the historical `20%` budget)
  - Note: the legacy `run_gpu_perf_gate.ps1` Criterion surface (`double_ma/gpu_kernel/sma_sma/200k/small`) is older than the current Phase 5 contract-split workstream surface; after the crash, I treated the direct contract-split rerun plus the serial dynamic-runner ratio as the authoritative verification evidence for this slice.

- [x] Add a weighted-rank rerank mode over the exact union shortlist as a real advanced request/API/UI option.
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, `web/src/stores/advancedStore.ts`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`, `web/src/services/analysisService.ts`, `web/src/components/AdvancedWorkspacePanel.vue`
  Rollout: completed as an advanced opt-in union mode; it is wired through the request schema, advanced UI selector, backend parsing, backend rerank implementation, and analysis-side request semantics without altering default union behavior when unset.
  Validation: `shortlist_union_mode_request_maps_supported_values`, `rerank_shortlist_by_union_mode_weighted_rank_balances_objective_ranks`, `gpu_kernel_shortlist_union_mode_request_matches_exhaustive_host_union_modes`, `optimizeRequestBuilder.test.ts`, and `optimizeService.test.ts` all passed on 2026-03-25.

  Recorded benchmark evidence after this Phase 5 slice:
  - default-off baseline: `search_only 84.638 ms`, `search_plus_heatmap 85.321 ms`, `search_plus_finalist_recompute 185.460 ms`, `end_to_end_product 88.632 ms`
  - feature-on (`extra_shortlist_objectives=Pnl,Omega`, `shortlist_union_mode=weighted_rank`): `search_only 84.319 ms`, `search_plus_heatmap 84.731 ms`, `search_plus_finalist_recompute 180.130 ms`, `end_to_end_product 88.915 ms`
  - observed `end_to_end_product` delta vs current default-off baseline: about `+0.3%`, within normal benchmark noise

- [x] Add an observation-count-aware effective-sample shrinkage / generic lower-confidence score family once exact `n_obs` semantics are pinned down for the supported GPU path.
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, `crates/strategies/src/double_ma/mod.rs`, optional frontend exposure files if productized
  Rollout: completed as the advanced opt-in `effective_sample_lcb_10` score mode on the supported exact `GpuKernel` route. The backend now installs exact pair observation context for scoring, exports that context on the backend result, and the analysis-side score reconstruction uses the same context so exported tables / CSV / comparisons stay exact for the supported route.
  Validation: `effective_sample_lcb_score_penalizes_shorter_observation_histories`, `effective_sample_pair_score_uses_exact_track_record_bars`, `search_metrics_count_promotes_to_full_rows_for_score_mode`, `build_backend_result_preserves_score_observation_context`, `optimizeRequestBuilder.test.ts`, `optimizeService.test.ts`, and `analysisService.test.ts` all passed on 2026-03-25.

  Recorded benchmark evidence after this Phase 5 slice:
  - default-off rerun: `search_only 81.793 ms`, `search_plus_heatmap 81.835 ms`, `search_plus_finalist_recompute 164.80 ms`, `end_to_end_product 85.567 ms`
  - feature-on (`score_mode=effective_sample_lcb_10`): `search_only 90.635 ms`, `search_plus_heatmap 90.560 ms`, `search_plus_finalist_recompute 90.555 ms`, `end_to_end_product 90.467 ms`
  - observed feature-on `end_to_end_product` delta vs current default-off baseline: about `+5.7%`
  - acceptance note: the enabled cost is moderate but well within the later user-approved ceiling. The legacy `search_plus_finalist_recompute` diagnostic becomes non-representative under this score mode because full-row promotion lets the path skip replay-like work.

- [x] Add the remaining math-first composite score families that stay row-derived and additive:
  - `omega_vs_sharpe` / asymmetry-style feature
  - hurdle-slack / robust-margin score family
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, `web/src/stores/advancedStore.ts`, `web/src/services/optimizeRequestBuilder.ts`, `web/src/services/optimizeService.ts`, `web/src/services/analysisService.ts`, `web/src/components/AdvancedWorkspacePanel.vue`
  Rollout: completed as advanced opt-in score modes; `omega_vs_sharpe_10` and `robust_margin_10` are wired through score-mode parsing, advanced UI selection, backend scoring, and analysis-side semantics without changing default ranking when unset.
  Validation: `omega_vs_sharpe_score_prefers_positive_asymmetry_gap`, `effective_score_uses_omega_vs_sharpe_score_mode_override`, `robust_margin_score_penalizes_single_weak_dimension`, `effective_score_uses_robust_margin_score_mode_override`, `optimizeRequestBuilder.test.ts`, and `optimizeService.test.ts` all passed on 2026-03-25.

  Recorded benchmark evidence after this Phase 5 slice:
  - default-off baseline: `search_only 84.638 ms`, `search_plus_heatmap 85.321 ms`, `search_plus_finalist_recompute 185.460 ms`, `end_to_end_product 88.632 ms`
  - feature-on (`score_mode=omega_vs_sharpe_10`): `search_only 97.573 ms`, `search_plus_heatmap 97.552 ms`, `search_plus_finalist_recompute 97.382 ms`, `end_to_end_product 97.651 ms`
  - feature-on (`score_mode=robust_margin_10`): `search_only 98.164 ms`, `search_plus_heatmap 99.129 ms`, `search_plus_finalist_recompute 100.930 ms`, `end_to_end_product 99.645 ms`
  - observed feature-on `end_to_end_product` deltas vs current default-off baseline: about `+10.2%` for `omega_vs_sharpe_10` and `+12.4%` for `robust_margin_10`
  - acceptance note: these deltas exceed the earlier conservative Phase 5 target, but they remain inside the later user-approved ceiling that tolerates moderate opt-in cost so long as the default path stays flat and no `>50%` regression is introduced

- [x] Extend surface robustness beyond the current neighborhood summary to include curvature / roughness / basin-min style summaries where they can be computed from the retained shortlist surface without reintroducing a heavy matrix payload.
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/double_ma_result.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`
  Rollout: completed as part of the existing shortlist neighborhood robustness diagnostics; the retained-surface summary now includes `basin_min_score`, `basin_floor_stability`, `roughness_rms_delta`, `roughness_stability`, `curvature_penalty`, and `curvature_stability` without widening the main product row or changing default ranking semantics.
  Validation: `shortlist_neighborhood_robustness_summary_reports_expected_plateau_shape`, `gpu_kernel_shortlist_slice_request_diagnostics_are_exposed_on_backend_result`, and the earlier Phase 3C serialized `double_ma_opt` default-off rerun all passed; the accepted benchmark evidence remains the `2026-03-24` Phase 3C slice (`search_only 77.99 ms`, `search_plus_heatmap 77.56 ms`, `search_plus_finalist_recompute 160.11 ms`, `end_to_end_product 78.04 ms`).

- [x] Add multi-score surface-agreement diagnostics for the retained shortlist surface, for example agreement between the primary score, robustness score, and auxiliary score heads.
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/double_ma_result.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, optional analysis/UI files if surfaced
  Rollout: completed as default-off retained-surface diagnostics within the existing shortlist neighborhood summary; agreement is currently exposed on the backend diagnostics surface only and does not alter default ranking semantics.
  Validation: `spearman_rank_corr_reports_expected_directionality`, `shortlist_neighborhood_robustness_summary_reports_expected_plateau_shape`, `gpu_kernel_shortlist_slice_request_diagnostics_are_exposed_on_backend_result`, and a serialized feature-off `double_ma_opt` rerun all passed on 2026-03-25.

  Recorded benchmark evidence after this Phase 5 slice:
  - default-off rerun: `search_only 89.393 ms`, `search_plus_heatmap 86.503 ms`, `search_plus_finalist_recompute 199.970 ms`, `end_to_end_product 90.035 ms`
  - observed `end_to_end_product` delta vs the current default-off baseline (`88.632 ms`): about `+1.6%`
  - acceptance note: this feature only executes when shortlist-neighborhood diagnostics are requested; the default-off hot path remained effectively flat on the main contract-split surface

- [x] Evaluate lower-priority shortlist explanation metrics such as dominance-count / hypervolume-style proxies only after the higher-value Phase 5 items are complete and only if they can stay shortlist-local rather than full-grid.
  Files: `crates/app_core/src/double_ma.rs`, `crates/app_core/src/gpu_backtest_kernel.rs`, optional frontend analysis/UI files
  Rollout: completed as a retained-surface diagnostic attached to the existing shortlist neighborhood robustness summary. The new explanation block reports shortlist-local Pareto proxy counts plus a simple hypervolume-style proxy over the retained local surface and does not alter ranking or shortlist semantics.
  Validation: `shortlist_neighborhood_robustness_summary_reports_expected_plateau_shape`, `gpu_kernel_shortlist_slice_request_diagnostics_are_exposed_on_backend_result`, the serialized `double_ma_opt` rerun, `run_gpu_perf_gate.ps1 -Mode candidate`, and `run_workflow_gate.ps1 -Mode candidate` all passed on 2026-03-25. Shortlist locality is explicit in code: the computation runs inside `summarize_shortlist_neighborhood_robustness(...)` from the retained `LocalStabilityResult` surface and does not request wider kernel rows or a full candidate matrix.

  Recorded benchmark evidence after this Phase 5 slice:
  - default-off rerun: `search_only 81.793 ms`, `search_plus_heatmap 81.835 ms`, `search_plus_finalist_recompute 164.80 ms`, `end_to_end_product 85.567 ms`
  - latest clean GPU perf gate: `gpu_criterion_mean_not_regressed_10pct` = `83.419451 ms` vs baseline `155.288513 ms`; `gpu_dynamic_strategy_ratio_not_regressed_20pct` = `75.5 ms` on vs `85.5 ms` off
  - latest clean workflow gate: `runtime_not_regressed_50pct` = `1685 ms` vs baseline `1744 ms`
  - acceptance note: this explanation block only executes when shortlist-neighborhood diagnostics are requested, so the default-off hot path remains unchanged on the main contract-split surface.

### Cross-Cutting Safety / Verification

- [x] After every completed phase, rerun the serialized `double_ma_opt` benchmark and record results in this file or a linked tracker.
- [x] After every kernel-state change, rerun targeted CUDA parity tests for the affected metrics or ranking modes.
- [x] Keep CPU-only compilation / tests green after every phase.
- [x] Preserve the current default single-pass product path behavior when a new capability is not explicitly enabled.
- [x] Preserve the current validation stage APIs and semantics unless a later tracked change explicitly updates them.
- [x] Preserve backward-compatible request parsing and existing UI defaults unless an explicit product decision changes them.
- [x] Add or update representative tests for backend route selection, request parsing, and frontend request building when a feature becomes user-selectable.

### Current Gate Status

- As of the final 2026-03-25 Phase 5 reruns, the broader repo-level candidate gates are clean again in this worktree:
  - `run_gpu_perf_gate.ps1 -Mode candidate`: PASS with `gpu_criterion_mean_not_regressed_10pct = 83.419451 ms` vs baseline `155.288513 ms`, `gpu_dynamic_strategy_ratio_not_regressed_20pct = 75.5 ms` on vs `85.5 ms` off
  - `run_workflow_gate.ps1 -Mode candidate`: PASS with `runtime_not_regressed_50pct = 1685 ms` vs baseline `1744 ms`
- The contract-split `double_ma_opt` reruns on the main `alma_alma / 200k / 58300pairs` surface remain the authoritative evidence for default-path cost on this workstream, with the repo-level gates now back to being supportive rather than caveated.

## Notes On Stale Or Qualified Review Items

- The earlier reviews sometimes speak as if a full candidate matrix is already retained by default. The current runtime is still tile-based, so matrix-neighborhood work must account for that.
- Active-time algebra is useful, but any claim of "exact active Sharpe" is conditional on observation-count reconstruction.
- Exact fee-stress transforms are valid only if commission-event semantics are proven or carried explicitly.
- Optimization-stage robustness proxies are additive. They do not replace walk-forward, bootstrap, SPA, CSCV/PBO, or cross-ticker validation.
