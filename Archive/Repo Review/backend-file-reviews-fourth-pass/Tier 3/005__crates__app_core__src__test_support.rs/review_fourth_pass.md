# Fourth-Pass Review: `crates/app_core/src/test_support.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/test_support.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/test_support.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/test_support.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/test_support.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The current third-pass bucket should **stand**. The live file is still tiny, test-only, and non-public: it is compiled only behind `#[cfg(test)]`, and it still owns just one helper that returns a `MutexGuard<'static, ()>` from a lazily initialized static `Mutex<()>`. This is not a production redesign target.

What justified a fourth pass is that the current third-pass packet stayed too abstract. The live question here is not generic “support helper truthfulness”; it is whether the crate’s env-sensitive tests are coordinating **process-global mutable state** in a way that is both explicit and conservative. On current Rust standard-library docs, `std::env::set_var` and `std::env::remove_var` are now documented as `unsafe`, and on non-Windows multi-threaded programs the docs say the only safe option is to avoid them entirely. That upstream contract makes this helper’s real role narrower and sharper than the third pass described.

## Fourth-Pass Research Log

I re-read the live source and the current third-pass packet first, then used the earlier packets only where the live file still matched. I checked `crates/app_core/src/lib.rs` to confirm that `test_support` is still test-only and not part of the runtime surface. I then checked nearby consumers and sibling test infrastructure to see what this lock is actually coordinating.

That downstream read materially changed the assessment. In `crates/app_core/src/double_ma.rs`, the helper is used around `VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL`, and that path restores the env var through `catch_unwind`, which makes poison handling less obviously necessary and more obviously policy-sensitive. In `crates/app_core/src/gpu_backtest_kernel.rs`, there is already a second `lock_test_env` helper with the same apparent purpose but a different poison policy, and that file contains multiple env-flag tests for `VECTORBT_KERNEL_*` variables.

I also re-checked the current upstream Rust docs that matter here:

- `std::env::set_var` and `std::env::remove_var` now document multi-threaded non-Windows environment mutation as unsafe in practice.
- `std::sync::Mutex` still documents poisoning as advisory rather than a soundness guarantee.
- `std::sync::OnceLock` remains an appropriate lazy static initializer and is itself never poisoned.
- The Rust Book section on test execution still warns that parallel tests must not depend on shared state, including environment variables.

## What Fourth Pass Sharpened

### The third-pass packet got the bucket right, but not the strongest reason why

The file still belongs in `Small surgical refactor`. The third pass was correct not to escalate this into a broader architecture task, and it was correct that the file’s main value is explicit coordination, not abstraction.

What it underweighted is that this helper is best understood as a **process-global env/process-state coordination seam for tests**, not just a generic support helper. That distinction matters because the current Rust docs make environment mutation a stricter contract than the existing packet acknowledged.

### The current packet’s “shared global lock” framing is too strong

The earlier packets implicitly treat `crates/app_core/src/test_support.rs` as the shared process-global env lock for backend tests. The live code makes that too strong.

`crates/app_core/src/gpu_backtest_kernel.rs` already defines its own `lock_test_env` with its own `TEST_ENV_LOCK` static and uses `unwrap_or_else(|e| e.into_inner())` instead of `expect("test env lock poisoned")`. That means the crate currently has at least **two distinct env-lock identities** for test code, not one universal coordinator. This is a real missed finding.

That does not change the action bucket, but it does change what should be claimed. This file is one shared env-state lock, not obviously the only one. Any packet that describes it as the singular cross-suite coordinator should be tightened.

### The earlier poison-recovery recommendation was too aggressive as a “safe-now” change

This is the most important adversarial correction.

The first-pass packet treated poison recovery as the smallest justified fix. That is not well-supported as a blanket recommendation. Recovering from poison is attractive for triage because it can prevent one failing test from causing a wave of `test env lock poisoned` follow-on failures. But for this specific helper, poison can also be a signal that a test panicked while holding the guard **after mutating process-global state**.

If later tests continue after that panic, they may run against leaked environment state. `double_ma.rs` mitigates this for the `VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL` toggle by restoring the env var through `catch_unwind`, but the visible `gpu_backtest_kernel.rs` env-flag tests restore env vars only on the normal path. That means “just recover from poison” is not clearly safe unless restoration-on-panic is already guaranteed for the guarded state.

So the earlier packet was directionally right that poisoning is advisory, but it was too strong in treating poison recovery as a low-risk local hardening step. In this file, that recommendation should be downgraded to **deferred until the guarded state is known to be panic-safe to restore**.

### The most important missing documentation point is not just purpose, but limits

A doc comment should not merely say “shared test lock.” It should say what this lock is and what it is not.

The right safe-now documentation is closer to:

- this is a test-only lock for repo-owned mutations of process-global env/state,
- it reduces intra-suite interference between known writers,
- it does **not** prove that arbitrary environment access is safe in parallel multi-threaded test runs on non-Windows platforms,
- callers that mutate env should restore state even on panic if they want poison recovery to be safe.

That is a materially stronger and more honest contract than the current packet proposed.

### The third-pass verification suggestion is too weak

`cargo test -p ta_app_core test_support -- --nocapture` is not a convincing verification target for this file. `test_support.rs` has no local tests, and filtering by the module/helper name risks exercising little or nothing relevant.

If this file changes, verification should target the actual env-sensitive consumers. The visible evidence points to at least two concrete surfaces:

- `strategy_dynamic` / `double_ma` tests that toggle `VECTORBT_DYNAMIC_STRATEGY_OPT_EXPERIMENTAL`, and
- `gpu_backtest_kernel` tests that toggle `VECTORBT_KERNEL_*` env flags.

That is a sharper and lower-noise validation plan than trying to run “test_support” by name.

## Implementation Priority

### Safe-now hardening

The safest immediate hardening is **documentation and verification tightening**, not behavior changes.

First, add rustdoc or an internal comment that identifies this helper as a test-only coordination lock for repo-owned process-global env/state mutations. The comment should explicitly avoid overselling the helper as a full cross-process or cross-thread safety proof.

Second, keep the current structural shape. `OnceLock<Mutex<()>>` is already an appropriate implementation here; replacing it with a fancier abstraction would add motion without adding confidence.

Third, if any local wording changes are made, make the failure mode more explicit than `"test env lock poisoned"`. A message that points to a prior panic while holding shared test process-state coordination would make later triage faster.

### Larger redesign still deferred

Two larger items are real, but they are not safe-now local edits.

The first is **unifying env-lock identity across the crate**. Because `gpu_backtest_kernel.rs` already ships its own `lock_test_env`, a real cleanup would centralize those helpers so env-sensitive tests actually serialize on one lock. That could be a good change, but it should happen only after auditing lock ordering with `lock_gpu_test` and any other global test locks.

The second is **eliminating raw process-env mutation as the primary test control surface**. The current Rust docs make clear that a mutex around known writers is not a full answer on multi-threaded non-Windows runs. The more robust long-term direction is scoped configuration injection, or at minimum restore-on-drop env guards that make panic cleanup explicit.

Only after that broader audit should blanket poison recovery be reconsidered. Until then, changing `expect(...)` to `into_inner()` in this file is more regression-prone than the earlier packet suggested.

## Verification If Changed

If the change stays local to `crates/app_core/src/test_support.rs`, verification should exercise the real env-sensitive call sites rather than the helper name itself.

Start with the dynamic-strategy env-toggle surface:

- `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture --test-threads=1`

If the crate is being validated in the CUDA-kernel feature lane, also exercise the GPU env-flag surface because that area already carries a sibling env lock and multiple `VECTORBT_KERNEL_*` toggles:

- `cargo test -p ta_app_core --features cuda-backtest-kernel gpu_backtest_kernel::tests:: -- --nocapture --test-threads=1`

If the change expands beyond this file into shared GPU-test helpers or lock unification, follow the repo guidance for higher-risk backend changes and run the broader quality gate with workspace tests.

## Regression And Feature-Preservation Constraints

Keep this helper test-only. `crates/app_core/src/lib.rs` still gates the module behind `#[cfg(test)]`, and that confinement is part of why the bucket stays small.

Do not claim that this lock makes `std::env::set_var` / `remove_var` fully safe on multi-threaded non-Windows runs. The upstream docs do not support that claim, and documenting otherwise would be misleading.

Do not change poison handling casually. If the lock is allowed to recover after panic, the guarded state must already be known to restore correctly on panic, or later tests may observe leaked env/process state and fail in harder-to-triage ways.

If env-lock unification is attempted later, preserve lock acquisition order relative to `lock_gpu_test`. The visible test helpers currently acquire env first and GPU second; changing that ordering without a full audit would risk new deadlocks or subtle serialization changes in CUDA test lanes.

There is no meaningful runtime, IPC, persistence, or GPU ABI risk from this file by itself because it is test-only. The real risk surface is **test reliability**, including CPU-only and `cuda-backtest-kernel` feature-gated validation lanes.

## Fourth-Pass Conclusion

`Bucket unchanged.` `crates/app_core/src/test_support.rs` still belongs in `Small surgical refactor`, and the third-pass packet was right not to escalate it into a redesign target.

What the current packet still misses is that this helper’s most important contract is **process-global env-state coordination under parallel test execution**, and current Rust docs make that contract stricter than the earlier review acknowledged. The live code also shows that this file is not the crate’s only env-lock helper, which weakens any claim that it is already the universal cross-suite coordinator.

The conservative fourth-pass outcome is:

- keep the bucket,
- strengthen the helper’s documentation and verification guidance now,
- do **not** treat poison recovery as an automatic safe-now fix,
- defer lock unification and env-control redesign until the crate’s panic-cleanup and lock-ordering story is audited.
