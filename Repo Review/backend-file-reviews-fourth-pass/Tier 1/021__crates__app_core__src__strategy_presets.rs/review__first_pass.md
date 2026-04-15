# Review Packet: `crates/app_core/src/strategy_presets.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_presets.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful preset/support module, but not fully review-ready because preset intent, inclusion rules, and parameter rationale are only implicit`
- Recommended action bucket: `Document only`
- Sensitivity tags: `Preset-contract`, `Test-support`, `Benchmark-inputs`, `Low-compute`

## What This File Owns

`strategy_presets.rs` owns a small set of canonical backend preset helpers used by tests, benches, and profiling tools:

- per-strategy preset constructors:
  - `3-96`
- grouped “double-like kernel” preset list:
  - `98-108`
- file-local tests:
  - `111-246`

## Classification

- Public surface: effectively yes
  - used by `double_ma.rs`, benches, and several backend helper binaries
- Hot path: no
- CUDA/runtime-critical: indirectly
  - these presets anchor route and parity coverage, but do not execute runtime logic themselves
- Support code: yes
  - mainly preset fixtures and representative cases

## Direct Interactions

### Backend interaction map

- `double_ma.rs` and `double_ma_validate.rs` use these presets heavily
- backend helper binaries use representative presets:
  - `src-tauri/src/bin/ta_profile_route_matrix.rs`
  - `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
  - `crates/app_core/src/bin/stage6b_kernel_report.rs`
  - `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- benches also use additional preset helpers:
  - `crates/app_core/benches/double_ma_opt.rs`

### Downstream verification impact if changed

- changing preset flags or numeric defaults alters representative route/parity coverage
- changing which presets participate in `double_like_kernel_strategy_cases` changes multiple broad test loops

## Standards And Research Log

This packet was written after targeted web research focused on internal API documentation standards and serialized/planner contract surfaces that these presets feed.

### Sources consulted

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - these helpers are low-complexity, but they encode representative backend cases
   - Standard informed:
     - even small support helpers should explain behaviorally important intent when many tests and tools depend on them

2. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why it was consulted:
     - preset helpers influence many downstream tests and route checks
   - Standard informed:
     - support fixtures should make their intended coverage explicit to avoid accidental narrowing

## Findings

## Contract Clarity

### 1. The file does not explain why these exact presets are the canonical representative cases

- Evidence:
  - helpers set specific flags and magic numeric values directly: `3-96`
  - the grouped case list at `98-108` has no comment explaining its inclusion criteria
- Why this matters:
  - these helpers are used as canonical backend fixtures, not casual examples
  - without documentation, maintainers may “clean up” numeric values or flags that are actually chosen to preserve route coverage
- Smallest justified fix:
  - add short comments describing what each preset is representative of and why its parameters should be treated as deliberate

## Module Boundaries And Test Coverage

### 2. `double_like_kernel_strategy_cases` is intentionally narrower than the full helper set, but that intent is easy to miss

- Evidence:
  - helpers exist for `vfct` and `marrt`: `32-49`
  - `double_like_kernel_strategy_cases` includes:
    - `composite_trend`
    - `regime_hybrid`
    - `breakout_momentum`
    - `pit`
    - `set`
    - `mts`
    - `orc`
  - and excludes `vfct` / `marrt`: `98-108`
  - those excluded helpers are still used elsewhere in `double_ma.rs`, `double_ma_validate.rs`, and benches
- Why this matters:
  - a maintainer could incorrectly conclude the excluded helpers are dead code
  - another maintainer could incorrectly add them to the grouped set and widen broad route loops without meaning to
- Smallest justified fix:
  - document that the grouped list is a curated representative subset, not an inventory of every preset helper

## Tests And Missing Verification

### 3. The tests do a good job locking down route expectations for the grouped cases

- Evidence:
  - label uniqueness and GPU-route expectations are tested at `120-246`
- Residual gap:
  - the file does not document why grouped-case tests exist separately from standalone preset helpers used elsewhere
- Smallest justified fix:
  - add a brief comment above `double_like_kernel_strategy_cases`

## Code-Quality Improvement Register

- Safe now:
  - document preset intent
  - document why the grouped case list is curated
  - document that numeric defaults are representative fixtures, not arbitrary values
- Defer:
  - expanding or reducing grouped preset coverage

## Technical-Debt Reduction Register

- Safe now:
  - comments and rustdoc only
- Defer:
  - regrouping preset families
- Do not casually remove:
  - `vfct_strategy_for_kernel`
  - `marrt_strategy_for_kernel`
  - grouped-case membership choices

## Performance Opportunity Register

- Safe structural work:
  - none needed
- Benchmark-sensitive:
  - changing preset membership or parameters can invalidate benchmark comparability

## Recommended Action

`Document only`

The code is fine. The main risk is accidental coverage drift caused by undocumented preset intent, not faulty implementation.

## Exact Local Verification Required If Changed

1. File-local preset tests:
   - `cargo test -p ta_app_core strategy_presets::tests:: -- --nocapture`
2. Representative Double-MA route/parity coverage:
   - `cargo test -p ta_app_core double_ma -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core strategy_presets::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change preset numeric values or flags without checking the route/parity surfaces that consume them
- Do not treat absence from `double_like_kernel_strategy_cases` as proof that a helper is unused
- Do not widen the grouped list casually; broad route-loop coverage depends on it
