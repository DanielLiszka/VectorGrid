# Second-Pass Review: `src-tauri/src/state.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/state.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/state.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as high-value shared runtime state whose main risks are identity, normalization, and observability semantics rather than broad architecture.

## Stale Check

The live source still owns:

- loaded/resampled dataset identity
- shared run/background task state
- live-bar upsert and reuse logic

The first-pass packet remains current.

## Second-Pass Research Log

1. `Arc::make_mut`
   - Source: <https://doc.rust-lang.org/std/sync/struct.Arc.html>
   - Why consulted in second pass:
     - confirm clone-on-write mutation semantics in the shared state paths
   - Additional constraint informed:
     - current copy-on-write behavior remains conservative and correct

2. Rust `canonicalize`
   - Source: <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
   - Why consulted in second pass:
     - revisit path-identity normalization
   - Additional constraint informed:
     - equivalent path spellings still miss reuse until identity is normalized more strongly

3. Atomic `Ordering`
   - Source: <https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html>
   - Why consulted in second pass:
     - sanity-check current conservative ordering choices
   - Additional constraint informed:
     - no new evidence suggested weakening current atomic discipline

4. Tauri state-management guidance
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why consulted in second pass:
     - ground the shared-state boundary review
   - Additional constraint informed:
     - this file remains the canonical owner of single-run exclusivity

## Delta vs First Pass

### 1. The pre-1980 timestamp bug also threatens inferred timeframe labels, not just file-format compatibility

- Why this matters:
  - normalization occurs before later label inference and cache reuse decisions
- Practical implication:
  - this remains a small but more widely visible correctness issue than the first pass emphasized

### 2. `upsert_live_bar()` still has three real outcomes but only exposes success/failure

- Why this matters:
  - deliberate stale-bar drops are hidden from higher layers
- Practical implication:
  - observability here remains a bounded improvement target

## Verification If Changed

- `cargo test -p ta_desktop_demo_app state_ -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_execution -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve load/resample invalidation
- Preserve exact-timestamp overwrite behavior
- Preserve single-run exclusivity
- Do not weaken conservative atomic semantics without evidence

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly sharpens the visibility of two small correctness gaps: the pre-1980 timestamp path affects more than storage, and stale live-bar drops are still not first-class outcomes at the API surface.
