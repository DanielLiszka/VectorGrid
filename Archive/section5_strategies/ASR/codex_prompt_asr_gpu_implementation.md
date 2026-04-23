# Codex implementation prompt — ASR section-5 GPU-native optimization and validation plan

You are working in a Rust/CUDA repo. Your job is to **fully implement the ASR-specific GPU-native optimization, validation, and benchmark plan** described in the attached planning documents, while preserving existing behavior and feature coverage.

This is a **real implementation task**, not an analysis-only task. Make code edits, add tests/benches, and run the relevant checks.

## Source-of-truth documents to read first

Read these documents before making changes, and treat them as the implementation specification:

1. `ASR_section5_gpu_native_optimization_plan.md`
2. `ASR_section5_gpu_native_optimization_plan_addendum.md`
3. `gpt54_feature_review_map.md` — specifically `## 5. Strategy-local GPU-native optimization and validation routes`

If these docs are attached rather than checked into the repo, use the attached versions as the source of truth. If paths differ slightly, locate them by basename.

## Primary repo files to modify

These are the main files that must be updated. Prefer these repo-relative paths if they exist:

- `crates/app_core/src/asr.rs`
- `crates/app_core/src/asr_gpu_kernel.rs`
- `crates/app_core/benches/gpu_native_expansion_opt.rs`
- `crates/app_core/benches/composite_validation_workflow.rs`

Also inspect and modify any directly related adjacent files if needed for correctness, tests, or bench registration, including:

- the ASR GPU kernel source / PTX embedding source if the repo contains the actual kernel implementation for `asr_backtest_metrics_f32` or `pack_rows_u64_ptr_f32`
- feature-flag declarations / Cargo manifests if new guarded instrumentation flags are added
- existing helper/test modules if the new helpers should live there instead of inline

If a listed file path is slightly different in the repo, locate it by basename and modify the canonical file.

---

## Mission and constraints

### Mission

Implement the ASR strategy performance/correctness plan with emphasis on:

- reducing unnecessary allocations
- reducing unnecessary host<->device and device<->device copies
- using uninitialized memory safely where full overwrite is guaranteed
- improving warm-path GPU throughput
- preserving route transparency and fallback correctness
- tightening host-side validation before kernel launch
- improving benchmark fidelity so GPU-labeled benches cannot silently measure fallback paths
- preserving feature behavior and CPU-reference parity

### Hard constraints

Do **not**:

- remove ASR features
- change ASR semantics for supported CPU or GPU routes
- broaden GPU support to overlays / stop-loss / take-profit in the same patch set unless the repo already contains a parity-complete implementation and you can prove equivalence
- weaken the CPU reference path; it remains the parity anchor
- add unsafe uninitialized-memory usage unless every element is provably overwritten before any read
- silently skip uploads based on weak or unsafe cache identity assumptions
- hide fallback behavior from benchmarks or callers

### Required engineering posture

- Prefer small, correctness-preserving wins first.
- Reuse existing repo helpers if equivalents already exist.
- Follow existing code style, naming, feature-flag patterns, and error handling.
- Favor explicit route/fallback assertions in benchmarks and tests.
- Keep higher-risk changes behind follow-up commits or feature gates when appropriate.

---

## What to implement

Implement the plan in the docs comprehensively, with the following priorities.

### Mandatory scope: implement these items fully

Treat the following as required unless impossible due to missing source/API. If something is impossible, explain exactly why in the final summary and leave the code in the best state short of that blocked item.

#### In `asr.rs`

1. Add a centralized GPU route-blocker surface:
   - add `AsrGpuKernelBlocker`
   - add `pub fn asr_gpu_kernel_blocker(strategy: &StrategyConfig) -> Option<AsrGpuKernelBlocker>`
   - use it from GPU entrypoints / benches / tests instead of duplicating inline blocker checks

2. Add centralized z-score fast-path logic:
   - add `pub fn asr_uses_direct_zscore_cuda_fast_path(cfg: &AsrStrategyParams) -> bool`
   - replace existing inline/default-route assumptions with this helper
   - keep current route behavior intact unless parity-backed expansion is explicitly implemented

3. Refactor `compute_asr_kaufman_stops_cuda_device(...)`:
   - hoist `CudaKaufmanstop::new(...)` wrapper construction out of per-direction closures if feasible
   - deduplicate repeated parameter setup / descriptor lookup / output-id lookup
   - avoid repeated warm-path setup work
   - preserve current fallback behavior

4. Add convenience helpers for device-native ASR signal use when appropriate:
   - e.g. a safe helper like `collect_device_row_ptrs(...)` or equivalent
   - centralize row-pointer arithmetic in `asr.rs` if that reduces duplication and future errors

5. Preserve explicit route boundaries:
   - overlays, stop-loss, and take-profit remain explicit GPU blockers unless a parity-complete implementation already exists and passes tests

#### In `asr_gpu_kernel.rs`

1. Extend `AsrCudaRuntime` into a real reusable runtime/cache/scratch owner.
   It should be able to reuse:
   - resident market series (`close`, `log returns`)
   - combo buffers
   - output buffers
   - dense signal buffers
   - Kaufman-stop buffers
   - row-pointer buffers
   - row-pack scratch buffers
   - optional host scratch buffers for dense-row compaction

2. Add or reuse helpers equivalent to the plan’s proposed primitives (names may differ if repo style suggests better names):
   - `upload_slice_reuse(...)`
   - `ensure_uninit_device_len(...)`
   - `fill_close_and_log_returns_f32(...)`
   - `copy_device_to_vec_uninit(...)`
   - `checked_i32(...)`
   - `checked_u32(...)`
   - `validate_combo_ranges(...)`
   - a centralized `build_launch_dims(...)` or equivalent checked launch builder
   - if safe cache identity exists: `upload_signal_if_changed(...)`
   - row remap helpers such as `remap_combo_rows(...)` / `remap_combo_rows_sorted(...)`
   - compaction heuristic such as `should_compact_rows(...)`

3. Implement resident market-series reuse:
   - avoid rebuilding `close_f32` and `lr_f32` from scratch on every call when the candle set is unchanged
   - combine close conversion and log-return generation into one pass
   - avoid repeated device allocation/upload for unchanged market series
   - do this in both `eval_asr_metrics_gpu_kernel(...)` and `eval_asr_metrics_gpu_kernel_device_rows(...)`

4. Replace zero-initialized output readback with safe uninitialized readback:
   - use a shared helper if one already exists in the repo
   - otherwise implement the `MaybeUninit` pattern safely
   - add a debug-only or feature-gated overwrite proof if useful (`asr-debug-full-output-check` or repo-equivalent)

5. Reuse combo and output buffers everywhere:
   - `fast_row_index`
   - `slow_row_index`
   - `start_t`
   - `end_t`
   - `d_out`

6. Improve dense signal handling:
   - reuse dense signal device buffers instead of allocating each call
   - if and only if correctness-safe cache identity exists, skip unchanged dense signal uploads
   - otherwise keep the always-upload path and only reuse allocations

7. Improve the device-row path without changing ASR scoring semantics:
   - compact to only the actually used fast/slow rows for the current combo batch
   - sort compacted row subsets ascending before gathering row pointers if that helps memory access behavior
   - add a heuristic so compaction is skipped when coverage is already near full and compaction would just add CPU overhead
   - remap combo row indices consistently into the compact domain
   - reuse row-pointer and packed-row buffers across calls

8. Mirror worthwhile row-subset compaction in the dense path:
   - if only a strict subset of rows is needed for the current combo batch, pack/upload only those rows when the heuristic says it helps
   - do not make this more complex than it needs to be; same ABI, same outputs

9. Add strict host-side validation before every launch:
   - validate combo fast-row ranges
   - validate combo slow-row ranges
   - validate combo start/end windows against the true kernel contract
   - replace unchecked integer casts with checked conversions
   - fail fast on invalid host inputs

10. Add opt-in timing instrumentation for ASR kernel phases if practical and aligned with repo style:
    - market prep/upload
    - dense signal upload
    - row-pointer upload
    - row packing
    - kernel launch
    - readback
    - decode
    - guard with a feature flag or debug cfg (for example `asr-cuda-phase-timing`) instead of affecting release behavior

11. Keep the existing GPU route blockers explicit:
    - overlays
    - stop-loss
    - take-profit
    - use the centralized helper from `asr.rs`

12. Search for the actual kernel/PTX source for `asr_backtest_metrics_f32` and `pack_rows_u64_ptr_f32`.
    - If the source exists in the repo and can be safely edited, implement the highest-value low-risk kernel-side improvement that matches the docs:
      - preferably a true row-pointer-native ASR scoring variant, **or**
      - a middle-ground pack-fusion / reduced-launch-count improvement
    - Only do this if the source is present and you can preserve parity with tests.
    - If kernel source is not available or is generated elsewhere, complete the full Rust-side plan and explicitly note the limitation.

#### In `gpu_native_expansion_opt.rs`

1. Add real ASR optimizer benchmark coverage. The current file does not benchmark ASR.

2. Add ASR request builders and benchmark cases for at least:
   - CPU ASR
   - GPU ASR default route
   - GPU ASR `long_only`
   - GPU ASR `allow_flip = false`
   - GPU ASR `trade_on_next_bar = true`

3. Add route assertions / preconditions so GPU-labeled ASR benches do not silently run on fallback routes.

4. Keep warm-path interpretation honest:
   - if the bench shape is warm-path dominated, label or document it clearly
   - do not present a warm-cache bench as a cold-start result

5. If the bench structure allows it cleanly, add narrower ASR-specific microbench coverage:
   - direct kernel route only
   - used-row-fraction sweeps for device-row compaction

#### In `composite_validation_workflow.rs`

1. Make ASR GPU validation benchmarks fail on fallback.
   - assert `backend_used == GPU`
   - assert no fallback reason
   - keep route metadata visible in failures/logging

2. Split ASR validation coverage into at least:
   - direct/native z-score fast-path cases
   - generic-zscore fallback-period-sweep cases

3. Keep ASR candidate generation broad enough to cover non-default route behavior:
   - vary `z_ma_type`
   - vary `z_devtype`
   - preserve valid ASR configs

4. If validation API supports it cleanly, add explicit negative-route / blocker cases (route assertions rather than mislabeling them as GPU throughput benches):
   - stop-loss enabled
   - take-profit enabled
   - overlays enabled

---

## Additional items from the addendum that should be implemented where feasible

These go beyond the first-wave plan but are still strongly desired if they can be done safely in the current repo.

1. **Cache correctness hardening**
   - Do **not** rely on weak sampled fingerprints (`ptr + first/mid/last bits`) as the final production-safe identity for skipping uploads.
   - Prefer correctness-safe generation keys / explicit cache keys plumbed from callsites.
   - If no safe identity is available, still reuse allocations but keep uploads unconditional.
   - Document which caches are identity-safe and which are not.

2. **Skip unchanged signal uploads where safe**
   - This is distinct from buffer reuse and should be treated as a first-class optimization.
   - Apply to dense signal matrices and Kaufman-stop vectors only when identity is correct.

3. **Compaction refinement**
   - sort compacted used-row sets ascending
   - add a compaction threshold heuristic (for example only compact when used coverage is materially below full coverage)
   - benchmark the threshold or at least choose a conservative default and note it

4. **Debug safety for uninitialized output**
   - add a guarded overwrite-proof or sentinel-based debug check so the release uninitialized readback path is backed by evidence

5. **Checked launch-dimension construction**
   - centralize block/grid/stride conversions and bounds checks
   - avoid scattered `as i32` / `as u32` conversions

6. **ASR kernel-phase timing / microbenchmark surface**
   - add timing instrumentation and, if the bench architecture supports it, a direct ASR kernel microbenchmark
   - separate copy-elimination wins from kernel-math wins

7. **Additional ASR optimizer benchmark variants**
   - include supported strategy-flag variants, not just default `StrategyConfig`

---

## What not to do in the first patch set

Unless the repo already contains a parity-complete implementation and tests prove it, do **not** do the following in the main patch set:

- broaden GPU semantics to overlays / stop-loss / take-profit
- rewrite ASR scoring semantics
- remove the CPU reference path
- assume non-default z-score direct support is safe without parity tests
- optimize away real producer->consumer boundaries that still exist
- introduce global runtime caching if thread-local caching is currently the stable model, unless you can justify and prove the concurrency model

Pinned memory, async copies, device-side top-k reduction, process-global runtime caching, kernel handle caching, and occupancy tuning are **optional phase-2 items**. Implement them only if:

- the necessary APIs/source exist in the repo,
- they are low-risk,
- they do not interfere with the main safe changes,
- and you can test them.

If you do not implement those optional items, leave concise TODO notes or a final summary section identifying them as follow-up work.

---

## Required test and validation coverage

Add or update tests so the implementation is defensible.

### Helper/unit tests

Add tests for helper logic where practical:

- row remap helpers (`remap_combo_rows`, sorted remap variant, etc.)
  - empty
  - repeated indices
  - sparse subset
  - full coverage
  - negative row -> error
  - out-of-range row -> error
- row packing helpers for dense subsets if those exist
- `asr_gpu_kernel_blocker(...)`
- `asr_uses_direct_zscore_cuda_fast_path(...)`
- checked launch-dimension helpers
- uninitialized-copy helper if there is a safe way to test it with device buffers in the repo’s test setup

### Parity tests

For GPU-supported ASR configs, add or update tests that compare:

- CPU reference path
- `eval_asr_metrics_gpu_kernel(...)`
- `eval_asr_metrics_gpu_kernel_device_rows(...)`

Cover at least these supported strategy flags:

- `long_only = true/false`
- `allow_flip = true/false`
- `trade_on_next_bar = true/false`
- representative `confirm_bars`
- representative `mom_fade_bars`
- `use_er_filter = true/false`
- direct-zscore route
- generic-zscore route

### Route-boundary tests

Explicitly verify that ASR GPU kernel routes reject or downgrade as expected for:

- overlays enabled
- stop-loss enabled
- take-profit enabled

Also verify that those same configs still work on the CPU reference route.

### Benchmark validation

Run the relevant benches/tests after implementation and ensure benchmark labeling is honest. At minimum run the most relevant ASR benches/tests that exist after your changes.

---

## Implementation workflow you should follow

1. Read the three source-of-truth documents fully.
2. Inspect the current implementations in the four primary files.
3. Search the repo for the ASR kernel/PTX source and related helpers.
4. Implement the changes in waves:
   - Wave 1: route helpers, Kaufman-stop cleanup, runtime reuse foundation, uninitialized readback, combo/output reuse, host-side validation
   - Wave 2: row-subset compaction and dense/device-row copy reductions
   - Wave 3: benchmark coverage and route-assertion hardening
   - Wave 4: optional kernel/PTX or phase-2 work if source/support exists
5. Run formatting, compile/tests, and the most relevant ASR benches feasible in the environment.
6. Fix any regressions.
7. Provide a final summary with changed files, tests/benches run, and any blocked optional items.

Do not stop after planning. Actually modify the code.

---

## Practical implementation details and preferred patterns

Use these patterns from the planning docs unless existing repo helpers already cover them:

### Runtime/cache design

- extend `AsrCudaRuntime` to own reusable scratch rather than only `context/module/stream`
- prefer allocation reuse first
- skip uploads only when identity is correctness-safe
- preserve current thread-local/runtime style unless a better repo-local pattern already exists

### Market-series handling

- merge close conversion + log-return computation into one host pass if still host-side
- reuse resident device close/log-return buffers when the candle set is unchanged
- if the repo already has a shared resident-market abstraction, prefer integrating with it instead of duplicating logic

### Uninitialized memory

- use `MaybeUninit` only when every element is guaranteed to be overwritten before any read
- prefer a shared repo helper if available
- optionally add a debug-only overwrite/sentinel check behind a feature flag

### Compaction

- compact only when it actually reduces work enough to matter
- sort used rows ascending before gathering row pointers when helpful
- keep remapping logic easy to audit
- avoid turning a simple fast path into an overengineered branch maze

### Validation and launch safety

- centralize combo-range checks
- centralize checked launch-dimension construction
- convert host invalid-input UB into deterministic Rust errors

### Benchmarks

- do not let a benchmark labeled `gpu_kernel` silently measure fallback/CPU work
- keep warm-path vs cold-path interpretation explicit
- separate route variants that exercise different ASR backend behavior

---

## Deliverables

Your final response should include:

1. A concise summary of what you changed.
2. The exact files modified.
3. Any new helpers/tests/benches added.
4. What you ran to validate the changes.
5. Whether the ASR kernel/PTX source was found and modified.
6. Any optional items intentionally left for follow-up, with reasons.
7. Any behavior/route limitations that still remain by design.

If something in the planning docs conflicts with the actual repo code, choose the safest implementation that preserves behavior, and explain the deviation in the final summary.

---

## Success criteria

The work is successful if all of the following are true:

- ASR warm-path GPU runs allocate and upload less than before
- unnecessary host zero-fills for output readback are removed safely
- market-series/device-buffer reuse is in place
- device-row mode avoids packing full unused row sets when that is beneficial
- benchmark labels are honest and fail on unexpected fallback
- CPU/GPU parity is preserved on supported GPU configs
- unsupported GPU feature cases remain explicit and correctly routed
- functionality is not removed and feature behavior is not broken

Proceed with implementation.
