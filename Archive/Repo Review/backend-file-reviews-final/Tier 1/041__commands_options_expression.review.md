# Final Consolidated Review: `src-tauri/src/commands_options_expression.rs`

- Review date: `2026-04-16`
- Source file: [commands_options_expression.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_options_expression.rs:1>)
- First-pass packet: [commands_options_expression.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_options_expression.rs.review.md:1>)
- Second-pass packet: [commands_options_expression.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_options_expression.rs.review.md:1>)
- Third-pass packet: [commands_options_expression.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_options_expression.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/041__src-tauri__src__commands_options_expression.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with reprioritization`
- Canonical final action bucket: `Larger refactor but defer`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
  - <https://doc.rust-lang.org/std/path/struct.Path.html#method.is_file>
  - <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
  - <https://docs.rs/bincode/latest/bincode/>

## Consolidated Judgment

The defer bucket should stand. This file still mixes too many contract-sensitive responsibilities for a "small cleanup" framing to be honest: replay caches, artifact creation/listing, CSV export, historical inspection, parent-artifact replay, and the public Tauri command surface. The third pass was directionally right about that and was also right to flag parent-artifact reopening and replay coupling as real problems.

The fourth pass is better because it changes the immediate safe-now set. The most credible bounded work is not export-path scoping. It is the summary maxima bug for all-negative data, signal-manifest cache corruption degrading to cache miss, first-hop containment on artifact entries during listing, and replacing the currently ineffective event-path replay-drift guard with a real recomputed-state check. It also sharpens the deeper deferred diagnosis: historical inspection needs stronger parent-content provenance, not only tuple-rank decoupling. No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Fix `best_robust_score` and `best_selection_expectancy_per_premium` so all-negative result sets do not report impossible `0.0` maxima.
- Make corrupt signal-manifest replay cache loads degrade to cache miss/rebuild instead of hard-failing the workflow.
- Validate artifact entries against `runs_dir` before the first read in artifact listing.
- If parent strategy kind is still resolved by reopening the parent artifact during listing, apply the same boundary discipline there.
- Replace the current dead event-path replay-drift guard with a real comparison against recomputed replay state.
- Add targeted tests for:
  - all-negative maxima
  - corrupt signal-manifest cache fallback
  - out-of-root or symlinked artifact entry rejection/skip
  - event-path replay drift detection

## Deferred Work

- Defer parent-content provenance and stable tuple identity redesign for historical inspection.
- Defer persisted replay-cache policy unification between signal-manifest and dataset caches.
- Defer feasibility-preview concurrency/run-guard policy changes until product intent is explicit.
- Defer export-path tightening unless it is done through an explicit command-scope or picker-backed contract.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_options_expression -- --nocapture`
- Add targeted tests for the safe-now items above.
- If parent-path containment logic changes during listing, add a tolerant-library-enumeration regression test so one bad artifact does not blank the entire list.
- If preview/run concurrency semantics change, re-run frontend/store tests for preview-versus-run behavior and cancellation/error semantics.

## Regression And Feature-Preservation Constraints

- Preserve current legitimate export UX; do not silently narrow `save_options_csv` to `runs/` or app-local data as a casual hardening step.
- Preserve current artifact-library compatibility for already-saved artifacts.
- Keep replay caches as accelerators only; corrupt cache data must not become authoritative.
- Do not present tuple identity alone as the full fix for historical inspection correctness.
- Keep library enumeration tolerant; one malformed artifact should not blank the whole list.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `commands_options_expression.rs` in `Larger refactor but defer`
- pull forward a different safe-now set than the third pass proposed
- prioritize summary-maxima correctness, cache miss-tolerance, first-hop containment, and real replay-drift checking now
- defer export-path policy, parent-content provenance, cache-policy unification, and preview concurrency redesign
