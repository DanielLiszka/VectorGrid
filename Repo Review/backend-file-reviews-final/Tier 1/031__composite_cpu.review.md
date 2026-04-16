# Final Consolidated Review: `crates/strategies/src/composite_cpu/mod.rs`

- Review date: `2026-04-15`
- Source file: [mod.rs](</C:/Rust Projects/ta_desktop_demo/crates/strategies/src/composite_cpu/mod.rs:1>)
- First-pass packet: [mod.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/strategies/src/composite_cpu/mod.rs.review.md:1>)
- Second-pass packet: [mod.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/strategies/src/composite_cpu/mod.rs.review.md:1>)
- Third-pass packet: [mod.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/strategies/src/composite_cpu/mod.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/031__crates__strategies__src__composite_cpu__mod.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid, but normalize prompt-artifact references`
- Canonical final action bucket: `Targeted hardening now; larger redesign still deferred`
- External research anchors:
  - <https://serde.rs/attr-default.html>
  - <https://serde.rs/container-attrs.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The earlier passes were right to treat this as more than a local CPU helper. This module owns public strategy behavior, a serde-visible request default surface, and part of the current `HardGate` CPU/GPU parity contract. That is why the broad refactor remains deferred. The fourth pass is also right that there is still meaningful safe-now hardening available without attempting that larger split.

The strongest bounded work is not "clean up ownership" in the abstract. It is pinning and documenting the real contracts that downstream code already depends on: `CompositeSignalMode` wire spellings and defaulting, the hidden pre-offset `start_t` expectation, the missing invalid-commission parity between metrics and curves, and the overlay-branch behaviors that current tests do not fully characterize.

No wrong fourth-pass packet is present in this folder. The packet belongs to the correct file. Like `030`, the issue is wording contamination from `source__...` and `context__...` references, not misplacement.

## Safe-Now Priority

- Add serde/default regression tests for `CompositeSignalMode`:
  - exact snake_case wire spellings
  - omitted-field default to `HardGate`
- Add targeted overlay-behavior tests that the current suite does not directly lock:
  - signal persistence
  - cooldown re-entry blocking
  - `allow_flip == false` behavior outside `HardGate`
- Document the public caller contract around `start_t`, including the pre-applied warmup and `trade_on_next_bar` offset assumption.
- Either align the invalid-commission sentinel behavior between metrics and curves, or explicitly codify and test the difference as an intentional contract.

## Deferred Work

- Defer deduplicating the metrics and curves transition engines.
- Defer moving the public evaluators to stronger validated `Result`-style APIs.
- Defer any attempt to enable non-`HardGate` base-composite GPU routes from this file alone.
- Defer broader module extraction until app-core consumers and GPU parity surfaces are reviewed together.

## Verification If Changed

- `cargo test -p ta_strategies -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`
- If `HardGate` behavior or parity logic changes and CUDA is available, rerun the relevant GPU fallback/parity coverage.
- If hot-loop logic changes materially, treat the patch as perf-sensitive and run the CPU perf gate.

## Regression And Feature-Preservation Constraints

- Preserve `CompositeSignalMode` wire spellings and omitted-field default behavior unless request compatibility is intentionally changed.
- Preserve current `HardGate` semantics unless CPU and GPU parity are rechecked together.
- Keep the snapshot of current entry/exit overlay behavior stable while expanding tests around it.
- Do not mix safe-now contract hardening with any larger GPU-route expansion or transition-engine rewrite.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- move `composite_cpu/mod.rs` to `Targeted hardening now; larger redesign still deferred`
- spend the immediate effort on serde/default coverage, overlay characterization, `start_t` contract clarity, and commission-sentinel parity
- preserve current `HardGate` semantics and downstream GPU parity expectations
- defer transition-engine deduplication, API reshaping, and route-matrix expansion until a dedicated follow-on pass
