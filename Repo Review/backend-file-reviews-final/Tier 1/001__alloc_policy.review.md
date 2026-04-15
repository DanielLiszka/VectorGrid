# Final Consolidated Review: `crates/app_core/src/alloc_policy.rs`

- Review date: `2026-04-14`
- Source file: [alloc_policy.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:1>)
- First-pass packet: [alloc_policy.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/alloc_policy.rs.review.md:1>)
- Second-pass packet: [review__second_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/alloc_policy.rs.review.md:1>)
- Third-pass packet: [review__third_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/alloc_policy.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/001__crates__app_core__src__alloc_policy.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid in substance but needs normalization`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/vec/struct.Vec.html>
  - <https://doc.rust-lang.org/std/mem/union.MaybeUninit.html>

## Consolidated Judgment

The first and second passes correctly focused on the real issue in this file: release-mode helper paths expose concrete `Vec<T>` values before the full visible range is initialized through [assume_init_vec](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:15>), [alloc_uninit_copy](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:33>), [alloc_uninit_f64](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:94>), [alloc_uninit_f32](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:112>), and the resize helpers at [134](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:134>), [152](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:152>), and [170](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:170>). The third pass drifted away from the live file and treated this as generic helper-scope debt. The fourth pass correctly recentered the review on container validity and identified a bounded hardening path that does not require a repo-wide `MaybeUninit` migration.

The only thing to normalize from the fourth pass is its custom bucket wording. The correct canonical bucket is still the standard repo bucket `Small surgical refactor`: land bounded hardening now, and keep the broader performance-preserving redesign deferred.

## Safe-Now Priority

- Harden the concrete scalar helper surface so newly exposed `Vec<T>` ranges are initialized before `set_len` becomes observable. The immediate targets are [alloc_f64_with_prefix_uninit_tail](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:43>), [alloc_f32_with_prefix_uninit_tail](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:69>), [alloc_uninit_f64](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:94>), [alloc_uninit_f32](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:112>), and the `resize_uninit_*` helpers.
- Narrow or remove [alloc_uninit_copy<T: Copy>](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:33>) as a generic escape hatch. If a generic helper remains, it should be limited to explicitly supported scalar types with a defined initialization story.
- Add release-profile tests and docs that describe the real contracts here. The current tests near [315](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:315>) mostly exercise overwrite discipline and debug-poison assumptions rather than the release-mode validity boundary.
- Preserve the already-sound matrix pattern in [make_uninit_matrix_f64](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:188>) and [make_uninit_matrix_f32](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/alloc_policy.rs:217>) as the local reference model.

## Deferred Work

- Keep any broader `Vec<MaybeUninit<T>>` propagation redesign deferred and benchmarked.
- Do not mix the safe-now hardening step with a multi-file scratch-buffer API redesign across `double_ma`, `ml_signal`, or GPU-adjacent callers.

## Verification If Changed

- `cargo check -p ta_app_core`
- `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- `cargo test -p ta_app_core alloc_policy -- --nocapture`
- `cargo test -p ta_app_core --release alloc_policy -- --nocapture`
- If helper semantics or trait bounds change, rerun representative downstream users such as `cargo test -p ta_app_core double_ma -- --nocapture`
- If initialization strategy changes affect hot paths, run the CPU perf gate before landing

## Regression And Feature-Preservation Constraints

- Preserve CPU-only and CUDA build availability.
- Preserve the observable warm-prefix contract for the prefix helpers.
- Do not regress the already-sound matrix helpers back toward concrete `Vec<T>` exposure.
- Treat throughput regression as the main implementation risk. Full-range initialization is safer, but it can perturb scratch-buffer hot paths if landed without measurement.
- Avoid half-migrated semantics where some helpers promise defined values and others still rely on implicit overwrite discipline.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- land bounded hardening now for the concrete scalar helpers
- keep the larger performance-preserving redesign deferred
- do not keep treating `unsafe fn` or generic “helper truthfulness” as the whole story, because the live problem is concrete `Vec<T>` validity
