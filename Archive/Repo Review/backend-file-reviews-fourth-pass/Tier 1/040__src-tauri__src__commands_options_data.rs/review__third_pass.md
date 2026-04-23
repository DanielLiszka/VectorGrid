# Third-Pass Review: `src-tauri/src/commands_options_data.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_options_data.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_options_data.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_options_data.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Full Third Pass

The second-pass packet already called this as `Bucket unchanged, but retry parsing and read-only semantics need sharper treatment`. The earlier targeted third pass stopped short of this file, so the expanded full-corpus pass reopened the live source to verify whether a dataset import and read surface with both path and persistence semantics still looked like the right framing under fresh research. The third pass tightened the packet around two concrete behaviors the second pass underweighted: read-only commands still create store directories, and partial import directories can block legitimate retries.

## Third-Pass Research Log

1. Tauri command boundary guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in third pass:
     - reconfirm that dataset import, list, and load commands are frontend-invokable product behavior
   - Third-pass constraint informed:
     - read-only versus mutating semantics need to be honest at the command boundary

2. Rust Path semantics
   - Source: <https://doc.rust-lang.org/std/path/struct.Path.html>
   - Why consulted in third pass:
     - ground the directory-existence and collision analysis
   - Third-pass constraint informed:
     - leftover directories are not enough to prove a valid dataset already exists

3. Tokio blocking-task guidance
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in third pass:
     - recheck how much work the current command wrappers do inside blocking tasks
   - Third-pass constraint informed:
     - longer-running import flows should stay explicit about side effects and retry semantics

## What Third Pass Sharpened

### 1. The read-only command surface is not actually read-only
- Evidence:
  - options_datasets_dir eagerly creates the dataset root.
  - list_options_datasets and load_options_dataset_manifest call into that helper before scanning or loading.
- Why this matters:
  - That means seemingly read-only commands can mutate the local store simply by being invoked.
- Practical implication:
  - Keep the existing bucket, but document and test this side effect explicitly or split the helper if true read-only semantics are needed.

### 2. Crash-leaked partial directories can become false collisions
- Evidence:
  - Import and Polygon-download paths reject any pre-existing dataset directory before proving a valid manifest is present.
- Why this matters:
  - A failed or interrupted import can leave behind a directory that blocks later retries even though no usable dataset exists.
- Practical implication:
  - Treat collision detection as manifest-backed dataset exists rather than directory exists if this area is hardened.

## Implementation Priority

- First hardening step: make retry and collision checks manifest-aware rather than directory-only.
- Second hardening step: separate true read-only listing and loading from directory-creating helpers if callers depend on read-only semantics.

## Verification If Changed

- cargo test -p ta_desktop_demo_app commands_options_data -- --nocapture
- manual desktop smoke covering failed import retry and read-only dataset listing and load flows

## Regression And Feature-Preservation Constraints

- Preserve current dataset manifest schema and store layout.
- Do not silently change read-only command behavior without frontend coordination.
- Keep import retry behavior explicit and test-backed if collision semantics are tightened.

## Third-Pass Conclusion

`Bucket unchanged, but retry and read-only semantics sharpened`

commands_options_data.rs stays in Small surgical refactor. The third pass mainly sharpens two concrete issues: read-only dataset commands still create the store root, and leftover partial directories can block later retries even when no valid dataset manifest exists.
