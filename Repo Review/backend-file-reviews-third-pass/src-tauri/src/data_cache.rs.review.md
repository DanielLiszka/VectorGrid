# Third-Pass Review: `src-tauri/src/data_cache.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/data_cache.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/data_cache.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/data_cache.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a persistence or path-handling surface whose behavior is mostly lexical rather than canonical still looked like the right framing under fresh research.

## Third-Pass Research Log

1. Rust fs::write semantics
   - Source: <https://doc.rust-lang.org/std/fs/fn.write.html>
   - Why consulted in third pass:
     - reconfirm what the standard helper actually guarantees for whole-file rewrites
   - Third-pass constraint informed:
     - plain overwrite helpers should not be mistaken for crash-safe persistence

2. Rust Path semantics
   - Source: <https://doc.rust-lang.org/std/path/struct.Path.html>
   - Why consulted in third pass:
     - recheck how much lexical normalization can really say about filesystem identity
   - Third-pass constraint informed:
     - path cleanup is not equivalent to true containment, identity, or cross-platform sameness

3. Rust canonicalize semantics
   - Source: <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
   - Why consulted in third pass:
     - ground the third-pass guidance around identity-sensitive path handling
   - Third-pass constraint informed:
     - future hardening should separate lexical cleanup from filesystem-backed identity checks

## What Third Pass Sharpened

### 1. The second-pass bucket still holds; the sharper issue is lexical policy versus real filesystem identity
- Why this matters:
  - This file still reads as compatibility-sensitive persistence glue rather than a candidate for broad redesign.
  - The second-pass call of `Bucket unchanged` still fits.
- Practical implication:
  - Keep the current bucket and prefer targeted hardening around overwrite paths and identity-sensitive path handling.

### 2. Future work should start with atomicity or clearer identity boundaries, not structural churn
- Why this matters:
  - The remaining risk is usually durability semantics or lexical path treatment, not missing abstraction layers.
- Practical implication:
  - If this area changes, make the first step tighter rewrite semantics or clearer path-key documentation/tests.

## Implementation Priority

- Harden overwrite semantics or identity-sensitive path handling before any broader refactor.
- Add regression tests that pin current normalization and persistence behavior.

## Verification If Changed

- cargo test -p ta_desktop_demo_app data_cache -- --nocapture
- manual desktop smoke around persistence, reload, and path-driven hydration for this surface

## Regression And Feature-Preservation Constraints

- Preserve current settings/state DTO shapes unless all consumers move together.
- Do not present lexical normalization as true filesystem identity or containment.
- Avoid changing persistence timing or overwrite behavior without explicit compatibility review.

## Third-Pass Conclusion

`Bucket unchanged`

data_cache.rs stays in `Small surgical refactor`. The third pass mainly sharpens the difference between lexical path cleanup and real persistence/identity guarantees rather than justifying a larger redesign.
