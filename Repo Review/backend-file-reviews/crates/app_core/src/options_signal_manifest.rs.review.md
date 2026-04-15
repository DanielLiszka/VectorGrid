# Review Packet: `crates/app_core/src/options_signal_manifest.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/options_signal_manifest.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Useful and small, but not fully public-review ready because out-of-range trade indices are silently clamped instead of being reported, session/calendar metadata is hardcoded to a UTC market-hours assumption, and the shared helper still emits a double-ma-specific error message even when called through non-double-ma builders`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `Artifact-contract`, `Options-workflow`, `Semantics-sensitive`, `Low-runtime-risk`

## What This File Owns

`options_signal_manifest.rs` owns construction of `OptionsSignalEventManifestV1` artifacts for the current supported “double-like” parent strategy lanes. It translates a parent artifact plus candle/trade curves into the signal-event manifest later consumed by the options-expression workflow.

It owns:

- local helpers:
  - `clamp_index`: `crates/app_core/src/options_signal_manifest.rs:9-11`
  - `infer_side`: `13-21`
  - `build_event_id`: `23-25`
  - `session_id_from_timestamp`: `27-30`
- shared manifest builder:
  - `build_options_signal_manifest_from_double_like_curves`: `33-138`
- public builders:
  - `build_options_signal_manifest_from_double_ma`: `140-169`
  - `build_options_signal_manifest_from_composite_trend`: `171-200`
  - `build_options_signal_manifest_from_regime_hybrid`: `202-231`
  - `build_options_signal_manifest_from_breakout_momentum`: `233-262`
- file-local test:
  - `264-326`

This file does not own:

- the `OptionsSignalEventManifestV1` type definition itself:
  - `crates/app_core/src/options_types.rs:433-449`
- parent-artifact selection and replay-cache orchestration:
  - `src-tauri/src/commands_options_expression.rs`
- downstream split assignment and options runtime evaluation:
  - `crates/app_core/src/options_expression.rs`

## Classification

- Public surface: yes
  - the module is public at `crates/app_core/src/lib.rs:51`
  - the public builder functions are re-exported via the module
- Hot path: no
  - this is artifact/materialization support, not runtime ranking
- CUDA/runtime-critical: no
- Strategy-local: partially
  - it is specific to the options-expression parent-signal bridge for currently supported double-like strategy lanes
- Low-risk support code: not entirely
  - it is small support code, but it creates a persisted contract that later drives cache keys, replay, and split semantics

## Direct Interactions

### Backend interaction map

- manifest type contract:
  - `crates/app_core/src/options_types.rs:433-449`
- signal-event type contract:
  - `crates/app_core/src/options_types.rs:401-431`
- Tauri-side signal-manifest materialization and replay-cache persistence:
  - `src-tauri/src/commands_options_expression.rs:1505-1586`
  - `791-839`
  - `1704`
  - `1727`
  - `1766`
- downstream manifest consumption:
  - cache hashing and split preparation in `crates/app_core/src/options_expression.rs:333-389`
  - `483-590`
  - `2504-2809`

### Downstream verification impact if changed

- changing event IDs or timestamp semantics affects:
  - signal-manifest replay caches
  - options-expression prepare-cache keys
  - tuple/event inspection row identity
- changing calendar or session semantics affects:
  - `SessionChronological` split assignment in `options_expression.rs`
  - overlap/coverage reporting that depends on event windows and sessions

## Standards And Research Log

This packet was written after targeted web research focused on public Rust documentation expectations, recoverable error design, and timestamp/session derivation behavior.

### Sources consulted

1. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - this file exports public builder functions with meaningful behavioral semantics but no rustdoc
   - Standard informed:
     - public functions should document examples and error/panic-relevant behavior

2. Rust Book, Error Handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why it was consulted:
     - the file exposes `Result<_, String>` builders for artifact creation
   - Standard informed:
     - recoverable contract failures should be explicit and intelligible to callers

3. `chrono::DateTime::from_timestamp_millis`
   - Source: <https://docs.rs/chrono/latest/chrono/struct.DateTime.html#method.from_timestamp_millis>
   - Why it was consulted:
     - session IDs are derived from timestamps through UTC `DateTime`
   - Standard informed:
     - the helper is explicitly UTC-based and returns `None` for out-of-range timestamps, which matters for session-label semantics

## Findings

## Correctness And Safety

### 1. Trade indices are silently clamped to the last candle instead of being rejected

- Evidence:
  - `clamp_index` saturates any index to `len - 1`:
    - `crates/app_core/src/options_signal_manifest.rs:9-11`
  - `entry_t` and `exit_t` both use that helper:
    - `56`
    - `62`
- Why it matters:
  - if upstream curve indices drift out of range, manifest generation will silently project the event onto the last available bar instead of reporting a bad parent-curve contract
  - that can hide upstream bugs and produce misleading event timestamps/prices
- Smallest justified fix:
  - reject out-of-range `entry_t` / `exit_t` with an explicit `Err(...)`
  - if saturation is intentionally needed for some legacy/open-trade case, document that narrow exception rather than silently applying it to all indices
- Review judgment:
  - medium correctness / artifact-integrity issue

## API And Naming

### 1. The shared helper still emits a `double_ma`-specific empty-result error for non-double-ma builders

- Evidence:
  - the shared helper returns:
    - `"double_ma drilldown produced no candidate signal events"`
    - `crates/app_core/src/options_signal_manifest.rs:118`
  - that helper is used by:
    - `build_options_signal_manifest_from_composite_trend`: `171-200`
    - `build_options_signal_manifest_from_regime_hybrid`: `202-231`
    - `build_options_signal_manifest_from_breakout_momentum`: `233-262`
- Why it matters:
  - the public wrappers are strategy-specific, but the surfaced error message currently misidentifies three of the four call paths
  - that weakens debugging clarity for caller-side replay materialization
- Smallest justified fix:
  - pass a caller label into the shared helper or use a strategy-neutral error message such as `"parent drilldown produced no candidate signal events"`
- Review judgment:
  - medium API/naming / error-clarity issue

## Error Handling

### 1. Public builders flatten all failures to `String` and provide no rustdoc contract for what can fail

- Evidence:
  - all four public builders return `Result<OptionsSignalEventManifestV1, String>`:
    - `140-169`
    - `171-200`
    - `202-231`
    - `233-262`
  - there is no rustdoc describing:
    - empty-candle failure
    - no-signal failure
    - missing candle timestamp/price failure
    - `trade_on_next_bar` semantics
- Why it matters:
  - callers at the Tauri boundary need to understand whether a failure indicates empty source data, invalid curve indices, or a legitimate “no candidate signals” outcome
- Smallest justified fix:
  - keep the outer `String` if desired
  - but document the current error cases, and consider a small internal error enum if this file grows beyond today’s size
- Review judgment:
  - medium error-handling / public-contract issue

## Module Boundaries And Technical Debt

### 1. Calendar and session semantics are currently hardcoded to UTC market hours

- Evidence:
  - session labels are derived with `DateTime::<Utc>::from_timestamp_millis(...)` and formatted as `%Y-%m-%d`:
    - `crates/app_core/src/options_signal_manifest.rs:27-30`
  - every event gets:
    - `session_id: session_id_from_timestamp(earliest_tradable_timestamp)`: `110`
    - `calendar_id: "market_hours_utc"`: `113`
  - the manifest-level `calendar_id` is also hardcoded:
    - `133`
- Why it matters:
  - this is a real semantics choice, not just incidental metadata
  - if parent artifacts later use richer calendar handling or exchange-local session semantics, this file will still stamp UTC-day session boundaries
- Smallest justified fix:
  - if current product posture is UTC-only, document it explicitly
  - otherwise thread the actual calendar/session contract into this builder instead of hardcoding it
- Review judgment:
  - medium module-boundary / semantics-sensitive issue

### 2. The four public builders are intentionally identical today, but the file does not explain that contract

- Evidence:
  - each public builder delegates directly to the same shared helper:
    - `140-262`
- Why it matters:
  - this is not wrong, but it obscures whether strategy-specific divergence is intentionally absent or simply deferred
  - future contributors may duplicate changes across wrappers without knowing whether a strategy-local split is expected later
- Smallest justified fix:
  - add a short module-level note explaining that current supported parent strategies all use the same “double-like curves -> signal manifest” projection
- Review judgment:
  - low-medium technical-debt / clarity issue

## Docs And Examples

### 1. The public builder contract is undocumented

- Evidence:
  - public builders at:
    - `crates/app_core/src/options_signal_manifest.rs:140`
    - `171`
    - `202`
    - `233`
  - missing documentation includes:
    - `trade_on_next_bar` meaning
    - index-clamping behavior
    - UTC session/calendar assumption
    - why `adjusted_underlier_price`, `regime_tag`, and `confidence_tag` are currently `None`
    - why every event starts in `OptionsEvaluationSplit::Search`
- Smallest justified fix:
  - add rustdoc to the public builders and/or a short module-level doc section
  - include one example describing decision-bar versus tradable-bar mapping

## Tests And Missing Verification

### 1. Current file-local coverage is too narrow for a persisted signal-manifest contract

- Evidence:
  - file-local verification:
    - `cargo test -p ta_app_core options_signal_manifest::tests:: -- --nocapture`
      - passed `1` test
  - the only file-local test currently pins:
    - previous-bar decision projection for `trade_on_next_bar`
    - bullish side inference
    - raw underlier price selection
    - UTC session-id formatting
    - `264-326`
- Missing tests I would want before changing this file:
  - bearish-side trade projection
  - open-trade projection with `base_exit_timestamp = None`
  - empty-candle failure
  - empty-signal failure for each wrapper family
  - out-of-range `entry_t` / `exit_t` behavior
  - genericized no-signal error text

## Performance, Allocation, And CUDA Concerns

### 1. This file is not a meaningful performance target

- Evidence:
  - work is simple event/materialization construction over already-available candles and trades:
    - `crates/app_core/src/options_signal_manifest.rs:33-138`
- Review judgment:
  - do not spend effort micro-optimizing this file before the artifact contract is clearer

## Code-Quality Improvement Register

- Safe now:
  - reject or explicitly document index clamping
  - genericize the no-signal error message
  - add rustdoc to public builders
  - document the UTC session/calendar assumption
- Coordination required:
  - threading a non-hardcoded calendar/session contract from caller-side parent artifacts
- Defer until later pass:
  - any broader re-shaping of the parent-signal manifest type

## Technical-Debt Reduction Register

- Safe to reduce now:
  - missing docs
  - misleading error text
  - missing tests
- Coordination required:
  - calendar/session contract generalization
  - any change to event ID semantics
- Useful debt that should not be “cleaned up” casually:
  - the strategy-specific wrapper functions themselves, because they make call sites explicit even though they currently share one implementation

## Performance Opportunity Register

- Safe structural improvements:
  - none urgent
- Benchmark-sensitive candidates:
  - none worth prioritizing
- Not the right place to chase performance without evidence:
  - helper-level string/format micro-optimizations

## Observability And Diagnostics Gaps

- out-of-range trade-index saturation is silent
- the UTC market-hours assumption is implicit
- the no-signal error text is strategy-misleading in three public wrappers

## Implementation Staging Guidance

- Safe now:
  - docs
  - additive tests
  - error-text cleanup
  - explicit index validation
- Coordination required:
  - calendar/session contract changes
- Defer until later pass:
  - broader manifest-type redesign

## Change Blast Radius

- Local only:
  - docs
  - additive tests
  - genericized error text
- Same subsystem:
  - index-validation or session/calendar changes affect manifest replay and options-expression preparation
- Cross-backend / frontend-backend contract:
  - manifest contract changes affect `commands_options_expression.rs` and cached replay artifacts

## Open Questions Or Assumptions

- assumption:
  - current product posture expects UTC market-hours sessionization for the options-expression workflow
- open question:
  - whether index clamping was an intentional tolerance rule or just the simplest way to avoid panics during early implementation
- open question:
  - whether strategy-specific wrappers are expected to diverge later once more parent artifact types are supported

## Recommended Action

`Small surgical refactor`

The right next pass is:

- make index behavior explicit
- fix the misleading generic error text
- document the UTC session/calendar assumption
- add basic contract tests

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core options_signal_manifest::tests:: -- --nocapture`
- if manifest replay or Tauri-side materialization is affected:
  - `cargo test -p ta_desktop_demo_app commands_options_expression::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- do not silently change event timestamps or event IDs without checking signal-manifest replay and options-expression cache behavior
- do not change the UTC session/calendar contract casually; treat it as a semantics change, not just a metadata cleanup
- do not remove the strategy-specific wrapper functions unless caller readability is preserved elsewhere
