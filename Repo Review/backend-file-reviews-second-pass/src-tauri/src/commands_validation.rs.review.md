# Second-Pass Review: `src-tauri/src/commands_validation.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_validation.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_validation.rs.review.md`
- First-pass action bucket: `Larger refactor but defer`
- Second-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a large, contract-heavy validation orchestrator and deferred broader cleanup because preserved-finalist semantics, cross-ticker behavior, and shared progress wiring are all product-sensitive.

## Stale Check

The live source still owns:

- shared validation run orchestration
- cross-ticker request shaping
- preserved-finalist selection and progress/event plumbing

The first-pass packet remains current.

## Second-Pass Research Log

1. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in second pass:
     - sharpen cancellation expectations around the blocking validation work
   - Additional constraint informed:
     - cancel remains cooperative only; extra work inside the blocking closure directly worsens cancel latency

2. Tauri state-management guidance
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why consulted in second pass:
     - ground the shared-state boundary review for validation orchestration
   - Additional constraint informed:
     - shared run ownership remains the right conceptual boundary even if the file is too large

3. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - confirm how strongly to treat request normalization and result shaping as IPC contract
   - Additional constraint informed:
     - silent aliasing decisions here are public compatibility behavior

## Delta vs First Pass

### 1. `select_cross_ticker_preserved_finalists()` still makes upstream candidate order part of validation semantics

- Why this matters:
  - truncation happens in incoming order rather than via a newly declared ranking rule
- Practical implication:
  - any attempt to "clean up" finalist handling is semantics-sensitive, not generic refactor fodder

### 2. `normalize_strategy_kind()` still silently maps unknown kinds to `DoubleMa`

- Why this matters:
  - that is tolerable for current aliases, but risky as more strategy kinds are added
- Practical implication:
  - this remains a real future-hardening point, but still fits better in deferred redesign than piecemeal cleanup

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_validation -- --nocapture`
- frontend validation service and store tests

## Regression And Feature-Preservation Constraints

- Preserve `validation_context` strictness
- Preserve preserved-finalist semantics and cross-ticker request stripping
- Do not claim hard cancellation semantics the blocking path does not actually provide

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Larger refactor but defer` bucket remains correct. The second pass mainly sharpens that preserved-finalist order and silent strategy-kind fallback are both real compatibility behaviors, which makes casual cleanup riskier rather than safer.
