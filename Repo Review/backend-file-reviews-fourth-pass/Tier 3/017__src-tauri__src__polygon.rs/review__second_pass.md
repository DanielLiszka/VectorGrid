# Second-Pass Review: `src-tauri/src/polygon.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/polygon.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/polygon.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a provider-integration and cache-policy boundary whose offline behavior, gap handling, and manifest replacement semantics should only change carefully.

## Stale Check

The live source still owns:

- Polygon/Massive credential and request handling
- cache-plan decisions
- same-day gap handling and manifest replacement

The first-pass packet remains current.

## Second-Pass Research Log

1. Massive custom-bars documentation
   - Source: <https://massive.com/docs/stocks/get_v2_aggs_ticker__stocksTicker__range__multiplier___timespan___from___to>
   - Why consulted in second pass:
     - verify how to interpret missing aggregate intervals
   - Additional constraint informed:
     - empty intervals can be legitimate "no eligible trades" gaps

2. Reqwest blocking module
   - Source: <https://docs.rs/reqwest/latest/reqwest/blocking/index.html>
   - Why consulted in second pass:
     - reconfirm the blocking-client posture
   - Additional constraint informed:
     - current bounded blocking use remains appropriate here

3. Keyring crate documentation
   - Source: <https://docs.rs/crate/keyring/latest>
   - Why consulted in second pass:
     - reassess credential-store fallback behavior
   - Additional constraint informed:
     - the current cache-only API-key requirement remains an offline-usability bug

4. Rust `std::fs::rename`
   - Source: <https://doc.rust-lang.org/std/fs/fn.rename.html>
   - Why consulted in second pass:
     - verify manifest replacement semantics
   - Additional constraint informed:
     - current manifest replacement still improves shape, not full durability

## Delta vs First Pass

### 1. Cache planning still trusts manifest min/max metadata plus file existence more than the file contents themselves

- Why this matters:
  - external modification can skew `FullHit` / `AppendOnly` decisions
- Practical implication:
  - manifest trust remains a bounded correctness risk if this area is touched

### 2. Same-day gap repair must remain best-effort because empty intervals are sometimes legitimate market data

- Why this matters:
  - tightening every gap into corruption detection would misread provider semantics
- Practical implication:
  - preserve current tolerance posture unless the upstream contract changes

## Verification If Changed

- existing polygon direct tests
- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_options_data -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve offline reuse of already-downloaded data
- Preserve best-effort credential fallback
- Preserve best-effort same-day gap handling unless upstream semantics change

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly strengthens two points: cache planning still trusts manifest metadata heavily, and missing provider intervals are not automatically data corruption.
