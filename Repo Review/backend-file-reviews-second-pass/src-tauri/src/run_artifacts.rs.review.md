# Second-Pass Review: `src-tauri/src/run_artifacts.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/run_artifacts.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/run_artifacts.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as the main artifact policy owner and kept the action bucket small because backward compatibility and path sandboxing dominate the risk profile.

## Stale Check

The live source still owns:

- artifact scanning, hydration, and detail loading
- latest-run selection
- persisted JSON rewrite helpers

The first-pass packet remains current.

## Second-Pass Research Log

1. Rust `std::fs::rename`
   - Source: <https://doc.rust-lang.org/std/fs/fn.rename.html>
   - Why consulted in second pass:
     - re-check replacement semantics for persisted artifact rewrites
   - Additional constraint informed:
     - direct rewrite paths still lack a more durable replace helper

2. Rust `std::fs::read_to_string`
   - Source: <https://doc.rust-lang.org/std/fs/fn.read_to_string.html>
   - Why consulted in second pass:
     - verify UTF-8/read failure behavior during artifact scans
   - Additional constraint informed:
     - unreadable or non-UTF-8 files still disappear as omission rather than diagnosis

3. Rust `std::fs::File`
   - Source: <https://doc.rust-lang.org/std/fs/struct.File.html>
   - Why consulted in second pass:
     - separate replacement shape from durability guarantees
   - Additional constraint informed:
     - explicit durability still is not guaranteed by the current write helpers

4. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - confirm that these omissions and fallbacks surface directly through desktop command paths
   - Additional constraint informed:
     - silent hydration fallback is public behavior, not just internal cleanup detail

## Delta vs First Pass

### 1. Hydration still invents `Utc::now()` when an artifact lacks `ts`, so malformed/old artifacts remain time-unstable at read time

- Why this matters:
  - the same artifact can present different effective timestamps across reads
- Practical implication:
  - if this is corrected, it should be done as explicit compatibility policy rather than silent cleanup

### 2. Corrupt or unreadable artifact files still become omission rather than diagnosis during scans

- Why this matters:
  - the library view can hide damage rather than surfacing it
- Practical implication:
  - still a valid bounded hardening target if scan behavior is revisited

## Verification If Changed

- `cargo test -p ta_desktop_demo_app run_artifacts -- --nocapture`
- focused overwrite/latest/stage5 alias tests
- workflow-artifact and history frontend tests

## Regression And Feature-Preservation Constraints

- Preserve backward compatibility and path sandboxing
- Preserve latest-run selection semantics unless migration is intentional
- Do not narrow tolerated historical artifact coverage casually

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens that timestamp invention and silent omission of unreadable files are still real compatibility and diagnostics gaps in the main artifact owner.
