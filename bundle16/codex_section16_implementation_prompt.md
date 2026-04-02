# Codex implementation prompt — Section 16 allocation policy, prefix initialization, and copy-elision discipline

## Mission

Implement only the remaining repo-valid section-16 backend work for allocation policy, prefix initialization, and copy-elision.

This is not a broad refactor. It is a narrow Rust/CUDA correctness-first pass. The current codebase already satisfies part of the original plan/addendum, so do not re-implement items that are already true.

Priority order:

1. fix confirmed correctness / UB hazards first
2. land parity-safe performance cleanup second
3. add the missing targeted regressions third
4. only then consider the bounded optional addendum item

---

## Read these source documents first

Before editing code, read:

1. `bundle16/section16_allocation_copy_elision_plan.md`
2. `bundle16/section16_allocation_copy_elision_addendum.md`
3. `gpt54_feature_review_map.md` section `## 16. Allocation policy, prefix initialization, and copy-elision discipline`

Use those docs together with the current repo state. If the older markdown text conflicts with current code, trust the code and the repo-state corrections below.

---

## Repo-State Validation And Scope Corrections

### Already validated / already true in this repo

- [x] `standalone_gpu_runtime.rs::ensure_host_f32_len(...)` already uses uninitialized-growth behavior on expansion and otherwise routes through the shared `resize_uninit_f32(...)` path. Do not treat it as an open zero-fill rewrite target.
- [x] Adjacent strategy-local GPU files that only call `ensure_host_f32_len(...)` remain local no-op surfaces for this pass: `asr_gpu_kernel.rs`, `rocb_gpu_kernel.rs`, `mkcs_gpu_kernel.rs`, `kqsr_gpu_kernel.rs`, `kvdf_gpu_kernel.rs`, `vcet_gpu_kernel.rs`, `carm_gpu_kernel.rs`, `cdmc_gpu_kernel.rs`, plus the VCC `host_out` path.
- [x] `double_ma.rs` already backfills missing rows in the ALMA batch, FRAMA batch, and generic `ScalarBatch` branches. Do not rework those branches.
- [x] `double_ma.rs` already has broad f64/f32 matrix parity coverage; the missing test surface is a targeted skipped-row scalar-fallback regression, not a general parity harness.
- [x] `ml_signal.rs::build_desired_positions(...)` is `#[cfg(test)]`. Runtime throughput work belongs in `validate_ml_signal_candidates(...)`; test-helper mirroring is optional and not part of the core bundle scope.
- [x] `ml_signal.rs` already preserves the behavioral guards `requested.fill(ML_SIGNAL_NO_REQUEST)` and `desired.fill(0)`. Keep them intact; do not track them as open implementation items.
- [x] `vcc_gpu_kernel.rs::compute_log_returns_f64(...)` already writes explicit `0.0` for invalid transitions. Preserve that behavior; do not track that branch as an open fix.
- [x] The addendum’s open question about whether `standalone_gpu_runtime.rs` still zero-fills host buffers is resolved by current code inspection. Do not keep “if it zero-fills, fix it” as an active branch in this prompt.

### Still valid and in scope

- [x] `alloc_policy.rs` reserve growth, typed resize helpers, poison-literal cleanup, and the new helper regressions are now implemented.
- [x] `vemr_gpu_kernel.rs` now routes `ensure_host_top_combo_tags_len(...)` through `resize_uninit_i32(...)` and includes the `len < capacity < target_len` regression.
- [x] `double_ma.rs` row-wise scalar fallback branches now backfill skipped rows with `NaN`, and the targeted fallback regression is present.
- [x] `gpu_backtest_kernel.rs` no longer zero-fills pure `i32` host readback staging in `copy_device_prefix_to_host_i32(...)`, the heatmap `host_count` path, or the global-row `fast_host` / `slow_host` scratch vectors.
- [x] `ml_signal.rs` runtime builders now use prefix-init allocation and uninitialized working-buffer resize where appropriate, with explicit invalid-branch `0.0` writes preserved.
- [x] `vcc_gpu_kernel.rs` now uses shared `f64` resize, prefix-init log returns, and local parity coverage.

### Valid but optional only

- [x] `ml_signal.rs::extract_pattern_matrix(...)` was safely rewritten to `Vec::with_capacity(...) + push(...)` while preserving the `Vec<Vec<u8>>` return type.

---

## File Scope

### Primary files to modify

- `crates/app_core/src/alloc_policy.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/vcc_gpu_kernel.rs`
- `crates/app_core/src/vemr_gpu_kernel.rs`

### Inspect-only / conditionally consult

- `crates/app_core/src/standalone_gpu_runtime.rs`
- `crates/app_core/src/double_ma_test_support.rs`

### Files that should remain locally unchanged in this task

- `crates/app_core/src/asr_gpu_kernel.rs`
- `crates/app_core/src/rocb_gpu_kernel.rs`
- `crates/app_core/src/mkcs_gpu_kernel.rs`
- `crates/app_core/src/kqsr_gpu_kernel.rs`
- `crates/app_core/src/kvdf_gpu_kernel.rs`
- `crates/app_core/src/vcet_gpu_kernel.rs`
- `crates/app_core/src/carm_gpu_kernel.rs`
- `crates/app_core/src/cdmc_gpu_kernel.rs`

### Benchmark / evidence paths to use, not redesign

- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `crates/app_core/benches/e2e_workflow_ml.rs`
- the existing perf/workflow gate scripts already present in the repo

Do not create new benchmark binaries for this task.

---

## Hard Constraints

- Do not turn this into a broad architecture rewrite.
- Do not add new product surfaces, strategy knobs, UI, or route-policy changes.
- Preserve external behavior unless the current behavior is a confirmed bug or UB hazard.
- Use uninitialized allocation only when every element is guaranteed to be written before any read.
- Keep full-fill allocation when the fill value is behaviorally observable.
- Keep debug-poison behavior aligned with the existing helper design.

### Explicit “do not optimize away” reminders

- [x] Do not remove `requested.fill(ML_SIGNAL_NO_REQUEST)` in `ml_signal.rs`.
- [x] Do not remove `desired.fill(0)` in `ml_signal.rs`.
- [x] Do not replace behaviorally observed `NaN` fallback buffers or rows with uninitialized storage.
- [x] Do not touch the ALMA CUDA host-prefill path unless a full-overwrite contract is explicit in the external API.
- [x] Do not flatten `pattern_hits` or redesign broader `MlSignal` feature extraction / candidate-loop throughput in this pass.
- [x] Do not duplicate local unsafe readback helpers into adjacent strategy-local GPU files.

### Performance-boundary guard

Treat the evidence boundary as:

- warm app-path runtime for `gpu_backtest_kernel.rs` host-readback work
- workflow / end-to-end ML runtime for `ml_signal.rs`

Do not claim success from a helper-only microbenchmark.

---

## Implementation Checklist

### Phase 1 — correctness and UB fixes

- [x] `alloc_policy.rs`: add a private `reserve_for_uninit_resize<T>(...)` helper that grows relative to `buf.len()`, not `buf.capacity()`.
- [x] `alloc_policy.rs`: route `resize_uninit_f32(...)` through the new reserve helper.
- [x] `alloc_policy.rs`: add `resize_uninit_f64(...)` with matching debug/release semantics.
- [x] `alloc_policy.rs`: add `resize_uninit_i32(...)` with matching debug/release semantics.
- [x] `alloc_policy.rs`: replace the hard-coded debug `f64` poison literal in `alloc_f64_with_prefix_uninit_tail(...)` with the named poison constant already defined in the file.
- [x] `alloc_policy.rs`: add a regression that exercises `old_len < old_capacity < new_len` and proves reserve growth is based on length, not capacity.
- [x] `alloc_policy.rs`: add a `resize_uninit_i32(...)` reuse-after-overwrite test.
- [x] `alloc_policy.rs`: add a `resize_uninit_f64(...)` reuse-after-overwrite test.

- [x] `vemr_gpu_kernel.rs`: replace `ensure_host_top_combo_tags_len(...)` custom reserve/set_len logic with `resize_uninit_i32(...)`.
- [x] `vemr_gpu_kernel.rs`: add a regression that exercises `len < capacity < target_len` and asserts `len == target_len` and `capacity >= target_len`.

- [x] `double_ma.rs`: patch only the row-wise scalar fallback branch in `ma_matrix_cpu_f64(...)` so skipped rows cannot remain uninitialized.
- [x] `double_ma.rs`: patch only the row-wise scalar fallback branch in `ma_matrix_cpu_f32_staging(...)` so skipped rows cannot remain uninitialized.
- [x] `double_ma.rs`: add or wire a targeted regression that forces a skipped scalar-fallback row and verifies the row is fully `NaN`.

### Phase 2 — low-risk performance cleanup

- [x] `gpu_backtest_kernel.rs`: change `copy_device_prefix_to_host_i32(...)` from `dst.resize(..., 0)` to shared uninitialized resize.
- [x] `gpu_backtest_kernel.rs`: replace `host_count = vec![0i32; heatmap_len]` with an uninitialized one-shot allocation suitable for full immediate overwrite.
- [x] `gpu_backtest_kernel.rs`: replace zero-filled `fast_host` / `slow_host` scratch allocation with empty vectors that are resized by the helper.
- [x] `gpu_backtest_kernel.rs`: leave the D2H copy boundary itself unchanged.

- [x] `ml_signal.rs`: convert `log_returns_from_close(...)` to prefix-init allocation with warm prefix length `1` and prefix value `0.0`.
- [x] `ml_signal.rs`: convert `prefix_sum(...)` to prefix-init allocation with warm prefix length `1` and prefix value `0.0`.
- [x] `ml_signal.rs`: convert `prefix_sum_squares(...)` to prefix-init allocation with warm prefix length `1` and prefix value `0.0`.
- [x] `ml_signal.rs`: keep explicit `0.0` writes in invalid-branch paths so zero semantics remain observable.
- [x] `ml_signal.rs`: convert only the runtime `requested` working buffer in `validate_ml_signal_candidates(...)` from initial fill allocation to shared uninitialized resize.
- [x] `ml_signal.rs`: convert only the runtime `desired` working buffer in `validate_ml_signal_candidates(...)` from initial fill allocation to shared uninitialized resize.
- [x] `ml_signal.rs`: add `log_returns_from_close(...)` parity coverage for empty input, single-element input, and invalid/non-positive transitions yielding `0.0`.
- [x] `ml_signal.rs`: add `prefix_sum(...)` / `prefix_sum_squares(...)` parity coverage against a straightforward reference.

- [x] `vcc_gpu_kernel.rs`: replace `ensure_host_slice_out(...)` local manual resize logic with `resize_uninit_f64(...)`.
- [x] `vcc_gpu_kernel.rs`: convert `compute_log_returns_f64(...)` to prefix-init allocation with warm prefix length `1` and prefix value `0.0`.
- [x] `vcc_gpu_kernel.rs`: keep explicit invalid-transition `0.0` writes intact.
- [x] `vcc_gpu_kernel.rs`: add a local log-returns parity test.

### Optional Phase 3 — addendum item only

- [x] `ml_signal.rs::extract_pattern_matrix(...)`: if touched, replace the nested zero fill with a safe `Vec::with_capacity(...) + push(...)` rewrite.
- [x] `ml_signal.rs::extract_pattern_matrix(...)`: keep the return type as `Vec<Vec<u8>>`.
- [x] `ml_signal.rs::extract_pattern_matrix(...)`: do not flatten the representation or redesign downstream consumers.

### Verification checklist

- [x] Run targeted tests for `alloc_policy`, `vemr_gpu_kernel`, `vcc_gpu_kernel`, `double_ma`, and `ml_signal` in debug mode.
- [x] Run the relevant targeted section-16 tests again in release mode.
- [x] Miri was not run; this remained optional and was intentionally deferred.
- [x] Existing warm app-path evidence surfaces were validated in compile/help mode via `ta_app_path_faceoff`; no fresh timing claim was made in this pass.
- [x] Existing workflow / e2e ML evidence surfaces were validated in compile mode via `e2e_workflow_ml --no-run`; no fresh timing claim was made in this pass.
- [x] Perf evidence was explicitly compile-surface validation only in this pass, not a new benchmark run.

---

## Acceptance Criteria

The patch set is acceptable only if all of the following are true:

- [x] `resize_uninit_f32(...)` no longer has the len-vs-capacity reserve bug.
- [x] `resize_uninit_f64(...)` exists and mirrors the shared helper semantics correctly.
- [x] `resize_uninit_i32(...)` exists and mirrors the shared helper semantics correctly.
- [x] `VemrCudaScratch::ensure_host_top_combo_tags_len(...)` no longer performs custom broken reserve math.
- [x] The row-wise scalar fallback branches in `double_ma.rs` now backfill skipped rows with `NaN`.
- [x] `gpu_backtest_kernel.rs` no longer zero-fills pure `i32` host readback staging before full overwrite.
- [x] `ml_signal.rs` prefix-only runtime builders now use prefix-init helpers correctly and still explicitly write `0.0` in invalid branches.
- [x] `requested.fill(ML_SIGNAL_NO_REQUEST)` and `desired.fill(0)` remain intact.
- [x] `vcc_gpu_kernel.rs` no longer uses local custom unsafe `f64` resize logic for `host_slice_out`.
- [x] Targeted debug tests pass.
- [x] Targeted release tests pass.
- [x] No feature was removed.
- [x] No broad architectural refactor was introduced.

---

## Final output required from Codex

When you finish the implementation, provide a concise report with:

1. Files changed
2. What was fixed
3. What performance waste was removed
4. Tests added
5. Tests run and results
6. Benchmark / evidence path used or deferred
7. Optional changes completed
8. Open follow-up items intentionally left for later

Also explicitly state:

- whether `standalone_gpu_runtime.rs` required a code change in this pass
- whether `extract_pattern_matrix(...)` was changed in this pass
- whether adjacent strategy-local GPU files remained unchanged because the shared helper path was already the right local surface

---

## Bottom-line implementation philosophy

Keep this pass:

- small
- correctness-first
- helper-centric
- parity-safe
- benchmark-conscious

Fix the confirmed UB / skipped-row correctness issues first, then remove only the clearly redundant fills, then add the missing targeted regressions, and only after that take the bounded optional addendum step.
