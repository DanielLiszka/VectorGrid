# Review Packet: `src-tauri/src/commands_dynamic.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_dynamic.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Not ready to accept as-is because the current IPC contract is precision-unsafe and partially unverified`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface-sensitive`, `Strategy-local`, `Semantics-sensitive`

## What This File Owns

`commands_dynamic.rs` is the Tauri command-boundary module for DynamicDoubleMa strategy IR preview/formatting helpers.

This file owns exactly three public commands:

- `preview_dynamic_double_ma_strategy_ir_json`: `src-tauri/src/commands_dynamic.rs:21-47`
- `format_dynamic_strategy_ir_json`: `src-tauri/src/commands_dynamic.rs:50-52`
- `encode_dynamic_strategy_constant_bits`: `src-tauri/src/commands_dynamic.rs:55-64`

It also owns two public response DTOs:

- `DynamicDoubleMaStrategyIrJsonPreviewResponse`: `src-tauri/src/commands_dynamic.rs:8-14`
- `DynamicStrategyConstantBitsResponse`: `src-tauri/src/commands_dynamic.rs:16-18`

This file does not own:

- strategy IR schema rules
- strategy-plan capability logic
- dynamic preview validation logic
- frontend builder state or optimize-store workflow
- runtime optimization execution

Those concerns are delegated to:

- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_plan.rs`
- frontend DynamicDoubleMa builder and optimize-service flows in `web/src/strategies/dynamicDoubleMaBuilder.ts` and `web/src/services/optimizeService.ts`

## Classification

- Public surface: yes
  - commands are re-exported in `src-tauri/src/commands.rs:74-76`
  - commands are registered in `src-tauri/src/lib.rs:189-191`
- Hot path: no
  - this file is preview/formatting support code, not runtime execution
- Runtime-critical: no
  - it affects pre-run UX and request correctness, not the hot execution loop
- Benchmark-sensitive: no
- CUDA/runtime-critical: no direct CUDA concern
- Strategy-local: yes
  - this file is specific to the DynamicDoubleMa strategy IR preview surface
- Low-risk support code: partially
  - operationally small, but contract-sensitive because it crosses the Tauri/JS boundary with strategy IR data

## Direct Interactions

### Backend owners this file delegates to

- strategy IR schema compatibility:
  - `crates/app_core/src/strategy_ir.rs:36-58`
- `StrategyIr` wire shape and `ValueRef::ConstantBits` representation:
  - `crates/app_core/src/strategy_ir.rs:102-136`
- preview capability resolution:
  - `crates/app_core/src/strategy_dynamic.rs:427-512`
- shared app-core exports:
  - `crates/app_core/src/lib.rs:253-285`

### Frontend contract anchors

- optimize-service preview invoke:
  - `web/src/services/optimizeService.ts:1034-1040`
- optimize-service preview normalization/summarization:
  - `web/src/services/optimizeService.ts:955-1004`
- optimize-service runtime request build that trusts `response.ir`:
  - `web/src/services/optimizeService.ts:1971-2013`
- builder-side constant bit encoding:
  - `web/src/strategies/dynamicDoubleMaBuilder.ts:129-133`
  - `web/src/strategies/dynamicDoubleMaBuilder.ts:272-278`
- task-center labels for the commands:
  - `web/src/stores/taskStore.ts:661-666`

### REPO_MAP contract anchors

- dynamic strategy IR review entrypoint:
  - `REPO_MAP.md:426-434`
- command-module ownership:
  - `REPO_MAP.md:578-579`
- dynamic strategy IR tooling surface:
  - `REPO_MAP.md:1035-1036`
- DynamicDoubleMa product note:
  - `REPO_MAP.md:1223`

## Standards and Research Log

This packet was written only after a targeted research pass against official or primary sources relevant to Tauri command arguments, JSON interoperability, precision-sensitive number transport, and Rust API/testing guidance.

### Sources consulted

1. Tauri command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file is entirely a Tauri command boundary
     - the frontend sends named IPC arguments into these commands
   - Standard informed:
     - command argument naming must match Tauri’s documented wire expectations
     - public command contracts should be explicit rather than relying on accidental behavior

2. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file defines public command responses that the frontend treats as stable data contracts
   - Standard informed:
     - public contracts should be predictable and avoid surprising lossy encodings

3. Rust Book testing
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why it was consulted:
     - this file has only smoke-level local tests and needed verification-gap assessment
   - Standard informed:
     - tests should cover the real behavior boundary, not just a happy path

4. RFC 8259 JSON interoperability guidance
   - Source: <https://www.rfc-editor.org/rfc/rfc8259>
   - Why it was consulted:
     - this file moves numeric strategy IR data across a JSON-based IPC boundary
   - Standard informed:
     - exact integer interoperability is not guaranteed outside the JavaScript-safe integer range
     - precision-sensitive numeric transport should avoid raw JSON numbers when the values exceed that safe range

5. Rust `f64::to_bits` documentation
   - Source: <https://doc.rust-lang.org/std/primitive.f64.html>
   - Why it was consulted:
     - this file encodes dynamic constants via `to_bits()`
   - Standard informed:
     - raw IEEE-754 bit-pattern transport is deliberate and should not be degraded by JSON number coercion

6. `serde_json` parse/pretty-print APIs
   - Source: <https://docs.rs/serde_json/latest/serde_json/fn.from_str.html>
   - Source: <https://docs.rs/serde_json/latest/serde_json/fn.to_string_pretty.html>
   - Why they were consulted:
     - this file parses user JSON into `StrategyIr` and returns pretty-printed JSON back to the frontend
   - Standard informed:
     - JSON parse/format helpers are fine here; the contract risk is not parsing itself, but what gets sent back over IPC afterward

## Applicable Current Standards

- Tauri command-surface standard:
  - IPC arg naming must be explicit and verified
  - frontend and Rust command parameter casing must not drift
- JSON interoperability standard:
  - precision-sensitive `u64` values should not cross into JS as raw JSON numbers
- DynamicDoubleMa builder standard:
  - constant operands are intentionally encoded as decimal string bit patterns in the builder
  - preview/build flows should preserve that safety invariant
- Public response contract standard:
  - `schema_compatibility` and similar fields should be stable wire types, not debug-format accidents

## Findings

### Correctness / Safety

#### 1. `preview_dynamic_double_ma_strategy_ir_json` returns `StrategyIr` back over IPC in a precision-unsafe form

Objective issue:

- `DynamicDoubleMaStrategyIrJsonPreviewResponse` includes `ir: StrategyIr` at `src-tauri/src/commands_dynamic.rs:8-14`
- `preview_dynamic_double_ma_strategy_ir_json` parses `ir_json` into `StrategyIr` and returns that structured `ir` value in the response at `src-tauri/src/commands_dynamic.rs:21-47`
- `ValueRef::ConstantBits.bits` in app-core is a raw `u64` with a custom deserializer only, not a custom serializer:
  - `crates/app_core/src/strategy_ir.rs:114-136`
- the frontend builder intentionally emits `constant_bits.bits` as a decimal string to avoid JS precision loss:
  - `web/src/strategies/dynamicDoubleMaBuilder.ts:129-133`
  - `web/src/strategies/dynamicDoubleMaBuilder.ts:272-278`
- but `optimizeService` later trusts `response.ir` from this command and passes it forward as a runtime request payload:
  - `web/src/services/optimizeService.ts:1983-1990`

Why this is a real bug:

- JavaScript cannot safely preserve arbitrary `u64` values as JSON numbers
- local reproduction confirms the precision loss:
  - parsing `{"bits":18446744073709551615}` in JS becomes `18446744073709552000`
  - that is a silent semantic mutation, not a display-only formatting change

Impact:

- previewed IR that contains `constant_bits` can be silently altered before execution
- this directly conflicts with the builder’s deliberate string-safe bit encoding

Smallest justified fix:

- stop returning structured `StrategyIr` over Tauri unless `constant_bits.bits` is serialized in a JS-safe form
- the smallest safe options are:
  - return only `pretty_json` / normalized JSON text and let the frontend keep string form as the source of truth
  - or introduce a precision-safe response DTO / serializer that emits `constant_bits.bits` as a decimal string

#### 2. The current command-argument casing contract is source-inconsistent and not actually verified

Objective issue:

- the Rust commands are plain `#[tauri::command]` functions with snake_case Rust parameter names:
  - `src-tauri/src/commands_dynamic.rs:21-28`
  - `src-tauri/src/commands_dynamic.rs:55-57`
- the frontend sends snake_case argument keys:
  - `ir_json`, `requested_backend`, `top_k`, `include_all` at `web/src/services/optimizeService.ts:1034-1040`
  - `value` at `web/src/services/optimizeService.ts:1505`
- `web/src/tauri/invoke.ts:76-89` forwards args verbatim
- no `rename_all = "snake_case"` attribute is present on the commands

Why this matters:

- Tauri’s documented default argument mapping expects camelCase unless the command opts into snake_case naming
- current tests only mock `invokeCommand`; they do not execute the real IPC boundary

Assessment:

- this is at least a high-confidence contract risk
- based on the documented default and the current source, it is very likely wrong unless some external conversion layer exists outside the inspected code

Smallest justified fix:

- add `#[tauri::command(rename_all = "snake_case")]` to the commands in this file
- or switch the frontend callers to camelCase
- then add one runtime-level contract test instead of relying on mocked `invokeCommand`

### API and Naming

#### 3. `schema_compatibility` is a debug-string wire contract, not a stable serialized contract

Objective issue:

- the response field is a plain `String` at `src-tauri/src/commands_dynamic.rs:9`
- it is populated via `format!("{schema_compatibility:?}")` at `src-tauri/src/commands_dynamic.rs:31-43`
- the source value is a real enum in app-core:
  - `crates/app_core/src/strategy_ir.rs:36-58`

Why this is weak:

- debug output is not a stable wire format
- it is lossy compared with the real enum
- frontend tests currently hardcode `"Compatible"` as the contract:
  - `web/src/services/optimizeService.test.ts:118`

Smallest justified fix:

- serialize a stable wire enum/string set deliberately instead of using Rust `Debug` formatting

### Error Handling

The file’s local error handling is otherwise reasonable.

Positive observations:

- JSON parse failures are explicit:
  - `parse strategy_ir json: ...`
- pretty-print failures are explicit:
  - `serialize strategy_ir json: ...`
- `encode_dynamic_strategy_constant_bits` correctly rejects non-finite input:
  - `src-tauri/src/commands_dynamic.rs:58-63`

No larger error-transport redesign is justified here.

### Module Boundaries / Technical Debt

This file is not large enough to justify a structural refactor.

The real tech-debt observation is narrower:

- `format_dynamic_strategy_ir_json` is part of the public Tauri surface:
  - `src-tauri/src/commands_dynamic.rs:50-52`
  - `src-tauri/src/lib.rs:190`
  - `web/src/tauri/commands.ts:110`
- but I found no actual frontend invoke site for it
- `encode_dynamic_strategy_constant_bits` has a service wrapper, but looks much less central than preview itself

Assessment:

- this is command-surface clutter, not a correctness bug
- do not remove these commands casually unless their call inventory is confirmed and tests are updated

### Docs / Examples

The main documentation gap is conceptual rather than prose-related:

- the file does not document whether `ir` in the preview response is intended for display, editing, or runtime execution reuse
- current code makes that ambiguity dangerous because the frontend reuses it as runtime data

Recommendation:

- if the precision-safe fix changes the response shape, document that `pretty_json` or a safe JSON payload is the canonical preview-edit surface

### Tests / Missing Verification

Positive evidence:

- targeted local verification passed:
  - `cargo test -p ta_desktop_demo_app commands_dynamic -- --nocapture`
  - result observed locally: `2 passed; 0 failed`
- local smoke tests cover:
  - preview happy path: `src-tauri/src/commands_dynamic.rs:72-103`
  - encode helper happy path: `src-tauri/src/commands_dynamic.rs:106-111`
- strong delegated backend coverage exists in app-core:
  - schema compatibility and constant-bit deserialization:
    - `crates/app_core/src/strategy_ir.rs:804-876`
  - dynamic preview behavior:
    - `crates/app_core/src/strategy_dynamic.rs:3467-3751`

Important gaps:

- no local or integration test covers the actual Tauri IPC argument casing for these commands
- no test covers a realistic `StrategyIr` preview response containing `constant_bits`
- frontend tests mock a partial preview payload and therefore miss the precision risk:
  - `web/src/services/optimizeService.test.ts:118-137`
- the encode-helper frontend test does not assert true bit semantics and would pass even if the Rust helper stopped returning `to_bits()` output:
  - `web/src/services/optimizeService.test.ts:786-802`
- `format_dynamic_strategy_ir_json` has no dedicated caller test because it appears unused

### Perf / Allocation / CUDA Concerns

No meaningful performance or CUDA findings.

This file is thin support code. The risk here is semantic correctness at the IPC boundary, not throughput.

## Recommended Action

Recommended bucket: `Small surgical refactor`

### Safe-now changes

- make the command argument casing explicit:
  - add `rename_all = "snake_case"` on these commands or switch the frontend to camelCase
- make the preview response precision-safe for `constant_bits`
- replace `schema_compatibility` debug-text output with a stable serialized wire value
- add one regression test for a realistic `StrategyIr` carrying `constant_bits`
- add one runtime-level contract test that exercises the actual Tauri arg names

### Document-only or defer-later work

- keep `format_dynamic_strategy_ir_json` only if there is a real product use; otherwise it is a later command-surface cleanup candidate
- keep `encode_dynamic_strategy_constant_bits` if the team still wants a backend-side canonical encoder, but verify its consumer value first

### Benchmark-sensitive or semantics-sensitive changes to avoid casually

- do not change the semantic meaning of `constant_bits` encoding
- do not casually change preview response fields without coordinated frontend updates

## Cross-File Coordination

Any meaningful fix in this file must be coordinated with:

- `web/src/services/optimizeService.ts`
- `web/src/strategies/dynamicDoubleMaBuilder.ts`
- `web/src/tauri/invoke.ts`
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- related frontend tests in `optimizeService.test.ts`, `dynamicDoubleMaBuilder.test.ts`, and `optimizeStore.actions.test.ts`

Practical implication:

- this file is small, but its contract changes are not isolated
- precision-safe transport changes must be handled as a mini cross-layer migration

## Code-Quality and Debt Register

High-value quality improvements:

- introduce a precision-safe preview DTO or serializer
- make command argument casing explicit
- replace debug-string `schema_compatibility` output with a stable wire value

Debt that is safe to reduce now:

- precision-unsafe preview transport
- implicit argument-casing reliance
- weak boundary verification

Debt that should be deferred:

- pruning `format_dynamic_strategy_ir_json`
- deciding whether backend-side constant-bit encoding should remain a public command at all

## Technical-Debt Reduction Staging

`Phase 1`

- make preview transport precision-safe
- make argument naming explicit
- stabilize `schema_compatibility` serialization

`Phase 2`

- add a real IPC-level contract test
- add a realistic `constant_bits` regression fixture across preview/build flow

`Phase 3`

- prune unused command-surface clutter such as `format_dynamic_strategy_ir_json` only if consumer inventory proves it is dead

## Performance Opportunity Register

There is no meaningful throughput work here.

The only worthwhile structural opportunity is payload-shape simplification:

- if one precision-safe canonical preview form is enough, avoid returning both a structured IR and pretty JSON without need

That is a payload-clarity opportunity, not a performance campaign.

## Implementation Staging

`Safe now`

- precision-safe preview response
- explicit `rename_all` policy
- better boundary tests

`Coordination required`

- optimize-service preview handling
- builder/runtime request source-of-truth decisions
- test fixture updates that currently assume `"Compatible"` debug text

`Defer`

- command pruning
- broader preview UX redesign

## Change Blast Radius

- `Local only`
  - local tests
  - internal comments
- `Same subsystem`
  - preview DTO shape and schema-compatibility serialization
- `Cross backend/frontend contract`
  - builder preview UX
  - runtime optimize request construction
  - task labels
  - any persisted preview-edit flows
  - frontend test fixtures and mocks

## Observability Gaps

Notable gaps:

- no structured signal separating parse failure from compatibility failure from transport-shape failure
- no explicit indication when precision-safe normalization occurs
- no runtime evidence for real Tauri arg-casing mismatches because tests mock `invokeCommand`

These gaps make this boundary more brittle than its size suggests.

## Open Questions and Assumptions

- Assumption:
  - DynamicDoubleMa preview should preserve exact `constant_bits` semantics all the way into runtime request construction
- Open question:
  - what is the canonical preview source of truth: structured DTO, safe JSON text, or both?
- Open question:
  - does backend-side constant-bit encoding still deserve a public command if the builder already has a precise browser-side implementation?
- Open question:
  - is `format_dynamic_strategy_ir_json` intentionally reserved for future UI use, or effectively dead surface today?

## Exact Local Verification Required If Changed

If this file changes, the minimum verification should be:

- `cargo test -p ta_desktop_demo_app commands_dynamic -- --nocapture`
- `cargo test -p ta_app_core preview_dynamic_double_ma_strategy_ir -- --nocapture`
- `cargo test -p ta_app_core deserialize_constant_bits_from_decimal_string -- --nocapture`
- `npm --prefix web run test -- src/services/optimizeService.test.ts src/stores/optimizeStore.actions.test.ts src/strategies/dynamicDoubleMaBuilder.test.ts src/components/OptimizeWorkspacePanel.test.ts src/services/optimizeRequestBuilder.test.ts src/utils/strategyOptimizationCapabilities.test.ts`

If the IPC argument naming or preview response shape changes, add a manual desktop smoke pass for:

- preview DynamicDoubleMa builder IR from the optimize workspace
- confirm preview metadata still reflects resolved backend and reasons
- confirm the previewed IR used for runtime execution preserves constant-bit semantics

## Final Assessment

`commands_dynamic.rs` is small and intentionally thin, but it is not safe to accept as-is because its current public preview response can silently corrupt `constant_bits` when it crosses the Tauri/JavaScript boundary, and its current command-argument casing contract is not explicitly aligned with Tauri’s documented expectations.

The good news is that the fixes are narrow. This does not need a redesign. It needs a precision-safe preview response, explicit IPC naming, and better boundary tests.
