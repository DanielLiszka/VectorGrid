# Final Consolidated Review: `crates/app_core/src/mkcs.rs`

- Review date: `2026-04-16`
- Source file: [mkcs.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/mkcs.rs:1>)
- First-pass packet: [mkcs.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/mkcs.rs.review.md:1>)
- Second-pass packet: [mkcs.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/mkcs.rs.review.md:1>)
- Third-pass packet: [mkcs.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/mkcs.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/029__crates__app_core__src__mkcs.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://serde.rs/field-attrs.html>

## Consolidated Judgment

The first three passes got the bucket right: `mkcs.rs` is a strategy-local, route-sensitive contract surface, not a refactor target. The fourth pass is the best implementation guide because it replaced generic "policy surface" caution with concrete file-local issues that are visible in the live code.

No wrong `review_fourth_pass.md` is present in this folder. The fourth-pass packet matches the live file and is materially stronger than the earlier abstract framing.

## Safe-Now Priority

- Document and test the current GPU-preferred-but-not-GPU-guaranteed behavior in `compute_indicator_f64()`. A non-CPU backend can still fall back to CPU when prepared CUDA inputs are absent or CUDA compute fails, and MKCS signal acquisition can therefore become mixed-provenance across separate indicator calls.
- Harden the `Result`-returning signal boundaries before touching desired-position semantics. The narrowest high-value change is to validate that:
  - `compute_mkcs_compression_signals()` returns a series matching `candles.close.len()`;
  - `compute_mkcs_breakout_signals()` returns upper/lower series matching each other and the candle length;
  - CUDA period sweeps report the expected row cardinality for the normalized period range.
- Add strategy-specific output-contract coverage for Donchian binding. `mkcs_indicator_ids()` should not silently allow upper and lower outputs to collapse to the same descriptor output, and `compute_mkcs_donchian_output_sweep_cuda_device()` should have an explicit test that pins its current "unknown preference means upper" behavior if that behavior is intentionally preserved.
- Add rustdoc on the public strategy config and signal/sweep carrier types explaining:
  - serde defaults are active compatibility contract,
  - `OnceLock<Result<...>>` makes the first descriptor-init outcome process-lifetime sticky,
  - device-resident sweeps are intentionally kept on-device for downstream consumers,
  - `build_mkcs_desired_positions()` expects caller-validated lengths and `start_t`.

## Deferred Work

- Do not flip the current best-effort GPU fallback policy into hard `GpuOnly` failure in a small patch.
- Do not redesign `build_mkcs_desired_positions()` into a new fallible public API in the same pass.
- Do not replace the raw device-pointer surfaces with wrapper types unless the wrapper encodes a real invariant and the wider GPU contract is being redesigned together.
- Do not prioritize stale bench cleanup ahead of the earlier correctness and observability fixes unless MKCS performance-sensitive code is also changing.

## Verification If Changed

- `cargo test -p ta_app_core mkcs::tests:: -- --nocapture`
- `cargo test -p ta_app_core mkcs_optimization_smoke_runs -- --nocapture`
- `cargo test -p ta_app_core mkcs::tests::compute_mkcs_signals_gpu_smoke_runs_with_stddev_and_donchian_surfaces --features cuda -- --nocapture`
- If CUDA sweep or descriptor-binding behavior changes:
  - representative MKCS composite optimization and validation paths
  - `cargo bench -p ta_app_core --bench mkcs_opt --no-run` if performance-sensitive MKCS code changed

## Regression And Feature-Preservation Constraints

- Preserve the current device-resident sweep posture and do not host-stage MKCS sweep intermediates.
- Preserve current config defaults and period-mapping behavior unless callers and stored artifacts are reviewed together.
- Treat current fallback-to-CPU behavior as a compatibility-facing route choice until explicitly redesigned.
- Keep strategy-local organization intact; there is no evidence-backed reason to genericize MKCS logic in this pass.

## Final Conclusion

Keep `mkcs.rs` in `Small surgical refactor`, treat the fourth pass as valid, and prioritize boundary validation, output-contract tests, and explicit route documentation. Hard semantic changes to fallback policy, public API shape, or GPU transport types remain deferred.
