# Final Consolidated Review: `src-tauri/src/run_artifacts.rs`

- Review date: `2026-04-15`
- Source file: [run_artifacts.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/run_artifacts.rs:1>)
- First-pass packet: [run_artifacts.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/run_artifacts.rs.review.md:1>)
- Second-pass packet: [run_artifacts.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/run_artifacts.rs.review.md:1>)
- Third-pass packet: [run_artifacts.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/run_artifacts.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/046__src-tauri__src__run_artifacts.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid as-is`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/path/struct.Path.html>
  - <https://doc.rust-lang.org/std/fs/struct.OpenOptions.html>
  - <https://doc.rust-lang.org/std/fs/fn.write.html>
  - <https://doc.rust-lang.org/std/fs/fn.rename.html>
  - <https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all>

## Consolidated Judgment

All four passes converge on the same bucket, and that still looks right. `run_artifacts.rs` is a large persistence surface, but the highest-value remaining issues are bounded and local enough to fix without starting a file-format or storage-policy redesign.

The fourth pass correctly sharpens the priority away from generic durability polish and toward live correctness gaps in overwrite handling, artifact normalization consistency, and latest-run selection. The packet belongs to the correct file and does not appear to be a misplaced copy. No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Tighten overwrite-path handling so a caller-supplied overwrite target that fails validation does not silently fall back to allocating a fresh output path.
- Unify summary-surface and hydration-surface normalization for fields such as `config_fingerprint`, inferred version, and timestamp fallback so one persisted artifact cannot disagree with itself depending on which IPC surface loaded it.
- Replace lexicographic latest-run selection with a correctness-oriented rule based on parsed run identity, timestamp semantics, or another explicit recency signal that works across both legacy and current filename prefixes.
- Add targeted tests that exercise overwrite rejection, legacy/current artifact normalization parity, and latest-run selection across mixed prefix families.

## Deferred Work

- Defer broader staged-write and durability policy redesign across all artifact families.
- Defer live-session append-path durability changes unless the repo decides to unify persistence guarantees for high-frequency session updates and canonical artifact saves.
- Defer typed artifact-model extraction and any schema migration.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app -- --nocapture`
- Add tests for overwrite-path validation failures and confirm they fail explicitly instead of allocating a new path silently.
- Add parity tests comparing hydrated artifact metadata with summary-listed metadata for the same file.
- Add a regression test proving that mixed `vectorbt_run_` and `vectorgrid_run_` filenames choose the newest artifact correctly.

## Regression And Feature-Preservation Constraints

- Preserve backward compatibility for existing artifact filenames and on-disk JSON shape.
- Do not turn a narrow overwrite-contract fix into a broader path-policy rewrite.
- Keep any summary/hydration normalization unification behaviorally additive for legacy artifacts.
- Avoid mixing storage-policy redesign with latest-run selection correctness fixes.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `run_artifacts.rs` in `Small surgical refactor`
- fix overwrite fallback behavior, normalization parity, and latest-run selection correctness now
- cover those changes with focused artifact-level regression tests
- defer broader durability-policy and schema redesign work
