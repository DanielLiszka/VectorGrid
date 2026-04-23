# Final Consolidated Review: `src-tauri/src/state.rs`

- Review date: `2026-04-16`
- Source file: [state.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/state.rs:1>)
- First-pass packet: [state.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/state.rs.review.md:1>)
- Second-pass packet: [state.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/state.rs.review.md:1>)
- Third-pass packet: [state.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/state.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/052__src-tauri__src__state.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://v2.tauri.app/develop/state-management/>
  - <https://doc.rust-lang.org/std/sync/struct.Arc.html>
  - <https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html>
  - <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>

## Consolidated Judgment

The fourth pass is valid and should govern the final review. It corrects the third-pass drift toward persistence language and puts the focus back where the live source actually is: runtime-state coordination.

The most important live issue is the timestamp and epoch contract around [`upsert_live_bar()`](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/state.rs:432>). Load-time normalization treats timestamps as milliseconds once they reach the 1980 epoch cutoff, while live upsert still uses a 2001-era `timestamp_uses_millis()` threshold. That mismatch can misread legitimate 1980-to-2001 millisecond series during live-bar mutation. The live-upsert path also never proves the resident dataset is epoch-based, which makes relative-timeline datasets unsafe live targets even if unit inference is fixed. The next strongest issue is runtime race posture: `load_price_data()` and `resample_loaded_data()` both have check-then-act windows where overlapping calls can create duplicate IDs, immediate eviction churn, or stale parent-version tagging.

## Safe-Now Priority

- Unify the live-bar timestamp heuristic with load-time normalization and reject or guard non-epoch resident datasets during live upsert.
- Add a second reuse check in `load_price_data()` after disk load but before base insert.
- Tighten `resample_loaded_data()` snapshotting so parent freshness is rechecked before derived insert.
- Make the live-bar mutation contract explicit for dataset kind and reject derived-dataset upserts if that is the intended model.
- Change the run-guard error string to generic run wording rather than optimize-only wording.

## Deferred Work

- Do not replace the shared `AtomicBool` run model with generation-scoped tokens in this pass.
- Do not collapse path identity to `canonicalize()` alone as a small refactor.
- Do not introduce a structured live-bar outcome enum unless the command boundary truly needs it.
- Do not reopen `Arc::make_mut` or `SeqCst` ordering as cleanup targets without stronger evidence.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app state_ -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_execution -- --nocapture`
- Add focused regressions for:
  - 1980-to-2001 millisecond live-upsert handling,
  - non-epoch live-upsert rejection/guard behavior,
  - overlapping-load second-check logic,
  - parent-freshness recheck during derived insert

## Regression And Feature-Preservation Constraints

- Preserve the current `LoadPriceDataResponse` schema and dataset-ID shape.
- Preserve clone-on-write mutation via `Arc::make_mut`.
- Preserve `SeqCst` ordering on run/background guards unless there is a deliberate concurrency proof to relax it.
- Preserve current append-at-tail and exact-timestamp overwrite behavior for supported live-upsert cases.
- Preserve retention and descendant-invalidation semantics while tightening overlapping-call behavior.

## Final Conclusion

Keep `state.rs` in `Small surgical refactor`. The right implementation target is a narrow runtime-coordination hardening pass around timestamp/epoch handling, overlapping load/resample windows, dataset-kind rules for live upserts, and accurate run-guard wording.

