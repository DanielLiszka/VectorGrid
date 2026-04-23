# Review Packet: `crates/app_core/src/composite_mo.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/composite_mo.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful multiobjective orchestration surface with good test coverage, but not yet accept-as-is because several public request and telemetry fields are narrower than their names suggest, and GPU-prescan / exact-grid behavior is under-specified at the API boundary`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Hot-path`, `Sampler-orchestration-critical`, `Telemetry-contract-sensitive`, `Benchmark-sensitive`

## What This File Owns

`composite_mo.rs` owns the multiobjective composite-optimization surface. It defines the public request/result/telemetry types, validates and decodes the multiobjective search space, optionally runs a stage-0 GPU prescan to narrow MA domains, orchestrates sampler selection across exact-grid / NSGA-II / random / TPE / QMC backends, evaluates candidate points, and rewrites final Pareto-point metrics onto the `Trade22` metric surface.

This file owns:

- public request/config surface:
  - `CompositeConstraints`: `crates/app_core/src/composite_mo.rs:71-75`
  - `CompositeMoSampler`: `78-83`
  - `CompositeMoObjectives`: `87-90`
  - `CompositeGpuPrescanObjectives`: `92-97`
  - `CompositePrescanMaPair`: `101-104`
  - `CompositeGpuPrescanConfig`: `107-129`
  - `CompositeMoRequest`: `156-191`
- public result/telemetry surface:
  - `CompositeMoParams`: `386-397`
  - `CompositeMoPoint`: `399-406`
  - `CompositeMoPrescanTelemetry`: `418-434`
  - `CompositeMoSearchTelemetry`: `436-449`
  - `CompositeMoTelemetry`: `451-455`
  - `CompositeMoResult`: `457-466`
- request normalization and decode helpers:
  - search-space validation: `594-615`
  - parameter decoding and type/period helpers: `617-1060`
  - objective validation: `907-920`
- main orchestration:
  - `run_composite_mo_optimization_blocking_with_candles`: `1132-3098`
- tests:
  - start at `3210`

This file does not own:

- the underlying multiobjective samplers
  - `ta_optimizer::mo::*`
- the base composite metrics engine
  - `ta_strategies::composite_cpu::*`
- the double-MA prescan optimizer itself
  - `run_double_ma_optimization_blocking_with_candles` from `double_ma.rs`

## Classification

- Public surface: yes
  - request/result/telemetry structs are part of the backend contract
- Hot path: yes
  - candidate evaluation, prescan narrowing, and sampler orchestration live here
- Runtime-critical: high
  - request interpretation and telemetry shape affect correctness and debugging
- CUDA/runtime-critical: medium
  - GPU is used only for the prescan lane, but that lane mutates the effective search domain
- Strategy-local: no
  - this is shared orchestration rather than a single-strategy implementation
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- composite search and final Trade22 metric recompute:
  - `crates/app_core/src/composite_cpu.rs`
  - especially `evaluate_composite_cpu_trade22_metrics`: `crates/app_core/src/composite_cpu.rs:1190-1318`
- shared MA utilities and Double-MA prescan:
  - `crates/app_core/src/double_ma.rs`
- predictive prescan lane:
  - `crates/app_core/src/prescan_predictive.rs`
- dynamic strategy plan compilation and artifacts:
  - `crates/app_core/src/strategy_dynamic.rs`
  - `crates/app_core/src/strategy_plan.rs`
- optimizer/sampler backends:
  - `crates/optimizer/src/mo.rs`
  - `crates/optimizer/src/lib.rs`
- shared strategy metrics:
  - `crates/strategies/src/composite_cpu/mod.rs`
  - `crates/strategies/src/double_ma/mod.rs`

### Downstream verification impact if changed

- any change to public request semantics affects:
  - Tauri command callers and any frontend/service layer that shapes composite MO requests
- any change to `gpu_prescan` behavior affects:
  - candidate-domain narrowing
  - exact-grid fallback
  - telemetry interpretation
- any change to result counters or telemetry fields affects:
  - tests that assert exact budgets and candidate counts
  - downstream consumers that use those counters for UX or reporting

### Test anchors

- random sampler smoke:
  - `crates/app_core/src/composite_mo.rs:3210-3280`
- QMC sampler smoke:
  - `3826-3892`
- prescan behavior:
  - `4043-4118`
  - `4120-4201`
  - `4203-4288`
- decode/repair behavior:
  - `3651-3817`
- dynamic-strategy parity:
  - `3367-3538`

## Standards And Research Log

This packet was written after targeted external review focused on public Rust API/telemetry contracts, Serde enum request-shape behavior, and multiobjective optimization terminology around Pareto / constrained-domination / sampler budget reporting.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file exposes several public request/result/telemetry structs
   - Standard informed:
     - public APIs should be predictable, documented, and dependable

2. Serde enum representations
   - Source: <https://serde.rs/enum-representations.html>
   - Why it was consulted:
     - `CompositeMoSampler` is an internally tagged request enum
   - Standard informed:
     - tagged enum request shapes should be explicit and unsurprising for callers

3. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - this file returns many recoverable `Result<_, String>` errors and also intentionally treats some candidate-decoding failures as constrained outcomes rather than hard errors
   - Standard informed:
     - recoverable request-shape failures and silent no-op behavior should be distinguished carefully

4. Deb, Pratap, Agarwal, Meyarivan (2002), NSGA-II
   - Source: <https://repository.ias.ac.in/9403/>
   - Why it was consulted:
     - this file exposes an `Nsga2` sampler and reports Pareto-front telemetry under constrained-domination language
   - Standard informed:
     - sampler-facing terminology and telemetry should match the actual search/evaluation contract

## Findings

## Correctness And Safety

### 1. Candidates that are already known-invalid still flow into the expensive metric path

- Evidence:
  - indicator validity can collapse to `n` through `unwrap_or(n)`:
    - `crates/app_core/src/composite_mo.rs:2324`
    - `2350`
    - `2375`
  - `start_t >= n` and mid-search cancellation only add constraint markers:
    - `2404-2408`
  - but evaluation then still continues into the expensive metrics path:
    - `2431-2548`
- Why it matters:
  - once the file knows the candidate is infeasible or cancelled, continuing into metrics evaluation wastes hot-path CPU time
  - for cancelled work in particular, the current behavior delays exit and keeps doing avoidable compute
- Smallest justified fix:
  - return `invalid_outcome(constraints)` immediately after `start_t >= n` or cancellation is detected
- Review judgment:
  - safe-now correctness/performance hardening

## API And Naming

### 1. The public `mode` field is explicitly ignored

- Evidence:
  - the request surface exposes `mode`:
    - `crates/app_core/src/composite_mo.rs:189-191`
  - the main entry point immediately discards it:
    - `1145-1146`
- Why it matters:
  - the code comment explains that this is a placeholder, but the public field still invites callers to think it changes execution
- Smallest justified fix:
  - either reject non-`None` / non-default `mode` values with a clear error
  - or document the field as ignored compatibility ballast
- Review judgment:
  - low-risk API clarification

### 2. `gpu_prescan` can be accepted but not actually applied when MA length domains are categorical and already below the safety cap

- Evidence:
  - prescan only executes when both `fast_len` and `slow_len` domains can be converted into `IntRange` tuples:
    - `crates/app_core/src/composite_mo.rs:1208-1228`
  - if they are not `IntRange`, the code only errors when the raw domain sizes exceed the safety cap:
    - `1554-1557`
  - otherwise execution continues without prescan telemetry and without search-space narrowing
- Why it matters:
  - a caller can provide `gpu_prescan` and receive neither an error nor evidence that prescan was skipped
  - that makes the request contract ambiguous
- Smallest justified fix:
  - reject `gpu_prescan` explicitly when `fast_len`/`slow_len` are not `IntRange`
  - or record an explicit telemetry note that prescan was requested but not applied
- Review judgment:
  - safe-now request-contract hardening

### 2. `requested_evaluations` means “requested search budget,” not necessarily “exact work planned,” and that is not obvious from the field name

- Evidence:
  - the field is public:
    - `436-449`
  - when exact-grid is used, the file still sources `requested_evaluations` from the original sampler budget:
    - budget derived from sampler config: `2777-2783`
    - exact-grid path chosen: `2941-2960`
    - telemetry field assignment: `3045-3050`
  - the current test suite locks this behavior:
    - `4120-4199`, especially `4183-4189`
- Why it matters:
  - consumers can read `requested_evaluations` as “exact-grid candidate count” even though `exact_grid_total_combinations` is the field that actually carries that value
- Smallest justified fix:
  - document the field as search budget rather than exact-grid combination count
  - or rename/split the field in a later compatibility-reviewed cleanup
- Review judgment:
  - documentation-first fix; rename is a broader API change

## Error Handling

### 1. Internal evaluation failures are flattened into generic infeasible points without dedicated telemetry

- Evidence:
  - indicator compute failures return infeasible points with generic constraint markers:
    - `crates/app_core/src/composite_mo.rs:2301-2374`
  - dynamic-plan compile/eval failures do the same:
    - `2480-2529`
  - telemetry only exposes:
    - `rejected_decode_or_order`
    - `rejected_prescan_pair`
    - `rejected_other_constraints`
    - `3042-3059`
- Why it matters:
  - a backend-internal indicator or dynamic-plan failure is currently hard to distinguish from ordinary rejected candidates
  - that weakens the usefulness of the search telemetry for debugging and regression triage
- Smallest justified fix:
  - add explicit telemetry counters for internal compute failure classes
  - at minimum, distinguish “decode/order” from “indicator/dynamic runtime failure”
- Review judgment:
  - safe-now observability improvement

### 2. Search/evaluation counters underreport total backend work because the final Trade22 recompute pass is outside the counted evaluation loop

- Evidence:
  - `done_counter` increments only inside the search-phase `eval_with_progress` closure:
    - `2924-2939`
  - after Pareto search finishes, the file reruns every Pareto point through CPU `Trade22` enrichment:
    - `3030-3037`
  - `actual_evaluations` and `total_evaluations` are then sourced from `done_counter`:
    - `3041`
    - `3093-3098`
- Why it matters:
  - the final result’s `total_evaluations` reads like a total-work count, but it excludes the post-search enrichment phase
  - this is not a search correctness bug, but it is a telemetry-contract mismatch
- Smallest justified fix:
  - document `total_evaluations` / `actual_evaluations` as search-phase counts
  - or add a second counter for post-search Trade22 recomputes
- Review judgment:
  - safe-now telemetry clarification

## Module Boundaries And Technical Debt

### 1. This file conflates public request/telemetry contract, search-space decoding, prescan orchestration, sampler wiring, dynamic-plan parity checks, and final metric enrichment

- Evidence:
  - public surface definitions at `71-191` and `386-466`
  - decode/repair helpers at `594-1060` and `2600-2775`
  - orchestration spanning `1132-3098`
  - tests beginning at `3210`
- Why it matters:
  - it is already large enough that request semantics, telemetry semantics, and evaluation semantics are easy to drift apart
  - the `mode` placeholder and evaluation-count ambiguity are examples of that drift
- Smallest justified fix:
  - do not rewrite it wholesale now
  - extract shared telemetry/contract helpers first if this file needs further growth
- Review judgment:
  - larger refactor exists, but should be deferred

### 2. Final Pareto-point metrics are intentionally rewritten onto a CPU `Trade22` surface, which is an important cross-file contract and should stay explicit

- Evidence:
  - the helper request forces `Backend::CpuOnly` and `metric_contract: Trade22`:
    - `crates/app_core/src/composite_mo.rs:468-508`
  - every returned Pareto point is recomputed through `evaluate_composite_cpu_trade22_metrics`:
    - `3030-3037`
  - the result advertises `metric_surface_kind: Some("trade22")`:
    - `3093-3098`
- Why it matters:
  - downstream readers should understand that the search phase and the final reported metric surface are not the same operational phase
  - this is intentional behavior, not redundant work to casually “optimize away”
- Review judgment:
  - semantics-sensitive; document clearly before attempting optimization

### 3. Composite MO artifacts are still one-way serialized, which blocks downstream campaign reuse

- Evidence:
  - public result/telemetry types derive `Serialize` but not `Deserialize`:
    - `crates/app_core/src/composite_mo.rs:399-466`
  - downstream campaign code explicitly warns that matching CompositeMo artifacts still rerun optimization because they are not deserializable for validation reuse:
    - `src-tauri/src/commands_campaign.rs:2596-2600`
- Why it matters:
  - this is not a file-local correctness bug, but it is real cross-file technical debt with user-visible runtime cost
- Smallest justified fix:
  - decide whether reuse is in scope; if yes, make the artifact shape round-trippable and add a deserialization test
- Review judgment:
  - larger cross-file debt item, but worth recording in the packet

## Docs And Examples

### 1. The public request and telemetry types need clearer semantics documentation

- Missing documentation topics:
  - `mode` is ignored
  - `gpu_prescan` only materially applies to `IntRange` MA-length domains
  - `requested_evaluations` is a budget-style field
  - `total_evaluations` excludes the post-search `Trade22` recompute pass
  - final `pareto[*].metrics` are CPU `Trade22` metrics even if prescan used GPU
- Smallest justified fix:
  - add Rustdoc to the public request/result/telemetry structs and fields above

## Tests And Missing Verification

### 1. Coverage is good, but the most important telemetry semantics are not pinned directly

- Current verification observed during this review:
  - `cargo test -p ta_app_core composite_mo -- --nocapture`
    - passed `18` tests
  - `cargo test -p ta_app_core composite_mo_smoke_random_returns_pareto -- --nocapture`
  - `cargo test -p ta_app_core composite_mo_smoke_qmc_sobol_returns_pareto -- --nocapture`
  - `cargo test -p ta_app_core composite_mo_gpu_prescan_narrows_large_len_ranges -- --nocapture`
  - all passed
- cross-file parity anchor:
  - `crates/app_core/src/composite_validate.rs:16851`
    - `composite_mo_candidates_validate_consistently`
- Strong existing tests already in file:
  - `composite_mo_fast_objectives_emit_trade22_metric_surface`: `3282-3365`
  - `composite_mo_gpu_prescan_uses_exact_grid_for_small_candidate_sets`: `4120-4201`
  - `composite_mo_repairs_macz_order_before_exact_grid_eval`: `3754-3817`
- Missing explicit coverage:
  - test that `gpu_prescan` requested against non-`IntRange` MA domains is either rejected or explicitly telemetered as skipped
  - test that `total_evaluations` and search telemetry intentionally exclude post-search `Trade22` recompute work
  - test that `mode` is rejected or documented as ignored

## Performance, Allocation, And CUDA Concerns

### 1. The final `Trade22` Pareto enrichment is a deliberate second pass and is currently serial CPU work

- Evidence:
  - final point recompute loop:
    - `crates/app_core/src/composite_mo.rs:3030-3037`
  - forced CPU backend in the helper request:
    - `472-507`
- Why it matters:
  - this is the biggest obvious post-search cost in the file
  - but it is tied directly to the final reported metric surface, so it should be treated as semantics-sensitive, not “free” cleanup
- Review judgment:
  - benchmark-sensitive; do not change casually

### 2. Prescan narrowing is a performance feature with correctness implications for the effective search domain

- Evidence:
  - prescan rewrites fast/slow length domains into categorical candidate sets:
    - `1460-1487`
  - allowed-pair filtering then participates in evaluation and repair:
    - `1893-1896`
    - `2622-2647`
    - `2795-2887`
- Why it matters:
  - prescan is not just an optimization detail; it changes which candidate set the samplers see
- Review judgment:
  - benchmark-sensitive and semantics-sensitive

## Code-Quality Improvement Register

- Safe now:
  - document or reject ignored `mode`
  - document or reject `gpu_prescan` when MA-length domains are not `IntRange`
  - clarify search-budget telemetry versus exact-grid candidate-count telemetry
  - clarify that final `total_evaluations` excludes post-search `Trade22` recompute work
- Defer:
  - broader decomposition of this file into request-surface, prescan, and sampler modules
  - changing the final Trade22 recompute flow

## Technical-Debt Reduction Register

- Safe-now debt to reduce:
  - ambiguous public field semantics
  - telemetry names that over-compress distinct concepts
  - under-documented prescan and final-enrichment behavior
- Defer-later debt:
  - splitting orchestration, decode/repair, and telemetry construction into smaller owners
- Useful debt that should not be “cleaned up” casually:
  - the explicit second-pass `Trade22` metric rewrite
  - the exact-grid fallback path after prescan narrowing

## Performance Opportunity Register

- Safe structural opportunities:
  - none more important than contract/documentation cleanup
- Benchmark-sensitive opportunities:
  - parallelizing or caching final `Trade22` recompute
  - changing when exact-grid is selected
  - altering prescan narrowing rules or repair behavior

## Recommended Action

- `Small surgical refactor`

Reasoning:

- the file’s strongest issues are contract and telemetry clarity problems, not broken search fundamentals
- the safe-now work is bounded:
  - document or guard ignored fields
  - clarify prescan applicability
  - separate search budget from exact-grid / post-search work in telemetry
- large algorithmic or performance rewrites are not justified without benchmark evidence

## Exact Local Verification Required If Changed

If request/telemetry semantics change:

- `cargo test -p ta_app_core composite_mo -- --nocapture`
- `cargo test -p ta_app_core composite_mo_smoke_random_returns_pareto -- --nocapture`
- `cargo test -p ta_app_core composite_mo_smoke_qmc_sobol_returns_pareto -- --nocapture`
- `cargo test -p ta_app_core composite_mo_fast_objectives_emit_trade22_metric_surface -- --nocapture`
- `cargo test -p ta_app_core composite_mo_gpu_prescan_narrows_large_len_ranges -- --nocapture`
- `cargo test -p ta_app_core composite_mo_gpu_prescan_uses_exact_grid_for_small_candidate_sets -- --nocapture`
- `cargo test -p ta_app_core composite_mo_candidates_validate_consistently -- --nocapture`

If decode/repair behavior changes:

- `cargo test -p ta_app_core composite_mo_decodes_categorical_numeric_and_bool_params -- --nocapture`
- `cargo test -p ta_app_core composite_mo_decodes_stringified_categorical_numeric_and_bool_params -- --nocapture`
- `cargo test -p ta_app_core composite_mo_repairs_macz_order_before_exact_grid_eval -- --nocapture`
- `cargo test -p ta_app_core composite_mo_pair_aware_repair_chooses_nearest_allowed_pair -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- do not remove the final `Trade22` rewrite merely to lower runtime
- do not change prescan narrowing rules without treating them as search-domain semantics changes
- do not repurpose telemetry field names without considering downstream consumers and existing tests
- preserve current sampler coverage and result-shape compatibility unless a broader API change is explicitly approved
