# Third-Pass Review: `src-tauri/src/commands_data.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_data.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_data.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_data.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Third Pass

The second pass already identified synchronous command-boundary cleanup and top-off tracking parity, but this file remained high ROI because it owns user-facing filesystem mutation and market-data root migration behavior.

## Third-Pass Research Log

1. Rust `canonicalize`
   - Source: <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
   - Why consulted in third pass:
     - check whether the current nested-root guard is resistant to alias paths and symlink-like surfaces
   - Third-pass constraint informed:
     - lexical absolute-path comparison is weaker than canonical containment comparison

2. Rust `Path`
   - Source: <https://doc.rust-lang.org/std/path/struct.Path.html>
   - Why consulted in third pass:
     - confirm what `starts_with` actually guarantees for raw paths
   - Third-pass constraint informed:
     - `starts_with` on raw persisted paths is not a substitute for canonical containment on the live filesystem

3. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in third pass:
     - keep checking whether the background Polygon flows depend on cooperative cancellation rather than wrapper abort
   - Third-pass constraint informed:
     - cooperative polling remains the real cancellation boundary for top-off/download work

## What Third Pass Sharpened

### 1. The market-data root migration guard is still too lexical for a path-sensitive move operation

- Evidence:
  - `set_market_data_dir_for_base` compares raw absolute paths
  - the persisted setting retains raw configured strings
  - nested-path checks rely on lexical `starts_with` behavior
- Why this matters:
  - alias paths, junctions/symlinks, and Windows case variants can bypass the "move into itself" style protections before directory movement begins
- Practical implication:
  - the highest-ROI third-pass hardening is canonicalized comparison for containment/migration checks only
  - keep persisted and displayed user paths in their current form

### 2. The second-pass top-off parity finding still stands, but the path-sensitive migration bug is the better immediate target

- Evidence:
  - `polygon_top_off_dataset` still shares the same background-task and cancel mechanics
  - frontend tracking parity is still weaker than the main download flow
- Why this matters:
  - this is still a contract issue, but it is less severe than incorrectly reasoning about where a market-data move can land
- Practical implication:
  - if only one small patch lands first, canonicalized migration checks should beat UI parity cleanup

### 3. The synchronous command-boundary concern remains valid

- Evidence:
  - `load_price_data`, `resample_price_data`, and `set_market_data_dir` still perform nontrivial state/filesystem work on synchronous command paths
- Practical implication:
  - keep this in scope for future cleanup, but the third pass does not justify inflating the bucket beyond surgical refactor

## Implementation Priority

- Highest-ROI hardening:
  - strengthen market-root comparison and nesting checks against canonical filesystem identity
- Secondary cleanup:
  - bring `polygon_top_off_dataset` task visibility/cancellation semantics into clearer parity with the main Polygon download flow

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
- `cargo test -p ta_desktop_demo_app --test settings_tests -- --nocapture`
- `npm --prefix web run test -- src/services/dataService.test.ts src/stores/taskStore.test.ts`

Also verify manually:

- market-data root migration still works across volumes
- canonicalized comparison does not rewrite displayed settings paths
- top-off cancellation and task visibility remain understandable in the desktop UI

## Regression And Feature-Preservation Constraints

- Preserve current absolute-path UX for market-data roots
- Use canonicalization for comparison/containment only, not as the persisted display path
- Preserve cross-volume move fallback behavior
- Preserve cooperative cancellation in the background Polygon helpers

## Third-Pass Conclusion

`Bucket unchanged, but the migration check is now the sharper bug`

The second-pass packet was right to keep this file in `Small surgical refactor`, but the third pass shows that the highest-ROI issue is no longer only synchronous command cleanup or top-off parity. The more concrete hardening target is the still-lexical market-data root migration guard.
