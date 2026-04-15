# Final Consolidated Review: `crates/app_core/src/iterative_sim.rs`

- Review date: `2026-04-14`
- Source file: [iterative_sim.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/iterative_sim.rs:1>)
- First-pass packet: [iterative_sim.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/iterative_sim.rs.review.md:1>)
- Second-pass packet: [iterative_sim.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/iterative_sim.rs.review.md:1>)
- Third-pass packet: [iterative_sim.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/iterative_sim.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/009__crates__app_core__src__iterative_sim.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid in substance but needs normalization`
- Canonical final action bucket: `Small surgical refactor`

## Consolidated Judgment

The second and third passes were right that this file carries real contract debt, but they over-escalated by treating that as proof the file had to wait for a broader result-type redesign. The fourth pass correctly recovered the smaller safe-now path: add an additive fallible boundary, document the semantics this loop actually uses, and pin the edge-case behavior before trying to align everything through a large shared redesign. Normalize that conclusion by keeping the change additive. The final packet should not imply that the current public function should be broken in place.

## Safe-Now Priority

- Add an additive fallible entrypoint such as `try_simulate_iterative_from_signals(...) -> Result<Metrics, ...>` while keeping the current API as a compatibility wrapper for now.
- Document the actual public semantics:
  - stop/take inputs are normalized silently
  - `slippage_vol_mult` uses same-bar realized move as its multiplier input
  - exposure and net exposure are derived from this loop’s state accounting, not an intrabar fractional model
- Reuse the shared risk finalization path for the subset of fields this simulator can already compute, instead of treating placeholder fields as proof the entire result shape must wait.
- Add focused contract tests for invalid input, intrabar tie rules, cooldown behavior, and exposure accounting on stop/take bars.

## Deferred Work

- Keep any larger `Metrics` redesign or cross-simulator result-shape unification deferred.
- Do not extract or share helper logic with other simulators until current iterative semantics are pinned more tightly.

## Verification If Changed

- `cargo test -p ta_app_core iterative_sim -- --nocapture`
- Rerun representative iterative-vs-vectorized comparison tests if this file already feeds those paths
- Add targeted tests for invalid input and stop/take edge cases before touching semantics

## Regression And Feature-Preservation Constraints

- Do not change the existing public function to return errors in place; keep the new fallible behavior additive.
- Pin current cooldown, exposure, and slippage semantics before changing them, because they affect validation outputs and iterative-vs-vectorized deltas.
- Treat filling previously zeroed metrics as a visible output change even if it is more truthful.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- change `iterative_sim.rs` back to `Small surgical refactor`
- add a fallible boundary and explicit contract docs/tests now
- keep broader result-type redesign work deferred
