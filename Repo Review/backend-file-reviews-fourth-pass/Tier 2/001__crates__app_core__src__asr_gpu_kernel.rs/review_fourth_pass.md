# Fourth-Pass Review: `crates/app_core/src/asr_gpu_kernel.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/asr_gpu_kernel.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The current third-pass bucket is still directionally right, but the current third-pass packet is too abstract to be a good maintainer guide for the live file.

The live source is not mainly asking for generic “ownership/lifetime assertions.” It is still a strategy-local CUDA wrapper with two concrete contract surfaces that deserve sharper treatment than the third pass gave them:

1. the exact raw-pointer rows that are actually uploaded and packed on the device-row route; and
2. the pointer-identity cache keys that decide whether resident close/log-return buffers and packed device-row matrices are reused.

That matters because the live source still contains the same bounded hardening opportunity the earlier packets identified, but the third-pass packet mostly collapsed it into generic caution. In other words: the bucket should stand, but the packet needed to recover specificity.

## Fourth-Pass Research Log

This fourth pass re-read the live source first, then checked the earlier packet history only to see where specificity had been lost.

Code surfaces rechecked in the live source:

- public raw-pointer DTOs: `crates/app_core/src/asr_gpu_kernel.rs:27-62`
- device-row validation: `crates/app_core/src/asr_gpu_kernel.rs:291-366`
- resident close/log-return cache identity: `crates/app_core/src/asr_gpu_kernel.rs:497-535`
- row compaction planning: `crates/app_core/src/asr_gpu_kernel.rs:369-450`
- row-pointer upload and packed-row reuse: `crates/app_core/src/asr_gpu_kernel.rs:978-1157`
- duplicated row-major vs device-row execution tails: `crates/app_core/src/asr_gpu_kernel.rs:602-844` and `846-1367`
- feature-gated local tests: `crates/app_core/src/asr_gpu_kernel.rs:1369-1484`

Repo-contract checks used for regression framing:

- the module is crate-public behind the CUDA feature gate, not just a private helper: `crates/app_core/src/lib.rs:3-4`
- the repo already uses a more typed “device slice plus owner” pattern elsewhere in the GPU runtime, which is relevant when judging whether raw `u64` pointers should remain the long-term shape: `crates/app_core/src/gpu_backtest_kernel.rs:2312-2338`

External primary references consulted for the fourth pass:

1. **NVIDIA CUDA Driver API, Unified Addressing / pointer attributes**
   - used to verify that stronger pointer provenance checks are technically possible via pointer attributes, including context/device queries
   - also important because the driver docs explicitly note that these attribute queries may surface prior asynchronous launch errors, which makes “just validate every pointer in the hot path” less trivial than it sounds

2. **NVIDIA CUDA C++ Best Practices Guide**
   - used to keep the performance posture honest: transfer avoidance and reuse are good, but measured behavior should not be churned casually

3. **`cust` docs for `DeviceBuffer`, `CopyDestination`, and raw device pointers**
   - used to confirm the cost/semantics difference between one-shot `from_slice` uploads and reused device buffers, and to re-ground the lifetime/validity expectations around raw device pointers

4. **Rust `thread_local!` / `LocalKey` docs**
   - used only to keep cache-scope claims accurate; the runtime cache here is thread-local, not a process-global contract

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **stand**.

This file is still best treated as a `Small surgical refactor`, because the highest-value changes are bounded and local:

- tighten row-pointer validation on the device-row path
- document the real cache invalidation contract
- fix verification guidance so the feature-gated tests actually run

What should **not** be done in this packet is just as important:

- no broad tail deduplication
- no compaction-threshold retuning
- no “generic GPU wrapper cleanup”
- no API-breaking raw-pointer redesign in the same change set

### What the current third-pass packet got right

The current third-pass packet got three things right.

First, it correctly resisted a broad algorithm or architecture refactor. This file is still wrapper/runtime code, not a place for strategy-math churn.

Second, it correctly treated upload reuse, launch shape, and cache behavior as benchmark-sensitive. The live source still makes a deliberate distinction between the row-major path, which performs one-shot uploads with `DeviceBuffer::from_slice` (`crates/app_core/src/asr_gpu_kernel.rs:681-693`), and the device-row path, which reuses uploaded row-pointer slices and packed matrices (`crates/app_core/src/asr_gpu_kernel.rs:978-1157`). That asymmetry is a feature, not accidental duplication.

Third, it correctly warned against overdescribing the thread-local runtime cache as a global guarantee. The cache is thread-local (`crates/app_core/src/asr_gpu_kernel.rs:181-183`), so any recommendation that talks about “the runtime cache” needs to stay thread-scoped.

### What the current third-pass packet missed or underweighted

#### 1. The most concrete safe-now bug surface is still row-pointer validation, not vague ownership assertions

`validate_inputs_device_rows` rejects null Kaufman pointers, but it does **not** reject null fast/slow row pointers (`crates/app_core/src/asr_gpu_kernel.rs:291-318`). Those fast/slow raw pointers are later uploaded and can drive `pack_rows_u64_ptr_f32` (`crates/app_core/src/asr_gpu_kernel.rs:978-1157`).

That means the highest-value bounded hardening is still about raw row-pointer validation.

But the fourth pass sharpens this further: the safest fix is **not** “reject every zero entry in every row-pointer array.” That would be broader than necessary and could reject dormant placeholder entries that the compaction path never uploads.

The narrower safer change is:

- after `prepare_compacted_row_plan`, validate the exact pointer slice that will actually be uploaded and packed
- if compaction is active, validate only the selected rows
- if compaction is skipped, validate the full row-pointer arrays, because the full arrays really are packed in that branch

That is a better recommendation than the current third-pass packet’s generic “tighten ownership/lifetime assertions.” It is more concrete and less regression-prone.

#### 2. The real contract is cache identity and immutability, not just lifetime

The current third-pass packet says “ownership and lifetime discipline,” which is directionally true but too soft for the live code.

The live source has two identity-based cache contracts:

- `ensure_resident_close_lr` keys resident close/log-return reuse by `len`, `host_ptr`, and `device_ptr` (`crates/app_core/src/asr_gpu_kernel.rs:502-510`)
- the device-row packed matrices key reuse by row-pointer signatures plus `cols` (`crates/app_core/src/asr_gpu_kernel.rs:1059-1157`)

That is not merely a lifetime concern. It is an **immutability** concern.

If callers reuse the same host/device identities while mutating underlying contents in place, this file can legally reuse stale derived data. The third-pass packet underweighted that by talking about ownership in the abstract instead of naming the actual invalidation rule.

A related nuance the third-pass packet also missed: when `close_device_ptr != 0`, the computation uses the device pointer, while `close_f64` mainly contributes length and cache identity (`crates/app_core/src/asr_gpu_kernel.rs:497-523`). That is a real API contract and should be documented explicitly if it is going to remain as-is.

#### 3. This is a crate-public raw-pointer API, not just a private helper

The module is publicly exposed behind feature flags (`crates/app_core/src/lib.rs:3-4`), and the raw-pointer DTOs/functions are public (`crates/app_core/src/asr_gpu_kernel.rs:27-62`, `602-1367`).

So the right near-term hardening is not only tests. It is also better local contract documentation on the DTOs and entry points.

The current third-pass packet underweighted this by framing the file like a private wrapper. It is better described as a public feature-gated GPU ABI surface with internal callers today.

#### 4. The current third-pass verification command is too weak and can mislead maintainers

The current third-pass packet recommends:

- `cargo test -p ta_app_core asr_gpu_kernel -- --nocapture`

That is not good enough for this file, because the entire module is feature-gated at the file level: `#![cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]` (`crates/app_core/src/asr_gpu_kernel.rs:1`).

So the third-pass verification advice can report a superficially successful run while not compiling or executing the file-local tests at all. That is an evidence-quality problem, not just a documentation nit.

#### 5. Same-device/context provenance is a real GPU ABI risk, but it is a deferred redesign, not a casual “assertion”

The current third-pass packet was right to worry about ownership, but it still underspecified the actual ABI hazard: `device_id` is chosen separately from the raw pointer inputs, and the file does not verify that those pointers belong to the selected device/context.

That gap affects:

- `close_device_ptr`
- every row pointer in `AsrGpuDeviceSignalMatrices`
- the Kaufman row pointers

The fourth-pass judgment is that this is real, but it is **not** a small “add assertions” change.

The NVIDIA Driver API does make stronger validation technically possible through pointer attributes, including context/device lookups. But introducing per-pointer driver queries into this hot path would change both performance and error behavior. That is exactly the sort of work that should be deferred unless the packet is explicitly expanded.

#### 6. The two execution entry points still duplicate enough logic that parity tests are more valuable than refactoring them now

The earlier packets were better than the current third-pass packet at keeping this in view.

`eval_asr_metrics_gpu_kernel` and `eval_asr_metrics_gpu_kernel_device_rows` still duplicate the backtest launch and host decode tail (`crates/app_core/src/asr_gpu_kernel.rs:602-844` and `846-1367`).

That does not justify a refactor in this packet. It **does** justify preferring a parity test over generic cleanup if the write set is reopened.

### Claims in the current third-pass packet that are too weak or too strong

The current third-pass packet’s biggest weakness is that its main recommendation is too vague:

> “Add or tighten ownership/lifetime assertions before refactoring.”

That is too weak to guide a safe implementation. The live file needs narrower actions than that.

A second overstatement is the implied idea that the file’s live risk is primarily “lifetime discipline.” The stronger reading from the source is:

- pointer validity for the exact packed row set
- cache invalidation by identity rather than contents
- same-device/context provenance on a public raw-pointer API

Those are related to lifetime, but they are not the same thing.

## Implementation Priority

### Safe-now hardening

#### 1. Validate the exact fast/slow row-pointer set that will be packed

This is the first change worth making.

Implementation shape:

- keep `validate_inputs_device_rows` for structural checks: lengths, combo bounds, window ranges, Kaufman non-null
- after compaction planning in `eval_asr_metrics_gpu_kernel_device_rows`, validate the exact row-pointer slice about to be uploaded
- when compaction is active, validate only the selected rows
- when compaction is skipped, validate the full row-pointer arrays because the full arrays are packed in that branch

Why this is the right bounded fix:

- it closes the raw-pointer gap that can actually reach the pack kernel
- it avoids overrejecting dormant unused entries in rows that are compacted away
- it preserves the current compaction/reuse behavior

Tests worth adding alongside it:

- selected compacted row contains a null pointer -> reject
- dense/no-compaction path contains a null pointer in any packed row -> reject
- compacted-away unused row contains a null pointer -> still accept

#### 2. Add contract notes to the public DTOs and entry points

This is the second safe-now change.

The public docs should explicitly state:

- every pointer used by a call must belong to the `device_id` context/device selected for the runtime
- all row pointers that are actually packed, plus both Kaufman row pointers, must be non-null and remain valid for the full evaluation
- if `close_device_ptr != 0`, it must describe the same logical close series as `close_f64`
- resident close/log-return reuse and packed-row reuse are identity-based; callers must not expect in-place mutations with stable identities to invalidate cached data automatically

This is small, honest hardening for a crate-public raw-pointer API.

#### 3. Fix the verification contract

The local verification guidance should always use the feature flags required to compile this module’s tests.

At minimum, the packet/guidance should stop recommending a featureless `cargo test` invocation for this file.

#### 4. Add a row-major vs device-row parity test if a GPU-backed test surface is already available

This is lower priority than pointer validation and documentation, but still a good safe-now hardening if the test harness already exists.

Reason:

- the two public entry points intentionally differ in staging, but they should still agree on metrics for identical logical inputs
- a parity test is a better near-term defense than refactoring duplicated launch/decode code

### Larger redesign still deferred

#### 1. Replace raw `u64` pointer DTOs with typed device-handle inputs

The repo already has a stronger typed pattern elsewhere, where a device slice reference is paired with an owner object (`crates/app_core/src/gpu_backtest_kernel.rs:2312-2338`).

A future ASR redesign could move in that direction so that:

- device identity is carried explicitly
- lifetimes/keepalive are less implicit
- the API surface becomes more honest about pointer provenance

That is a good idea, but it is not a small change.

#### 2. Consider debug-mode or opt-in pointer provenance checks

If this surface needs stronger runtime validation later, pointer-attribute queries are the likely direction.

But this is deferred because it changes:

- hot-path cost
- error surfacing behavior
- CUDA-driver coupling in a strategy-local wrapper

That needs its own measured pass.

#### 3. Deduplicate the shared launch/decode tail only after parity coverage exists

There is enough duplication to justify a future refactor, but not enough to justify doing it blind.

The correct order is:

1. add parity protection
2. then consider extracting the common launch/decode tail

#### 4. Do not pull the row-major path into the same caching model without a fresh invalidation design

The row-major path currently performs one-shot `from_slice` uploads for its signal matrices (`crates/app_core/src/asr_gpu_kernel.rs:681-693`), while the device-row path explicitly reuses uploaded row-pointer slices and packed matrices (`crates/app_core/src/asr_gpu_kernel.rs:978-1157`).

Any attempt to “unify” these behaviors is redesign work. It would reopen invalidation, ownership, and performance questions that do not need to move in a small surgical packet.

## Verification If Changed

Minimum verification:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel asr_gpu_kernel -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_cpu -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`

If the pointer-validation change lands, add targeted coverage for:

- null packed-row pointer rejection in the compacted branch
- null packed-row pointer rejection in the dense/no-compaction branch
- acceptance of unused null placeholders when compaction excludes those rows

If cache semantics, compaction policy, or upload reuse changes at all, run the GPU perf gate as well:

- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`

## Regression And Feature-Preservation Constraints

Preserve these properties unless the packet is explicitly expanded:

- both public ASR GPU entry points must remain supported
- the device-row path must keep its upload-skipping and packed-row reuse behavior
- the row-major path must not be silently pulled into a new caching model
- the current compaction heuristic should remain benchmark-owned, not style-owned
- the thread-local per-device runtime cache should remain thread-scoped
- the module’s feature-gated build boundary must remain intact so CPU-only builds still work without CUDA

Specific regression traps to avoid:

- do **not** reject every row-pointer slot unconditionally if the goal is only to protect the actually packed rows
- do **not** add hot-path driver pointer-attribute queries as part of a supposedly small refactor
- do **not** convert this surface into an `unsafe`-API redesign inside the same packet; if that happens at all, it should be deliberate and separately reviewed
- do **not** change the compaction threshold or reuse keys without both parity and performance review

## Fourth-Pass Conclusion

The current third-pass **bucket should stand**: `Small surgical refactor` is still the right action bucket.

But the current third-pass packet itself is too generic. It got the high-level caution right, yet it missed the most useful bounded implementation advice.

The fourth-pass call is:

- keep the bucket
- recover the concrete pointer-validation and cache-contract findings
- make the raw-pointer API contract explicit
- fix the verification guidance so maintainers actually run the feature-gated tests

Safe-now hardening is still available here, but it is narrower than “add assertions” and smaller than a redesign. The best next move is exact packed-row pointer validation plus better public contract notes, while leaving pointer-provenance enforcement, typed handle redesign, and broader path deduplication deferred.
