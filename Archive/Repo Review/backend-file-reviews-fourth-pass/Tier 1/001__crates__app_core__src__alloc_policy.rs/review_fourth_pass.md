# Fourth-Pass Review: `crates/app_core/src/alloc_policy.rs`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/alloc_policy.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/alloc_policy.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/alloc_policy.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/alloc_policy.rs.review.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Bounded hardening now; larger redesign still deferred`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet kept the defer bucket but drifted away from the live file. The current source is not mainly a “support/instrumentation seam”; it is still a low-level allocation-policy module that converts `Vec<MaybeUninit<T>>` into `Vec<T>` at `15-20`, returns concrete `Vec<T>` values from release-mode uninitialized helpers across `33-184`, and separately keeps the matrix path in `Vec<MaybeUninit<_>>` across `188-310`. That source shape re-raises the earlier soundness question, not a generic helper-scope question.

This fourth pass therefore re-checked the live source against current Rust docs to answer a narrower question the current packet left unresolved: is full soundness still only a larger deferred redesign, or is there a bounded hardening step the packet missed?

## Fourth-Pass Research Log

1. `std::vec::Vec::{set_len, from_raw_parts, spare_capacity_mut}`
   - Rechecked the exact safety preconditions.
   - Important outcome: `set_len` requires the newly included range to already be initialized, and `spare_capacity_mut` is the standard stable API for fill-then-`set_len`.

2. `std::mem::MaybeUninit`
   - Rechecked layout guarantees and the validity boundary.
   - Important outcome: `MaybeUninit<T>` shares size/alignment/ABI with `T`, but exposing uninitialized memory through a real `T`-typed container is still unsound.

3. Rustonomicon: Working With Uninitialized Memory / Exception Safety
   - Rechecked the rule that uninitialized memory must not be interpreted as `T`, and that unsafe code must not leave a container in a state where a panic or ordinary observation can witness uninitialized `T` values.

4. Clippy `uninit_vec`
   - Rechecked the current lint guidance.
   - Important outcome: the exact “reserve, `set_len`, then let something else write later” pattern is treated as a correctness issue, with `MaybeUninit` / `spare_capacity_mut` as the intended alternative.

## What Fourth Pass Sharpened

### 1. The current third-pass bucket should change

The third-pass packet was directionally right about one thing only: a full performance-preserving redesign is still larger work. But its actual framing is not faithful to the live source, and its defer posture is too strong.

A bounded hardening step exists now for most of this file. The concrete `f32`, `f64`, and `i32` helpers do not have to keep returning logically uninitialized `Vec<T>` values. They can be rewritten on stable Rust to initialize the exposed elements before `set_len`, using the existing poison/sentinel policy and `Vec::spare_capacity_mut` or equivalent fill discipline. That keeps the external helper signatures stable and avoids whole-repo `Vec<MaybeUninit<T>>` propagation.

So the right fourth-pass call is not “do the full redesign now.” It is “do bounded hardening now, and keep the larger redesign deferred.”

### 2. The third-pass packet missed the live correctness problem

The live file still does the thing the first and second passes originally called out:

- `assume_init_vec` rebuilds `Vec<T>` from `Vec<MaybeUninit<T>>`.
- `alloc_uninit_copy`, `alloc_uninit_f32`, `alloc_uninit_f64`, the prefix helpers, and the resize helpers all expose `Vec<T>` values or lengths in release mode before the full element range is initialized.
- The matrix helpers are different: they stay typed as `Vec<MaybeUninit<_>>` until the caller performs explicit writes.

That is the file’s main distinction, and the third-pass support-seam framing did not engage with it.

### 3. One earlier recommendation was too weak: “just make the helpers `unsafe fn`”

This needs correction.

Making the `Vec<T>`-returning helpers `unsafe fn` would improve honesty at the call boundary, but it does **not** by itself repair the implementation. The relevant stdlib preconditions apply at the point where `Vec::from_raw_parts` and `Vec::set_len` are called. If those operations are invoked while the newly exposed range is not yet initialized, the unsafe operation is already out of contract. In other words, an `unsafe fn` wrapper can shift responsibility for later use, but it cannot retroactively satisfy a violated `set_len` or `from_raw_parts` precondition.

That means the first-pass “smallest fix” was incomplete, and the current packet never corrected it.

### 4. A bounded hardening path exists now, and the packet missed it

The most important fourth-pass finding is that the concrete scalar helpers have a stable, signature-preserving hardening route:

- For fresh allocation helpers returning `Vec<f32>` / `Vec<f64>`, initialize the buffer with the existing poison/sentinel value before exposing `len`.
- For prefix helpers, initialize the full buffer with the sentinel, then overwrite only the prefix with the requested observable prefix value.
- For `resize_uninit_*`, initialize only the grown tail before `set_len`, and use `truncate` on shrink.

That can be expressed with `Vec::spare_capacity_mut()` and a fill step, or with the current debug-style fill strategy if benchmarking shows it is acceptable. Either way, this is materially smaller than a repo-wide `MaybeUninit` API migration, and it removes the release-mode UB from the concrete scalar helpers.

The one obvious outlier is `alloc_uninit_copy<T: Copy>`. That helper cannot be kept as a broad `T: Copy` escape hatch if the implementation is changed to defined-value sentinel initialization. It should be narrowed to explicitly supported scalar types, or routed through a sealed internal trait that provides a valid sentinel value per type.

### 5. What the current packet got right

The prior packet was still right on a few important constraints:

- Do not do a casual repo-wide cleanup that blindly replaces these helpers with `vec![0; n]` or similar without perf evidence.
- The matrix helpers are already closer to the sound end-state and are the right local reference model.
- Any full redesign that pushes `MaybeUninit` into `double_ma`, `ml_signal`, `gpu_backtest_kernel`, or the standalone GPU runtime is still multi-file and benchmark-sensitive.

Those points should survive.

### 6. What was underweighted or overstated

Underweighted:
- The third pass did not say plainly enough that the live source is still centered on release-mode container validity, not on helper “truthfulness” in the abstract.
- The existence of a bounded hardening route for most concrete helpers was missed entirely.

Overstated:
- The idea that this area is mainly a support/instrumentation seam is not supported by the live file.
- The earlier implication that `unsafe fn` alone would be a sufficient first fix is too strong.

Secondary note:
- The current debug poison story is useful but weaker than the earlier packet implied. The float poison bit patterns in this file are ordinary finite values, not NaNs, so premature reads can still flow through arithmetic more quietly than the word “poison” suggests. That is not the primary bug, but it is worth understanding honestly if the repo relies on debug poison for diagnostics.

## Implementation Priority

### Safe-now hardening

1. Harden the concrete scalar helpers now.
   - Rework `alloc_uninit_f32`, `alloc_uninit_f64`, `alloc_f32_with_prefix_uninit_tail`, `alloc_f64_with_prefix_uninit_tail`, `resize_uninit_f32`, `resize_uninit_f64`, and `resize_uninit_i32` so the newly exposed `Vec<T>` range is initialized before `set_len`.
   - Prefer `spare_capacity_mut` / fill discipline because it is the exact stable stdlib API intended for this fill-then-`set_len` pattern.

2. Remove or shrink the generic escape hatch.
   - `alloc_uninit_copy<T: Copy>` should not stay as unconstrained `T: Copy`.
   - Replace it with typed helpers or a sealed internal trait implemented only for the repo-approved scalar types that have an explicit sentinel value.

3. Treat `assume_init_vec` as migration debt, not a long-term primitive.
   - Once the concrete `Vec<T>` helpers stop using `Vec<MaybeUninit<T>> -> Vec<T>` transmutation, this helper should either disappear or become confined to places that can actually prove full initialization.

4. Add release-profile behavior checks immediately.
   - The current tests validate debug behavior and overwrite discipline, but they do not exercise the live release branches that matter most.

### Larger redesign still deferred

1. The performance-preserving end-state is still a broader migration toward `MaybeUninit`-typed scratch buffers or `spare_capacity_mut`-based caller filling through the hot consumers.
   - That is the route that recovers the original “skip initialization” performance purpose without relying on invalid `Vec<T>` states.

2. Unify the module around one safety model.
   - Today the matrix helpers already use the sounder boundary while the scalar helpers do not.
   - A later redesign should make that asymmetry disappear.

3. Do not mix half-migrated APIs casually.
   - This area is shared by hot CPU and GPU-adjacent code, so a staged migration must preserve caller clarity at each step.

## Verification If Changed

- `cargo check -p ta_app_core`
- `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- `cargo test -p ta_app_core alloc_policy -- --nocapture`
- `cargo test -p ta_app_core --release alloc_policy -- --nocapture`

If helper semantics or trait bounds change, also run:
- `cargo test -p ta_app_core double_ma -- --nocapture`
- `cargo test -p ta_app_core ml_signal -- --nocapture`

If CUDA-facing helpers change:
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_backtest_kernel -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel standalone_gpu_runtime -- --nocapture`

If release-mode helpers move from “skip init” to sentinel initialization:
- run the CPU perf gate before landing
- run the GPU perf gate when CUDA staging paths are affected
- serialize perf runs; do not overlap benchmark/profile sessions

## Regression And Feature-Preservation Constraints

- Preserve current CPU-only and CUDA build availability.
- Preserve the observable warm-prefix contract: requested prefix values must still be immediately readable, and the remaining region must still be safe for later overwrite-heavy use.
- Do not regress the already-sound matrix helpers back toward `Vec<T>` exposure.
- Expect the main risk to be performance, not API/IPC/persistence breakage:
  - this is `pub(crate)` infrastructure
  - there is no public IPC or persistence schema tied directly to this module
  - the real sensitivity is host-side bandwidth and scratch-buffer throughput in hot CPU/GPU paths
- If the repo wants to keep the original “no full initialization on fresh allocation” performance goal, that remains a later redesign, not a reason to leave the current release path unsound.

## Fourth-Pass Conclusion

The current third-pass bucket should **change**.

The larger redesign is still deferred. A full, performance-preserving migration to `MaybeUninit`-typed caller flows remains multi-file work across hot `ta_app_core` consumers.

But the current packet over-deferred, because it missed a real bounded hardening step that exists now. Most of the concrete scalar helper surface can be made sound on stable Rust without changing its signatures, by initializing the exposed range before `set_len` and narrowing the generic `alloc_uninit_copy` escape hatch. That is a materially smaller change than a repo-wide `MaybeUninit` propagation, and it is the right next step for a public-facing codebase.

So the correct fourth-pass posture is:

- **land bounded hardening now**
- **keep the broader performance-preserving redesign deferred**
- **do not keep repeating that `unsafe fn` alone is enough, because it is not**
