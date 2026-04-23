# Third-Pass Review: `crates/app_core/src/double_ma_validate.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/double_ma_validate.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_validate.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_validate.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got An Expanded Third Pass

The second-pass packet correctly saw layer-split semantics, but it still understated how much of the public request surface this app-core file does not truly own.

## Third-Pass Research Log

1. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - reassess whether this file's public request fields are stronger than the app-core layer actually honors
   - Third-pass constraint informed:
     - public APIs should not silently accept semantically meaningful inputs they do not truly own

2. ARCH SPA reference
   - Source: <https://bashtage.github.io/arch/doc/multiple-comparison/generated/arch.bootstrap.SPA.html>
   - Why consulted in third pass:
     - sharpen how much inherited SPA ambiguity matters at the validation-owner layer
   - Third-pass constraint informed:
     - nullable SPA state is too weak a contract when the caller thinks SPA was requested

## What Third Pass Sharpened

### 1. `cross_ticker` and `validation_context` are still public request fields that the core layer does not actually own

- Evidence:
  - the core request type still accepts both fields
  - the wrapper strips/reapplies `cross_ticker` later
  - the wrapper enforces `validation_context` before the core call
  - wrapper logic also overwrites `runtime_ms` after its post-pass work
- Why this matters:
  - this is stronger than "split semantics"
  - the app-core API still embeds wrapper-owned behavior and silently ignores part of its visible request contract
- Practical implication:
  - this remains larger redesign work, not a safe local patch

### 2. This file also inherits the stronger SPA ambiguity from `validation_pipeline.rs`

- Evidence:
  - the run still forwards SPA calculation and builds its verdict from nullable SPA output
- Why this matters:
  - `spa: None` can still blur:
    - not requested
    - requested but skipped
    - requested but aborted/invalid
- Practical implication:
  - any future cleanup should keep the file aligned with the validation-pipeline contract redesign rather than patching SPA semantics here in isolation

## Implementation Priority

- Highest-value redesign target:
  - stop exposing wrapper-owned behavior as if it were natively owned by the app-core validation API
- Keep aligned:
  - whatever fix lands for SPA contract clarity in `validation_pipeline.rs`

## Verification If Changed

- `cargo test -p ta_app_core double_ma_validation -- --nocapture`
- direct tests for app-core `cross_ticker` handling
- direct tests for `validation_context` semantics
- direct tests for SPA-requested versus SPA-skipped behavior

## Regression And Feature-Preservation Constraints

- Preserve current headless/core callers that intentionally omit `validation_context`
- Preserve wrapper-owned cross-ticker post-processing and run-verdict recomputation
- Do not silently turn `cross_ticker` into a hard error for wrapper paths without coordinated changes
- Keep candidate-level MA-param override behavior intact

## Third-Pass Conclusion

`Bucket unchanged, but the app-core API surface is weaker than second pass stated`

`double_ma_validate.rs` still belongs in `Larger refactor but defer`. The third pass makes the real problem more explicit: the app-core layer still exposes wrapper-owned `cross_ticker`, `validation_context`, and timing semantics as if it owned them directly, and it also inherits the now-sharper SPA ambiguity from the validation pipeline.
