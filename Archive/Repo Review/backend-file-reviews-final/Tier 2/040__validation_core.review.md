# Final Consolidated Review: `crates/app_core/src/validation_core.rs`

- Review date: `2026-04-16`
- Source file: [validation_core.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/validation_core.rs:1>)
- First-pass packet: [validation_core.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/validation_core.rs.review.md:1>)
- Second-pass packet: [validation_core.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_core.rs.review.md:1>)
- Third-pass packet: [validation_core.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/validation_core.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/040__crates__app_core__src__validation_core.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://docs.rs/rayon/latest/rayon/iter/trait.IndexedParallelIterator.html>

## Consolidated Judgment

The fourth pass correctly recenters this file on internal validation invariants instead of serde-facing compatibility language. That framing should stand. One strengthening is needed in the final consolidation, though: the repo moved part of the observation-floor policy downstream after the earlier passes, so the local clamp discussion now needs to be more precise.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live file and should govern the final review.

[`validation_core`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/lib.rs:106>) is crate-private, and the real contract here is the orchestration between `start_t`, `sample_obs`, deterministic seed plumbing, and optional CSCV finalization. The tuple returned by `process_one` still wants stronger intent signaling and stronger invariant tests. Separately, [`validation_core.rs:41`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/validation_core.rs:41>) still pre-clamps `sample_obs` with `max(2)`, while [`compute_probability_metrics()`](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/validation_pipeline.rs:288>) now also floors internally after first recording `requested_obs`. That means the local clamp is no longer the sole owner of the policy, and its continued presence is now a repo-current behavior choice rather than an obvious invariant requirement.

## Safe-Now Priority

- Add focused tests for:
  - `common_start_t` being the maximum returned start,
  - skipping CSCV slice-builder work entirely when `cscv_cfg` is `None`,
  - deterministic replay when `process_one` actually consumes RNG output.
- Add a short contract comment on the meaning of the returned tuple: `(candidate, candidate_start_t_for_common_cscv_alignment, sample_observations_for_probability_metrics)`.
- After the tests exist, decide deliberately whether the local `obs.max(2)` should remain:
  - keep it if the intended exposed contract is still "minimum 2 track-record bars",
  - remove it if the intended exposed contract is now "preserve requested observation count while flooring PSR/DSR internally."

## Deferred Work

- Do not change the tuple to a new struct in this pass unless caller touch points are already open for edit.
- Do not partially type only the local progress phases while downstream phases remain string-based.
- Do not parallelize the CSCV slice-builder closure path as a cleanup.

## Verification If Changed

- `cargo test -p ta_app_core validation_core::tests:: -- --nocapture`
- `cargo test -p ta_app_core double_ma_validate -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`
- If the local observation clamp changes, add an explicit regression that locks down the intended `track_record_bars` behavior.

## Regression And Feature-Preservation Constraints

- Preserve the semantic distinction between `start_t` and `sample_obs`; they feed different downstream math.
- Preserve deterministic seed plumbing unless reproducibility is intentionally re-baselined.
- Treat any change to the local observation floor as externally meaningful to validation metrics and reports.
- Keep this file framed as an internal orchestration seam, not as a public serialization boundary.

## Final Conclusion

Keep `validation_core.rs` in `Small surgical refactor`. The next change set should be tests and intent-signaling first, with the local `sample_obs` pre-clamp treated as a deliberate follow-up decision rather than an unquestioned invariant.
