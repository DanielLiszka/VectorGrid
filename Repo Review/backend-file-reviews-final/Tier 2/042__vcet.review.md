# Final Consolidated Review: `crates/app_core/src/vcet.rs`

- Review date: `2026-04-16`
- Source file: [vcet.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/vcet.rs:1>)
- First-pass packet: [vcet.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/vcet.rs.review.md:1>)
- Second-pass packet: [vcet.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vcet.rs.review.md:1>)
- Third-pass packet: [vcet.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vcet.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/042__crates__app_core__src__vcet.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://serde.rs/field-attrs.html>
  - <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
  - <https://doc.rust-lang.org/std/macro.thread_local.html>
  - <https://doc.rust-lang.org/std/primitive.u16.html>

## Consolidated Judgment

The fourth pass is valid and materially better than the earlier packets because it identifies a real safe-now hardening lane without widening the file into a redesign target.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live file and should govern the final review.

`vcet.rs` is still a public, semantics-sensitive, CPU/CUDA-adjacent strategy owner. The next improvements should stay local: better branch coverage around normalization and position semantics, one additive route-diagnostic improvement in [`compute_indicator_f64()`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/vcet.rs:488>), and a defensive pointer-arithmetic guard in [`VcetOwnedDeviceMatrix::from_shared_row()`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/vcet.rs:229>). The file also has a real but deferred efficiency topic in the double generic `chandelier_exit` dispatch, and that should remain deferred rather than "cleaned up" locally.

## Safe-Now Priority

- Expand file-local tests for:
  - mixed `resolve_vcet_periods()` map-flag combinations,
  - `compute_vcet_start_t()` with `trade_on_next_bar`,
  - `build_vcet_desired_positions()` branches such as `allow_flip = false`, `long_only = true`, and `max_trade_bars`.
- Add an additive route diagnostic when a non-CPU backend falls through to CPU because `prepared_cuda_inputs` is absent.
- Replace `saturating_add` with checked pointer addition in `VcetOwnedDeviceMatrix::from_shared_row()` and return a descriptive error on overflow.

## Deferred Work

- Do not tighten serde defaults or turn current normalization behavior into new hard validation failures.
- Do not rewrite the `OnceLock<Result<...>>` indicator-binding model in this pass.
- Do not try to locally optimize the double `chandelier_exit` dispatch without broader adapter/parity review.
- Do not change the asymmetry between host-staged VIDYA fallback and Chandelier CUDA dispatch without tracing the full caller contract.

## Verification If Changed

- `cargo test -p ta_app_core vcet::tests:: -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`
- If CUDA-path behavior or diagnostics change, rerun representative VCET coverage under `--features cuda,cuda-backtest-kernel`.

## Regression And Feature-Preservation Constraints

- Preserve the public serde-driven `VcetStrategyParams` contract unless callers and persisted artifacts are audited together.
- Preserve current effective VCET semantics in `resolve_vcet_periods()`, `compute_vcet_start_t()`, and desired-position generation unless a broader validation review is being done.
- Preserve owner-backed device-lifetime semantics in `VcetOwnedDeviceMatrix`; do not simplify away the keepalive model.
- Treat additive diagnostics as test-sensitive if any existing tests or snapshots assert exact messages.

## Final Conclusion

Keep `vcet.rs` in `Small surgical refactor`. The right implementation is tests plus small guardrails: better branch coverage, one missing fallback diagnostic, and checked device-pointer arithmetic. Larger binding, performance, or config-contract rewrites remain deferred.
