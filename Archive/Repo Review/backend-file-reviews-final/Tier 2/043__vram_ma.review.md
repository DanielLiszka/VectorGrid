# Final Consolidated Review: `crates/app_core/src/vram_ma.rs`

- Review date: `2026-04-16`
- Source file: [vram_ma.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/vram_ma.rs:1>)
- First-pass packet: [vram_ma.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/vram_ma.rs.review.md:1>)
- Second-pass packet: [vram_ma.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vram_ma.rs.review.md:1>)
- Third-pass packet: [vram_ma.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vram_ma.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/043__crates__app_core__src__vram_ma.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Document only`
- External research anchors:
  - <https://doc.rust-lang.org/reference/visibility-and-privacy.html>
  - <https://doc.rust-lang.org/reference/items/use-declarations.html>

## Consolidated Judgment

The fourth pass is valid and should govern the final review. It corrects the earlier misclassification of this file as a public serde/default contract surface.

[`vram_ma.rs`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/vram_ma.rs:1>) is a crate-private, feature-gated internal shim over upstream `vector_ta` CUDA moving-average helpers, not an externally reachable `ta_app_core` API seam. The live source contains no local runtime behavior beyond conditional compilation and re-exporting, so the real review value here is intent clarity, not refactor churn or invented contract tests.

## Safe-Now Priority

- If this file is touched, tighten the re-export intent to `pub(crate) use` so visibility matches the enclosing crate-private module.
- Add a short module comment explaining that this is an internal GPU bridge over `vector_ta::cuda::moving_averages::vram_ma`.
- Keep the source unchanged if no surrounding GPU-plumbing edit already needs to touch it.

## Deferred Work

- Do not promote this module or its members to the public `ta_app_core` API in this pass.
- Do not remove the wrapper unless the in-crate consumers are updated together and the upstream-path coupling is intentionally accepted.
- Do not invent serde, DTO, or request-contract tests for a file that does not own that behavior.

## Verification If Changed

- `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`
- If feature gating changes, also run `cargo check -p ta_app_core`
- If the wrapper or imports move, add one representative CUDA consumer check:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_backtest_kernel -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve the current dual CUDA feature gate and keep CPU-only builds working.
- Keep the current in-crate import seam available unless all internal callers move together.
- Do not attach performance, validation, or persistence claims to changes here; this file does not own those surfaces.

## Final Conclusion

Keep `vram_ma.rs` in `Document only`. The fourth pass correctly narrows this to a crate-private visibility-and-intent cleanup surface, with optional `pub(crate) use` tightening and a short comment as the only meaningful safe-now changes.

