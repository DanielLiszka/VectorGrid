# Second-Pass Review: `crates/app_core/src/timeframe.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/timeframe.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/timeframe.rs.review.md`
- First-pass action bucket: `Document only`
- Second-pass action bucket: `Document only`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated this file as small policy code where session anchoring and label normalization need clear documentation more than code changes.

## Stale Check

The live source still:

- anchors buckets to the first bar of each session/day
- canonicalizes labels like `120s -> 2m`
- preserves absent optional series as absent/`NaN`

The first-pass packet remains current.

## Second-Pass Research Log

1. pandas `DataFrame.resample`
   - Source: <https://pandas.pydata.org/docs/reference/api/pandas.DataFrame.resample.html>
   - Why consulted in second pass:
     - contrast common wall-clock/origin semantics with this session-anchored implementation
   - Additional constraint informed:
     - current anchoring is a deliberate trading-session choice, not a generic wall-clock resample default

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why consulted in second pass:
     - ground behavior-heavy helper docs
   - Additional constraint informed:
     - the anchoring rule remains the main contract to document

3. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in second pass:
     - reassess strict validation of labels and sorted timestamps
   - Additional constraint informed:
     - current strictness remains appropriate for a core timeframe helper

## Delta vs First Pass

### 1. The anchoring difference from common resample defaults is now more concrete

- Why this matters:
  - users familiar with wall-clock resampling may otherwise misread the current behavior
- Practical implication:
  - keep this file in `Document only`

## Verification If Changed

- `cargo test -p ta_app_core timeframe::tests:: -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve first-bar-of-day anchoring
- Preserve strict sorted-timestamp checks
- Preserve absent-field-as-`NaN` behavior

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Document only` bucket remains correct. The second pass mainly clarifies that the session-anchored behavior intentionally differs from common wall-clock resampling defaults.
