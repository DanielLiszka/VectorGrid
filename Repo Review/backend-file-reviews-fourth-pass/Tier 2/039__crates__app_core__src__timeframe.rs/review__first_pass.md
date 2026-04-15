# Review Packet: `crates/app_core/src/timeframe.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/timeframe.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful utility module, but not fully review-ready because anchoring semantics, optional-field propagation, and back-compat label rules are only implicit`
- Recommended action bucket: `Document only`
- Sensitivity tags: `Time-series`, `Data-shaping`, `Semantics-sensitive`, `Low-compute`

## What This File Owns

`timeframe.rs` owns two related helpers:

- timeframe-label parsing:
  - `parse_timeframe_label`: `crates/app_core/src/timeframe.rs:27-85`
- candle resampling:
  - `resample_candles`: `88-314`
- file-local tests:
  - `317-454`

## Classification

- Public surface: yes
  - shared utility behavior likely to be reused across backend request/data shaping
- Hot path: no
  - setup/data-transformation work, not inner-loop optimization compute
- CUDA/runtime-critical: no
- Semantics-sensitive: yes
  - bar anchoring and field propagation affect downstream analytics meaningfully

## Direct Interactions

### Backend interaction map

- this module is a shared utility layer rather than a strategy-local implementation
- downstream consumers depend on:
  - accepted timeframe labels
  - resampled OHLCV semantics
  - handling of missing optional candle fields

### Downstream verification impact if changed

- changing label parsing changes accepted request inputs
- changing bucket anchoring changes resampled timestamps and all downstream metric series
- changing optional-field handling changes whether consumers see `NaN` columns or apparently populated fields

## Standards And Research Log

This packet was written after targeted web research focused on time-series resampling semantics and documentation expectations for behavior-heavy helpers.

### Sources consulted

1. `pandas.DataFrame.resample`
   - Source: <https://pandas.pydata.org/docs/reference/api/pandas.DataFrame.resample.html>
   - Why it was consulted:
     - this file implements session-aware resampling semantics that differ from common wall-clock assumptions
   - Standard informed:
     - bucket anchoring and aggregation policy should be documented explicitly because users often assume a default calendar alignment

2. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the helpers encode several behaviorally important decisions not obvious from signatures
   - Standard informed:
     - data-shaping helpers should document defaulting, accepted inputs, and error conditions

3. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why it was consulted:
     - this module validates input labels, sorted timestamps, vector length consistency, and finite close values
   - Standard informed:
     - strict early validation is appropriate for data-shaping utilities

## Findings

## Semantics And Contract Clarity

### 1. Session anchoring is one of the file’s most important behaviors, but it is only discoverable from the implementation and tests

- Evidence:
  - resampling resets bucket origin when the day changes and anchors buckets to the first bar of that day: `120-181`
  - the test suite explicitly locks this in via `resample_anchors_to_first_bar_of_day`: `354-399`
- Why this matters:
  - many readers will assume buckets anchor to clock boundaries or midnight
  - the current behavior is sensible for trading sessions, but it is surprising unless documented
- Smallest justified fix:
  - add rustdoc stating that resampling is session-origin anchored per day, not global-clock anchored

## Input Compatibility

### 2. Back-compat support for second-based labels is useful, but under-documented

- Evidence:
  - `parse_timeframe_label` accepts `"120s"` and canonicalizes known values like `120s -> 2m`: `27-46`
  - the test suite explicitly locks this in: `320-336`
- Why this matters:
  - this is a contract decision, not just a parser convenience
  - callers need to know whether raw-second labels are legacy-only, fully supported, or subject to future removal
- Smallest justified fix:
  - document accepted suffixes and the canonicalization rule

## Data-Field Semantics

### 3. Optional OHLCV field propagation is correct but easy to misread without docs

- Evidence:
  - open/high/low/volume are tracked only when the source flags indicate presence: `201-263`
  - missing optional fields remain `NaN` and output flags remain false: `289-314`
  - the test suite locks this in: `436-454`
- Why this matters:
  - without documentation, a maintainer could “clean up” the code by filling synthetic values for absent fields
  - that would silently change consumer expectations
- Smallest justified fix:
  - add rustdoc summarizing:
    - close is required
    - open uses first finite value in bucket
    - high/low aggregate finite extrema
    - volume sums finite values
    - absent optional fields remain absent/`NaN`

## Validation

### 4. Validation is appropriately strict and should be preserved

- Evidence:
  - rejects empty labels and invalid suffixes: `27-85`
  - rejects empty candles, inconsistent lengths, unsorted timestamps, non-positive intervals, and non-finite close values: `88-263`
- Why this matters:
  - these failures protect downstream analytics from subtly bad data
- Safe-now conclusion:
  - keep the strict validation posture

## Code-Quality Improvement Register

- Safe now:
  - document session anchoring
  - document second-label canonicalization
  - document optional-field aggregation/propagation rules
- Defer:
  - changing anchoring policy
  - broad parser expansion beyond current suffixes

## Technical-Debt Reduction Register

- Safe now:
  - rustdoc only
- Defer:
  - broader timeframe abstraction work
- Do not casually remove:
  - current session-origin anchoring
  - current strict sorted-timestamp requirement
  - current missing-field-as-`NaN` semantics

## Performance Opportunity Register

- Safe structural work:
  - none needed
- Benchmark-sensitive:
  - none

## Recommended Action

`Document only`

The implementation and tests are already solid. The real improvement is to write down the resampling contract so future “cleanup” edits do not accidentally change trading-session semantics.

## Exact Local Verification Required If Changed

1. File-local tests:
   - `cargo test -p ta_app_core timeframe::tests:: -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core timeframe::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not re-anchor buckets to wall-clock boundaries without an explicit product decision
- Do not synthesize optional OHLCV fields when the source data marks them absent
- Do not drop support for second-based labels casually; request compatibility may depend on it
