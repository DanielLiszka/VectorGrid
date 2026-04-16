# Final Consolidated Review: `crates/app_core/src/ml_signal.rs`

- Review date: `2026-04-16`
- Source file: [ml_signal.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/ml_signal.rs:1>)
- First-pass packet: [ml_signal.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/ml_signal.rs.review.md:1>)
- Second-pass packet: [ml_signal.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/ml_signal.rs.review.md:1>)
- Third-pass packet: [ml_signal.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/ml_signal.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/030__crates__app_core__src__ml_signal.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/io/struct.BufWriter.html>
  - <https://doc.rust-lang.org/std/fs/struct.File.html>
  - <https://doc.rust-lang.org/std/fs/fn.rename.html>
  - <https://docs.rs/csv/latest/csv/struct.Writer.html>

## Consolidated Judgment

All four passes agree that `ml_signal.rs` is not a redesign-first target, but the fourth pass is the first one that correctly identifies the highest-value safe-now work: persistence truthfulness, score-cache truthfulness after timestamp alignment, and export/bundle behavior that looks stronger than the live code actually guarantees.

No wrong `review_fourth_pass.md` is present in this folder. The fourth-pass packet matches the live source and is the correct governing review for the next implementation pass.

## Safe-Now Priority

- Harden write publication order and flush behavior.
  - `write_json_file()` currently relies on drop-time flush behavior instead of an explicit `flush()`.
  - the dataset CSV is opened at the final destination path before export completes;
  - score-cache JSON writes follow the same "final path first" pattern.
  - The safest next step is same-directory staging for primary outputs, explicit flush, then rename into place, while keeping sidecar publication behind primary success.
- Recompute score-cache truth from the rows that actually matched the current candle timestamps.
  - `load_score_cache()` currently counts probability presence before timestamp alignment and returns summary fields from persisted cache metadata.
  - Add an effective matched-row summary after alignment and reject zero actual matched probability rows instead of silently accepting a no-overlap cache that degrades into a flat validation run.
- Make returned public fields truthful:
  - separate actual matched observation count from the clamped metric count used for probability math so `track_record_bars` stops overstating sparse coverage;
  - align export manifest `first_timestamp` / `last_timestamp` semantics with exported-row bounds, or explicitly document them as source-candle-span fields.
- Validate dynamic-indicator output lengths in `feature_frame()` before the export path indexes those series directly.
- Add targeted tests for:
  - staged publication / explicit flush behavior,
  - zero-overlap score-cache rejection,
  - export manifest timestamp truth,
  - bundle-listing resilience when one manifest is malformed.

## Deferred Work

- Do not treat temp-file-plus-rename as a true multi-file transaction. It is a per-file hardening strategy, not all-or-nothing publication across CSV plus sidecars.
- Do not stealth-redesign split-window artifact semantics in this pass. Overlapping windows and row-level `split_id` compression need a versioned artifact decision, not a local tweak.
- Do not broaden this into a module split or full schema redesign just because the file is large.
- Do not attempt a feature-schema identity redesign in the same patch as the write/load truthfulness fixes.

## Verification If Changed

- `cargo test -p ta_app_core ml_signal::tests:: -- --nocapture`
- `cargo bench -p ta_app_core --bench ml_signal_workflow --no-run`
- `cargo bench -p ta_app_core --bench ml_signal_validation --no-run`
- If score-cache load semantics change:
  - focused validation-cache contract tests
  - zero-overlap and partial-overlap regression tests
- If export publication changes:
  - direct `export_ml_signal_dataset_with_candles()` artifact tests
  - bundle-listing / manifest-classification tests

## Regression And Feature-Preservation Constraints

- Preserve the current workflow split: dataset export, score-cache build/load, validation, policy sweep, and bundle inspection remain owned here.
- Preserve additive compatibility for existing ML artifacts unless a versioned migration is being introduced.
- Do not weaken the current long-only versus two-sided validation distinctions while fixing cache truthfulness.
- Keep same-directory staging if file rename is used; cross-filesystem rename is not a safe default.

## Final Conclusion

Keep `ml_signal.rs` in `Small surgical refactor`, treat the fourth pass as valid, and prioritize write/load truthfulness plus targeted export and bundle tests. Split-window redesign, broader schema cleanup, and module decomposition remain deferred.
