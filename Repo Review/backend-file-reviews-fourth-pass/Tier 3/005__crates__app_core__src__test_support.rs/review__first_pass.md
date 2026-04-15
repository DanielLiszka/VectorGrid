# Review Packet: `crates/app_core/src/test_support.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/test_support.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Tiny internal helper, but not fully review-ready because the lock-poisoning policy and cross-suite coordination role are implicit`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Test-infrastructure`, `Global-state`, `Semantics-sensitive`, `Low-compute`

## What This File Owns

`test_support.rs` owns one thing:

- a process-global environment/test-state lock for backend tests:
  - `lock_test_env`: `crates/app_core/src/test_support.rs:4-9`

Although the file is tiny, the helper is used by many backend tests that mutate or depend on global process state.

## Classification

- Public surface: no
  - `pub(crate)` and `#[cfg(test)]`
- Hot path: no
- Runtime-critical: no
- Test-critical: yes
  - failures here fan out into many suites

## Direct Interactions

### Backend interaction map

- current users include:
  - `crates/app_core/src/strategy_dynamic.rs`
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/composite_cpu.rs`
  - `crates/app_core/src/composite_mo.rs`
  - `crates/app_core/src/composite_validate.rs`
  - `crates/app_core/src/options_expression.rs`
  - `crates/app_core/src/options_expression_gpu.rs`
  - `crates/app_core/src/ml_signal.rs`

### Downstream verification impact if changed

- changing lock behavior changes how env-sensitive tests fail, serialize, or cascade after a panic

## Standards And Research Log

This packet was written after targeted web research focused on lazy test helpers and mutex poison semantics.

### Sources consulted

1. `std::sync::OnceLock`
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why it was consulted:
     - the helper lazily initializes a global `Mutex`
   - Standard informed:
     - the helper is correctly using `OnceLock` for one-time setup

2. `std::sync::Mutex`
   - Source: <https://doc.rust-lang.org/std/sync/struct.Mutex.html>
   - Why it was consulted:
     - this helper currently aborts on poison
   - Standard informed:
     - mutex poisoning is real state that callers may choose to recover from rather than always propagating

3. Rust Book testing chapter
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why it was consulted:
     - this helper exists purely to support test reliability
   - Standard informed:
     - test infrastructure should minimize cascading false failures

## Findings

## Correctness And Test Reliability

### 1. One failing env-sensitive test can poison the lock and cascade into unrelated later failures

- Evidence:
  - the helper does `lock().expect("test env lock poisoned")`: `7-9`
  - in practice, `strategy_dynamic` default-threaded suite currently shows this exact cascade:
    - first failure:
      - `prepare_runnable_dynamic_double_ma_plan_defers_cpu_context_until_cpu_execution`
    - later failures:
      - `preview_dynamic_double_ma_strategy_ir_falls_back_to_cpu_for_halftrend_shape`
      - `preview_dynamic_double_ma_strategy_ir_falls_back_to_cpu_for_non_cross_exit_mode`
      - both fail with `test env lock poisoned`
- Why this matters:
  - a single real assertion failure becomes multiple noisy follow-on failures
  - that increases triage cost and can hide the original root cause
- Smallest justified fix:
  - recover from poison for test infrastructure, for example by taking the poisoned guard’s inner value instead of panicking immediately

## Contract Clarity

### 2. The helper’s role as a cross-suite global-state coordinator is not documented

- Evidence:
  - the file contains only the helper implementation, with no rustdoc or comments
  - many modules use it, implying repo-wide test-state coordination rather than a file-local helper
- Why this matters:
  - maintainers need to know this lock is protecting process-global env/state, not arbitrary shared data
  - without that context, duplicate ad hoc test locks are more likely
- Smallest justified fix:
  - add rustdoc stating that this lock is for env/global-process-state mutation in tests

## Tests And Missing Verification

### 3. There is no direct file-local test, so behavior is only exercised transitively

- Evidence:
  - `test_support.rs` has no file-local tests
- Why this matters:
  - this is acceptable for a tiny helper, but it raises the value of clear documentation and conservative behavior
- Smallest justified fix:
  - at minimum, document poison policy
  - optionally add a narrow test in one consumer module that guards expected recovery behavior

## Code-Quality Improvement Register

- Safe now:
  - document helper purpose
  - stop turning poison into a second failure if the repo wants better test triage
- Defer:
  - introducing a larger env-guard abstraction unless test-state complexity keeps growing

## Technical-Debt Reduction Register

- Safe now:
  - rustdoc
  - poison-recovery behavior for test-only infrastructure
- Defer:
  - broad test-state harness redesign
- Do not casually remove:
  - the single shared lock itself; several suites rely on it

## Performance Opportunity Register

- Safe structural work:
  - none needed
- Benchmark-sensitive:
  - none

## Recommended Action

`Small surgical refactor`

This helper is small, but it is worth hardening because it amplifies failures across multiple suites. The main improvement is better failure isolation, not architectural change.

## Exact Local Verification Required If Changed

1. Current sensitive reproducer:
   - `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture`
2. Serialized comparison run:
   - `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture --test-threads=1`

### Verification run for this packet

- Failed:
  - `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture`
- Passed:
  - `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture --test-threads=1`

## Regression Risk And Feature-Preservation Constraints

- Do not remove the shared test lock without auditing all env-sensitive test users
- If you recover from poison, keep the helper test-only and document that the choice is for better failure isolation, not production semantics
