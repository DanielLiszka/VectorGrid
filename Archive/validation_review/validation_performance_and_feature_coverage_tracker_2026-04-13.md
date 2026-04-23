# Validation Performance And Feature Coverage Tracker

Last updated: 2026-04-13

## Purpose

Track the repo-wide validation improvement program defined in `validation_performance_and_feature_coverage_master_plan_2026-04-13.md`.

This tracker is intentionally execution-oriented. It is the working ledger for:

- strategy-family coverage audits
- validation performance bundles
- route/fallback truth checks
- measurement ownership
- next implementation slices
- parity-to-`DoubleMa` assessments and exceptions

## Status Legend

Workstream status:

- `Not started`
- `Audit in progress`
- `Ready to implement`
- `Implementing`
- `Blocked`
- `Done`

Coverage posture:

- `Full`
  - existing module support appears complete for the bucket
- `Partial`
  - some existing validation modules are missing or restricted
- `CPU-tail`
  - module exists, but current route still finishes meaningful work on CPU
- `CPU-preferred`
  - repo posture intentionally prefers CPU on current measured workloads
- `Mixed`
  - GPU and CPU both matter on the current route
- `Audit`
  - current repo support needs explicit verification before claims are hardened

Parity target posture:

- `Match`
  - aim to match the current `DoubleMa` validation baseline
- `Near-match`
  - most `DoubleMa` modules should map cleanly, with limited justified exceptions
- `Selective`
  - only some `DoubleMa` modules map meaningfully to the bucket
- `Exception`
  - the bucket is expected to keep explicit non-parity areas for real reasons

## Program Workstreams

| Workstream | Status | Primary scope | Primary files | Evidence owner | Next action |
| --- | --- | --- | --- | --- | --- |
| Phase 0 baseline and coverage inventory | Audit in progress | build strategy-by-strategy coverage map and timing ownership | `REPO_MAP.md`, `gpt54_feature_review_map.md`, validation core/command files | tracker + targeted code inspection | continue replacing placeholder posture with file-backed bucket notes and parity exceptions |
| Phase 1 residual `DoubleMa` pass | Implementing | residual `DoubleMa` latency, routed-family truth, cross-ticker overhead | `double_ma_validate.rs`, `commands_validation.rs`, `ta_double_ma_validation_cost.rs` | `ta_double_ma_validation_cost.rs` | keep the new TrendQuality ADX-off CPU fallback truthful while narrowing the remaining exact-combo GPU parity repair |
| Phase 2 shared validation core | Implementing | shared loop cost, scratch reuse, deterministic measurement | `validation_core.rs`, `validation_pipeline.rs`, `iterative_sim.rs` | targeted tests + workflow/cost evidence | rerun perf gates on the landed order-stat and perturb-streaming slice, then decide whether the next hotspot is still shared-core or has moved to strategy-local helper parity |
| Phase 3 orchestration and contract overhead | Implementing | request shaping, cross-ticker reruns, validation launch overhead | `commands_validation.rs`, `validation_common.rs`, `validationRequestBuilder.ts`, `validationService.ts` | app-path timing + tests | preserve the landed cross-ticker pre-sizing/Pareto cleanup and only deepen canonicalization work if new timing still justifies it |
| Phase 4 routed `DoubleMa` family coverage | Audit in progress | routed-profile module and route-policy truth | `double_ma_validate.rs`, routed profile helpers, request builders | `DoubleMa` cost harness + targeted tests | keep the audited routed trio truthful and close the remaining double-like-profile inventory |
| Phase 5 `CompositeCpu` and `CompositeMo` | Audit in progress | validation module coverage, fallback rules, CPU-preferred policy | `composite_validate.rs`, `composite_cpu.rs`, `composite_mo.rs` | composite bench plus any new headless timing harness | keep the broad CPU-side module coverage truthful and document the narrow plain-base GPU eligibility contract instead of implying a `CompositeMo`-specific hard blocker |
| Phase 6 standalone Cluster A | Audit in progress | `Asr`, `Vcc`, `Vemr`, `Rocb`, `Vcet` | strategy-local files plus `composite_validate.rs` | composite bench + targeted per-strategy timing | continue per-strategy truth/perf audit after the `Vcc` and `Vcet` route updates |
| Phase 7 standalone Cluster B | Audit in progress | `Mkcs`, `Kqsr`, `Carm`, `Kvdf`, `Cdmc` | strategy-local files plus `composite_validate.rs` | composite bench + targeted per-strategy timing | preserve the completed coverage audit, keep the new Carm/Cdmc active-route parity locked, and only revisit dedicated reopt-helper dispatch if timing justifies it |
| Phase 8 special validation flows | Audit in progress | `MlSignal` validation cost and replay/policy-sweep latency | `commands_ml_signal.rs`, `ml_signal.rs`, `mlSignalService.ts`, ML signal benches | `ml_signal_validation.rs` plus ML workflow evidence | keep score-cache validation and policy sweep as separate timing surfaces; do not fold dataset export or score-cache build into the validation cost boundary |
| Phase 9 closeout and governance sync | Not started | keep docs and route truth synchronized | `REPO_MAP.md`, tracker, adjacent validation docs | doc review | update routing docs when posture materially changes |

## Strategy Bucket Tracker

| Bucket | Current validation posture | Coverage posture | DoubleMa parity target | Primary files | Evidence owner | Status | Next action |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `DoubleMa` core | strongest current validation anchor; first throughput wave already landed | Full for core modules, with residual perf audit still needed | Baseline | `double_ma_validate.rs`, `validation_core.rs`, `validation_pipeline.rs` | `ta_double_ma_validation_cost.rs`, `double_ma_validation_workflow.rs` | Ready to implement | confirm remaining residual bottlenecks before touching downstream families |
| Routed `DoubleMa` profiles | `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` inherit the full `DoubleMa` validation stack with route-specific ADX constraints; TrendQuality-without-ADX fixed-candidate validation on `GpuKernel`/`Auto` now falls back to CPU for parity safety | Full via inherited engine plus documented route exception | Match | `double_ma_validate.rs`, `composite_profile_route.rs`, request builders | `DoubleMa` cost harness + targeted tests | Audit in progress | keep routed-profile route exceptions honest while the exact-combo TrendQuality GPU parity repair remains open |
| Other double-like profiles | `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, `Vfct` inherit the full `DoubleMa` validation engine; remaining caveats are generic DoubleMa route limits plus profile-specific candle-field or product-surface restrictions | Full via inherited engine plus documented input/route caveats | Match | `double_ma_validate.rs`, strategy validators, request builders | `DoubleMa` cost harness + targeted tests | Audit in progress | targeted `GpuKernel` smoke now covers `Pit` SPA and `Vfct` CSCV; keep the remaining route/input caveats documented rather than treating the bucket as a feature gap |
| `CompositeCpu` | own validation family; CPU-side module coverage is broad and GPU fixed-candidate validation exists only for a narrow plain-base contract | Near-match with strict GPU eligibility contract | Near-match | `composite_validate.rs`, `composite_cpu.rs` | `composite_validation_workflow.rs` | Audit in progress | keep the narrow base-GPU eligibility contract explicit and preserve the real remaining parity gaps: partial cross-ticker scope and narrower aux analytics |
| `CompositeMo` | CPU-side validation module coverage is broad through the shared composite validator; route posture remains CPU-preferred in practice | Broad CPU-side coverage / route-limited | Exception | `composite_validate.rs`, `composite_mo.rs` | composite workflow evidence plus any added headless harness | Audit in progress | describe the current GPU limit as candidate-shape-based plain-base eligibility rather than a strict provenance blocker and keep cross-ticker/aux-analytics gaps explicit |
| Cluster A | `Asr`, `Vcc`, `Vemr`, `Rocb`, `Vcet` | Mixed / CPU-preferred depending on strategy, but current route coverage is broader than the first tracker draft implied across the bucket | Near-match | `composite_validate.rs`, strategy-local files | `composite_validation_workflow.rs` | Audit in progress | keep explicit route-policy exceptions truthful and separate the remaining perturb/helper parity work from broader module-coverage claims |
| Cluster B | `Mkcs`, `Kqsr`, `Carm`, `Kvdf`, `Cdmc` | Mixed / CPU-preferred depending on strategy | Near-match | `composite_validate.rs`, strategy-local files | `composite_validation_workflow.rs` | Audit in progress | keep cross-ticker and aux-analytics gaps explicit; if runtime work resumes here, measure the post-repair Carm/Cdmc mixed-route cost before reopening dedicated reopt-helper work |
| `MlSignal` | separate CPU-only score-cache validation workflow; current lane is score-cache validation plus policy sweep | Selective / score-cache-native | Selective | `commands_ml_signal.rs`, `ml_signal.rs`, `mlSignalService.ts` | `ml_signal_validation.rs` | Audit in progress | keep it as a selective-parity bucket, use score-cache validation and policy sweep as separate timing surfaces, and do not treat absent `DoubleMa` aux modules as missing parity unless score-cache-native equivalents are actually planned |

## Known Route-Policy Notes To Preserve

- `DoubleMa` fixed-candidate validation can use CPU or GPU main routes, but direct GPU cross-ticker validation is still a fallback-sensitive area.
- `CompositeTrend` and `BreakoutMomentum` target `GpuKernel` without ADX and `GpuHybridKernel` with ADX.
- `RegimeHybrid` currently targets `GpuHybridKernel` in its ADX-bearing form.
- TrendQuality requests without ADX confirmation currently fall back to CPU on `GpuKernel`/`Auto` fixed-candidate validation because the exact-combo GPU validation path is not parity-safe there yet.
- `CompositeCpu` GPU validation is only eligible for plain base-composite `HardGate` finalists with uniform strategy/indicator contracts, vectorized engine, and no inline `cross_ticker`.
- `CompositeMo` remains CPU-preferred for validation on current repo posture, but the current GPU limit is candidate-shape-based plain-base eligibility rather than a dedicated provenance gate.
- `Asr`, `Vemr`, and `Rocb` currently keep `Auto` CPU-preferred on measured validation workloads.
- `Vcc`, `Carm`, and `Cdmc` are mixed-route strategies.
- `Vcet` now has runtime-tested `GpuKernel` coverage for walk-forward, reopt walk-forward, CSCV/PBO, SPA, perturb, and trade bootstrap.

## Parity Exception Reasons

Use one of these when a bucket cannot fully match the current `DoubleMa` validation baseline:

- strategy semantics do not map cleanly to the module
- exactness/parity constraints require CPU-reference or CPU-tail handling
- current route architecture does not support the module without disproportionate complexity
- product posture intentionally remains CPU-preferred on measured workloads
- the module is irrelevant to the workflow shape of the bucket

## Module Coverage Audit Matrix

Status meanings in this table:

- `Implemented`
- `CPU-tail`
- `Mixed`
- `Limited`
- `Audit`

| Bucket | Main pass | Walk-forward | Reopt WF | CSCV/PBO | SPA | Perturb | Bootstrap | Cross-ticker | Aux analytics | DoubleMa parity summary | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `DoubleMa` core | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Baseline | use as the initial truth anchor; residual perf audit still required |
| Routed `DoubleMa` profiles | Implemented via inherited engine | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | current routed trio matches the `DoubleMa` module stack, but one route-policy exception now preserves parity via CPU fallback rather than GPU-safe exactness | ADX still changes the expected backend route, and TrendQuality without ADX confirmation now falls back to CPU on `GpuKernel`/`Auto` fixed-candidate validation |
| Other double-like profiles | Implemented via inherited engine | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | inherited `DoubleMa` module coverage is effectively full; remaining differences are generic DoubleMa route caveats and profile-specific input/product restrictions, not missing validation features | `Pit` still has a native device-regime path under investigation during long CUDA sweeps; `Marrt` / `Vfct` current MA restrictions are product-surface constraints rather than separate validator blockers |
| `CompositeCpu` | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Partial | Partial | target near-match; current systematic gaps are cross-ticker scope and narrower aux analytics versus `DoubleMa` | current GPU path is narrower than “uniform HalfTrend only”: it requires plain base-composite `HardGate`, uniform HalfTrend/MACZ/OTT/strategy/MA types, vectorized engine, and no inline `cross_ticker` |
| `CompositeMo` | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Partial | Partial | selective parity only; route-limited more than feature-limited | CPU-side module coverage is broad through the shared composite validator, but current GPU eligibility is candidate-shape-based plain-base support rather than a strict `CompositeMo` blocker |
| Cluster A | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Partial | Partial | target near-match per strategy; remaining parity gaps are cross-ticker scope and narrower aux analytics, not core module absence | `Vcet` route-truth was stale and is now corrected; `Vcc` GPU walk-forward/reopt dispatch is now wired |
| Cluster B | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Implemented | Partial | Partial | target near-match per strategy; real remaining parity gaps are cross-ticker scope, narrower aux analytics, and measured mixed-route policy truth | mixed-route and CPU-preferred policies still need evidence-backed per-strategy notes |
| `MlSignal` | Implemented | Limited | Limited | Limited | Limited | Limited | Limited | Limited | Limited | selective mapping only; score-cache workflow is not a normal fixed-candidate family | score-cache-backed iterative replay plus PSR/DSR, robust score / Pareto, run verdict, and score coverage metadata are implemented; walk-forward, reopt walk-forward, CSCV/PBO, SPA, perturb, trade bootstrap, cross-ticker, and DoubleMa aux analytics do not currently map |

## Phase 0 Audit Notes

- Composite-side validation feature coverage is broader than the older high-level notes implied. Static review of `composite_validate.rs` shows broad CPU-route support for `CompositeCpu`, `CompositeMo`, and the audited standalone buckets across walk-forward, reopt walk-forward, perturb, trade bootstrap, CSCV/PBO, SPA, verdicts, and the currently exposed composite aux analytics.
- Composite-side `cross_ticker` is broader than the first tracker draft implied, but it still remains partial relative to the `DoubleMa` baseline. Both `DoubleMa` and non-`DoubleMa` families use command-layer reruns after the main validation pass, and composite `preserved_finalists` already retains CSCV/PBO plus hard-gate thresholds, but it does not yet rerun the full walk-forward/reopt/SPA/perturb/bootstrap stack.
- `Vcet` route truth was stale in preview/doc surfaces. Runtime tests in `composite_validate.rs` already cover `GpuKernel` parity for walk-forward, reopt walk-forward, CSCV/PBO, SPA, perturb, and trade bootstrap; the preview layer and `REPO_MAP.md` have been updated to match.
- TrendQuality without ADX confirmation is now an explicit routed-`DoubleMa` parity exception rather than an untracked bug. `GpuKernel` and `Auto` fixed-candidate validation fall back to CPU on that path while the exact-combo GPU regime bug remains under investigation.
- `Vcc` now uses the dedicated GPU walk-forward and reopt walk-forward helpers on the `GpuKernel` route. CPU-reference signal preparation still matters, so the overall bucket remains mixed rather than fully GPU-native.
- `Asr` and `Vemr` perturb parity is now locked on the `GpuKernel` route by routing their perturb helper through the same exact-series metric path already used for other non-authoritative raw-GPU families. Targeted regression tests now cover both strategies.
- `Carm` and `Cdmc` fixed-candidate `GpuKernel` walk-forward parity is now repaired on the active route by evaluating windows from the exact desired-position series rather than raw per-window GPU metrics.
- helper-direct drawdown-risk tests plus route-level Carm/Cdmc CPU-match tests now lock that active-route parity, while the shipped reopt path stays on the generic exact-series reducer instead of direct family-specific helper dispatch.
- `composite_validation_workflow` now provides a concrete Cluster B timing split on the shared 50k/8 `wf_reopt_cscv_spa` surface: CPU `Carm` measures about `98.5-98.9 ms` versus CPU `Cdmc` about `694.5-696.5 ms`, and `GpuKernel` `Carm` about `167.9-175.1 ms` versus `GpuKernel` `Cdmc` about `744.6-767.0 ms`. Do not assume `Cdmc` should lead future perf work simply because its route policy is more GPU-favorable.
- the GPU Criterion suite is no longer blocked by the old base-composite route-shape mismatch after the bench candidate normalization, but it still later hits an `Mkcs` 100k/32 illegal-memory-access fallback in the GPU bench harness. Targeted `mkcs_validation_gpu_kernel*` tests still pass, so treat that as a long-benchmark/runtime issue rather than a blanket Mkcs parity regression.
- routed `DoubleMa` profiles are no longer a real feature-gap bucket on the current repo surface. `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` inherit the full `DoubleMa` validation stack; their remaining differences are route-policy exceptions such as ADX-driven backend selection.
- the remaining double-like profiles are also no longer a meaningful feature-gap bucket. `Pit`, `Set`, `Mts`, `Orc`, `Marrt`, and `Vfct` inherit the full `DoubleMa` validation stack; their real caveats are generic DoubleMa route limits plus candle-field or product-surface restrictions.
- direct `GpuKernel` smoke now covers the highest-caveat inherited profiles too: `Vfct` on CSCV/PBO and `Pit` on SPA both have passing targeted validation tests in `double_ma_validate.rs`.
- composite-family aux analytics are narrower than the `DoubleMa` baseline. Current composite candidates expose `regime_slice_robustness` and `friction_stress`, but not `entry_margin_fragility`, `underwater_duration`, `checkpoint_robustness`, or `benchmark_slice_summary`.
- `CompositeCpu` and `CompositeMo` are both broader on CPU-side validation coverage than the old tracker wording implied. Their real remaining parity gaps are still partial composite `cross_ticker` scope, narrower aux analytics, and a narrow plain-base GPU eligibility contract.
- `CompositeMo` is primarily route-limited rather than feature-limited. CPU-side validation coverage is broad, but current GPU fixed-candidate validation is candidate-shape-based plain-base support rather than a provenance-specific `CompositeMo` gate.
- `MlSignal` is a selective-parity exception. Its canonical validation timing boundary is score-cache-backed iterative replay in `run_ml_signal_validation_blocking_with_candles(...)`, while `run_ml_signal_policy_sweep_blocking_with_candles(...)` is a separate sweep timing surface over axis expansion plus the same replay core. Dataset export and score-cache build are upstream workflow costs, not part of the validation timing line.
- `MlSignal` currently supports score-cache contract enforcement, replay with iterative execution settings, PSR/DSR hard gates, robust score / robust Pareto, run verdict, and score coverage metadata. Walk-forward, reopt walk-forward, CSCV/PBO, SPA, perturb, trade bootstrap, cross-ticker, and DoubleMa aux analytics do not currently map to the score-cache-native product shape.

## Cluster A Strategy Notes

| Strategy | Current posture | Coverage posture | Status | Next action |
| --- | --- | --- | --- | --- |
| `Asr` | GPU route exists and module coverage is broad on `GpuKernel`, but `Auto` currently prefers CPU | CPU-preferred / broad module coverage | Audit in progress | keep the explicit `Auto->CpuOnly` policy truthful and decide whether new timing evidence justifies challenging the current CPU-preferred posture |
| `Vcc` | mixed validation route with CPU-reference signal prep; `GpuKernel` now has dedicated GPU walk-forward + reopt walk-forward dispatch | Mixed | Audit in progress | measure remaining CPU-reference prep cost and decide whether further GPU path widening is worth it |
| `Vemr` | GPU route exists and module coverage is broad on `GpuKernel`, but `Auto` currently prefers CPU | CPU-preferred / broad module coverage | Audit in progress | keep the explicit `Auto->CpuOnly` policy truthful and decide whether new timing evidence justifies challenging the current CPU-preferred posture |
| `Rocb` | GPU route exists, module coverage is broad on `GpuKernel`, and `Auto` currently prefers CPU | CPU-preferred / broad module coverage | Audit in progress | keep the explicit `Auto->CpuOnly` policy truthful and decide whether new timing evidence is worth collecting beyond the existing route-policy lock-in |
| `Vcet` | main GPU fixed-candidate route exists and current runtime tests cover walk-forward, reopt, CSCV/PBO, SPA, perturb, and trade bootstrap on `GpuKernel` | Near-match | Audit in progress | keep preview/doc surfaces aligned and isolate any remaining non-parity areas such as cross-ticker |

## Cluster B Strategy Notes

| Strategy | Current posture | Coverage posture | Status | Next action |
| --- | --- | --- | --- | --- |
| `Mkcs` | GPU route exists, but current posture remains CPU-preferred or fallback-sensitive | CPU-preferred | Audit in progress | preserve the audited full module coverage and measure whether helper-route widening is worth it |
| `Kqsr` | GPU route exists, but current posture remains CPU-preferred | CPU-preferred | Audit in progress | preserve the audited full module coverage and measure the remaining OHLC staging cost |
| `Carm` | mixed validation route; `Auto` currently prefers CPU | Mixed / CPU-preferred | Audit in progress | keep the active `GpuKernel` walk-forward parity locked; current 50k/8 workflow timing already shows `Carm` materially cheaper than `Cdmc`, so only reopen dedicated reopt-helper work if a narrower bottleneck appears |
| `Kvdf` | GPU route exists, but current posture remains CPU-preferred | CPU-preferred | Audit in progress | preserve the audited full module coverage and measure remaining HLCV staging cost before changing posture |
| `Cdmc` | more GPU-favorable than `Carm`, but still mixed | Mixed | Audit in progress | keep the active `GpuKernel` walk-forward parity locked; current 50k/8 workflow timing is still materially slower than `Carm`, so isolate that cost before reopening dedicated reopt-helper work |

## Immediate Execution Queue

1. Phase 0 inventory is now file-backed across all current validation buckets; keep the tracker aligned as runtime work changes posture.
2. Re-run the residual `DoubleMa` validation hotspot audit and lock the baseline modules that later buckets should be measured against.
3. Decide for each non-`DoubleMa` bucket which gaps are:
   - parity work to implement
   - parity work to defer
   - or documented exceptions
4. After that, choose the next implementation slice based on the highest-value combination of:
   - performance benefit
   - feature-parity gain
   - route-truth improvement
5. The first execution candidate after the baseline audit should usually be one of:
   - residual `DoubleMa` cleanup
   - deeper shared validation-core reductions
   - exact-combo `DoubleMa` TrendQuality GPU parity repair behind the current CPU fallback
   - the `Mkcs` 100k/32 GPU-bench illegal-memory-access/fallback issue that now blocks the later Cluster B Criterion run
