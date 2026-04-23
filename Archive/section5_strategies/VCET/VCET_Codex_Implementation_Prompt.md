# Codex implementation prompt: VCET GPU-native optimization, copy-elision, and validation coverage

Use the prompt below with your OpenAI Codex agent.

---

You are working inside a Rust repository that already contains the VCET strategy and its CUDA/GPU-kernel path.

Your job is to **fully implement the VCET optimization plan and addendum** described in the attached markdown design documents, with a strong emphasis on:

- performance,
- correctness,
- copy-elision,
- minimizing unnecessary H2D/D2H transfers,
- using uninitialized allocation only where it is actually safe,
- preserving existing features and behavior,
- and improving benchmark/test coverage for VCET specifically.

## Authoritative design references

Read these files first and treat them as the implementation guide:

1. `gpt54_feature_review_map.md`
   - especially **`## 5. Strategy-local GPU-native optimization and validation routes`**
   - and **`## 16. Allocation policy, prefix initialization, and copy-elision discipline`**
2. `VCET_GPU_Optimization_Implementation_Plan.md`
3. `VCET_GPU_Optimization_Implementation_Plan_Addendum.md`

These three docs are the requirements source of truth. Follow them closely.

## Primary repo files to modify

Find and modify these repo files (paths may vary slightly in the repo; search by filename if needed):

1. `vcet.rs`
2. `vcet_gpu_kernel.rs`
3. `gpu_native_expansion_opt.rs`
4. `composite_validation_workflow.rs`

## Files you may inspect for patterns, and only modify if strictly necessary

Use these as references/patterns when helpful, but keep changes minimal and local unless they are required for compilation or for a shared helper import:

- `gpu_backtest_kernel.rs` (resident upload/runtime scratch/copy-elision patterns)
- any shared allocation helper file that defines or should define `alloc_uninit_copy` or the repo’s equivalent uninitialized-allocation helper
- any type-definition file needed only to wire `vcet: None` / `vcet: Some(...)` into composite request/candidate structs if that field is not already present where the benchmark files expect it
- module export files if new public helpers must be exposed

Do **not** broaden scope into unrelated strategies.

## Scope and constraints

This implementation is **VCET-only**.

The objective is to improve the **strategy-local GPU-native path** for VCET without removing features or breaking functionality.

### Hard constraints

- Preserve VCET behavior and feature semantics.
- Preserve current strategy functionality including:
  - `long_only`
  - `allow_flip`
  - `trade_on_next_bar`
  - `require_vidya_slope`
  - `max_trade_bars`
  - commission handling
- Preserve current explicit rejection of unsupported execution overlays / stop-loss / take-profit in the VCET GPU kernel path unless the repo already has a safe way to support them without changing scope.
- Do not silently change CPU/GPU parity semantics.
- Do not add speculative optimizations that widen ABI scope unless they are explicitly marked optional below and can be completed safely.
- Prefer additive or compatibility-preserving edits. Keep existing public entry points working where practical.
- Use uninitialized memory only when every element is guaranteed to be overwritten before any read.
- Do not replace behaviorally meaningful initialized allocations with uninitialized memory.
- Keep route/fallback behavior explicit. Do not allow a “GPU-native” path to silently degrade through host bounce or CPU fallback without a surfaced reason.

### Performance priorities

In priority order:

1. Remove avoidable host/device bounce paths.
2. Reuse resident GPU buffers across repeated VCET evaluations.
3. Avoid repeat uploads of static/reused data.
4. Replace wasteful zero-filled readback buffers with safe uninitialized allocation helpers where the destination is fully overwritten.
5. Strengthen validation so invalid work is rejected before upload/launch.
6. Add VCET-specific optimization and validation benchmarks so performance changes can be measured.
7. Preserve or improve parity and correctness.

## What to implement

Implement the main plan plus the addendum. The main deliverable is working code, not just comments.

---

# A. `vcet.rs` changes

## A1. Cache descriptor metadata instead of repeatedly resolving it

The plan calls out repeated dynamic-indicator descriptor scans in:

- `resolve_output_id(...)`
- `resolve_param_key(...)`
- `compute_vcet_vidya_signals(...)`
- `compute_vcet_chandelier_signals(...)`
- `compute_vcet_vidya_signal_cuda_device(...)`

Implement a small cached metadata layer for VCET-related indicator descriptors/keys/outputs, using the repo’s preferred one-time-init mechanism (`OnceLock`, equivalent static cache, or another low-overhead safe cache already used in this codebase).

Cache at least:

- VIDYA param keys for short/long/alpha
- VIDYA output id
- Chandelier param keys for period/mult/use_close
- Chandelier output ids for long and short

Keep the existing generic resolution helpers if other code still needs them, but route VCET hot-path setup through the cached metadata.

## A2. Replace small heap-allocated param vectors with stack arrays

In the following functions, replace tiny `Vec<IndicatorParamKV>` allocations with fixed-size arrays passed as slices:

- `compute_vcet_vidya_signals(...)`
- `compute_vcet_chandelier_signals(...)`
- `compute_vcet_vidya_signal_cuda_device(...)`

Do not use `MaybeUninit` for this. Just use plain stack arrays.

## A3. Split strict vs permissive GPU-prep helpers for VIDYA device output

The current GPU-prep path should not silently request `HostF32` and then re-upload that host output back to device for the GPU-native route.

Implement a **strict device-only helper** for the GPU-native path. Representative naming is fine, but keep the intent obvious, e.g.:

- `compute_vcet_vidya_signal_cuda_device_strict(...)`

Behavior requirements:

- Try to get device-resident VIDYA output directly.
- If direct `DeviceF32` output is unavailable, return a clear error instead of bouncing through host output and re-uploading.
- Keep a permissive helper only if needed for non-hot-path or compatibility surfaces.
- Update the actual GPU-native VCET caller(s) to use the strict helper.

Do **not** silently CPU-fallback inside the strict GPU-native path.

## A4. Reduce duplicate Chandelier setup/dispatch where safely possible

`compute_vcet_chandelier_signals(...)` currently treats long and short outputs as separate logical fetches.

Implement the safest possible reduction in duplicated work without widening scope too much:

- centralize and cache shared Chandelier metadata,
- avoid rebuilding equivalent param structures twice,
- and if the current dynamic indicator API can safely retrieve paired outputs together, do that.

If the dynamic API cannot safely do paired retrieval without broader refactoring, keep behavior the same but ensure all shared work is factored once and leave a clear TODO only for the broader paired-output change.

## A5. Optional but worthwhile: cache `CudaChandelierExit` handle per thread/device if cheap and safe

If `compute_vcet_chandelier_signal_cuda_device(...)` is repeatedly constructing `CudaChandelierExit::new(device_id as usize)`, add a small per-thread/per-device cache only if:

- it is simple,
- compile-safe,
- does not widen scope,
- and clearly preserves behavior.

If that wrapper is already cheap or caching would be messy, keep it as-is and document that in your final summary.

## A6. Add/expand `vcet.rs` tests

Add focused tests for CPU/reference logic and boundary semantics:

- `compute_vcet_start_t(...)` with `trade_on_next_bar = true`
- equality-semantics coverage around:
  - `close == vidya`
  - `close == ce_long`
  - `close == ce_short`
- `long_only` should not flip short
- any small deterministic boundary case that protects `require_vidya_slope` behavior

Do not weaken existing tests.

---

# B. `vcet_gpu_kernel.rs` changes

This is the most important implementation surface.

## B1. Strengthen input validation aggressively before launch

Replace or extend `validate_inputs(...)` so it validates both shape and value bounds before any upload or launch.

At minimum validate:

- `signals.cols > 0`
- `signals.close_f32.len() == signals.cols`
- row-count vector lengths match `vidya_rows` / `ce_rows`
- `signals.cols <= i32::MAX`
- `signals.vidya_rows <= i32::MAX`
- `signals.ce_rows <= i32::MAX`
- combo counts fit in `i32`
- all combo vectors have equal length
- `vidya_row_index[i]` is in range `[0, vidya_rows)`
- `ce_row_index[i]` is in range `[0, ce_rows)`
- `start_t[i] >= 0`
- `end_t[i] >= 0`
- `start_t[i] < end_t[i]`
- `end_t[i] <= cols`
- row-pointer entries are nonzero / non-null

Return precise error messages.

This is both a correctness requirement and a performance guard.

## B2. Add an owning uploaded-batch type for VCET GPU evaluation

Implement a reusable uploaded-batch API so repeated VCET evaluations can reuse resident device buffers instead of re-uploading everything on every call.

Use the plan and addendum as the shape guide.

The uploaded batch should own or retain the resources needed for repeated eval. Representative design:

- device id
- series length / row counts / combo count metadata
- resident `d_close`
- resident `d_lr`
- resident device row-pointer buffers for VIDYA / Chandelier long / Chandelier short
- resident combo buffers (or scratch-managed combo buffers if you choose that split)
- resident output buffer
- and, importantly, the owners needed to keep underlying device matrices alive if row pointers refer into owned matrices

The addendum specifically calls out the lifetime problem in a raw-pointer-only struct. Fix that.

It is acceptable to implement either of these designs as long as lifetime safety is preserved:

1. a single uploaded batch that owns all resident buffers and owner matrices, or
2. an uploaded signal batch plus runtime scratch object for per-call combo/output reuse

If you split the design, keep it clean and explicit.

## B3. Preserve the existing one-shot public wrapper for compatibility

Keep `eval_vcet_metrics_gpu_kernel_device_rows(...)` working.

Refactor it internally to use the new upload/eval path.

The compatibility wrapper can do:

1. validate/upload,
2. eval,
3. return decoded metrics.

But it should no longer duplicate logic if the new batch/eval API exists.

## B4. Reuse log returns instead of recomputing them every call

Today the hot path uploads close prices and recomputes log returns each evaluation.

Move log-return computation into the upload phase for the resident batch.

Requirements:

- compute `d_lr` once per uploaded signal batch / resident market batch,
- do not recompute it on every warm evaluation,
- preserve correctness.

Do **not** fuse the log-return kernel into the main backtest kernel unless it is already trivial and clearly safe. The plan explicitly treats that as a non-first-step optimization.

## B5. Reuse runtime scratch where appropriate

Augment `VcetCudaRuntime` with a small scratch/cache layer, inspired by the adjacent shared GPU runtime pattern, but keep it local and simple.

Use it for things like:

- `d_out`
- combo buffers (`d_combo_vidya`, `d_combo_ce`, `d_combo_start`, `d_combo_end`)
- reusable host readback buffer
- cached launch caps / kernel handles if clean and low-risk

If you choose not to cache kernel handles, at least structure the code so that it can be added later.

If you do cache scratch buffers, size-check and reuse them rather than reallocating on every call.

## B6. Use uninitialized allocation only for fully overwritten buffers

Replace zero-filled host readback allocation such as:

- `vec![0.0f32; out_len]`

with the repo’s shared helper (preferred) or a small local safe wrapper only if a shared helper does not exist.

Preferred shape:

- import the shared helper from the repo allocation policy module, e.g. `alloc_uninit_copy`, **if that exists**
- use it for host readback buffers that are fully overwritten by `copy_to(...)`

Requirements:

- only use uninitialized allocation for buffers that are definitely fully overwritten before any read
- do not change observable semantics for partially initialized or behaviorally visible buffers
- if you introduce any `unsafe`, keep it minimal and well documented

## B7. Keep the sync-before-readback unless async semantics are fully audited

Do not remove the explicit stream synchronization before readback unless you also implement and verify the necessary async-copy semantics. The plan explicitly warns against speculative removal here.

## B8. Preserve GPU-path feature constraints

Keep the explicit error for execution overlays / stop-loss / take-profit in the VCET kernel path unless the repo already has a localized, safe support path. Do not silently change feature coverage.

## B9. Optional next-stage ABI improvement: contiguous matrices + row-stride instead of row-pointer table uploads

This is a higher-risk but high-value improvement.

If you can implement it end-to-end safely, with preserved functionality, clear tests, and no unrelated blast radius, you may replace row-pointer-table uploads with a contiguous-matrix + stride-based ABI for VCET signals.

However:

- do **not** do this if it will cause wide repo churn,
- do **not** do it if you cannot complete parity-safe changes in the current session,
- and do **not** let this block the required lower-risk wins.

If not implemented, leave it as an explicit TODO in comments and mention it in the final summary.

## B10. Add/expand `vcet_gpu_kernel.rs` tests

Add tests for:

- length mismatch rejection
- overlay/stop-loss/take-profit blocker behavior
- out-of-range row indices rejection
- invalid `start_t` / `end_t` rejection
- null row-pointer rejection if testable at this layer
- uploaded-batch / lifetime-safety logic if feasible without GPU runtime dependency issues

If GPU runtime is not available in the environment, still add unit tests for pure validation logic and non-launch guard behavior.

---

# C. `gpu_native_expansion_opt.rs` benchmark changes

VCET is currently missing from the optimization benchmark surface. Add it.

## C1. Add VCET imports and request wiring

Add the necessary VCET strategy params import(s).

If the base composite request struct supports a `vcet` field, wire it in and keep other strategies as `None`.

If the field exists elsewhere but the current benchmark file is outdated, make the minimal changes required to compile.

## C2. Add a VCET-specific candle generator

Create a synthetic candle generator that is suitable for VCET trend/reversal behavior. Use the plan’s suggested spirit: not random noise, but a deterministic series with trends, pullbacks, and reversals that can exercise VIDYA + Chandelier logic.

Representative name:

- `make_vcet_trend_reversal_candles(...)`

## C3. Add a VCET request builder

Representative name:

- `make_vcet_req(...)`

Pattern it after the existing per-strategy request builders.

## C4. Add VCET CPU and GPU benchmark groups

Add VCET to the optimization benchmark harness with the same style as the existing strategies.

At minimum add:

- CPU benchmark case(s)
- GPU-kernel benchmark case(s)

Preserve route probe printing.

The benchmark output should print route diagnostics like the existing cases, including:

- `backend_used`
- `backend_route`
- `backend_fallback_reason`

Do not allow silent fallback to masquerade as a real GPU result.

## C5. Prefer separating timing boundaries where practical

If it is feasible without excessive churn, add more than one VCET benchmark surface:

1. end-to-end request benchmark,
2. warm-eval microbench using the new uploaded-batch API,
3. optionally an upload-only/cold-path benchmark.

If that is too invasive for the current benchmark harness, at minimum add the end-to-end VCET benchmark now and note the warm-eval split in comments/final summary.

## C6. Add VCET to `criterion_group!`

Make sure the new VCET benches are actually included.

---

# D. `composite_validation_workflow.rs` changes

VCET is currently missing from the validation benchmark surface. Add it.

## D1. Add VCET imports and candidate wiring

Add the necessary VCET imports.

If `CompositeParams` supports a `vcet` field, wire `vcet: None` or `vcet: Some(...)` as appropriate.

If the field is missing only because this file is stale relative to the rest of the repo, make the smallest compile fix required.

## D2. Add VCET candidate generator

Add a dedicated candidate builder for VCET.

Representative name:

- `make_vcet_candidates(n: usize) -> Vec<CompositeParams>`

Use the same style as the existing per-strategy candidate builders.

Make the generated parameter space deterministic and sensible for VCET.

## D3. Add VCET CPU validation profile runs

Add VCET to the CPU validation profile calls alongside existing strategy-specific validation runs.

## D4. Add VCET GPU validation profile runs

Add VCET to the GPU validation profile calls alongside existing strategy-specific validation runs.

## D5. Add one dedicated large GPU VCET validation group

Add at least one dedicated VCET GPU validation benchmark group on a larger candle/candidate set, similar in spirit to the existing dedicated ASR GPU workflow.

Preserve route probe printouts.

## D6. Keep route diagnostics visible

Like the other validation cases, print route metadata and fallback reason so VCET benchmark evidence is trustworthy.

---

# E. Copy-elision and uninitialized-allocation policy

Follow this exactly.

## Safe places to use uninitialized allocation

Use the shared helper (preferred) or a minimal safe wrapper for:

- host readback buffers that are fully overwritten by `DeviceBuffer::copy_to(...)`
- reusable scratch buffers that are guaranteed to be fully overwritten before reads
- device buffers that are always fully written before any read, when the existing code already safely uses `DeviceBuffer::uninitialized(...)` / async equivalent

## Do not use uninitialized allocation for these

Do **not** convert these to uninitialized memory unless you can prove full overwrite before any read and preserve semantics:

- `build_vcet_desired_positions(...)` output vector (the zero-fill is behaviorally meaningful)
- any partially written host vectors
- small fixed param collections (use stack arrays, not `MaybeUninit`)
- any user-visible or observable buffer whose initial fill matters

## Preferred policy

- Prefer existing shared alloc-policy helpers over new bespoke `unsafe`.
- If a helper already exists (e.g. `alloc_uninit_copy` or equivalent), use it.
- If not, add the smallest possible shared/local helper with strong safety comments.
- Keep unsafe blocks tiny and documented.
- Do not add speculative debug-poison machinery unless it is already part of the repo’s alloc policy or can be added cleanly.

---

# F. Optional extensions beyond the core scope

Implement these only if they are clearly local, correctness-safe, and you can finish them completely without destabilizing the required work.

## F1. Contiguous matrix + stride ABI

Eliminate row-pointer-table uploads by moving VCET signal batches to contiguous matrices plus stride metadata.

Only do this if the required code changes are local and end-to-end.

Otherwise, leave a TODO and mention it in the final summary.

## F2. Pinned host memory / async copies for large cold batches

Only consider this after the lower-risk reuse work is complete.

Do not let this delay or complicate the required implementation.

## F3. Explicit route diagnostics for strict device-prep failure

If device-only prep fails, preserve a clear surfaced reason. This is worth doing if there is a natural local place to thread the error message through.

## F4. Additional no-poison or uninitialized-allocation tests

If you add more uninitialized readback helpers, add focused tests or assertions that make it harder for uninitialized junk to leak into decoded results.

---

# G. Suggested implementation order

Follow this order unless the repo structure forces a nearby variation:

1. Read the three design docs listed above.
2. Inspect current `vcet.rs` and `vcet_gpu_kernel.rs` surfaces.
3. Implement stricter validation in `vcet_gpu_kernel.rs`.
4. Implement metadata caching and stack-array param cleanup in `vcet.rs`.
5. Split strict/permissive VIDYA device helpers and route GPU-native use to strict mode.
6. Implement the uploaded-batch + lifetime-safe owner model in `vcet_gpu_kernel.rs`.
7. Add runtime scratch reuse for combo/output/readback buffers.
8. Replace wasteful zero-filled host readback buffers with shared uninitialized-allocation helper(s).
9. Refactor the one-shot wrapper to use upload + eval.
10. Add/expand VCET tests in `vcet.rs` and `vcet_gpu_kernel.rs`.
11. Add VCET optimization benchmarks in `gpu_native_expansion_opt.rs`.
12. Add VCET validation benchmarks in `composite_validation_workflow.rs`.
13. Run formatting / tests / compile checks.
14. Only then consider optional higher-risk changes.

---

# H. Acceptance criteria

Your implementation is not done until all of the following are true.

## Correctness and behavior

- VCET functionality still works.
- Existing tests continue to pass.
- New tests cover the new validation logic and key VCET boundary semantics.
- GPU-native strict device prep no longer silently bounces through host output in the hot path.
- Unsupported GPU-path features are still explicitly rejected rather than silently changed.

## Performance structure

- Warm repeated VCET evaluations no longer re-upload all static data every call.
- Log returns are not recomputed on every warm eval.
- Host readback no longer zero-fills buffers that are fully overwritten.
- Tiny hot-path heap allocations for param lists are removed.
- Invalid inputs are rejected before costly uploads/launches.

## Benchmark coverage

- VCET exists in `gpu_native_expansion_opt.rs`.
- VCET exists in `composite_validation_workflow.rs`.
- Route/fallback diagnostics are printed for the new VCET benchmark cases.

## Safety and scope

- Uninitialized allocation is only used where it is safe.
- Lifetime safety for device row ownership is improved, not weakened.
- Changes are localized to VCET and the minimal wiring required for VCET benchmarks.

---

# I. Validation commands to run

Run the relevant formatting, compile, and test commands that make sense for the repo.

At minimum, try to run the most targeted commands possible. Examples (adapt to repo structure):

- `cargo fmt --all`
- `cargo test --lib vcet -- --nocapture`
- `cargo test vcet -- --nocapture`
- `cargo test --features cuda,cuda-backtest-kernel vcet_gpu_kernel -- --nocapture`
- `cargo test --features cuda,cuda-backtest-kernel --no-run`
- targeted benchmark compile checks if feasible

If the environment lacks CUDA hardware/runtime, still do as much as possible:

- compile-check feature-gated code,
- run non-GPU unit tests,
- run pure validation tests,
- and clearly state what could not be executed.

---

# J. Deliverables and final report format

When you are done, provide a concise but complete final report with these sections:

1. **Files changed**
   - list every file modified
   - note whether each change was required or optional

2. **What was implemented**
   - metadata caching
   - stack-array param cleanup
   - strict device-only VIDYA prep
   - uploaded-batch / scratch reuse
   - readback copy-elision
   - validation hardening
   - benchmark additions
   - test additions

3. **Performance-impact summary**
   - explicitly call out which H2D/D2H copies were removed or amortized
   - explicitly call out which allocations were eliminated or reused
   - explicitly call out where uninitialized allocation is now used and why it is safe

4. **Correctness/parity notes**
   - anything preserved intentionally
   - any remaining parity risks or TODOs

5. **What you ran**
   - formatting
   - tests
   - compile checks
   - benchmarks if any

6. **Remaining optional follow-ons not implemented**
   - contiguous matrix + stride ABI
   - pinned host memory / async copies
   - any other deferred higher-risk ideas

---

# K. Important reminders

- Do not silently widen scope.
- Do not break other strategies.
- Do not remove features.
- Do not replace behaviorally meaningful initialized buffers with uninitialized allocation.
- Do not leave the GPU-native hot path with a hidden HostF32 -> device bounce.
- Do not consider the task done without adding VCET benchmark coverage in both benchmark files.
- Prefer small, high-confidence, performance-positive edits over broad speculative refactors.

Implement the changes now.
