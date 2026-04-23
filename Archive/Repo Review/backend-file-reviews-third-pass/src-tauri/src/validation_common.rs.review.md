# Third-Pass Review: `src-tauri/src/validation_common.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/validation_common.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/validation_common.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/validation_common.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a shared validation helper that currently uses lexical path identity still looked like the right framing under fresh research. The third pass tightened the path-key finding because the current lowercasing step is not just approximate identity; on case-sensitive filesystems it can collapse distinct datasets into one entry.

## Third-Pass Research Log

1. Rust Path semantics
   - Source: <https://doc.rust-lang.org/std/path/struct.Path.html>
   - Why consulted in third pass:
     - reconfirm how much lexical path cleanup can really say about filesystem identity
   - Third-pass constraint informed:
     - string normalization should not be presented as true dataset identity across platforms

2. Rust canonicalize semantics
   - Source: <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
   - Why consulted in third pass:
     - ground the cross-platform path-identity discussion
   - Third-pass constraint informed:
     - identity-sensitive dedupe should be explicit about whether it is lexical or filesystem-backed

3. Rust API Guidelines dependability
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether the current helper is stronger than documentation-only concern
   - Third-pass constraint informed:
     - helpers that drop distinct user inputs need to make that policy explicit and test-backed

## What Third Pass Sharpened

### 1. Lowercasing the path key is a real cross-platform input-loss bug, not just fuzzy identity
- Evidence:
  - normalize_path_key lowercases every dataset path and normalizes slashes.
  - sanitize_cross_ticker_config deduplicates datasets through that lowered key.
- Why this matters:
  - On case-sensitive filesystems, distinct dataset paths can collapse into one entry before validation sees them.
- Practical implication:
  - Keep the existing bucket, but treat path-key policy as correctness-sensitive and cross-platform, not merely cosmetic normalization.

### 2. Cross-ticker dedupe should stay explicit about lexical versus filesystem identity
- Why this matters:
  - Future cleanup could otherwise keep the same bug while only renaming the helper.
- Practical implication:
  - If this path is hardened, pick and document the identity model explicitly and pin it with cross-platform tests.

## Implementation Priority

- Clarify or fix the identity model used for cross-ticker dataset dedupe before broader helper cleanup.
- Add tests that prove distinct case-sensitive dataset inputs are not accidentally dropped.

## Verification If Changed

- cargo test -p ta_desktop_demo_app validation_common -- --nocapture
- representative cross-ticker config tests on case-variant dataset paths

## Regression And Feature-Preservation Constraints

- Preserve current gate semantics and JSON canonicalization behavior unless coordinated consumers move together.
- Do not describe lexical path lowering as true filesystem identity.
- Keep cross-ticker dedupe explicit and test-backed if the policy is changed.

## Third-Pass Conclusion

`Bucket unchanged, but cross-platform path loss sharpened`

validation_common.rs stays in Small surgical refactor. The third pass mainly sharpens one specific bug: lowercasing dataset paths for dedupe can collapse distinct inputs on case-sensitive filesystems, so this is stronger than a generic path identity warning.
