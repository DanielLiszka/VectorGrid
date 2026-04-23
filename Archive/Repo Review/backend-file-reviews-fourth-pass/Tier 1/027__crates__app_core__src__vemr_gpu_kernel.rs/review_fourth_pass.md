# Fourth-Pass Review: crates/app_core/src/vemr_gpu_kernel.rs

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/vemr_gpu_kernel.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Fourth-pass action bucket: `Benchmark-sensitive; do not touch yet`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This fourth pass was warranted because the current third-pass packet is directionally correct on bucketing, but too compressed in how it explains the risk surface. The live file is not just a small ownership-and-lifetime wrapper around a CUDA kernel. It is also a strategy-local ABI adapter with several silent contracts baked into the Rust side: raw device-pointer DTO shape, metric-row stride, objective-id encoding, strategy-flag packing, close/LR cache identity, and result-variant selection.

That distinction matters in a Tier 1, benchmark-sensitive review. A packet that says only “preserve ownership and lifetime” is conservative, but still incomplete. The sharper question is not merely whether this file owns data safely. It is whether this file stays in lockstep with the VEMR kernel ABI and with the upstream caller contract that feeds it externally-owned device memory.

My bucket call does **not** change. The third-pass bucket should stand. What changes in this fourth pass is the reason: the file is benchmark-sensitive not only because of cache reuse and thread-local runtime state, but because seemingly tiny cleanup here can desynchronize a strategy-local Rust/CUDA contract that is only partially encoded in types.

## Fourth-Pass Research Log

I re-read the live source first, then compared it against the existing review history and the local shared-kernel context.

In the live file, the key contract surfaces are all explicit in the Rust layer:

- raw signal DTOs carry naked `u64` pointers and shape metadata, but no owner/keepalive or device provenance (`VemrGpuDeviceSignalMatrices`, lines 30-45)
- scratch reuse depends on uninitialized device/host buffers being fully overwritten before later readback (`VemrCudaScratch`, lines 65-136)
- runtime state is cached in a `thread_local!` map keyed by `device_id` (`VEMR_GPU_RUNTIME`, lines 220-222)
- the Rust side hard-codes a VEMR-local objective-id table and metric-row decoder (`objective_to_vemr_kernel_id`, `objective_score`, `decode_vemr_metrics_row`, lines 373-399)
- the public entry point chooses between `Full` and `TopK` using leaf-level gating over `include_all`, `heatmap_bins`, `top_k.max(1)`, and a local `256` cap (`eval_vemr_metrics_gpu_kernel_device_rows`, lines 664-700)
- the close/LR cache key is pointer-identity based, not content-identity based (`VemrCloseSignature`, lines 91-96; cache use, lines 721-748)
- file-local tests still cover only row/range/stride validation plus one host-buffer-growth case (lines 1015-1095)

I also checked the shared GPU-kernel context file for drift-prone similarities and differences. That comparison matters because this VEMR wrapper reuses shared module loading and launch-shape helpers, but it does **not** share every ABI convention with the generic double-MA kernel path.

Notably:

- the shared kernel path names bit `1u32 << 4` as `STRAT_SIGNED_EXPOSURE` (`context__02__crates__app_core__src__gpu_backtest_kernel.rs`, lines 55-63), while VEMR packs that same bit as a bare literal at lines 823-833 in the live file
- the shared kernel path also uses a `256` cap when `include_all=false` (`context__02__crates__app_core__src__gpu_backtest_kernel.rs`, lines 12628-12631), which supports the idea that VEMR’s local cap is deliberate rather than accidental
- but the shared kernel path’s generic objective-id mapping is **not** the same as VEMR’s mapping (`context__02__crates__app_core__src__gpu_backtest_kernel.rs`, lines 4657-4666, versus live VEMR lines 373-381)

That last point is the most important sharpening from this pass. The file is not merely “using shared infrastructure.” It is using shared infrastructure while preserving a strategy-local objective encoding. That is exactly the sort of contract that can be broken by a well-meaning cleanup that tries to “align” small helpers across modules.

External research reinforced the conservative reading rather than weakening it:

- the current `cust::memory::DeviceBuffer` docs still state that `DeviceBuffer::uninitialized` requires the caller to initialize contents before reading them, which matches the live scratch-buffer discipline here
- the current Rust `thread_local!` docs still describe thread-local storage keys rather than a process-global cache model, which supports the existing warning against overdescribing cache scope
- the current CUDA Driver API docs still document pointer-attribute queries (`cuPointerGetAttribute`) and context-specific address spaces, which means deeper same-device/same-context checks are technically possible, but they would be a **design change**, not a free safety assertion

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet got the most important classification call right: this file should remain in `Benchmark-sensitive; do not touch yet`.

It also got three narrower points right:

1. `thread_local!` runtime state should not be described as process-global.
2. strategy-local GPU organization is still the right default posture.
3. small wrapper-layer changes can regress performance or parity more easily than the file size suggests.

Those calls should survive into any public-facing review.

### What the current third-pass packet missed or underweighted

The current packet underweights four concrete issues that are visible in the live source.

#### 1. The Rust side is a strategy-local ABI adapter, not just an ownership wrapper

The live file hard-codes several kernel-facing conventions in Rust:

- `VEMR_METRICS_STRIDE` is tied directly to `NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE` (line 26)
- `decode_vemr_metrics_row` reuses the shared non-double-MA metric decoder (lines 397-399)
- `objective_to_vemr_kernel_id` uses a VEMR-local objective ordering (lines 373-381)
- `objective_score` independently defines host-side ranking semantics (lines 385-394)
- strategy flags are packed manually, including a bare `1u32 << 4` literal (lines 823-833)

That is more than ordinary “ownership and lifetime discipline.” It is ABI coupling.

This matters because the generic shared kernel path uses a **different** objective-id ordering (`MaxDrawdown` is `2` there, but `6` here; `Sortino`/`Omega` also move) in `objective_kind_kernel_index` (`context__02__crates__app_core__src__gpu_backtest_kernel.rs`, lines 4657-4666). Any future cleanup that assumes these encodings are globally interchangeable would be regression-prone.

#### 2. The raw-pointer DTO contract is sharper than the packet described

`VemrGpuDeviceSignalMatrices` carries `u64` raw pointers for close, z-score, Bollinger bands, and ATR rows, but it does not carry:

- the owning allocation
- a keepalive handle
- a device id per pointer
- any provenance that lets the callee verify same-device or same-context validity

The live validator checks non-null pointers, row counts, strides, and index/range bounds (lines 224-325). It does **not** validate pointer provenance. The runtime then binds a CUDA context by the separate `device_id` argument and launches against those raw pointers (lines 701-710, 837-943).

So the real caller contract is not merely “keep the buffers alive.” It is:

- keep the pointed-to allocations alive
- pass pointers that belong to the active device/context for `device_id`
- keep host-side metadata (`close_f32`, `cols`, row counts, strides) aligned with those external allocations

That contract should be called out explicitly. The current packet’s ownership/lifetime framing is true, but not specific enough.

#### 3. The close/LR cache key is a pointer-identity heuristic with both stale-hit and over-invalidation risk

`VemrCloseSignature` contains only `len`, `host_ptr`, and `device_ptr` (lines 91-96). The entry point uses that signature to decide whether to skip re-uploading close prices and skip recomputing `d_lr` (lines 721-748).

That means the cache assumes pointer identity implies content identity.

Concretely, that underweights two opposite failure modes:

- **stale-hit risk**: if the host slice or external device memory is mutated in place without changing pointer identity, the code can reuse stale `d_close` / `d_lr`
- **over-invalidation risk**: in external-device-pointer mode, a different host mirror slice pointer can force an unnecessary recompute even when the device allocation and its contents are unchanged

This is more precise than the second-pass packet’s generic “input immutability” remark, and it is worth stating explicitly because it shapes what “safe-now” work is realistic. Replacing this with a stronger key is not a free hardening win; it changes cache cost and invalidation behavior in a hot path.

#### 4. The `Full` vs `TopK` contract is broader than the current packet implied

The current packet was right that `Full` vs `TopK` is semantically important, but it still underdescribed the actual gate.

The live entry point does **not** select `TopK` based only on `include_all=false`. The path is gated by all of the following (lines 691-700):

- `top_k` is normalized with `top_k.max(1)`
- `include_all` must be false
- `heatmap_bins` must be zero
- `topk_select` must be strictly smaller than `combo_count`
- `topk_select` must be `<= 256`

That means:

- `top_k == 0` does not mean “no shortlist”; it means “one-row shortlist request”
- `include_all=false` with `top_k >= combo_count` still returns `Full`, not `TopK`
- `heatmap_bins` changes not just performance posture, but the **result variant** and D2H shape, even though this leaf API does not expose a heatmap result itself

This is a stronger API-coupling point than the current third-pass packet captured.

### What the current third-pass packet overstated or framed too loosely

The biggest overstatement is its sentence that “the real contract here is ownership and lifetime discipline, not broader algorithm refactor.”

That framing is too narrow. The live source shows at least three parallel contracts:

1. ownership / lifetime of externally-owned device memory
2. cache-identity and result-shape semantics in the Rust wrapper
3. strategy-local Rust/CUDA ABI alignment for flags, objective ids, and metric decode

The packet also says to “add or tighten ownership/lifetime assertions before refactoring.” That is a weak recommendation in its current form. This file does not have enough typed metadata to validate the most important pointer-provenance properties cheaply or completely. Stronger runtime assertions would either be:

- redundant with the current null/shape checks, or
- a real behavior/perf/design change that starts querying CUDA pointer attributes and reasoning about contexts

That work is not “obviously safe-now.” It belongs in a more deliberate redesign lane.

## Implementation Priority

### Safe-now hardening

Safe-now work should stay non-behavioral and should make the live contracts harder to accidentally misread.

First, document the raw-pointer DTO contract at the public boundary. `VemrGpuDeviceSignalMatrices` and `eval_vemr_metrics_gpu_kernel_device_rows` should say plainly that all supplied row-major pointers must remain alive and valid for the active CUDA device/context identified by `device_id`, and that `close_f32` is still part of the cache identity even when `close_device_ptr` is non-zero.

Second, document the close-cache heuristic where it actually matters. The code at lines 721-748 is not a generic resident-series cache; it is a pointer-identity reuse shortcut. A comment there should state that in-place mutation of the underlying close series at the same host/device address is outside the supported contract.

Third, make the leaf result-selection contract explicit. The gate at lines 695-700 should be commented as a **public semantic rule**, not just a performance rule, because it changes whether callers receive `VemrGpuEvalResult::Full` or `VemrGpuEvalResult::TopK`.

Fourth, remove the bare flag mystery. Replacing `flags |= 1u32 << 4` with a named local constant or a short ABI comment is justified. That is a true safe-now hardening change because it does not alter code generation, but it materially improves auditability.

Fifth, add one tiny non-CUDA unit test that pins the VEMR-local objective-id table. That is the safest test addition this pass can justify. It would specifically protect against future “cleanup” that tries to reuse the generic `gpu_backtest_kernel` objective-id ordering even though the live VEMR ordering is different.

### Larger redesign still deferred

The bigger redesigns remain deferred.

The right long-term direction is to stop passing naked `u64` pointers and instead pass a typed device-signal bundle that can carry:

- owner/keepalive handles
- device provenance
- an explicit cache epoch or content version for the close series

That would make pointer validity and cache invalidation a type-level contract instead of a comment-level contract.

A second deferred redesign is to centralize the VEMR kernel ABI surface. Today, objective-id mapping, metric-row decode, and strategy-flag packing are spread across local Rust helpers and implicit kernel expectations. A shared strategy-local ABI module or generated constant surface would reduce drift risk, but that is not a “safe-now” edit in a benchmark-sensitive file.

A third deferred redesign is to separate “requested result mode” from “current device-top-k eligibility.” Right now `include_all`, `heatmap_bins`, `top_k`, and the local `256` cap all collapse into a single branch that decides both runtime behavior and result variant. A richer mode enum would be cleaner, but changing it now would ripple into upstream callers and regression coverage.

### Recommendations that would risk regressions if implemented casually

Several tempting changes are still too risky for a casual hardening pass.

Do **not** change the close-cache key without measurement and parity review. A content hash, device-pointer-only key, or host-pointer-only key would each change invalidation cost or correctness assumptions.

Do **not** “align” `objective_to_vemr_kernel_id` with the generic objective table just because both files use `ObjectiveKind`. The live encodings differ, and that difference is almost certainly kernel-ABI-significant.

Do **not** remove the `combo_dummy_ptr`/`d_top_dummy` plumbing just because the host side ignores the dummy output. The generic gather kernel ABI clearly expects that companion column, and this wrapper is satisfying that contract.

Do **not** change `top_k == 0` normalization or the current `use_device_topk` gate without checking upstream expectations. The leaf currently treats zero as one, and it falls back to `Full` in several seemingly “shortlist” cases. That behavior may already be baked into callers.

Do **not** genericize this file into a shared non-double-MA kernel wrapper until the VEMR-local objective-id and flag conventions are first made explicit and reverified.

## Verification If Changed

If only comments, rustdoc, or a named local constant replacement are added, the minimum bar is the file-local test surface plus a compile check under the CUDA feature set.

Recommended verification for any change in or around this file:

1. File-local guard tests:
   - `cargo test -p ta_app_core vemr_gpu_kernel::tests:: --features cuda,cuda-backtest-kernel -- --nocapture`

2. Strategy-family coverage that exercises the VEMR GPU path through its real owning surface:
   - `cargo test -p ta_app_core composite_cpu --features cuda,cuda-backtest-kernel -- --nocapture`
   - `cargo test -p ta_app_core composite_validate --features cuda,cuda-backtest-kernel -- --nocapture`

3. Representative benchmark coverage before any behavior change touching cache reuse, result gating, or top-k staging:
   - `cargo bench -p ta_app_core --bench vemr_opt --features cuda,cuda-backtest-kernel`

4. If cache policy, objective-id mapping, or result-mode gating changes at all, rerun the repo’s GPU perf gate:
   - `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

The current packet history already treated this as a measurement-sensitive area. Fourth pass does not weaken that requirement.

## Regression And Feature-Preservation Constraints

Preserve the current third-pass bucket.

Preserve the current `Full` versus `TopK` **behavior**, not just the enum definition. In this file that means preserving the full gate: `top_k.max(1)`, `include_all`, `heatmap_bins`, `combo_count`, and the local `256` limit (lines 695-700).

Preserve the current strategy-local objective-id mapping unless the kernel ABI is deliberately changed and reverified end to end. The generic shared mapping in `gpu_backtest_kernel.rs` is not proof that VEMR should use the same table.

Preserve the current metric-row decode alignment between `VEMR_METRICS_STRIDE` and `decode_non_double_ma_gpu_metrics_row`. Any drift there is a silent correctness risk, not just a style issue.

Preserve the current flag layout unless it is replaced with a clearly coordinated constant surface. The bare `1u32 << 4` literal is a readability problem, but the underlying bit position is still ABI-sensitive.

Preserve the current assumption that uninitialized buffers are fully overwritten before readback. `AGENTS.md` explicitly keeps uninitialized allocation as acceptable only when every later-read element is written before any read and when the safety assumptions stay adjacent to the code path (`context__AGENTS.md`, lines 141-144). This file is still operating under exactly that rule.

Finally, preserve strategy-locality. `vemr_gpu_kernel.rs` is exported as a strategy-local CUDA companion under the gated module surface (`context__01__crates__app_core__src__lib.rs`), and the current repo guidance still prefers strategy-local organization over broad generic execution layers for strategy-specific GPU code. That posture remains correct here.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: `Benchmark-sensitive; do not touch yet`.

What changed in this fourth pass is not the bucket, but the explanation. The third-pass packet was right that this file is sensitive and should not be casually refactored. It was wrong to compress the risk surface down to ownership and lifetime alone.

The live source shows a more specific contract stack:

- externally-owned raw device pointers must stay alive and valid for the active CUDA context
- close/LR reuse is governed by a pointer-identity cache heuristic, not by content identity
- result shape is controlled by a leaf-level product gate, not just by `include_all`
- the Rust file is carrying strategy-local CUDA ABI details for objective ids, metric decode, and flags

That sharper framing leads to a narrower and safer recommendation set.

Safe-now hardening is still possible, but it should stay limited to contract documentation, naming the magic flag bit, and perhaps one tiny non-CUDA unit test that pins the VEMR-local objective-id mapping. Larger work—typed device ownership, stronger provenance checking, cache-key redesign, or wrapper genericization—should remain deferred until parity and benchmark validation are part of the plan.
