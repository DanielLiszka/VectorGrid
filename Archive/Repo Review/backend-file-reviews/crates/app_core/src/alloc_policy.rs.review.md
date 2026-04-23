# Review Packet: `crates/app_core/src/alloc_policy.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/alloc_policy.rs`
- Packet status: `Reviewed`
- Public-review readiness: `High-leverage allocation helper for hot CPU/GPU paths, but not public-review-ready because several release-mode helpers currently materialize Vec<T> values whose full initialized-state contract is not upheld`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Unsafe-sensitive`, `Performance-sensitive`, `CUDA-runtime-sensitive`, `Allocation-policy-sensitive`

## What This File Owns

`alloc_policy.rs` owns low-level allocation helpers used to avoid full initialization on overwrite-heavy numeric paths in `ta_app_core`.

This file owns:

- raw conversion from `Vec<MaybeUninit<T>>` to `Vec<T>`:
  - `assume_init_vec`: `crates/app_core/src/alloc_policy.rs:15-20`
- reserve/resize helpers used before manual overwrite:
  - `reserve_for_uninit_resize`: `25-29`
  - `resize_uninit_f32`: `134-149`
  - `resize_uninit_f64`: `152-167`
  - `resize_uninit_i32`: `170-185`
- scalar buffer allocation helpers:
  - `alloc_uninit_copy`: `33-39`
  - `alloc_uninit_f64`: `94-109`
  - `alloc_uninit_f32`: `112-127`
- prefix-initialized tail-uninitialized helpers:
  - `alloc_f64_with_prefix_uninit_tail`: `43-66`
  - `alloc_f32_with_prefix_uninit_tail`: `69-92`
- matrix allocation helpers returning `Vec<MaybeUninit<_>>`:
  - `make_uninit_matrix_f64`: `188-214`
  - `make_uninit_matrix_f32`: `217-243`
  - `init_matrix_prefixes_f64`: `246-277`
  - `init_matrix_prefixes_f32`: `280-311`
- local unit tests:
  - `321-425`

This file does not own:

- the numeric algorithms that later overwrite these buffers
  - those live in modules such as `double_ma.rs`, `ml_signal.rs`, `gpu_backtest_kernel.rs`, `vcc_gpu_kernel.rs`, and `vemr_gpu_kernel.rs`
- device allocation itself
  - that lives in CUDA/runtime modules using `cust`

## Classification

- Public surface: internal-only
- Hot path: yes
  - this file exists specifically to support performance-sensitive loops
- Runtime-critical: high
  - mistakes here can invalidate entire numeric pipelines
- CUDA/runtime-critical: high
  - multiple GPU runtime modules depend on these helpers
- Strategy-local: no
- Low-risk support code: no
  - this is low-level unsafe allocation policy

## Direct Interactions

### Backend interaction map

- heavy CPU-path use in `double_ma.rs`:
  - `crates/app_core/src/double_ma.rs:8080`
  - `crates/app_core/src/double_ma.rs:8339`
  - `crates/app_core/src/double_ma.rs:8472`
  - `crates/app_core/src/double_ma.rs:10473`
  - `crates/app_core/src/double_ma.rs:10581`
  - `crates/app_core/src/double_ma.rs:10852`
  - `crates/app_core/src/double_ma.rs:11894`
- ML signal helpers:
  - `crates/app_core/src/ml_signal.rs:549`
  - `crates/app_core/src/ml_signal.rs:799`
  - `crates/app_core/src/ml_signal.rs:807`
  - `crates/app_core/src/ml_signal.rs:2023-2025`
- GPU/runtime consumers:
  - `crates/app_core/src/gpu_backtest_kernel.rs:28-30`
  - `crates/app_core/src/gpu_backtest_kernel.rs:1477`
  - `crates/app_core/src/gpu_backtest_kernel.rs:1554`
  - `crates/app_core/src/gpu_backtest_kernel.rs:2532`
  - `crates/app_core/src/gpu_backtest_kernel.rs:2815`
  - `crates/app_core/src/gpu_backtest_kernel.rs:6227-6228`
  - `crates/app_core/src/gpu_backtest_kernel.rs:13311`
  - `crates/app_core/src/gpu_backtest_kernel.rs:15655-15660`
  - `crates/app_core/src/standalone_gpu_runtime.rs:1-4`
  - `crates/app_core/src/standalone_gpu_runtime.rs:91-93`
  - `crates/app_core/src/vcc_gpu_kernel.rs:3`
  - `crates/app_core/src/vcc_gpu_kernel.rs:124`
  - `crates/app_core/src/vcc_gpu_kernel.rs:193`
  - `crates/app_core/src/vemr_gpu_kernel.rs:3`
  - `crates/app_core/src/vemr_gpu_kernel.rs:126-134`
- other numeric consumers:
  - `crates/app_core/src/composite_mo.rs:1636`

### Downstream verification impact if changed

- any soundness-preserving refactor will ripple into:
  - `double_ma.rs`
  - `ml_signal.rs`
  - `gpu_backtest_kernel.rs`
  - `standalone_gpu_runtime.rs`
  - `vcc_gpu_kernel.rs`
  - `vemr_gpu_kernel.rs`
- any type-signature change from `Vec<T>` to `Vec<MaybeUninit<T>>` is inherently multi-file

### Test anchors

- local tests:
  - `crates/app_core/src/alloc_policy.rs:321-425`

## Standards And Research Log

This packet was written after targeted external review focused on Rust's uninitialized-memory rules, `Vec` length invariants, and safer low-level capacity APIs for overwrite-heavy paths.

### Sources consulted

1. `std::mem::MaybeUninit`
   - Source: <https://doc.rust-lang.org/std/mem/union.MaybeUninit.html>
   - Why it was consulted:
     - this file uses `MaybeUninit` as the basis for nearly every helper
   - Standard informed:
     - uninitialized memory must not be treated as initialized `T` until the initialization contract is actually satisfied

2. `Vec::set_len`
   - Source: <https://doc.rust-lang.org/std/vec/struct.Vec.html#method.set_len>
   - Why it was consulted:
     - multiple helpers set vector length before caller overwrite
   - Standard informed:
     - `set_len` is only sound when the newly included range is initialized before the vector is treated as containing `T`

3. `Vec::from_raw_parts`
   - Source: <https://doc.rust-lang.org/std/vec/struct.Vec.html#method.from_raw_parts>
   - Why it was consulted:
     - `assume_init_vec` reconstructs a `Vec<T>` from a `Vec<MaybeUninit<T>>`
   - Standard informed:
     - the first `length` values must already be properly initialized values of type `T`

4. `Vec::spare_capacity_mut`
   - Source: <https://doc.rust-lang.org/std/vec/struct.Vec.html#method.spare_capacity_mut>
   - Why it was consulted:
     - it is the standard-library API intended for filling reserved memory as `MaybeUninit<T>` before a later `set_len`
   - Standard informed:
     - sound overwrite-heavy code should generally keep memory typed as `MaybeUninit<T>` until initialization is complete

5. Rustonomicon: Working With Uninitialized Memory
   - Source: <https://doc.rust-lang.org/nomicon/uninitialized.html>
   - Why it was consulted:
     - this file is effectively a repo-wide uninitialized-memory policy layer
   - Standard informed:
     - interpreting uninitialized memory as a value of type `T` is undefined behavior

## Findings

## Correctness And Safety

### 1. Several release-mode helpers currently create `Vec<T>` values whose full element range is not initialized

- Evidence:
  - `assume_init_vec` rebuilds a `Vec<T>` from `Vec<MaybeUninit<T>>` using the full current length:
    - `crates/app_core/src/alloc_policy.rs:15-20`
  - `alloc_uninit_copy` sets `len` and immediately returns `Vec<T>`:
    - `33-39`
  - `alloc_uninit_f64` and `alloc_uninit_f32` do the same:
    - `94-109`
    - `112-127`
  - `resize_uninit_f32`, `resize_uninit_f64`, and `resize_uninit_i32` extend length without initializing the grown tail:
    - `134-185`
  - prefix helpers initialize only the prefix, then still return a full-length `Vec<f32>` / `Vec<f64>`:
    - `43-66`
    - `69-92`
- Why it matters:
  - according to `Vec::from_raw_parts`, the first `length` values must already be properly initialized values of type `T`
  - according to the Rustonomicon, interpreting uninitialized memory as a value of type `T` is undefined behavior
  - these helpers are intentionally performance-oriented, but their release-mode contract currently exceeds what the standard library documents as sound
- Smallest justified fix:
  - the smallest boundary-hardening fix is to make the `Vec<T>`-returning and `Vec<T>`-resizing uninit helpers `unsafe fn` and add explicit `# Safety` docs requiring full overwrite before any read or borrow
  - the fully sound direction is to keep buffers as `MaybeUninit<T>` until initialization is complete and only then expose them as `Vec<T>`
  - that stronger direction likely requires a broader refactor across consumers, so it is not a one-line patch
- Review judgment:
  - objective soundness issue

### 2. `alloc_uninit_copy<T: Copy>` is too generic for the actual safety contract

- Evidence:
  - helper signature:
    - `crates/app_core/src/alloc_policy.rs:33`
  - current call sites happen to use primitive numeric types such as `f32` and `i32`:
    - `gpu_backtest_kernel.rs:6227-6228`
    - `gpu_backtest_kernel.rs:15655-15660`
    - `standalone_gpu_runtime.rs:91`
- Why it matters:
  - `Copy` does not mean “all bit patterns are valid” or “safe to treat as uninitialized until overwrite”
  - a future caller could use a `Copy` type with stricter validity requirements and inherit unsound behavior
- Smallest justified fix:
  - narrow this helper to explicitly supported primitive types
  - or make the contract unsafe and document exactly what callers must guarantee
- Review judgment:
  - important API-safety issue, even if current call sites are numeric

## API And Naming

### 1. Several helpers read like ordinary allocation utilities even though they carry unsafe initialization contracts

- Evidence:
  - names such as:
    - `alloc_uninit_f64`
    - `resize_uninit_f32`
    - `alloc_f64_with_prefix_uninit_tail`
  - all are safe `pub(crate)` functions
- Why it matters:
  - the current naming hints at uninitialized behavior, but it does not express that misuse can cross into UB territory
  - the private `assume_init_vec` helper is also safe despite having a strict invariant
- Smallest justified fix:
  - at minimum, add explicit safety-contract comments at each helper
  - consider making the raw conversion helper unsafe even if it remains private
- Review judgment:
  - documentation/naming debt that increases maintenance risk

## Error Handling

### 1. OOM/overflow paths intentionally panic, which is acceptable here but should stay understood as policy

- Evidence:
  - matrix helpers use:
    - `checked_mul(...).expect(...)`: `190-192`, `219-221`
    - `try_reserve_exact(...).expect(...)`: `194-196`, `223-225`
- Why it matters:
  - this is a low-level allocation module, so panicking on impossible-size/oom conditions can be acceptable
  - however, this is a process-fatal policy, not recoverable error handling
- Review judgment:
  - not the primary problem in this file
  - document as intentional if this module is kept as the shared allocation policy layer

## Module Boundaries And Technical Debt

### 1. The file mixes a sound `MaybeUninit` matrix API with less sound `Vec<T>` uninit helpers

- Evidence:
  - matrix builders return `Vec<MaybeUninit<_>>`:
    - `188-311`
  - scalar/vector helpers expose `Vec<T>` directly after `set_len` / `from_raw_parts`:
    - `15-20`
    - `33-185`
- Why it matters:
  - there are effectively two safety models in one module
  - the matrix side shows the sounder direction already exists in the codebase
- Smallest justified fix:
  - use the matrix-style `MaybeUninit` boundary as the reference model for any future cleanup
  - in the meantime, the `Vec<T>` helpers should at least advertise their caller-side preconditions as unsafe contracts

### 2. Debug poison is useful, but it does not make the release path sound

- Evidence:
  - debug assertions switch to explicit poison values instead of true uninitialized memory:
    - `3-10`
    - `53-60`
    - `79-86`
    - `123-130`
    - `145-148`
    - `163-166`
    - `181-184`
    - `203-210`
    - `232-239`
- Why it matters:
  - debug builds are better at catching premature reads
  - the release build still relies on the unsound path, and tests run in debug by default
- Review judgment:
  - positive diagnostic technique, but not a substitute for a sound release contract

## Docs And Examples

### 1. The module needs stronger local documentation around which helpers are sound-by-construction and which are only performance-motivated contracts

- Missing documentation topics:
  - why `assume_init_vec` is valid at each call site
  - which helpers require full overwrite before any observation
  - why matrix helpers use `MaybeUninit` slices directly while scalar helpers expose `Vec<T>`

## Tests And Missing Verification

### 1. Existing local tests are useful but do not exercise the release-mode soundness boundary

- Verified in this review pass:
  - `cargo test -p ta_app_core alloc_policy -- --nocapture`
    - passed `9` tests
- Existing strengths:
  - prefix initialization helpers:
    - `crates/app_core/src/alloc_policy.rs:322-349`
  - overwrite-after-allocation behavior:
    - `352-393`
  - matrix prefix behavior:
    - `397-425`

### 2. Current tests do not prove the release path is sound

- Evidence:
  - `cargo test` runs with `debug_assertions`, so the poison-value branches are exercised rather than the true release-mode uninitialized branches
- Additional review observation:
  - sampled hot-path consumers appear disciplined and typically overwrite before read, which lowers immediate misuse risk but does not fix the helper boundary itself
- Missing verification found in this review pass:
  - no release-profile test or Miri-style validation of the `set_len` / `from_raw_parts` contracts
  - no tests proving that all real call sites fully overwrite every element before any read

### 3. Exact local verification required if changed

- `cargo check -p ta_app_core`
- `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- `cargo test -p ta_app_core alloc_policy -- --nocapture`
- `cargo test -p ta_app_core --release alloc_policy -- --nocapture`
- `cargo test -p ta_app_core double_ma -- --nocapture`
- `cargo test -p ta_app_core ml_signal -- --nocapture`
- if CUDA-facing helpers change:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_backtest_kernel -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel standalone_gpu_runtime -- --nocapture`
- if a soundness refactor changes allocation strategy materially:
  - rerun repo perf gates before landing any “filled vs uninitialized” policy change

## Performance, Allocation, And CUDA Concerns

### 1. The performance motivation is legitimate

- Evidence:
  - this file is imported directly by hot CPU and GPU runtime modules
  - many consumers immediately overwrite large buffers
- Review judgment:
  - the repo is justified in caring about allocation/init cost here

### 2. Replacing these helpers with blanket zero-fill or `vec![0; n]` without measurement would be the wrong cleanup posture

- Why it matters:
  - a sound fix is needed, but it should not blindly erase the module's performance purpose
- Better direction:
  - use `MaybeUninit`/`spare_capacity_mut` style APIs to preserve overwrite-heavy behavior while restoring soundness
- Review judgment:
  - benchmark-sensitive; do not touch casually

## Code-Quality Improvement Register

- Safe now:
  - make the `Vec<T>`-returning and `Vec<T>`-resizing uninit helpers `unsafe fn` with `# Safety` docs
  - add explicit safety-contract comments to each helper
  - narrow `alloc_uninit_copy` away from unconstrained `T: Copy`
  - document that debug poison is diagnostic-only, not proof of soundness
- Larger but justified:
  - migrate `Vec<T>`-returning uninit helpers toward `MaybeUninit`-typed APIs
- Defer:
  - any broad “simplify all this to zero-fill” rewrite without perf evidence

## Technical-Debt Reduction Register

- Safe to reduce now:
  - under-documented unsafe contracts
  - generic overbreadth in `alloc_uninit_copy`
- Defer because it spans multiple files:
  - changing return types from `Vec<T>` to `Vec<MaybeUninit<T>>`
  - propagating a sound initialization model through `double_ma`, GPU runtime modules, and ML helpers
- Useful debt that should not be “cleaned up” casually:
  - explicit attention to overwrite-heavy allocation cost in hot paths

## Performance Opportunity Register

- Safe structural improvement:
  - align future helpers around `MaybeUninit` slices and `spare_capacity_mut`
- Benchmark-sensitive or semantics-sensitive:
  - replacing current helpers with always-initialized buffers
  - changing host-buffer reuse patterns in GPU modules

## Regression Risk And Feature-Preservation Constraints

- Feature-preservation rule:
  - preserve current CPU and CUDA route availability
  - preserve overwrite-heavy buffer reuse behavior where it materially matters
- Regression risk if changed carelessly:
  - hot numeric paths can slow down substantially
  - GPU staging buffers can regress
  - partially migrated APIs can introduce subtle initialization bugs
- Safe change posture:
  - treat this as a soundness-and-performance refactor, not a style cleanup
  - prefer staged migration with targeted benchmarks and consumer-by-consumer verification
