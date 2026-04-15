# Review Packet: `crates/app_core/src/double_ma_backend.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_backend.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Internally coherent support module, but not fully review-ready as-is because it contains hidden routing-policy constants, lightly misleading unused parameters, and duplicated capability-report behavior that could drift from the stronger route owners`
- Recommended action bucket: `Document only`
- Sensitivity tags: `Internal-policy-surface`, `Route-contract-sensitive`, `Low-throughput-risk`

## What This File Owns

`double_ma_backend.rs` owns small policy helpers that translate Double-MA backend and hybrid-indicator routing state into normalized labels, blocker lists, and lightweight capability summaries for the surrounding optimization code.

This file owns:

- backend label normalization:
  - `backend_kind_label`: `crates/app_core/src/double_ma_backend.rs:10-19`
- hybrid enablement flag parsing:
  - `hybrid_backend_enabled`: `21-30`
- kernel blocker list construction:
  - `gpu_kernel_topk_supported`: `32-38`
  - `gpu_kernel_blockers_for_request`: `40-79`
- strategy capability report assembly:
  - `build_strategy_capability_report_for_request`: `81-108`
- hybrid route/capability/staging labeling:
  - `map_hybrid_capability_label`: `110-117`
  - `map_hybrid_route_label`: `119-125`
  - `indicator_staging_reason`: `127-148`
  - `should_cpu_stage_indicator`: `150-167`
  - `build_indicator_route_plan_summary`: `169-208`

## Classification

- Public surface: no
  - everything is `pub(crate)` or private
- Hot path: low
  - policy helper code only
- CUDA/runtime-critical: indirect
  - affects route selection and route explanations, but does not execute kernels
- Strategy-local: yes
  - Double-MA backend policy only
- Low-risk support code: mostly yes

## Direct Interactions

- consumed from the main optimization hub:
  - `crates/app_core/src/double_ma.rs:18-22`
- depends on strategy semantics:
  - `ta_strategies::double_ma::StrategyConfig`
- depends on MA capability planning:
  - `crates/app_core/src/ma_policy.rs`
- capability report shape comes from:
  - `crates/app_core/src/strategy_gpu_runtime.rs`

## Standards And Research Log

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this module defines policy helpers whose outputs are consumed by larger runtime surfaces
   - Standard informed:
     - helper semantics should be predictable and narrowly named

2. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - route/blocker helpers can drift if their policy assumptions stay implicit
   - Standard informed:
     - internal policy helpers still benefit from explicit ownership and caveat notes

3. Rust Book, How to Write Tests
   - Source: <https://doc.rust-lang.org/book/ch11-01-writing-tests.html>
   - Why it was consulted:
     - this file’s main quality bar is the accuracy of its policy matrix and blocker explanations
   - Standard informed:
     - route/blocker matrices should be pinned with focused tests

## Findings

## Correctness And Safety

### 1. No in-file correctness defect was proven

- Evidence:
  - blocker logic and route summary construction are straightforward and already tested:
    - `210-300`
- Review judgment:
  - current behavior is coherent

## API And Naming

### 1. `gpu_kernel_blockers_for_request` and `build_strategy_capability_report_for_request` advertise broader policy inputs than they currently honor

- Evidence:
  - `_objective` is accepted but unused:
    - `crates/app_core/src/double_ma_backend.rs:45`
  - `_score_mode` is accepted but unused:
    - `88`
- Why it matters:
  - the signatures imply that objective and score mode affect kernel viability, but today they do not
  - that can mislead future callers into assuming the helper already covers those policy lanes
- Smallest justified fix:
  - document that those arguments are currently compatibility placeholders
  - if they are no longer needed, remove them only after checking all call sites
- Review judgment:
  - low-severity API clarity issue

## Module Boundaries And Technical Debt

### 1. Capability reporting here is a lightweight policy mirror and could drift from stronger route owners

- Evidence:
  - `build_strategy_capability_report_for_request` is marked `#[allow(dead_code)]`:
    - `81`
  - it derives support from the same blocker helper used elsewhere, but does not incorporate all runtime route adjustments from `double_ma.rs`
- Why it matters:
  - dead or lightly used mirrors are a common drift source in routing code
- Smallest justified fix:
  - keep the helper only if it is still needed
  - otherwise remove or bind it to parity tests with the main route-resolution path
- Review judgment:
  - document/defer

### 2. The top-k kernel threshold is a hidden policy constant

- Evidence:
  - `top_k <= 256` when `include_all=false`:
    - `32-38`
    - surfaced as blocker text at `63-66`
- Why it matters:
  - this is a meaningful route-policy constraint, but the rationale is not documented here
- Smallest justified fix:
  - document that the cap is a route-policy limit and should only move with corresponding kernel/runtime verification
- Review judgment:
  - benchmark-sensitive policy constant; do not casually change

## Docs And Examples

### 1. Route-label and blocker policy should be documented at module level

- Why it matters:
  - this file is the narrow contract layer between `ma_policy` and the larger optimization hub
- Smallest justified fix:
  - add module comments describing:
    - what this file owns
    - what the main `double_ma.rs` route resolver still owns

## Tests And Missing Verification

### 1. Existing tests cover positive/blocker basics, but not env-flag parsing or top-k threshold boundaries

- Evidence:
  - tests currently cover plain support and common blockers:
    - `215-300`
  - no direct test for `VECTORBT_ENABLE_GPU_HYBRID_KERNEL`
  - no threshold-boundary test around `top_k == 256` / `257`
- Smallest justified fix:
  - add focused tests for env parsing and top-k boundary behavior

## Performance, Allocation, And CUDA Concerns

### 1. No performance issue was identified in this file

- Review judgment:
  - not the place to chase throughput

## Recommended Action

`Document only`

The file is doing the right kind of work, but its policy boundaries and hidden constants should be documented more explicitly.

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core capability_report_marks_gpu_supported_for_plain_sma_request -- --nocapture`
- any added boundary/env tests in this module

## Regression Risk And Feature-Preservation Constraints

- do not change blocker semantics casually
- treat the top-k kernel threshold as benchmark-sensitive
- preserve current route-summary vocabulary unless downstream diagnostics are updated together
