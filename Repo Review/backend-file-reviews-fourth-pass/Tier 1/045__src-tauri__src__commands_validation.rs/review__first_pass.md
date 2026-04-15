# Review Packet: `src-tauri/src/commands_validation.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_validation.rs`
- Packet status: `Reviewed`
- Public-review readiness: `High-leverage validation command surface with substantial direct coverage, but still too large and duplicated to count as fully hardened common orchestration`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Public-surface-sensitive`, `Runtime-critical`, `Semantics-sensitive`, `Cross-ticker-sensitive`

## What This File Owns

`commands_validation.rs` owns the Tauri/backend orchestration layer for validation workflows.

This file owns:

- public Tauri command entrypoints:
  - `run_composite_validation`: `src-tauri/src/commands_validation.rs:341-428`
  - `run_double_ma_validation`: `src-tauri/src/commands_validation.rs:578-673`
  - `get_price_bars`: `src-tauri/src/commands_validation.rs:1060-1074`
- validation-context gatekeeping:
  - `validate_validation_context`: `src-tauri/src/commands_validation.rs:131-185`
- cross-ticker validation orchestration:
  - generic driver: `217-339`
  - composite path: `430-533`
  - double-MA path: `675-883`
  - preserved-finalist selector: `804-832`
  - headless double-MA helper: `884-938`
- cross-ticker request shaping:
  - composite request builder: `533-576`
  - double-MA request builder: `834-882`
- candle alignment and price-bar shaping:
  - cross-ticker horizon alignment: `62-91`
  - price-bar aggregation: `940-1058`
- a large direct test suite:
  - `1079-3624`

This file does not own:

- the underlying validation engines
  - those are delegated into `ta_app_core::*validation*_blocking_with_candles(...)`
- report/workbook export
  - handled later by report modules

## Classification

- Public surface: yes
  - commands are invoked from the frontend and registered in the Tauri command set
- Hot path: moderate
  - this file itself is orchestration, but it sits directly on heavy validation workflows
- Runtime-critical: high
  - validation workflows and cross-ticker reruns depend on it
- CUDA/runtime-critical: indirectly yes
  - backend/validation engine choices flow through it
- Strategy-local: mixed
  - shared infrastructure with strategy-specific allowances and routing lists
- Low-risk support code: no

## Direct Interactions

### Backend interaction map

- state/runtime access:
  - `crate::state::AppState`
  - `crate::state::LoadedDataContext`
- shared validation helpers:
  - `super::validation_common`
- validation engines in `ta_app_core`
  - composite and double-MA validation blocking entrypoints
  - run-verdict construction
- headless consumer:
  - `src-tauri/src/bin/ta_headless_validate.rs:677`

### Frontend contract anchors

- command allowlist:
  - `web/src/tauri/commands.ts:44-45`
  - `web/src/tauri/commands.ts:98`
- validation command selection:
  - `web/src/services/validationService.ts:144-147`
- validation context and cross-ticker UX:
  - `web/src/services/validationService.ts:888`
  - `web/src/services/validationService.ts:1413-1491`
- workflow/store tests:
  - `web/src/services/validationService.test.ts`
  - `web/src/stores/validationStore.actions.test.ts`

## Standards And Research Log

This packet was written after a targeted external review focused on Tauri command/state contracts and `spawn_blocking` usage for CPU-heavy orchestration.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file is a large public command surface
   - Standard informed:
     - command contracts, argument typing, and error behavior are part of the frontend API

2. Tauri state management
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why it was consulted:
     - commands depend on managed `AppState`
   - Standard informed:
     - state access is part of the command boundary and should be explicit and disciplined

3. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why it was consulted:
     - both validation commands hand heavy work off to `spawn_blocking`
   - Standard informed:
     - using blocking-task handoff for CPU-heavy validation work is appropriate, but those closures should stay focused on blocking orchestration rather than unbounded incidental logic

4. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this module exposes public commands and shared runtime policy
   - Standard informed:
     - clear contracts, representative tests, and explicit failure behavior matter on public orchestration surfaces

## Findings

## Correctness And Safety

### 1. The direct local Rust lane is not green: one preserved-finalists run-verdict test currently fails

- Evidence:
  - `cargo test -p ta_desktop_demo_app commands_validation -- --nocapture` produced:
    - `29 passed`
    - `1 failed`
  - failing test:
    - `cross_ticker_headless_double_ma_recomputes_run_verdict_after_filtering`
    - failure site: `src-tauri/src/commands_validation.rs:1573`
  - expected assertions after applying cross-ticker preserved-finalists filtering:
    - `res.candidates.is_empty()`
    - `res.run_verdict.approved_candidate_count == 0`
    - `!res.run_verdict.approved`
    - `1588-1619`
- Why it matters:
  - regardless of whether the code or the test is stale, this is an existing local inconsistency in a runtime-critical path
- Review judgment:
  - this is the highest-priority objective issue in the file

### 2. Cross-ticker horizon alignment has a fallback mode that is strong enough to deserve explicit documentation and direct tests

- Evidence:
  - overlap-based alignment happens first: `62-76`
  - when there is no overlap and the cross dataset is longer, the function falls back to the last `base_len` bars of the cross series: `78-85`
- Why it matters:
  - that fallback changes the semantic basis of cross-ticker comparison from timestamp overlap to tail-length alignment
- Current coverage:
  - there are tests for overlap and borrow-fast-path behavior:
    - `1190-1207`
  - there is no direct test pinning the no-overlap tail-slice fallback
- Smallest justified fix:
  - document this fallback explicitly and add a test for the no-overlap branch

## API And Naming

### 1. `normalize_strategy_kind` is a hard-coded routing registry inside a command module

- Evidence:
  - `src-tauri/src/commands_validation.rs:104-129`
- Why it matters:
  - strategy-routing policy is hand-maintained here instead of drawing from a more canonical strategy registry
  - when new strategy kinds are added elsewhere, this file can silently default to `DoubleMa`
- Smallest justified fix:
  - either document the deliberate fallback or move toward a shared registry source later

### 2. The file mixes public command API, internal orchestration, headless helpers, and data-export utilities

- Evidence:
  - public commands
  - internal generic cross-ticker driver
  - headless helper
  - `get_price_bars`
  - all in one `3630`-line module
- Why it matters:
  - the module boundary does not help reviewers isolate changes by concern

## Error Handling

### 1. Stringly typed errors are consistent with the command surface but keep complex failure causes coarse

- Evidence:
  - nearly every public/internal helper returns `Result<_, String>`
- Why it matters:
  - context-rich failures from validation-context mismatch, cache/data loading, and cross-ticker reruns all collapse to strings
- Review judgment:
  - acceptable for Tauri transport, but deeper typed errors would help refactors later

### 2. Validation-context enforcement is strict and command-sensitive

- Evidence:
  - `validation_context is required`: `137`
  - strategy/data/timeframe mismatch errors: `146-184`
- Why it matters:
  - this is a strong contract decision on the public command surface
- Review judgment:
  - probably correct, but it should be treated as public-surface-sensitive behavior

## Module Boundaries And Technical Debt

### 1. Composite and double-MA validation flows are heavily duplicated

- Evidence:
  - mirrored command bodies:
    - `341-428`
    - `578-673`
  - mirrored cross-ticker application functions:
    - `430-533`
    - `675-883`
  - mirrored request builders:
    - `533-576`
    - `834-882`
- Why it matters:
  - parity fixes can land in one path and drift from the other
  - the preserved-finalists failing test increases confidence that this surface is fragile

### 2. The file is doing too many jobs at once

- Current roles mixed together:
  - public Tauri command boundary
  - state/candle loading
  - validation-context checking
  - cross-ticker orchestration
  - preserved-finalists policy
  - headless helper
  - chart-price-bar API
  - extensive tests
- Review judgment:
  - the module should likely be split later, but not as opportunistic churn

## Docs And Examples

### 1. The cross-ticker fallback and preserved-finalists policies need explicit contract comments

- Missing documentation topics:
  - no-overlap tail-slice alignment fallback
  - preserved-finalists cap of `4`
  - which validation modules are intentionally stripped in cross-ticker rerun requests
  - strategy-kind normalization fallback to `DoubleMa`

## Tests And Missing Verification

### 1. The file has a strong direct test surface, but it is no longer fully passing

- Direct Rust lane:
  - `30` targeted tests
  - `29` passing
  - `1` failing locally
- Strongly covered areas:
  - validation-context mismatch/acceptance cases
  - cross-ticker request shaping
  - preserved-finalists selection bounds
  - many headless cross-ticker smoke paths
- Remaining gaps:
  - no explicit test for no-overlap tail-slice alignment
  - no async command-surface tests in this file itself
  - no direct tests for `get_price_bars` command wrapper behavior beyond builder logic

### 2. Frontend validation orchestration tests are green, but they do not replace Rust-side behavioral proof

- Frontend lanes passed:
  - `web/src/services/validationService.test.ts`
  - `web/src/stores/validationStore.actions.test.ts`
  - `web/src/tauri/commands.test.ts`
- Limitation:
  - they verify invocation/orchestration, not the inner run-verdict and cross-ticker semantics

## Performance, Allocation, And CUDA Concerns

### 1. Cross-ticker reruns are intentionally heavy and should be treated as benchmark-sensitive orchestration

- Evidence:
  - repeated reruns across datasets in `apply_cross_ticker_validation_generic`: `217-339`
  - candle slicing can allocate owned series slices: `24-37`, `78-85`
- Review judgment:
  - this is not a place for casual micro-optimizations
  - any throughput work should be benchmark-backed because it affects real validation cost

### 2. `spawn_blocking` usage is appropriate for the main command bodies

- Positive note:
  - offloading CPU-heavy validation work from the async command surface is the correct general pattern

## Cross-File Interaction Map

### Most likely coordination points if this file changes

- `super::validation_common`
- `crate::state::AppState`
- `ta_app_core` validation entrypoints
- `web/src/services/validationService.ts`
- `src-tauri/src/bin/ta_headless_validate.rs`

### Downstream consumers needing verification

- validation command invocation from frontend
- headless validation flow
- cross-ticker preserved-finalists behavior
- price-bar drilldown consumers

## Code-Quality Improvement Register

### Safe now

- add a direct test for no-overlap tail-slice alignment
- document preserved-finalists and alignment fallback behavior
- resolve the currently failing preserved-finalists run-verdict test or explicitly mark it if it is obsolete

### Coordination required

- changing validation-context strictness
- changing preserved-finalists selection rules
- changing cross-ticker rerun request stripping behavior

### Defer until later pass

- split composite/double-MA orchestration into shared smaller units
- move strategy-kind routing away from this command module

## Technical-Debt Reduction Register

### Debt safe to reduce now

- missing documentation for strong policy behaviors
- no-overlap alignment test gap
- unresolved failing test signal

### Debt that spans multiple files

- duplicated composite/double-MA validation orchestration
- shared strategy-kind registry / command-compatibility rules

### Debt that should not be “cleaned up” casually

- strict validation-context enforcement
- preserved-finalists cap and filtering semantics
- cross-ticker rerun module stripping, because verdict semantics depend on it

## Performance Opportunity Register

- Safe structural opportunities:
  - limited documentation/test hardening only
- Benchmark-sensitive opportunities:
  - any attempt to change cross-ticker rerun cost, candle slicing, or preserved-finalists execution policy
- Not the right place to chase performance first:
  - yes, until correctness and test status are cleaner

## Observability And Diagnostics Gaps

- the file has no explicit diagnostic note for the no-overlap alignment fallback
- the failing preserved-finalists test indicates an unresolved behavioral mismatch in a critical path

## Open Questions And Assumptions

- open question:
  - is the failing preserved-finalists double-MA run-verdict test exposing a real bug, or is the test stale relative to current verdict policy
- assumption:
  - strict `validation_context` enforcement is intentional and part of the product’s artifact-integrity guarantees

## Regression Risk And Feature-Preservation Constraints

- Do not change preserved-finalists filtering or rerun stripping casually; those behaviors directly affect validation verdict semantics.
- Do not loosen `validation_context` checks casually; that would widen the command contract.
- Treat any cross-ticker performance or alignment change as semantics-sensitive and benchmark-sensitive.

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app commands_validation -- --nocapture`
   - currently fails locally in `cross_ticker_headless_double_ma_recomputes_run_verdict_after_filtering`
2. `npm --prefix ./web run test -- src/services/validationService.test.ts src/stores/validationStore.actions.test.ts src/tauri/commands.test.ts`
3. Manual headless/desktop spot-check if preserved-finalists or alignment behavior changes

## Recommended Action

`Larger refactor but defer`

This file already has meaningful direct coverage, so the highest-value next work is not blind decomposition. It is:

- resolve the existing failing preserved-finalists signal
- document the strongest policy behaviors
- add the missing alignment test

After that, a deliberate split of the duplicated composite/double-MA orchestration would be justified.
