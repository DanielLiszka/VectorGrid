# Final Consolidated Review: `crates/app_core/src/cdmc.rs`

- Review date: `2026-04-14`
- Source file: [cdmc.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc.rs:1>)
- First-pass packet: [cdmc.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/cdmc.rs.review.md:1>)
- Second-pass packet: [review__second_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/cdmc.rs.review.md:1>)
- Third-pass packet: [review__third_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/cdmc.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/002__crates__app_core__src__cdmc.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid in substance but needs normalization`
- Canonical final action bucket: `Larger refactor but defer`
- External research anchors:
  - <https://vectoralpha.dev/projects/ta/indicators/correlation_cycle/>
  - <https://vectoralpha.dev/projects/ta/indicators/dpo/>
  - <https://vectoralpha.dev/projects/ta/indicators/decycler/>
  - <https://vectoralpha.dev/projects/ta/indicators/mama/>

## Consolidated Judgment

The first pass found real local hardening opportunities but underestimated how semantics-sensitive this file is. The second pass correctly raised the bucket once it became clear that the file is layering product behavior on top of upstream Correlation Cycle `state`. The third pass then over-weighted silent GPU downgrade as the top issue. The fourth pass corrected the ranking: the sharper structural problem is the public sweep-helper contract around [CdmcPeriodSweepDevice::first_valid_for_period](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc.rs:150>), which currently returns a single `period - 1` style answer even though the module serves DPO, Correlation Cycle `angle`, Correlation Cycle `state`, and Decycler with materially different warmup behavior.

Normalize the fourth pass by keeping the standard repo bucket and by treating silent GPU downgrade as observability debt, not as the safe-now contract rewrite for this file.

## Safe-Now Priority

- Treat [first_valid_for_period](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc.rs:150>) as an under-specified public helper. If the type cannot be redesigned yet, document the helper as inexact and stop presenting it as a general exact warmup boundary.
- Add narrow hardening to [resolve_output_id](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc.rs:287>): preserve fallback for single-output indicators, but fail clearly for unresolved multi-output indicators instead of silently binding the wrong output.
- Document and test the current strategy-local policy in one place:
  - current `cycle_state` interpretation in [build_cdmc_desired_positions](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc.rs:883>)
  - fixed Correlation Cycle threshold behavior
  - [resolve_cdmc_periods](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc.rs:217>) normalization
  - the magnitude-only behavior of [phase_threshold](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc.rs:41>)
- Improve preferred-GPU fallback observability without changing the fallback contract in this file.

## Deferred Work

- Keep any reinterpretation of `cycle_state` semantics deferred until CPU behavior, GPU parity, optimization ranking, and validation expectations are reviewed together.
- Keep exact warmup metadata redesign deferred until the sweep-helper API can carry indicator- and output-specific metadata instead of a one-size-fits-all contract.
- Do not turn preferred-GPU fallback into strict failure here without reviewing the owning route-policy layer.

## Verification If Changed

- `cargo test -p ta_app_core cdmc -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`
- If CUDA-facing sweep or fallback behavior changes:
  - `cargo test -p ta_app_core --features cuda cdmc -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc -- --nocapture`
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve CPU/GPU behavioral parity across `cdmc.rs`, any CUDA kernel path, and validation reuse paths.
- Do not change [first_valid_for_period](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/cdmc.rs:150>) in place and call it a cleanup; downstream consumers may already compensate around the current weak contract.
- Treat any `cycle_state` reinterpretation or threshold change as product behavior work, not as documentation cleanup.
- Preserve saved-config replay unless validation tightening is paired with an explicit compatibility decision.
- Keep single-output fallback compatibility if output-resolution hardening is added.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `cdmc.rs` in `Larger refactor but defer`
- move the top structural concern to the under-specified sweep-helper warmup contract
- keep safe-now changes narrow: output-resolution hardening, policy documentation/tests, and fallback observability
