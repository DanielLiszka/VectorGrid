# Review Packet: `crates/app_core/src/double_ma_ma_kind.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_ma_kind.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Small, clear support helper that is close to accept-as-is; the remaining work is mostly around boundary documentation and a slightly broader negative-test matrix`
- Recommended action bucket: `Accept as-is`
- Sensitivity tags: `Internal-support`, `Low-risk`

## What This File Owns

This module owns moving-average lookup and validation for the Double-MA optimizer.

It owns:

- registry lookup:
  - `lookup_ma_info`: `crates/app_core/src/double_ma_ma_kind.rs:1-7`
- Double-MA suitability validation:
  - `validate_ma_kind`: `9-23`

## Classification

- Public surface: no
  - `pub(crate)` only
- Hot path: no
- CUDA/runtime-critical: no
- Strategy-local: yes
- Low-risk support code: yes

## Direct Interactions

- consumed from:
  - `crates/app_core/src/double_ma.rs:23`
- backed by:
  - `vector_ta::indicators::moving_averages::registry`

## Standards And Research Log

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this helper gates a user-visible MA selection contract
   - Standard informed:
     - validation errors should be clear and specific

2. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - this module is pure validation logic returning `Result<(), String>`
   - Standard informed:
     - invalid input should fail explicitly and descriptively

3. Rust Book, How to Write Tests
   - Source: <https://doc.rust-lang.org/book/ch11-01-writing-tests.html>
   - Why it was consulted:
     - boundary validators are best protected with small focused negative tests
   - Standard informed:
     - reject-path behavior should be pinned

## Findings

## Correctness And Safety

### 1. No objective correctness issue was identified

- Evidence:
  - unknown, non-period-based, and multi-output rejection logic is straightforward:
    - `9-23`

## Docs And Examples

### 1. The contract is clear in code but undocumented

- Smallest justified fix:
  - add a short module comment that Double-MA only accepts period-based, single-output MAs

## Tests And Missing Verification

### 1. Tests do not explicitly lock non-period-based or multi-output rejection paths

- Evidence:
  - current tests cover:
    - valid MA acceptance: `29-37`
    - unknown MA rejection: `39-43`
- Smallest justified fix:
  - add registry-driven negative tests for:
    - a known non-period-based MA, if present
    - a known multi-output MA, if present

## Recommended Action

`Accept as-is`

The implementation is already small and coherent. The only worthwhile additions are a short ownership comment and a broader negative-test matrix.

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core validate_ma_kind -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- preserve current error wording unless callers or tests are updated intentionally
- do not broaden accepted MA shapes without reviewing downstream optimizer assumptions
