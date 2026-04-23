# Review Packet: `src-tauri/src/commands_analysis.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_analysis.rs`
- Packet status: `Reviewed`
- Recommended action bucket: `Accept as-is`
- Sensitivity tags: `Public-surface-sensitive`, `Runtime-critical`

## What This File Owns

`commands_analysis.rs` is a thin Tauri command-boundary adapter for analysis follow-up actions. It owns four desktop commands:

- `compute_double_ma_drilldown`
- `compute_composite_cpu_drilldown`
- `compute_double_ma_local_stability`
- `compute_composite_cpu_local_stability`

The file does not own strategy math, analysis semantics, persistence, reporting, or cancellation policy. Its job is to:

- accept request DTOs re-exported from `ta_app_core`
- resolve `req.data_id` into loaded candles via `AppState`
- move heavy follow-up work onto `tauri::async_runtime::spawn_blocking(...)`
- return app-core results over the Tauri command surface as `Result<_, String>`

Local size and shape:

- small support/orchestration module
- `53` lines
- no unsafe code
- no allocation-heavy or algorithmically complex local logic

## Classification

- Public surface: yes
  - all four functions are `#[tauri::command]` exports in `src-tauri/src/commands_analysis.rs:7-57`
- Hot path: no
  - the expensive work lives in `ta_app_core`; this file only resolves state and dispatches
- CUDA/runtime-critical: runtime-relevant, but not CUDA-critical
  - incorrect edits can break the desktop command boundary or move heavy work onto the wrong runtime path
- Strategy-local: no
  - strategy-specific behavior is delegated to app-core
- Low-risk support code: mostly yes, but public-surface-sensitive

## Direct Interactions

### Upstream registration and command surface

- Re-exported from `src-tauri/src/commands.rs:40-43`
- Registered into the Tauri invoke handler in `src-tauri/src/lib.rs:179-182`
- Included in the frontend known-command registry in `web/src/tauri/commands.ts:99-102`
- Tracked and cancel-mapped in `web/src/stores/taskStore.ts:223-226` and `web/src/stores/taskStore.ts:276-279`

### App state and runtime dependencies

- Reads candles through `AppState::get_candles` in `src-tauri/src/state.rs:408-420`
- Uses `tauri::State<'_, AppState>` in all four commands
- Uses `tauri::async_runtime::spawn_blocking(...)` in all four commands

### App-core execution targets

- `ta_app_core::compute_double_ma_drilldown_blocking_with_candles`
  - defined in `crates/app_core/src/double_ma.rs:4482-4518`
- `ta_app_core::compute_double_ma_local_stability_blocking_with_candles`
  - defined in `crates/app_core/src/double_ma.rs:4654-4695`
- `ta_app_core::compute_composite_cpu_drilldown_blocking_with_candles`
  - defined in `crates/app_core/src/composite_cpu.rs:11578-11620`
- `ta_app_core::compute_composite_cpu_local_stability_blocking_with_candles`
  - defined in `crates/app_core/src/composite_cpu.rs:11854-11895`

### Frontend consumers and contract coverage

- Drilldown request construction in `web/src/services/analysisService.ts:1951-1999`
- Local-stability request construction in `web/src/services/analysisService.ts:2022-2085`
- Frontend command contract tests in `web/src/services/analysisService.test.ts:317-420`
- Composite command-family tests in `web/src/services/analysisService.test.ts:612-701`
- Explicit drilldown-gap surfacing tests in `web/src/services/analysisService.test.ts:1267-1320`

### Related backend callers outside this Tauri module

- Live execution reuses the same app-core drilldown functions in `src-tauri/src/commands_execution.rs`
- Options-expression follow-up code reuses `compute_double_ma_drilldown_blocking_with_candles` in `src-tauri/src/commands_options_expression.rs`

## Standards and Research Log

This packet was written only after a targeted research pass against official sources relevant to a thin Tauri command adapter.

### Sources consulted

1. Tauri command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why: this file is a Tauri command boundary
   - Relevant guidance:
     - async commands are preferred for heavy work to avoid UI freezes
     - non-async commands run on the main thread
     - `Result` return types are the standard error transport shape for commands

2. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why: public-facing command modules still need a defensible review bar for naming, dependability, and debuggability
   - Relevant guidance:
     - argument validation and dependability matter on public surfaces
     - public APIs should remain predictable and debuggable

3. Rust API Guidelines naming guidance
   - Source: <https://rust-lang.github.io/api-guidelines/naming.html>
   - Why: command names are part of a stable frontend/backend contract
   - Relevant guidance:
     - functions should use `snake_case`
     - naming churn on public surfaces should buy real clarity

4. Rust Book error handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why: this file is mostly a `Result`-returning boundary layer
   - Relevant guidance:
     - error paths should propagate clearly
     - avoid masking meaningful failure context unnecessarily

5. Rust Book testing
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why: this file has no local Rust tests, so testing expectations need to be explicit
   - Relevant guidance:
     - behavior-critical boundaries should have regression coverage somewhere close to the contract

6. Clippy documentation
   - Source: <https://doc.rust-lang.org/clippy/>
   - Why: this file is a small boundary adapter where correctness, suspiciousness, complexity, and perf lints are sufficient guardrails

## Applicable Current Standards

- Tauri command-boundary standard:
  - heavy work should not run inline on the UI-sensitive command path
  - command errors should be transported consistently
- Rust public-surface standard:
  - stable names and predictable contracts matter more than local cleverness
- Repo-specific backend standard:
  - preserve behavior and feature coverage
  - prefer small, surgical changes over refactors
  - do not invent performance work where the file is not the hot path

## Findings

### Correctness / Safety

No objective correctness or safety defect was identified in this file itself.

Why:

- each command resolves candles before dispatch
- each command offloads heavy work with `spawn_blocking`
- each command forwards the app-core function result directly
- there is no unsafe code, raw pointer handling, custom synchronization, or manual allocation logic here

Notable positive behavior:

- keeping the expensive work behind `spawn_blocking` is the correct runtime posture for this Tauri boundary
- command ownership is narrow and explicit

### API and Naming

No naming issue found.

The exported command names are explicit, `snake_case`, and mirror the frontend command contract exactly. Changing them would create public-surface churn with no clear quality gain.

### Error Handling

The file uses a repo-consistent but lossy error transport style:

- state lookup failures are returned as `String`
- join failures from `spawn_blocking(...).await` are flattened with `.map_err(|e| e.to_string())?`

Assessment:

- this is acceptable in the current repo command model
- it is not ideal if the long-term goal is richer typed error transport over Tauri
- this is not a file-local fix target because changing it in isolation would create command-surface inconsistency

Review conclusion:

- treat typed Tauri error transport as a larger deferred refactor, not a local action item for this file

### Module Boundaries / Technical Debt

The module boundary is good.

Positive points:

- the file does not leak strategy logic into the Tauri layer
- it keeps app-core ownership in app-core
- it keeps state lookup and runtime handoff close to the command entrypoint

Minor technical debt:

- there is visible repetition across the four command wrappers

Assessment of the repetition:

- current duplication is small and highly readable
- abstracting it away would likely reduce clarity faster than it reduces maintenance cost
- this is not justified refactor material at the current size

### Docs / Examples

There is no local rustdoc or module comment.

Assessment:

- low severity
- acceptable for an internal Tauri command module
- if this command family grows, a short module comment documenting the invariant would become worthwhile:
  - fetch dataset from `AppState`
  - offload heavy work with `spawn_blocking`
  - forward app-core result and error semantics unchanged

### Tests / Missing Verification

This is the main local gap.

Objective issue:

- `commands_analysis.rs` has no direct Rust-side tests

Context:

- frontend contract coverage is strong in `web/src/services/analysisService.test.ts`
- state lookup behavior is tested through `AppState`
- app-core drilldown and local-stability semantics are tested in app-core
- but the wrapper itself has no local regression net for:
  - wrong command wiring
  - wrong request forwarding
  - wrong runtime-handoff behavior
  - accidental future divergence between the four wrapper patterns

Assessment:

- this is real test debt
- it is still moderate, not severe, because the file is thin and other layers already cover most semantics
- if this file is changed, direct command-boundary test coverage becomes warranted

### Perf / Allocation / CUDA Concerns

No local performance problem found.

Important boundary observation:

- this file is not the hot path
- any attempt to “optimize” the wrapper itself is likely churn
- the only material runtime concern is preserving `spawn_blocking`

Do not touch casually:

- do not inline app-core analysis work into the async command body
- do not move candle lookup or app-core dispatch into a pattern that obscures failure boundaries or ownership

## Recommended Action

`Accept as-is`

Reasoning:

- the file is small, explicit, and correctly scoped
- no objective correctness, safety, naming, or performance defect was found locally
- the only notable gap is missing direct Rust-side wrapper coverage, but that is not strong enough to justify churn in a stable thin adapter

Secondary deferred note:

- if the repo later standardizes structured Tauri error transport, revisit this file as part of that broader command-surface refactor

## Safe-Now vs Sensitive Work

### Safe now

- add a short module-level comment if the command family expands
- add targeted direct Rust integration or command-boundary tests if this file is modified anyway

### Semantics-sensitive

- changing command names
- changing request or response types
- changing `Result<_, String>` transport shape

### Runtime-sensitive

- removing or bypassing `spawn_blocking`
- changing when `AppState::get_candles` is called
- changing ownership of the resolved candle data before dispatch

### Benchmark-sensitive

- none locally

This file is runtime-sensitive, not benchmark-sensitive.

## Cross-File Coordination

If this file changes, the main coordinated surfaces are:

- frontend invoke/request handling in `web/src/services/analysisService.ts`
- tracked-command labels and cancellation wiring in `web/src/stores/taskStore.ts`
- shared app-core analysis semantics in `crates/app_core/src/double_ma.rs` and `crates/app_core/src/composite_cpu.rs`
- backend reuse sites in `src-tauri/src/commands_execution.rs` and `src-tauri/src/commands_options_expression.rs`

Practical implication:

- wrapper-only edits can stay local
- command names, DTOs, or result-shape changes immediately become a frontend/backend coordination task
- semantic changes to drilldown or local-stability outputs belong in app-core review packets, not here

## Code-Quality and Debt Register

High-value quality improvements:

- add a tiny direct Rust wrapper-test harness if this module is modified
- add a short invariant comment if the command family expands
- keep the four wrappers symmetric so future changes do not drift per-command

Debt that is safe to reduce now:

- missing direct wrapper coverage

Debt that should be deferred:

- repo-wide typed Tauri error transport
- generic wrapper abstraction for four already-readable commands

## Performance Opportunity Register

There is no meaningful wrapper-local optimization target here.

Useful performance guidance:

- preserve `spawn_blocking`
- do not add caching or inline compute here
- if app-core follow-up work is profiled or optimized later, keep this wrapper transparent enough that timing attribution stays obvious

## Implementation Staging

`Safe now`

- add wrapper-level tests if the file is touched
- add a short invariant comment if the family grows

`Coordination required`

- command-name, DTO, or response-shape changes with frontend consumers
- semantic result changes with app-core analysis owners

`Defer`

- typed error transport migration
- wrapper abstraction cleanup

## Change Blast Radius

- `Local only`
  - comments
  - direct wrapper tests
- `Same subsystem`
  - wrapper dispatch logic and app-core call routing
- `Cross backend/frontend contract`
  - command names
  - request fields
  - result shapes
  - tracked-command metadata

## Observability Gaps

Current boundary-level diagnostics are minimal.

Notable gaps:

- no per-command timing or lightweight command-level tracing
- no explicit `data_id` or strategy-family context in boundary failures
- join-failure context from `spawn_blocking(...).await` is flattened to string text only

These are not blockers, but they make wrapper regressions harder to diagnose than they need to be.

## Open Questions and Assumptions

- Assumption:
  - app-core remains the authoritative owner of drilldown and local-stability semantics
- Open question:
  - should thin command-boundary tests live in `src-tauri/tests/` or remain primarily at the service-contract layer?
- Open question:
  - if the repo standardizes typed Tauri errors later, should all four analysis commands migrate together as one contract change?

## Exact Local Verification Required If Changed

If this file changes, run at minimum:

1. Backend confidence:
   - `cargo test --workspace`

2. Frontend command-contract coverage:
   - `npm --prefix web run test -- src/services/analysisService.test.ts src/stores/taskStore.lifecycle.test.ts`

3. Command-surface and generated-command consistency:
   - `npm --prefix web run lint`

Recommended additional verification if command names, request shape, or invoke wiring changes:

4. Broad repo confidence:
   - `powershell -File scripts/run_repo_quality_gate.ps1`

## Regression and Feature-Preservation Constraints

This file sits on a public desktop command surface. Any future change must preserve:

- the four command names unless a coordinated frontend/backend migration is intentional
- the current request and result contract semantics
- the current off-main-work posture
- current strategy-family routing expectations from `analysisService.ts`

The intended improvement posture for this file is additive hardening only. Do not remove behavior, narrow command coverage, or “simplify” by collapsing command families without a larger approved contract change.
