# Third-Pass Review: `src-tauri/src/commands_validation.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_validation.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_validation.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_validation.rs.review.md`
- Second-pass action bucket: `Larger refactor but defer`
- Third-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got An Expanded Third Pass

This file sits exactly on the wrapper/core boundary for validation. The second-pass packet was directionally right, but it still treated some wrapper-owned semantics as if they were merely orchestration details.

## Third-Pass Research Log

1. Tauri calling Rust
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in third pass:
     - re-check whether this wrapper should be treated as the semantic owner of its IPC contract
   - Third-pass constraint informed:
     - silent aliasing or field reinterpretation here is public compatibility behavior

2. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in third pass:
     - re-evaluate how much of the returned validation surface belongs to wrapper-owned blocking work
   - Third-pass constraint informed:
     - wrapper-side post-processing materially affects result semantics, not only latency

3. Rust API Guidelines dependability guidance
   - Source: <https://rust-lang.github.io/api-guidelines/dependability.html>
   - Why consulted in third pass:
     - judge whether exposing wrapper-owned fields through reused app-core DTOs should count as contract debt
   - Third-pass constraint informed:
     - public APIs should not silently accept or reinterpret semantically meaningful inputs

## What Third Pass Sharpened

### 1. The IPC contract still embeds wrapper-owned semantics in reused app-core DTOs

- Evidence:
  - `validation_context` is required and enforced here before the app-core call
  - `cross_ticker` is stripped before the app-core call and reapplied later in the wrapper
  - `runtime_ms` is overwritten after wrapper-owned post-processing completes
- Why this matters:
  - this file is doing more than "orchestration"
  - it is the real semantic owner of fields that still appear to belong to the app-core request/result types
- Practical implication:
  - the deferred redesign should eventually split wrapper IPC DTOs from direct app-core DTO reuse

### 2. `normalize_strategy_kind()` is riskier than the second pass emphasized because it participates in validation-context gating

- Evidence:
  - unknown strategy kinds still normalize to `DoubleMa`
  - the normalized value is used to decide whether a validation context is compatible with the command
- Why this matters:
  - future strategy additions can be silently accepted or misrouted at the compatibility boundary
- Practical implication:
  - this is a real future-hardening point, but it still belongs in the larger deferred bucket rather than isolated cleanup

### 3. Preserved-finalist ordering still matters, but it is now clearly secondary to the wrapper/core contract problem

- Practical implication:
  - keep the earlier ordering-warning documented
  - prioritize DTO ownership and field-semantics clarity first

## Implementation Priority

- Highest-value redesign target:
  - stop presenting wrapper-owned `validation_context`, `cross_ticker`, and `runtime_ms` semantics as if they were native app-core DTO behavior
- Safe-now work remains limited:
  - documentation and additive clarification, not casual structural cleanup

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_validation -- --nocapture`
- frontend validation service/store tests
- targeted wrapper tests for `validation_context`, `cross_ticker`, and `runtime_ms` semantics

## Regression And Feature-Preservation Constraints

- Preserve strict `validation_context` behavior at the Tauri boundary unless callers migrate together
- Preserve wrapper-owned cross-ticker application and result recomputation behavior
- Do not claim hard cancellation semantics the blocking path does not actually provide
- Do not silently change strategy-kind alias behavior without auditing existing artifacts and requests

## Third-Pass Conclusion

`Bucket unchanged, but the wrapper owns more semantics than second pass stated`

The file still belongs in `Larger refactor but defer`, but the third pass makes the core issue clearer. The problem is not only preserved-finalist order or alias cleanup; it is that this IPC layer still owns real validation semantics while reusing app-core DTO shapes that imply otherwise.
