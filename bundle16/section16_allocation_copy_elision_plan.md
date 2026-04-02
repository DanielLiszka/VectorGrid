
# Section 16 implementation plan: allocation policy, prefix initialization, and copy-elision discipline

Prepared for the section-16 review surface covering shared allocation helpers, prefix-initialized series builders, device-readback host buffers, and row-major matrix assembly.

## Surface under review

This surface is the repo's cross-cutting allocation and copy-elision contract. The important distinction is not “unsafe vs safe” in the abstract, but rather:

- when a full fill is **observable behavior** and must stay,
- when only a **warm prefix** is observable and the tail is overwrite-only,
- when a host buffer is just a **device readback destination** and prefill is wasted,
- and when a remaining cost is an **unavoidable producer-to-destination copy boundary** rather than an allocation problem.

## Files to review

### Primary files

- `alloc_policy.rs`
- `double_ma.rs`
- `gpu_backtest_kernel.rs`
- `ml_signal.rs`

### Adjacent hot-path files included in this review pass

- `asr_gpu_kernel.rs`
- `vcc_gpu_kernel.rs`
- `vemr_gpu_kernel.rs`
- `rocb_gpu_kernel.rs`
- `mkcs_gpu_kernel.rs`
- `kqsr_gpu_kernel.rs`
- `kvdf_gpu_kernel.rs`
- `vcet_gpu_kernel.rs`
- `carm_gpu_kernel.rs`
- `cdmc_gpu_kernel.rs`

## Extra context

- Keep the first pass **parity-safe** and **small**.
- Fix correctness / UB hazards before measuring micro-optimizations.
- Prefer shared helpers in `alloc_policy.rs` over new local unsafe code.
- Treat the relevant timing boundary as **warm app-path runtime** unless explicitly noted otherwise.
- Do **not** remove full fills when the fill value is later observed as behavior.

---

## 1. Surface understanding

The attached files already show the intended design direction:

- `alloc_policy.rs` centralizes `MaybeUninit` helpers and debug-poison conventions.
- `double_ma.rs` already uses prefix-initialized log-return output and uninitialized row-major staging in multiple paths.
- `gpu_backtest_kernel.rs` already uses uninitialized pageable/pinned host staging for many `f32` device readbacks.
- `ml_signal.rs` and `vcc_gpu_kernel.rs` still contain a few straightforward full-buffer fills where only a prefix or a later overwrite is required.
- Several strategy-local GPU files route host readback through a shared helper (`ensure_host_f32_len`) that is **not** present in this bundle, so those files are partially opaque from a section-16 perspective.

The highest-value implementation order is:

1. fix the two **confirmed correctness/UB** issues,
2. patch the **confirmed wasted fills**,
3. audit the shared helper layer for the remaining strategy-local readback surfaces,
4. only then consider riskier or API-contract-dependent elision work.

---

## 2. Findings

### Confirmed issue 1

- Severity: **critical**
- Type: **correctness**
- Location: `alloc_policy.rs:124-139`, `resize_uninit_f32`
- Timing boundary affected: **app-path**
- Why it matters: the helper reserves based on `capacity` instead of `len`:

  ```rust
  if buf.capacity() < len {
      buf.reserve_exact(len - buf.capacity());
  }
  ```

  `Vec::reserve_exact(additional)` interprets `additional` relative to the **current length**, not relative to the current capacity. That means a vector with `len < capacity < new_len` can remain under-capacity and then immediately hit `set_len(new_len)`, which is UB.
- Recommended change: add a private reserve helper that computes growth from `buf.len()`, not `buf.capacity()`, and route all uninitialized resize helpers through it.

### Confirmed issue 2

- Severity: **critical**
- Type: **correctness**
- Location: `vemr_gpu_kernel.rs:129-136`, `VemrCudaScratch::ensure_host_top_combo_tags_len`
- Timing boundary affected: **app-path**
- Why it matters: the exact same reserve bug exists here:

  ```rust
  if self.host_top_combo_tags.capacity() < len {
      self.host_top_combo_tags
          .reserve(len - self.host_top_combo_tags.capacity());
  }
  unsafe { self.host_top_combo_tags.set_len(len) };
  ```

  This can under-reserve and then set the vector length past capacity before device readback.
- Recommended change: delete this custom growth logic and replace it with a shared `resize_uninit_i32` helper from `alloc_policy.rs`.

### Confirmed issue 3

- Severity: **high**
- Type: **correctness / parity**
- Location: `double_ma.rs:10183-10203`, scalar fallback branch in `ma_matrix_cpu_f64`
- Timing boundary affected: **app-path**
- Why it matters: the function allocates `values` with uninitialized storage, but the scalar fallback path does **not** track `row_written` and does **not** call `fill_missing_rows_with_nan_f64(...)` before returning. If a row is skipped because `series.len() != n` or because a bounds check fails, that row can remain uninitialized instead of being `NaN`.
- Recommended change: add `row_written` tracking in the fallback branch and call `fill_missing_rows_with_nan_f64(...)` before returning.

### Confirmed issue 4

- Severity: **high**
- Type: **correctness / parity**
- Location: `double_ma.rs:10474-10497`, scalar fallback branch in `ma_matrix_cpu_f32_staging`
- Timing boundary affected: **app-path**
- Why it matters: this is the `f32` twin of the previous bug. The fallback path writes copied rows but does not mark or backfill skipped rows.
- Recommended change: add `row_written` tracking in the fallback branch and call `fill_missing_rows_with_nan_f32(...)` before returning.

### Confirmed issue 5

- Severity: **medium**
- Type: **performance**
- Location: `gpu_backtest_kernel.rs:5551-5572`, `copy_device_prefix_to_host_i32`
- Timing boundary affected: **app-path**
- Why it matters: the helper currently does `dst.resize(elems, 0)` before a full D2H overwrite. The D2H copy is unavoidable; the zero fill is not.
- Recommended change: switch this helper to `resize_uninit_i32(dst, elems)` and keep the copy boundary exactly as-is.

### Confirmed issue 6

- Severity: **medium**
- Type: **performance**
- Location: `gpu_backtest_kernel.rs:14763-14766`, `gpu_backtest_kernel.rs:14823-14826`
- Timing boundary affected: **app-path**
- Why it matters:
  - `host_count = vec![0i32; heatmap_len]` is fully overwritten by `copy_to(...)`.
  - `fast_host` and `slow_host` are prefilled with zeros before `copy_device_prefix_to_host_i32(...)`.
- Recommended change:
  - allocate `host_count` uninitialized for the one-shot `copy_to(...)` path,
  - initialize `fast_host` / `slow_host` as empty vectors and let the fixed helper resize them uninitialized.

### Confirmed issue 7

- Severity: **medium**
- Type: **performance**
- Location: `ml_signal.rs:547-557`, `789-803`
- Timing boundary affected: **app-path**
- Why it matters:
  - `log_returns_from_close` only needs slot `0` initialized to `0.0`; every later slot is written exactly once.
  - `prefix_sum` and `prefix_sum_squares` only need prefix slot `0` initialized to `0.0`; every later slot is written once.
- Recommended change: use `alloc_f64_with_prefix_uninit_tail(...)` for all three functions.

### Confirmed issue 8

- Severity: **medium**
- Type: **performance**
- Location: `ml_signal.rs:1979-1982`, `2013-2015`
- Timing boundary affected: **app-path**
- Why it matters: the initial allocations for `requested` and `desired` are fully filled immediately by `fill_requested_targets(...)` and `fill_desired_positions(...)`. The initial `vec![...]` fills are redundant.
- Recommended change: allocate these working vectors with uninitialized resize helpers once, then continue to use the existing `requested.fill(...)` / `desired.fill(...)` logic inside the fill functions.

### Confirmed issue 9

- Severity: **medium**
- Type: **performance**
- Location: `vcc_gpu_kernel.rs:123-136`, `203-215`
- Timing boundary affected: **app-path**
- Why it matters:
  - `compute_log_returns_f64` has the same “only the first slot must be initialized” shape as `double_ma::compute_log_returns_f64`.
  - `ensure_host_slice_out(...)` is a local manual uninitialized-resize implementation; once `resize_uninit_f64(...)` exists, that local unsafe path should be removed.
- Recommended change:
  - switch `compute_log_returns_f64(...)` to `alloc_f64_with_prefix_uninit_tail(...)`,
  - replace `ensure_host_slice_out(...)` with a single `resize_uninit_f64(...)` call.

### Open question 1

- Severity: **low**
- Type: **open question**
- Location: `asr_gpu_kernel.rs`, `rocb_gpu_kernel.rs`, `mkcs_gpu_kernel.rs`, `kqsr_gpu_kernel.rs`, `kvdf_gpu_kernel.rs`, `vcet_gpu_kernel.rs`, `carm_gpu_kernel.rs`, `cdmc_gpu_kernel.rs`, and `vcc_gpu_kernel.rs` for `host_out`
- Timing boundary affected: **unknown**
- Why it matters: these files call `ensure_host_f32_len(...)`, but the shared implementation lives in `standalone_gpu_runtime`, which was **not** part of the provided bundle. If that helper still zero-fills, these files are leaving D2H prefill waste on the table. If it already uses uninitialized growth, they are fine.
- Recommended change: audit the shared helper first. Do **not** duplicate local unsafe logic in each strategy file unless the shared helper cannot be fixed centrally.

### Open question 2

- Severity: **low**
- Type: **open question**
- Location: `double_ma.rs:11316-11319`, `ma_matrix_cuda_host_f32` ALMA path
- Timing boundary affected: **unknown**
- Why it matters: `let mut out = vec![f32::NAN; expected_len];` might be avoidable if `alma_batch_into_host_f32(...)` guarantees full overwrite of the host destination. That guarantee is not visible in this file bundle.
- Recommended change: leave this alone in the first pass. Only revisit after checking the CUDA MA API contract.

---

## 3. Safe optimization opportunities

### A. Centralize shared uninitialized resize helpers

Add these to `alloc_policy.rs`:

- `resize_uninit_f64`
- `resize_uninit_i32`
- a private reserve helper that grows from `len`, not `capacity`

This immediately fixes the confirmed correctness bug in `alloc_policy.rs`, fixes the mirrored bug in `vemr_gpu_kernel.rs`, and removes a local unsafe path in `vcc_gpu_kernel.rs`.

### B. Fix `double_ma` scalar fallback NaN backfill

This is a correctness fix first and a parity fix second. It is small, local, and does not change the happy-path batch layout.

### C. Remove avoidable `i32` D2H prefills in `gpu_backtest_kernel.rs`

The copies remain. Only the pre-copy memset disappears.

### D. Use prefix-initialized `f64` allocation in `ml_signal.rs`

The shapes are ideal for `alloc_f64_with_prefix_uninit_tail(...)`:

- log returns: warm prefix length = 1
- prefix sums: warm prefix length = 1
- prefix sum of squares: warm prefix length = 1

### E. Remove redundant initial fills in `ml_signal` working vectors

Only the **initial** allocation fill is redundant. The later `requested.fill(ML_SIGNAL_NO_REQUEST)` and `desired.fill(0)` are **not** redundant and must remain.

### F. Remove local unsafe resize logic in `vcc_gpu_kernel.rs`

Once `resize_uninit_f64(...)` exists, `ensure_host_slice_out(...)` should become a one-line shared-helper call.

### G. Keep behaviorally observable fills in place

Do **not** change these in the first pass:

- `double_ma.rs:7418` — `nan_row: Vec<f32> = vec![f32::NAN; n];`
- `double_ma.rs:10194` / `10485` — `unwrap_or_else(|_| vec![f64::NAN; n])`
- `double_ma.rs:11316` — ALMA `vec![f32::NAN; expected_len]` until the API overwrite contract is confirmed
- `ml_signal.rs:1890` — `requested.fill(ML_SIGNAL_NO_REQUEST);`
- `ml_signal.rs:1907` — `desired.fill(0);`
- `gpu_backtest_kernel.rs:4928`, `4941` — algorithmic score/default vectors, not overwrite-only staging buffers

---

## 4. Higher-risk optimization opportunities

### A. Generalize a shared generic resize helper only after the first pass

A second-pass cleanup could introduce a generic uninitialized resize primitive under a small typed façade. That is not required to land the first round of fixes.

### B. Revisit `ma_matrix_cuda_host_f32` ALMA host prefill only if the CUDA API contract is explicit

That path may be optimizable, but only after confirming the host buffer is always fully written.

### C. Centralize all strategy-local host-metric D2H staging only after auditing `standalone_gpu_runtime`

Do not patch eight strategy files blindly if the shared helper already does the right thing.

---

## 5. Benchmark and perf-gate review

### Timing boundary

The relevant timing boundary for almost all section-16 wins here is **warm app-path runtime**:

- repeated MA matrix assembly on the CPU
- repeated GPU D2H readback and host decode
- repeated `MlSignal` validation / policy-loop working-buffer setup
- repeated VCC detailed-mode slice readback

### What should be measured after the correctness fixes land

1. A warm `GpuKernel` / exact-shortlist path that exercises:
   - heatmap readback (`host_count`)
   - global-row shortlist readback (`fast_host` / `slow_host`)

2. An `MlSignal` validation run with a nontrivial candidate set, so the requested/desired working vectors are reused across multiple candidates.

3. A VCC detailed-mode run that exercises `host_slice_out`.

### What not to do in this pass

- Do not add new benchmark binaries first.
- Do not make Nsight claims from this bundle alone.
- Do not claim route-matrix changes; nothing here changes backend selection.
- Do not benchmark before the two correctness bugs are fixed.

### Evidence path

Use the repo’s existing benchmark/gate path once these patches are in place. This bundle does **not** include the perf-gate scripts themselves, so the safe recommendation is:

- correctness patch first,
- then warm app-path verification using the repo’s current perf-gate / workflow harness path,
- then only add instrumentation if a result is still ambiguous.

---

## 6. Test review

## Immediate tests to add

### `alloc_policy.rs`

Add:

1. a reserve arithmetic regression that proves growth is computed from `len`, not `capacity`
2. a reuse-after-overwrite test for `resize_uninit_i32`
3. a reuse-after-overwrite test for `resize_uninit_f64`

### `vemr_gpu_kernel.rs`

Add:

1. a regression that creates `len < capacity < target_len` for `host_top_combo_tags`
2. asserts `len == target_len` and `capacity >= target_len` after `ensure_host_top_combo_tags_len(...)`

### `ml_signal.rs`

Add:

1. `log_returns_from_close` parity test for:
   - zero-length input
   - one-element input
   - invalid/NaN / non-positive transitions yielding `0.0`
2. a `prefix_sum` / `prefix_sum_squares` parity test against a reference implementation

### `vcc_gpu_kernel.rs`

Add:

1. a `compute_log_returns_f64` parity test mirroring the `ml_signal` test shape

### `double_ma.rs`

The ideal regression is:

- force the scalar fallback path,
- skip at least one row,
- assert the skipped row is fully `NaN`.

Because the supporting test hook that would force the fallback path is not visible in this bundle, there are two safe options:

1. **Preferred**: use existing `double_ma_test_support.rs` shims if they already let you force batch failure.
2. **Fallback**: add a very small test-only extraction around the fallback row-write/finalize sequence so you can unit-test the “skipped rows become `NaN`” rule directly.

---

## 7. Suggested improvement plan

### Phase 1 — correctness / safety first

1. Patch `alloc_policy.rs`:
   - fix `resize_uninit_f32`
   - add `resize_uninit_f64`
   - add `resize_uninit_i32`
   - add reserve arithmetic regression tests

2. Patch `vemr_gpu_kernel.rs`:
   - remove custom reserve/set_len logic
   - switch to `resize_uninit_i32`
   - add a regression test

3. Patch `double_ma.rs`:
   - add `row_written` tracking and `fill_missing_rows_with_nan_*` in both scalar fallback branches

Do **not** benchmark until these three items are done.

### Phase 2 — low-risk performance cleanup

4. Patch `gpu_backtest_kernel.rs`:
   - fix `copy_device_prefix_to_host_i32`
   - remove one-shot `vec![0i32; ...]` readback prefills

5. Patch `ml_signal.rs`:
   - prefix-init log returns
   - prefix-init prefix sums
   - remove redundant initial requested/desired fills

6. Patch `vcc_gpu_kernel.rs`:
   - prefix-init local log returns
   - replace local f64 host-scratch unsafe resize with shared helper

### Phase 3 — shared-helper audit

7. Audit `ensure_host_f32_len(...)` in the shared runtime layer that was not provided here.
   - If it zero-fills, fix it centrally and leave the eight strategy-local files unchanged.
   - If it already uses uninitialized growth, document “no further action”.

### Phase 4 — optional follow-up

8. Revisit `ma_matrix_cuda_host_f32` ALMA host prefill only after confirming the external CUDA MA API contract.

---

## 8. Detailed code edits and snippets

## 8.1 `alloc_policy.rs`

### Goals

- fix the reserve arithmetic bug
- add shared `f64` / `i32` resize helpers
- keep debug-poison behavior

### Recommended patch

```diff
@@
 #[cfg_attr(not(any(debug_assertions, test)), allow(dead_code))]
 const DEBUG_F32_POISON_BITS: u32 = 0x1111_1111;
+#[cfg_attr(not(any(debug_assertions, test)), allow(dead_code))]
+const DEBUG_I32_POISON_BITS: u32 = 0x1111_1111;
@@
 #[inline(always)]
 fn assume_init_vec<T>(mut buf: Vec<MaybeUninit<T>>) -> Vec<T> {
     let ptr = buf.as_mut_ptr() as *mut T;
     let cap = buf.capacity();
     let len = buf.len();
     mem::forget(buf);
     unsafe { Vec::from_raw_parts(ptr, len, cap) }
 }
+
+#[inline(always)]
+fn reserve_for_uninit_resize<T>(buf: &mut Vec<T>, len: usize) {
+    if len > buf.len() {
+        // Reserve relative to current len, not current capacity.
+        buf.reserve(len - buf.len());
+    }
+}
@@
     #[cfg(debug_assertions)]
     {
         for _ in 0..len {
-            buf.push(MaybeUninit::new(f64::from_bits(0x1111_1111_1111_1111)));
+            buf.push(MaybeUninit::new(f64::from_bits(DEBUG_F64_POISON_BITS)));
         }
     }
@@
 pub(crate) fn resize_uninit_f32(buf: &mut Vec<f32>, len: usize) {
     #[cfg(not(debug_assertions))]
     {
-        if buf.capacity() < len {
-            buf.reserve_exact(len - buf.capacity());
-        }
+        reserve_for_uninit_resize(buf, len);
         unsafe {
             // SAFETY: callers must fully initialize each element before any read.
             buf.set_len(len);
         }
     }
@@
     {
         buf.resize(len, f32::from_bits(DEBUG_F32_POISON_BITS));
     }
 }
+
+#[cfg_attr(not(test), allow(dead_code))]
+#[inline(always)]
+pub(crate) fn resize_uninit_f64(buf: &mut Vec<f64>, len: usize) {
+    #[cfg(not(debug_assertions))]
+    {
+        reserve_for_uninit_resize(buf, len);
+        unsafe {
+            // SAFETY: callers must fully initialize each element before any read.
+            buf.set_len(len);
+        }
+    }
+
+    #[cfg(debug_assertions)]
+    {
+        buf.resize(len, f64::from_bits(DEBUG_F64_POISON_BITS));
+    }
+}
+
+#[cfg_attr(not(test), allow(dead_code))]
+#[inline(always)]
+pub(crate) fn resize_uninit_i32(buf: &mut Vec<i32>, len: usize) {
+    #[cfg(not(debug_assertions))]
+    {
+        reserve_for_uninit_resize(buf, len);
+        unsafe {
+            // SAFETY: callers must fully initialize each element before any read.
+            buf.set_len(len);
+        }
+    }
+
+    #[cfg(debug_assertions)]
+    {
+        buf.resize(len, DEBUG_I32_POISON_BITS as i32);
+    }
+}
```

### Recommended tests

```diff
@@
     use super::{
         alloc_f32_with_prefix_uninit_tail, alloc_f64_with_prefix_uninit_tail, alloc_uninit_f32,
-        alloc_uninit_f64, init_matrix_prefixes_f32, init_matrix_prefixes_f64,
-        make_uninit_matrix_f32, make_uninit_matrix_f64, resize_uninit_f32,
+        alloc_uninit_f64, init_matrix_prefixes_f32, init_matrix_prefixes_f64,
+        make_uninit_matrix_f32, make_uninit_matrix_f64, reserve_for_uninit_resize,
+        resize_uninit_f32, resize_uninit_f64, resize_uninit_i32,
     };
@@
     fn resize_uninit_buffers_can_be_reused_after_overwrite() {
         let mut values_f32 = vec![1.0, 2.0];
         resize_uninit_f32(&mut values_f32, 4);
         values_f32.copy_from_slice(&[3.0, 4.0, 5.0, 6.0]);
         assert_eq!(values_f32, vec![3.0, 4.0, 5.0, 6.0]);
     }
+
+    #[test]
+    fn reserve_for_uninit_resize_grows_from_len_not_capacity() {
+        let mut values = Vec::with_capacity(4);
+        values.extend_from_slice(&[1.0f32, 2.0]);
+        reserve_for_uninit_resize(&mut values, 6);
+        assert!(values.capacity() >= 6);
+    }
+
+    #[test]
+    fn resize_uninit_i32_buffers_can_be_reused_after_overwrite() {
+        let mut values = vec![1, 2];
+        resize_uninit_i32(&mut values, 5);
+        values.copy_from_slice(&[3, 4, 5, 6, 7]);
+        assert_eq!(values, vec![3, 4, 5, 6, 7]);
+    }
+
+    #[test]
+    fn resize_uninit_f64_buffers_can_be_reused_after_overwrite() {
+        let mut values = vec![1.0, 2.0];
+        resize_uninit_f64(&mut values, 5);
+        values.copy_from_slice(&[3.0, 4.0, 5.0, 6.0, 7.0]);
+        assert_eq!(values, vec![3.0, 4.0, 5.0, 6.0, 7.0]);
+    }
```

### Notes

- Using `reserve(...)` instead of `reserve_exact(...)` is intentional here; scratch buffers are hot-path reuse buffers, and amortized growth is usually a better default.
- If you want capacity-tight behavior instead, keep the helper but swap `reserve(...)` for `reserve_exact(...)`.

---

## 8.2 `double_ma.rs`

### Goals

- ensure skipped rows in scalar fallback paths become `NaN`, not uninitialized garbage

### Recommended patch: `ma_matrix_cpu_f64`

```diff
@@
         _ => {
+            let mut row_written = vec![false; rows];
             for (row, &period) in periods.iter().enumerate() {
                 let series = vector_ta::indicators::moving_averages::ma::ma_with_kernel(
                     ma_type,
                     vector_ta::indicators::moving_averages::ma::MaData::Candles {
                         candles,
@@
                 if dst_off + n > values.len() {
                     continue;
                 }
                 values[dst_off..dst_off + n].copy_from_slice(&series);
+                row_written[row] = true;
             }
+            fill_missing_rows_with_nan_f64(&mut values, n, &row_written);
         }
     }
```

### Recommended patch: `ma_matrix_cpu_f32_staging`

```diff
@@
         _ => {
+            let mut row_written = vec![false; rows];
             // Fall back to row-wise CPU assembly without materializing a full f64 matrix first.
             for (row, &period) in periods.iter().enumerate() {
                 let series = vector_ta::indicators::moving_averages::ma::ma(
                     ma_type,
                     vector_ta::indicators::moving_averages::ma::MaData::Candles {
@@
                 }
                 for (dst, &src) in values[dst_off..dst_off + n].iter_mut().zip(series.iter()) {
                     *dst = src as f32;
                 }
+                row_written[row] = true;
             }
+            fill_missing_rows_with_nan_f32(&mut values, n, &row_written);
             Ok((values, n))
         }
     }
```

### Test note

The best regression is to force the scalar fallback path and verify a skipped row becomes all-`NaN`. If your existing `double_ma_test_support.rs` can force the batch path to fail, wire the regression through that support instead of adding a large new test shim here.

---

## 8.3 `gpu_backtest_kernel.rs`

### Goals

- keep D2H copies
- remove only the avoidable `i32` host prefill work

### Recommended patch

```diff
@@
-use crate::alloc_policy::{alloc_uninit_copy, alloc_uninit_f32, resize_uninit_f32};
+use crate::alloc_policy::{
+    alloc_uninit_copy, alloc_uninit_f32, resize_uninit_f32, resize_uninit_i32,
+};
@@
 fn copy_device_prefix_to_host_i32(
     src: &DeviceBuffer<i32>,
     dst: &mut Vec<i32>,
     elems: usize,
     stream: &Stream,
 ) -> Result<(), String> {
-    dst.resize(elems, 0);
+    resize_uninit_i32(dst, elems);
     if elems == 0 {
         return Ok(());
     }
@@
-                let mut host_count = vec![0i32; heatmap_len];
+                let mut host_count: Vec<i32> = alloc_uninit_copy(heatmap_len);
                 h_count
                     .copy_to(&mut host_count)
                     .map_err(|e| e.to_string())?;
@@
-        let mut fast_host = vec![0i32; global_rows_len];
-        let mut slow_host = vec![0i32; global_rows_len];
+        let mut fast_host: Vec<i32> = Vec::new();
+        let mut slow_host: Vec<i32> = Vec::new();
         copy_device_prefix_to_host_i32(global_fast, &mut fast_host, global_rows_len, bt_stream)?;
         copy_device_prefix_to_host_i32(global_slow, &mut slow_host, global_rows_len, bt_stream)?;
```

### Notes

- This is a pure host-side memset-elision patch.
- The producer-to-destination copy (`cuMemcpyDtoHAsync_v2` / `copy_to`) is still the required boundary.
- Do **not** change the `f32` D2H helper; it already follows the intended pattern.

---

## 8.4 `ml_signal.rs`

### Goals

- replace full fills with prefix-init where only slot `0` must be initialized
- remove redundant initial working-vector fills
- keep observable `requested.fill(...)` and `desired.fill(...)`

### Recommended patch

```diff
@@
+use crate::alloc_policy::{alloc_f64_with_prefix_uninit_tail, resize_uninit_i32};
 use crate::composite_validate::ValidationArtifactContext;
 use crate::double_ma::BackendUsed;
@@
 fn log_returns_from_close(close: &[f64]) -> Vec<f64> {
-    let mut out = vec![0.0; close.len()];
+    let mut out = alloc_f64_with_prefix_uninit_tail(close.len(), 1, 0.0);
     for i in 1..close.len() {
         let prev = close[i - 1];
         let next = close[i];
         if prev.is_finite() && next.is_finite() && prev > 0.0 && next > 0.0 {
             out[i] = (next / prev).ln();
+        } else {
+            out[i] = 0.0;
         }
     }
     out
 }
@@
 fn prefix_sum(values: &[f64]) -> Vec<f64> {
-    let mut prefix = vec![0.0; values.len() + 1];
+    let mut prefix = alloc_f64_with_prefix_uninit_tail(values.len() + 1, 1, 0.0);
     for (idx, value) in values.iter().enumerate() {
         prefix[idx + 1] = prefix[idx] + *value;
     }
     prefix
 }
@@
 fn prefix_sum_squares(values: &[f64]) -> Vec<f64> {
-    let mut prefix = vec![0.0; values.len() + 1];
+    let mut prefix = alloc_f64_with_prefix_uninit_tail(values.len() + 1, 1, 0.0);
     for (idx, value) in values.iter().enumerate() {
         prefix[idx + 1] = prefix[idx] + (*value * *value);
     }
     prefix
 }
@@
 fn build_desired_positions(
     score_points: &[Option<ScorePoint>],
     params: &MlSignalPolicyParams,
 ) -> Vec<i32> {
-    let mut requested = vec![ML_SIGNAL_NO_REQUEST; score_points.len()];
+    let mut requested = Vec::new();
+    resize_uninit_i32(&mut requested, score_points.len());
     fill_requested_targets(score_points, params, &mut requested);
-    let mut desired = vec![0i32; score_points.len()];
+    let mut desired = Vec::new();
+    resize_uninit_i32(&mut desired, score_points.len());
     fill_desired_positions(&requested, params, &mut desired);
     desired
 }
@@
     let gates = sanitize_hard_gates(min_psr, min_dsr, None, None, None);
     let lr = log_returns_from_close(&candles.close);
-    let mut requested = vec![ML_SIGNAL_NO_REQUEST; score_points.len()];
-    let mut desired = vec![0i32; score_points.len()];
+    let mut requested = Vec::new();
+    resize_uninit_i32(&mut requested, score_points.len());
+    let mut desired = Vec::new();
+    resize_uninit_i32(&mut desired, score_points.len());
```

### Keep these lines exactly as behaviorally required

```rust
requested.fill(ML_SIGNAL_NO_REQUEST);
desired.fill(0);
```

Do **not** remove them.

### Recommended tests

```diff
@@
     #[test]
+    fn log_returns_from_close_preserves_zero_prefix_and_invalid_rows() {
+        let lr = log_returns_from_close(&[100.0, 110.0, f64::NAN, 90.0, 0.0, 95.0]);
+        assert_eq!(lr.len(), 6);
+        assert_eq!(lr[0], 0.0);
+        assert!(lr[1].is_finite());
+        assert_eq!(lr[2], 0.0);
+        assert_eq!(lr[3], 0.0);
+        assert_eq!(lr[4], 0.0);
+        assert_eq!(lr[5], 0.0);
+    }
+
+    #[test]
+    fn prefix_sum_helpers_match_reference() {
+        let values = [1.0, -2.0, 3.5, 0.5];
+        assert_eq!(prefix_sum(&values), vec![0.0, 1.0, -1.0, 2.5, 3.0]);
+        assert_eq!(
+            prefix_sum_squares(&values),
+            vec![0.0, 1.0, 5.0, 17.25, 17.5]
+        );
+    }
+
+    #[test]
     fn score_target_respects_thresholds_and_delta() {
         ...
     }
```

---

## 8.5 `vcc_gpu_kernel.rs`

### Goals

- remove local unsafe f64 resize logic
- use shared prefix-init for log returns

### Recommended patch

```diff
@@
-use crate::alloc_policy::alloc_uninit_copy;
+use crate::alloc_policy::{alloc_f64_with_prefix_uninit_tail, resize_uninit_f64};
@@
     fn ensure_host_out(&mut self, required: usize) {
         ensure_host_f32_len(&mut self.host_out, required);
     }

     fn ensure_host_slice_out(&mut self, required: usize) {
-        if self.host_slice_out.capacity() < required {
-            self.host_slice_out = alloc_uninit_copy(required);
-        } else {
-            #[cfg(not(debug_assertions))]
-            unsafe {
-                self.host_slice_out.set_len(required);
-            }
-            #[cfg(debug_assertions)]
-            {
-                self.host_slice_out.resize(required, f64::NAN);
-            }
-        }
+        resize_uninit_f64(&mut self.host_slice_out, required);
     }
@@
 fn compute_log_returns_f64(close_f64: &[f64]) -> Vec<f64> {
-    let mut out = vec![0.0f64; close_f64.len()];
+    let mut out = alloc_f64_with_prefix_uninit_tail(close_f64.len(), 1, 0.0);
     for t in 1..close_f64.len() {
         let p = close_f64[t];
         let pm = close_f64[t - 1];
         out[t] = if p.is_finite() && pm.is_finite() && p > 0.0 && pm > 0.0 {
             p.ln() - pm.ln()
         } else {
             0.0
         };
     }
     out
 }
```

### Recommended test

```diff
@@
     #[test]
+    fn compute_log_returns_f64_preserves_zero_prefix_and_invalid_rows() {
+        let out = compute_log_returns_f64(&[100.0, 110.0, f64::NAN, 90.0, 0.0, 95.0]);
+        assert_eq!(out[0], 0.0);
+        assert!(out[1].is_finite());
+        assert_eq!(out[2], 0.0);
+        assert_eq!(out[3], 0.0);
+        assert_eq!(out[4], 0.0);
+        assert_eq!(out[5], 0.0);
+    }
+
+    #[test]
     fn validate_inputs_rejects_out_of_range_combo_rows() {
         ...
     }
```

### Notes

- `host_out` still goes through `ensure_host_f32_len(...)`, so VCC still has one remaining shared-helper dependency after this local patch.

---

## 8.6 `vemr_gpu_kernel.rs`

### Goals

- remove the mirrored len-vs-capacity bug
- use the shared `i32` resize helper

### Recommended patch

```diff
@@
-use crate::alloc_policy::resize_uninit_f32;
+use crate::alloc_policy::{resize_uninit_f32, resize_uninit_i32};
@@
     fn ensure_host_top_combo_tags_len(&mut self, len: usize) {
-        if self.host_top_combo_tags.capacity() < len {
-            self.host_top_combo_tags
-                .reserve(len - self.host_top_combo_tags.capacity());
-        }
-        // Safety: the device readback fully overwrites this prefix before any read.
-        unsafe { self.host_top_combo_tags.set_len(len) };
+        resize_uninit_i32(&mut self.host_top_combo_tags, len);
     }
```

### Recommended test

```diff
@@
 mod tests {
-    use super::{validate_inputs, VemrGpuComboBatch, VemrGpuDeviceSignalMatrices};
+    use super::{validate_inputs, VemrCudaScratch, VemrGpuComboBatch, VemrGpuDeviceSignalMatrices};
@@
     #[test]
+    fn ensure_host_top_combo_tags_len_grows_from_len_not_capacity() {
+        let mut scratch = VemrCudaScratch::default();
+        scratch.host_top_combo_tags = Vec::with_capacity(4);
+        scratch.host_top_combo_tags.extend_from_slice(&[1, 2]);
+
+        scratch.ensure_host_top_combo_tags_len(6);
+
+        assert_eq!(scratch.host_top_combo_tags.len(), 6);
+        assert!(scratch.host_top_combo_tags.capacity() >= 6);
+    }
+
+    #[test]
     fn validate_inputs_rejects_out_of_bounds_rows() {
         ...
     }
```

---

## 8.7 Files with no confirmed direct local patch from the provided surface

These files should **not** get local unsafe edits from this review alone:

- `asr_gpu_kernel.rs`
- `rocb_gpu_kernel.rs`
- `mkcs_gpu_kernel.rs`
- `kqsr_gpu_kernel.rs`
- `kvdf_gpu_kernel.rs`
- `vcet_gpu_kernel.rs`
- `carm_gpu_kernel.rs`
- `cdmc_gpu_kernel.rs`

### Why

From the provided files, each of these only exposes call sites like:

```rust
ensure_host_f32_len(&mut self.host_metrics, required);
...
.copy_to(rt.scratch.host_metrics.as_mut_slice())
```

The actual resize/fill behavior lives in `standalone_gpu_runtime`, which was **not** included in this bundle.

### Action

- Audit the shared helper once.
- If it zero-fills, fix it centrally.
- Leave these eight files unchanged unless the shared helper cannot be fixed.

---

## 9. Per-file action map

### `alloc_policy.rs`
- **Must change**
- Fix correctness bug
- Add shared `f64` / `i32` resize helpers
- Add tests

### `double_ma.rs`
- **Must change**
- Fix skipped-row NaN fill in both scalar fallback branches
- Add regression if existing test support can force the fallback path

### `gpu_backtest_kernel.rs`
- **Should change**
- Remove avoidable `i32` prefill in helper and one-shot D2H readbacks

### `ml_signal.rs`
- **Should change**
- Prefix-init three `f64` builders
- Remove redundant initial requested/desired allocations
- Keep internal fill semantics unchanged

### `vcc_gpu_kernel.rs`
- **Should change**
- Prefix-init local log returns
- Replace local unsafe f64 resize logic with shared helper

### `vemr_gpu_kernel.rs`
- **Must change**
- Fix mirrored reserve bug via shared `resize_uninit_i32`

### `asr_gpu_kernel.rs`
- **No local patch confirmed**
- Shared helper audit only

### `rocb_gpu_kernel.rs`
- **No local patch confirmed**
- Shared helper audit only

### `mkcs_gpu_kernel.rs`
- **No local patch confirmed**
- Shared helper audit only

### `kqsr_gpu_kernel.rs`
- **No local patch confirmed**
- Shared helper audit only

### `kvdf_gpu_kernel.rs`
- **No local patch confirmed**
- Shared helper audit only

### `vcet_gpu_kernel.rs`
- **No local patch confirmed**
- Shared helper audit only

### `carm_gpu_kernel.rs`
- **No local patch confirmed**
- Shared helper audit only

### `cdmc_gpu_kernel.rs`
- **No local patch confirmed**
- Shared helper audit only

---

## 10. Comprehensive implementation checklist

### Phase 1 — correctness and safety

- [ ] `alloc_policy.rs`: add `reserve_for_uninit_resize`
- [ ] `alloc_policy.rs`: fix `resize_uninit_f32` to reserve from `len`, not `capacity`
- [ ] `alloc_policy.rs`: add `resize_uninit_f64`
- [ ] `alloc_policy.rs`: add `resize_uninit_i32`
- [ ] `alloc_policy.rs`: replace the hard-coded debug poison literal in `alloc_f64_with_prefix_uninit_tail` with `DEBUG_F64_POISON_BITS`
- [ ] `alloc_policy.rs`: add reserve arithmetic regression test
- [ ] `alloc_policy.rs`: add `resize_uninit_i32` reuse-after-overwrite test
- [ ] `alloc_policy.rs`: add `resize_uninit_f64` reuse-after-overwrite test

- [ ] `vemr_gpu_kernel.rs`: replace custom `host_top_combo_tags` reserve/set_len logic with `resize_uninit_i32`
- [ ] `vemr_gpu_kernel.rs`: add regression test for `len < capacity < target_len`

- [ ] `double_ma.rs`: patch `ma_matrix_cpu_f64` scalar fallback to track `row_written`
- [ ] `double_ma.rs`: patch `ma_matrix_cpu_f64` scalar fallback to call `fill_missing_rows_with_nan_f64`
- [ ] `double_ma.rs`: patch `ma_matrix_cpu_f32_staging` scalar fallback to track `row_written`
- [ ] `double_ma.rs`: patch `ma_matrix_cpu_f32_staging` scalar fallback to call `fill_missing_rows_with_nan_f32`
- [ ] `double_ma.rs`: add or wire a regression that forces the scalar fallback path and verifies skipped rows become `NaN`

### Phase 2 — low-risk performance cleanup

- [ ] `gpu_backtest_kernel.rs`: import `resize_uninit_i32`
- [ ] `gpu_backtest_kernel.rs`: change `copy_device_prefix_to_host_i32` from `dst.resize(..., 0)` to `resize_uninit_i32`
- [ ] `gpu_backtest_kernel.rs`: replace heatmap `host_count = vec![0i32; ...]` with uninitialized one-shot allocation
- [ ] `gpu_backtest_kernel.rs`: replace `fast_host` / `slow_host` zero-filled vectors with empty vectors that are resized by the helper
- [ ] `gpu_backtest_kernel.rs`: leave the D2H copy boundary unchanged

- [ ] `ml_signal.rs`: import `alloc_f64_with_prefix_uninit_tail`
- [ ] `ml_signal.rs`: import `resize_uninit_i32`
- [ ] `ml_signal.rs`: convert `log_returns_from_close` to prefix-init allocation
- [ ] `ml_signal.rs`: convert `prefix_sum` to prefix-init allocation
- [ ] `ml_signal.rs`: convert `prefix_sum_squares` to prefix-init allocation
- [ ] `ml_signal.rs`: convert `build_desired_positions` initial `requested` allocation to uninitialized resize
- [ ] `ml_signal.rs`: convert `build_desired_positions` initial `desired` allocation to uninitialized resize
- [ ] `ml_signal.rs`: convert `validate_ml_signal_candidates` initial `requested` allocation to uninitialized resize
- [ ] `ml_signal.rs`: convert `validate_ml_signal_candidates` initial `desired` allocation to uninitialized resize
- [ ] `ml_signal.rs`: keep `requested.fill(ML_SIGNAL_NO_REQUEST)` exactly as behaviorally required
- [ ] `ml_signal.rs`: keep `desired.fill(0)` exactly as behaviorally required
- [ ] `ml_signal.rs`: add `log_returns_from_close` parity test
- [ ] `ml_signal.rs`: add `prefix_sum` / `prefix_sum_squares` parity test

- [ ] `vcc_gpu_kernel.rs`: import `resize_uninit_f64`
- [ ] `vcc_gpu_kernel.rs`: import `alloc_f64_with_prefix_uninit_tail`
- [ ] `vcc_gpu_kernel.rs`: replace `ensure_host_slice_out` with `resize_uninit_f64`
- [ ] `vcc_gpu_kernel.rs`: convert `compute_log_returns_f64` to prefix-init allocation
- [ ] `vcc_gpu_kernel.rs`: add local log-returns parity test

### Phase 3 — shared-helper audit

- [ ] Audit `ensure_host_f32_len(...)` in the shared runtime layer outside this bundle
- [ ] If it zero-fills, fix it centrally and leave `asr/rocb/mkcs/kqsr/kvdf/vcet/carm/cdmc` unchanged
- [ ] If it already uses uninitialized growth, document those files as no-op for section 16
- [ ] Confirm whether `vcc_gpu_kernel.rs` `host_out` is already covered by the shared helper

### Phase 4 — verification

- [ ] Run unit tests for `alloc_policy.rs`, `ml_signal.rs`, `vcc_gpu_kernel.rs`, and `vemr_gpu_kernel.rs`
- [ ] Run the `double_ma` test that covers fallback-row `NaN` behavior
- [ ] Re-run the existing repo perf/gate path on a warm app-path workload
- [ ] Verify `gpu_backtest_kernel` heatmap/global-row D2H paths still decode correctly
- [ ] Verify `MlSignal` validation metrics are bit-for-bit / behaviorally unchanged
- [ ] Verify VCC detailed-mode slice metrics are unchanged
- [ ] Re-check any file that still performs a full fill and confirm it is behaviorally required

### Explicit “do not optimize” reminders

- [ ] Do **not** remove `requested.fill(ML_SIGNAL_NO_REQUEST)`
- [ ] Do **not** remove `desired.fill(0)`
- [ ] Do **not** replace `nan_row` fallback buffers that are behaviorally observed as `NaN`
- [ ] Do **not** touch the CUDA ALMA host prefill path until the overwrite contract is confirmed
- [ ] Do **not** duplicate local unsafe readback helpers into strategy files before auditing the shared runtime helper
