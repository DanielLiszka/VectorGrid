# Review Packet: `crates/app_core/src/double_ma_test_support.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_test_support.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Thin and low-risk test-support shim, but not fully review-ready as-is because it duplicates the preset-builder surface from strategy_presets.rs without documenting why this extra wrapper layer exists or which tests are supposed to depend on it`
- Recommended action bucket: `Document only`
- Sensitivity tags: `Test-support`, `Internal-test-contract`, `Low-throughput-risk`

## What This File Owns

`double_ma_test_support.rs` owns a small test-facing forwarding layer that exposes representative Double-MA-family preset builders to the large `double_ma.rs` test module via `pub(super)` helpers.

It owns:

- forwarded representative strategy presets:
  - `trend_quality_strategy_for_kernel`: `crates/app_core/src/double_ma_test_support.rs:4-6`
  - `regime_hybrid_strategy_for_kernel`: `9-11`
  - `breakout_strategy_for_kernel`: `14-16`
  - `vfct_strategy_for_kernel`: `18-20`
  - `marrt_strategy_for_kernel`: `22-24`
  - `pit_strategy_for_kernel`: `26-28`
  - `set_strategy_for_kernel`: `30-32`
  - `mts_strategy_for_kernel`: `34-36`
  - `orc_strategy_for_kernel`: `38-40`
- forwarded case list:
  - `double_like_kernel_strategy_cases`: `42-44`

This file does not own:

- the actual preset definitions:
  - `crates/app_core/src/strategy_presets.rs:3-108`
- the preset validation/route-compilation tests:
  - `crates/app_core/src/strategy_presets.rs:110-249`
- the runtime logic exercised by those presets:
  - `crates/app_core/src/double_ma.rs`

## Classification

- Public surface: no
  - `pub(super)` only
- Hot path: no
  - test support only
- CUDA/runtime-critical: no direct runtime ownership
  - helps exercise GPU-capable preset routes in tests, but never runs in production paths
- Strategy-local: yes
  - Double-MA-family preset test helpers
- Low-risk support code: yes

## Direct Interactions

### Backend interaction map

- included by the main Double-MA module:
  - `crates/app_core/src/double_ma.rs:78`
- consumed from the large `double_ma.rs` test surface:
  - grouped import: `crates/app_core/src/double_ma.rs:14122-14126`
  - representative uses:
    - `20662`
    - `23629-23634`
- forwards entirely into:
  - `crates/app_core/src/strategy_presets.rs:3-108`

### Downstream verification impact if changed

- changes here primarily affect:
  - `double_ma.rs` unit/integration-style tests
- if wrappers are removed or renamed, update:
  - the large `double_ma.rs` test module
  - any nearby `super::double_ma_test_support::*` imports

## Standards And Research Log

This packet was written after targeted web research focused on test-organization guidance and Rust API clarity around duplicated ownership surfaces.

### Sources consulted

1. Rust Book, Test Organization
   - Source: <https://doc.rust-lang.org/book/ch11-03-test-organization.html>
   - Why it was consulted:
     - this file exists purely to support tests, so the key standard is clarity of test ownership and organization
   - Standard informed:
     - test-support modules should make their scope and intended consumers explicit

2. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - even internal support helpers benefit from clear ownership and low-ambiguity naming
   - Standard informed:
     - duplicated entry surfaces should be intentionally bounded

3. Rust API Guidelines, Documentation
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this file duplicates another module’s surface with almost no explanation
   - Standard informed:
     - helper layers should document why they exist and what contract they protect

## Findings

## Correctness And Safety

### 1. No in-file correctness defect was proven

- Evidence:
  - every helper is a direct forwarder into `strategy_presets.rs`
  - there is no mutation, caching, or derived logic in this file
- Review judgment:
  - current implementation is behaviorally safe

## API And Naming

### 1. The naming is clear locally, but the module creates a second API surface for the same presets

- Evidence:
  - each helper is a one-to-one forwarder into `strategy_presets.rs`
  - examples:
    - `trend_quality_strategy_for_kernel`: `crates/app_core/src/double_ma_test_support.rs:4-6`
    - upstream owner: `crates/app_core/src/strategy_presets.rs:3-8`
    - `double_like_kernel_strategy_cases`: `42-44`
    - upstream owner: `strategy_presets.rs:98-108`
- Why it matters:
  - maintainers now have two import surfaces for the same preset builders:
    - the real owner (`strategy_presets.rs`)
    - the local test shim (`double_ma_test_support.rs`)
  - that is manageable, but should be clearly intentional
- Smallest justified fix:
  - document that this module exists only to keep the large `double_ma.rs` test surface decoupled from the broader preset-owner namespace
  - if that indirection is no longer needed, remove it in a small coordinated cleanup rather than letting both surfaces drift indefinitely

## Module Boundaries And Technical Debt

### 1. `#[allow(dead_code)]` on some wrappers signals partial drift or legacy compatibility

- Evidence:
  - `trend_quality_strategy_for_kernel`: `crates/app_core/src/double_ma_test_support.rs:3`
  - `regime_hybrid_strategy_for_kernel`: `8`
  - `breakout_strategy_for_kernel`: `13`
- Why it matters:
  - the wrappers are cheap, but the `dead_code` suppressions indicate this layer is not obviously the canonical import path even inside tests
  - that makes the module look more like leftover indirection than a clearly owned test boundary
- Smallest justified fix:
  - document why these helpers remain here
  - if they truly are no longer needed in this shim, remove them only after updating the consuming tests
- Review judgment:
  - low-severity tech-debt issue

### 2. The real preset ownership lives in `strategy_presets.rs`, not here

- Evidence:
  - preset definitions and their dedicated tests are all located in:
    - `crates/app_core/src/strategy_presets.rs:3-249`
- Why it matters:
  - this file should stay deliberately thin
  - any future logic added here would create a third layer of preset semantics and would likely be the wrong place for it
- Smallest justified fix:
  - keep this module as a forwarding boundary only
  - put any new preset logic or invariant tests in `strategy_presets.rs`

## Docs And Examples

### 1. The module needs a brief comment explaining why it exists separately from `strategy_presets.rs`

- Why it matters:
  - without a short note, a future maintainer can easily see this file as pointless duplication and remove or bypass it without understanding the current test organization choice
- Smallest justified fix:
  - add a module comment describing:
    - this is a `double_ma.rs` test-support shim
    - preset definitions remain owned by `strategy_presets.rs`

## Tests And Missing Verification

### 1. This module has no local tests, but its forwarded contract is currently well covered through the real owner

- Evidence:
  - targeted preset-owner runs succeeded:
    - `cargo test -p ta_app_core double_like_kernel_strategy_cases -- --nocapture`
    - result: `2` tests passed
    - `cargo test -p ta_app_core representative_composite_presets_compile_to_expected_gpu_indicator_routes -- --nocapture`
    - result: `1` test passed
  - those tests live in:
    - `crates/app_core/src/strategy_presets.rs:120-249`
- Review judgment:
  - current verification is adequate for a forwarding shim
  - adding local tests here would likely duplicate the real owner’s coverage

## Performance, Allocation, And CUDA Concerns

### 1. No performance issue was identified in this file

- Review judgment:
  - not a performance surface

## Code-Quality Improvement Register

- Safe now:
  - add a module-level ownership comment
  - document whether the `dead_code` allowances are intentional compatibility shims
- Coordination required:
  - removing or renaming wrappers used by `double_ma.rs` tests
- Defer until later pass:
  - collapsing this file entirely, if the test organization is deliberately simplified

## Technical-Debt Reduction Register

- Safe to reduce now:
  - ambiguity about why this module exists
- Coordination required:
  - wrapper removal or import-surface consolidation
- Useful debt that should not be “cleaned up” casually:
  - the real preset-owner tests in `strategy_presets.rs`; those are doing meaningful route-contract work

## Performance Opportunity Register

- Safe structural improvements:
  - none identified
- Benchmark-sensitive candidates:
  - none identified
- Not the right place to chase performance:
  - this module should remain a pure test-support shim

## Observability And Diagnostics Gaps

- no runtime observability issue was identified
- the main gap is documentation of ownership, not missing diagnostics

## Implementation Staging Guidance

- Safe now:
  - module comment
  - clarify intent of `dead_code` allowances
- Coordination required:
  - wrapper deletion or import-path collapse
- Defer until later pass:
  - broader test-organization simplification, if desired

## Change Blast Radius

- Local only:
  - comments or local cleanup inside this shim
- Same subsystem:
  - `double_ma.rs` tests that import the shim directly
- Cross-backend/runtime:
  - none

## Open Questions Or Assumptions

- assumption:
  - this module exists to keep the enormous `double_ma.rs` test surface locally organized without making `strategy_presets.rs` the only import anchor
- open question:
  - whether the three `#[allow(dead_code)]` wrappers are intentionally reserved for nearby tests or are simply legacy leftovers

## Recommended Action

`Document only`

The file is safe and intentionally lightweight. The main improvement is to make the ownership boundary explicit so this forwarding layer does not look accidental.

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core double_like_kernel_strategy_cases -- --nocapture`
- `cargo test -p ta_app_core representative_composite_presets_compile_to_expected_gpu_indicator_routes -- --nocapture`
- rerun any `double_ma.rs` tests that import `super::double_ma_test_support::*` if wrapper names or visibility change

## Regression Risk And Feature-Preservation Constraints

- do not move preset logic into this file
- do not casually delete wrappers that still anchor `double_ma.rs` tests
- preserve the current preset labels and preset semantics unless the real owner in `strategy_presets.rs` is updated together
