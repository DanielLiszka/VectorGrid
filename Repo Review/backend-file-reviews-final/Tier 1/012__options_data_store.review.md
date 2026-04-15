# Final Consolidated Review: `crates/app_core/src/options_data_store.rs`

- Review date: `2026-04-14`
- Source file: [options_data_store.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/options_data_store.rs:1>)
- First-pass packet: [options_data_store.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/options_data_store.rs.review.md:1>)
- Second-pass packet: [options_data_store.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/options_data_store.rs.review.md:1>)
- Third-pass packet: [options_data_store.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/options_data_store.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/012__crates__app_core__src__options_data_store.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid in substance but needs normalization`
- Canonical final action bucket: `Small surgical refactor`

## Consolidated Judgment

The fourth pass found the right class of issues, but it needs normalization so the final packet does not over-tighten persisted-data contracts casually. The safe-now direction is still `Small surgical refactor`: improve header and metadata honesty, make manifest version support explicit, and validate quote sufficiency against the retained dataset rather than the pre-dedup stream. The key normalization is scope: treat these as persistence-boundary hardening changes that need fixture-led rollout, not as cleanup that can silently redefine import semantics.

## Safe-Now Priority

- Add same-contract metadata consistency checks when appending rows to an existing `contract_id`. The importer should not keep the first contract metadata and blindly append later rows that disagree on fields downstream consumers rely on.
- Reject duplicate normalized headers up front instead of letting the last normalized match silently win.
- Enforce manifest-version support on read so the persistence boundary is honest about what it can deserialize and interpret.
- Validate `TopOfBookQuote` sufficiency against retained rows after deduplication, not only against the raw incoming stream.

## Deferred Work

- Keep any broader persistence migration or compatibility fallback plan separate from this hardening pass.
- Do not silently redefine `manifest.row_count` or the current duplicate-timestamp first-seen-wins behavior in the same patch.

## Verification If Changed

- `cargo test -p ta_app_core options_data_store -- --nocapture`
- Add fixture-led tests for:
  - conflicting contract metadata on append
  - duplicate normalized headers
  - unsupported manifest version on read
  - quote sufficiency before vs. after dedup

## Regression And Feature-Preservation Constraints

- Tightening root or contract metadata validation can reject datasets that currently import successfully.
- Manifest-version enforcement can strand older stored manifests if there is no explicit compatibility path.
- Moving quote sufficiency checks after dedup can make some currently accepted datasets fail import.
- Treat persisted manifest and duplicate-timestamp behavior as output semantics, not as incidental implementation detail.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `options_data_store.rs` at `Small surgical refactor`
- harden metadata/header/version/quote validation now
- keep persistence-semantics redefinition and migration planning out of scope for this patch
