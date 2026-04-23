# Fourth-Pass Review: `crates/app_core/src/standalone_gpu_runtime.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/standalone_gpu_runtime.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/standalone_gpu_runtime.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/standalone_gpu_runtime.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/standalone_gpu_runtime.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The live file is only ~95 lines, but it sits on a CUDA-runtime-critical seam and uses three different kinds of sensitive policy in a very compressed form: identity-based cache keys, uninitialized host/device scratch allocation, and prefix-copy helpers that present a `Result` API.

The third-pass packet kept the bucket unchanged, which is still correct, but it drifted upward into abstract cache-scope and runtime-truth language that is not strongly anchored to this file itself. `standalone_gpu_runtime.rs` does not own planner policy, thread-local diagnostics, `OnceLock`, IPC, persistence, or GPU ABI. Its actual review surface is narrower and more concrete: bounds behavior, overwrite-before-read contracts, and cache-key invalidation assumptions.

That is why this file warranted one more pass: not to widen the action bucket, but to replace some abstract framing with a more evidence-backed local hardening list.

## Fourth-Pass Research Log

This pass re-read the live source and prior packets first, then checked the local repo guidance and a few primary-source API docs that directly govern the helper contracts.

1. Live source review
   - `standalone_gpu_runtime.rs:9-95`
   - What it established:
     - the file owns only identity keys, a resident close/return cache payload, a slice hash helper, a usize→i32 guard, two prefix-copy helpers, and one host scratch resize helper
     - there is no local telemetry, no planner logic, no `thread_local!`, and no `OnceLock`

2. Repo guidance re-check
   - `AGENTS.md:139-144`
   - `REPO_MAP.md:1010-1025`
   - What it established:
     - this repo explicitly allows malloc-like / uninitialized scratch only when every element is guaranteed to be written before any read, and it says those safety assumptions should stay next to the code path
     - `standalone_gpu_runtime.rs` is named as one of the hot consumers of that policy, so the local helper comments matter more than usual here

3. `cust::memory::DeviceBuffer` docs
   - What it established:
     - `DeviceBuffer::uninitialized` is unsafe because callers must initialize contents before any read
     - `drop_async` is stream-ordered and is specifically tied to async free / memory-pool semantics
     - `uninitialized_async` has stricter stream-ordering constraints than the current helper uses

4. `cust` `device_slice.rs` source on docs.rs
   - What it established:
     - `DeviceSlice::index` panics on out-of-bounds ranges
     - `CopyDestination::copy_from` / `copy_to` assert exact length equality
   - Why it matters here:
     - `copy_host_to_device_prefix` and `copy_device_to_host_prefix` currently return `Result<_, String>`, but they can still panic before producing that `Result` if the caller hands them a prefix longer than the buffer (`standalone_gpu_runtime.rs:73-86`)

5. Rust `DefaultHasher` docs
   - What it established:
     - the algorithm is unspecified across releases
   - Why it matters here:
     - the earlier packets were right to treat `UploadSignature` as a local reuse heuristic rather than durable identity, but that is not the highest-priority issue in this file

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **stand**.

This is still a `Small surgical refactor`, but the real justification is tighter than the third-pass packet suggested: there are a few local correctness/contract hardenings worth doing, and none of them require widening the module or rethinking the shared-runtime design.

### What the current packet got right

The current packet was directionally right on four important points.

First, it correctly resisted a wider refactor. This helper should stay small and utility-shaped.

Second, it was right not to recommend replacing identity-based cache keys with full content hashing by default. For close-series caches, that would trade a cheap identity check for per-call O(n) host work on a hot path.

Third, it was right that `UploadSignature` should be treated as a local heuristic, not durable public identity.

Fourth, it was right that allocation-reuse behavior is likely the reason this file exists and should not be abstracted away casually.

### What the current packet missed or underweighted

#### 1. The prefix-copy helpers have a real panic surface that is more concrete than the packet's abstract cache discussion

`copy_host_to_device_prefix` and `copy_device_to_host_prefix` both slice the device buffer with `buf.index(..values.len())` before copying (`standalone_gpu_runtime.rs:73-86`). In `cust`, that indexing path panics on out-of-bounds ranges, and the copy traits assert exact length equality.

So these helpers do **not** currently behave like fully fallible helpers even though their signatures return `Result<_, String>`. If a caller hands them a prefix larger than the device buffer, the failure mode is a panic, not an `Err`.

That is a better fourth-pass finding than the third-pass packet's generalized cache/runtime-contract language, because it is local, concrete, and cheap to harden.

#### 2. `ensure_host_f32_len` carries the same overwrite-before-read contract as `ensure_device_buffer_len`, and the earlier packets barely called that out

The first-pass packet correctly called out the unsafe invariant on `DeviceBuffer::uninitialized`, but the host-side scratch helper is just as contract-sensitive in this repo.

`ensure_host_f32_len` uses `alloc_uninit_copy(required)` or `resize_uninit_f32(buf, required)` (`standalone_gpu_runtime.rs:89-94`). Local repo guidance explicitly says that such allocation is only valid when every element is overwritten before any read, and that those assumptions should stay next to the code path (`AGENTS.md:139-144`, `REPO_MAP.md:1010-1025`).

That means the missing comment is not only a device-memory problem. It is also a host-scratch problem.

#### 3. The pointer-identity key is not just "contract-sensitive"; it depends on caller-side immutability for the cache lifetime

`CloseSeriesKey` only records `len`, `host_ptr`, and `device_ptr` (`standalone_gpu_runtime.rs:10-23`). That means same-pointer / same-length in-place mutation is indistinguishable from identical content.

The earlier packets described this as a fast identity key, which is true, but they underweighted the sharper contract: this is only valid while callers treat the referenced close series and borrowed device pointer as immutable for the cache lifetime, or explicitly invalidate when reusing the same allocation for different data.

That is still not a reason to switch to content hashing by default. It is a reason to document the immutability / invalidation rule more explicitly.

#### 4. The third-pass verification advice is weaker than the earlier packet's adapter-level verification

The third-pass packet suggested `cargo test -p ta_app_core standalone_gpu_runtime -- --nocapture` plus generic cache/reset/telemetry tests.

That is not well grounded in the live file. This file has no local test module, no telemetry, and no planner/runtime routing surface. The more credible verification surface is still the dependent CUDA adapter coverage that actually exercises these helpers. The third-pass verification section became more abstract at exactly the point where it should have stayed more concrete.

### What the current packet overstated or weakly supported

#### 1. The `OnceLock` / `thread_local!` research is not tied closely enough to this source file

The third-pass packet pulled in `OnceLock` and `thread_local!` reasoning, but the live file contains neither. That research is relevant to larger runtime modules such as `gpu_backtest_kernel.rs`; it is not the best frame for a 95-line helper module that only owns cache keys, small copy helpers, and scratch-sizing helpers.

The result is that the packet sounds more sweeping than the file warrants.

#### 2. "Planner/runtime rewrites" is not the right regression frame here

This module does not own planner logic. The practical regression surface is much narrower:

- CUDA runtime correctness
- stale-cache reuse if identity assumptions are violated
- host/device scratch safety if partially written buffers are read
- minor performance regressions if cheap identity checks are replaced with expensive content checks

That is the level this packet should stay on.

### Safe-now hardening

These are the changes I would classify as safe-now and worth doing if the file is touched.

1. Add explicit length guards in both prefix-copy helpers before calling `buf.index(..values.len())`.
   - Return an `Err(...)` when the requested prefix exceeds the device buffer length.
   - This aligns the implementation with the public shape of the helpers: they already promise fallible behavior via `Result`.
   - This is a narrow correctness hardening with negligible runtime cost compared with the actual copy.

2. Add local safety comments to **both** `ensure_device_buffer_len` and `ensure_host_f32_len`.
   - For the device helper, state that any newly grown region is uninitialized and must be fully written before any read.
   - For the host helper, state the same thing for resized host scratch.
   - This is directly aligned with the repo's allocation-policy guidance, and it is the kind of local contract the repo explicitly asks to keep next to the code path.

3. Add a short contract comment to `CloseSeriesKey`.
   - The important point is not just that the key is identity-based.
   - The important point is that cache reuse is only sound while the referenced close series / borrowed device buffer are treated as immutable for that cache lifetime, or callers invalidate when reusing the same allocation for different contents.

4. Optionally tighten the helper names or comments around `UploadSignature`, but only as documentation.
   - The earlier packets were basically right here.
   - I would not spend code churn on this unless nearby comments are already being edited.

### Larger redesign still deferred

These ideas remain deferred, and the current packet should have been more explicit that they are **not** safe-now changes.

1. Do **not** replace `CloseSeriesKey` with content hashing by default.
   - That would add host-side O(n) work to a path that currently uses cheap identity checks.
   - If stale-cache reuse ever becomes a real observed problem, the safer direction is an explicit epoch/invalidation signal at the owning runtime layer, not silent full-content hashing in this helper.

2. Do **not** opportunistically switch `ensure_device_buffer_len` to `uninitialized_async`.
   - `cust` documents stricter stream-order requirements for async allocation than for the current synchronous allocation path.
   - That is a coordinated runtime change, not a local polish edit.

3. Be cautious about changing the reallocation order inside `ensure_device_buffer_len`.
   - The current implementation drops the old buffer asynchronously before allocating the new one (`standalone_gpu_runtime.rs:63-68`).
   - That means the helper does not have strong "leave old state untouched on failure" semantics today.
   - There is a reasonable argument for allocating first and swapping only after success, but that changes error-path behavior and should be treated as a deliberate runtime change, not a drive-by cleanup.

4. Do **not** create a new local scratch-allocation abstraction here.
   - If the repo wants safer or more expressive uninitialized-scratch APIs, that belongs in `alloc_policy.rs`, which the repo map already names as the canonical surface.

## Implementation Priority

1. **Highest priority:** add prefix-length guards to the copy helpers.
   - This is the one fourth-pass item that upgrades a real failure mode from panic to `Err` without changing the intended fast path.

2. **Second priority:** add explicit overwrite-before-read comments to the host/device scratch helpers.
   - This is low-risk and directly aligned with repo policy.

3. **Third priority:** document the `CloseSeriesKey` immutability/invalidation contract.
   - This is important for future maintainers, but it is documentation hardening, not evidence of a live bug.

Everything larger than that should remain deferred.

## Verification If Changed

The third-pass packet's verification plan is too abstract for this file.

If only comments change, a normal compile / lint lane is enough.

If code changes are made, especially the prefix-length guards, the important check is not a file-local target. The important check is the dependent CUDA adapter coverage that actually uses this helper surface. In other words:

- prefer the strategy-local CUDA adapter tests the earlier packet pointed at over a `standalone_gpu_runtime`-named target, because the live file has no local test module
- re-run the owning adapter tests under `--features cuda,cuda-backtest-kernel`
- if practical, also run a broader `ta_app_core` CUDA-featured test pass for the standalone GPU adapter family touched by this helper

I would explicitly **not** treat "telemetry" verification as a required check for this file. There is no telemetry surface here to preserve.

## Regression And Feature-Preservation Constraints

- Preserve the current grow-only reuse behavior unless a broader runtime review is being done.
- Preserve identity-based cache keys by default; do not replace them with per-call content hashing casually.
- Preserve the current scratch-allocation posture; do not switch to zero-fill or NaN-fill buffers unless behavior actually requires observable initialization.
- If adding bounds checks to the copy helpers, keep the success path allocation-free and branch-light.
- Do not switch to async allocation in this helper without auditing caller stream-order assumptions.
- Treat any reallocation-order change in `ensure_device_buffer_len` as a behavior change, not a comment-only cleanup.
- There is no meaningful IPC, persistence, or GPU ABI contract in this file itself. The real regression surface is runtime correctness, cache invalidation behavior, and CUDA-adapter verification.

## Fourth-Pass Conclusion

The third-pass bucket should **remain** `Small surgical refactor`, but the current third-pass packet is not the best framing of why.

What it got right: keep the module small, keep allocation reuse, and do not overreact by replacing local hash/identity heuristics with heavier machinery.

What it missed or underweighted: the copy helpers can panic despite returning `Result`, the host scratch helper shares the same overwrite-before-read contract as the device helper, and the pointer-identity cache key depends on a stronger immutability/invalidation contract than the earlier packets said explicitly.

What it overstated: `OnceLock` / `thread_local!` / planner / telemetry framing that belongs more to larger runtime modules than to this helper.

So the conservative fourth-pass call is:

- **Safe now:** add prefix-length guards, add local contract comments, and document the immutability assumption behind `CloseSeriesKey`.
- **Still deferred:** content-based cache keys, async-allocation rewrites, or broader runtime abstraction changes.

That keeps the bucket unchanged while making the packet materially sharper and less regression-prone than the current third-pass version.
