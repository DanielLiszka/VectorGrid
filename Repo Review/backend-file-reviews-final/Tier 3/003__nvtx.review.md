# Final Consolidated Review: `crates/app_core/src/nvtx.rs`

- Review date: `2026-04-16`
- Source file: [nvtx.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/nvtx.rs:1>)
- First-pass packet: [nvtx.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/nvtx.rs.review.md:1>)
- Second-pass packet: [nvtx.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/nvtx.rs.review.md:1>)
- Third-pass packet: [nvtx.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/nvtx.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 3/003__crates__app_core__src__nvtx.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://nvidia.github.io/NVTX/doxygen/group___m_a_r_k_e_r_s___a_n_d___r_a_n_g_e_s.html>
  - <https://docs.rs/libloading/latest/libloading/struct.Library.html>
  - <https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-must_use-attribute>
  - <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>

## Consolidated Judgment

The first three packets correctly kept this file in a small-bounded bucket, but they stayed too abstract. The fourth pass is valid and should govern the final review because it identifies the direct source-level issues that actually matter in [RangeGuard](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/nvtx.rs:106>), [scope](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/nvtx.rs:150>), [range](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/nvtx.rs:159>), and the loader path at [api](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/nvtx.rs:252>) / [try_load_api](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/nvtx.rs:256>).

The strongest live issue is that disabled guards are not fully inert today because `Drop` reaches `api()` before it proves that nothing was ever pushed. The next strongest issues are missing `#[must_use]` on the public RAII surface, pop bookkeeping that assumes push succeeded, and a loader path that requires symbols the active runtime path does not use.

## Safe-Now Priority

- Make disabled guards truly inert in `Drop` before calling `api()`.
- Add `#[must_use]` to the public RAII guard surface.
- Track push success before scheduling a pop.
- Narrow symbol requirements to the active fallback path instead of requiring unused richer symbols.
- Add focused docs/tests for env gating, fallback naming, and current no-op behavior.

## Deferred Work

- Re-enabling domain/color support.
- Any public API reshape around typed domains.
- Registered-string optimization or broader NVTX v3 integration changes.
- Any profiler-visible rename or nesting-policy change.

## Verification If Changed

- `cargo test -p ta_app_core nvtx::tests:: -- --nocapture`
- `cargo check -p ta_app_core --features nvtx-profiling`
- `cargo check -p ta_desktop_demo_app --features nvtx-profiling`
- Re-run Nsight/NVTX scripts if capture semantics or range names change.

## Regression And Feature-Preservation Constraints

- Preserve the current top-level capture names targeted by existing scripts.
- Keep setup failures best-effort and non-fatal.
- Do not casually activate domain-qualified ranges or richer push APIs.
- Keep the `libloading` lifetime invariant explicit and preserved.

## Final Conclusion

Keep `nvtx.rs` in `Small surgical refactor`. The fourth pass is valid, no wrong fourth-pass file is present, and it is the first packet that sharpens the live work into direct correctness fixes instead of generic support-seam caution.
