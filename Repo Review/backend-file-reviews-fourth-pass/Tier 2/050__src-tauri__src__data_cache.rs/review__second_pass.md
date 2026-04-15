# Second-Pass Review: `src-tauri/src/data_cache.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/data_cache.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/data_cache.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a performance-sensitive but mostly sound cache layer whose main risks are durability semantics and ambiguous existence/error handling.

## Stale Check

The live source still has the same main characteristics:

- temp-file plus rename replacement shape
- permissive corrupt-cache fallback
- `MaybeUninit`-based fast-path loading

The first-pass packet remains current.

## Second-Pass Research Log

1. `MaybeUninit`
   - Source: <https://doc.rust-lang.org/core/mem/union.MaybeUninit.html>
   - Why consulted in second pass:
     - verify the unsafe slab-read assumptions around the fast path
   - Additional constraint informed:
     - the current approach remains acceptable only because the loader fully initializes before use

2. Rust `Path::try_exists` and `std::fs::exists`
   - Sources:
     - <https://doc.rust-lang.org/std/path/struct.Path.html>
     - <https://doc.rust-lang.org/std/fs/fn.exists.html>
   - Why consulted in second pass:
     - refine the `exists()` ambiguity review
   - Additional constraint informed:
     - the current existence pre-checks are still ambiguous and unnecessary

3. Rust `std::fs::rename` and `std::fs::File`
   - Sources:
     - <https://doc.rust-lang.org/std/fs/fn.rename.html>
     - <https://doc.rust-lang.org/std/fs/struct.File.html>
   - Why consulted in second pass:
     - separate replacement shape from crash-durability guarantees
   - Additional constraint informed:
     - the current flush-plus-rename path improves replacement shape but still does not fsync for durability

## Delta vs First Pass

### 1. The temp-file path still improves replacement shape but not crash durability

- Why this matters:
  - flush on the writer is not the same as persisting file and directory metadata
- Practical implication:
  - if durability is revisited, do not overstate what the current rename path already guarantees

### 2. `min_ts` / `max_ts` remain a missed cheap sanity-check opportunity on load

- Why this matters:
  - they can help reject obviously mismatched or stale cache contents earlier
- Practical implication:
  - still a bounded cleanup target if this layer is touched

## Verification If Changed

- `cargo test -p ta_desktop_demo_app data_cache -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
- `cargo test -p ta_desktop_demo_app state_ -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve permissive corrupt-cache fallback
- Preserve the `MaybeUninit` fast path unless measured otherwise
- Preserve current cache-format compatibility unless a migration is planned

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens the durability reading: temp-write-plus-rename improves replacement shape, but it still does not provide full crash-durability guarantees.
