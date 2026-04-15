# Second-Pass Review: `src-tauri/src/commands_dynamic.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_dynamic.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_dynamic.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass identified two main contract risks:

- precision-unsafe `u64` transport for `constant_bits` across the Tauri/JS boundary
- an unverified and source-inconsistent command-argument casing contract

## Stale Check

The live source still matches the first-pass concerns:

- `DynamicDoubleMaStrategyIrJsonPreviewResponse` still returns structured `StrategyIr`
- `schema_compatibility` is still produced via `Debug` formatting
- `format_dynamic_strategy_ir_json` still takes `StrategyIr` directly over IPC
- the commands still do not opt into explicit `rename_all = "snake_case"`

No source drift weakened the first-pass packet.

## Second-Pass Research Log

1. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - reconfirm command argument naming and IPC-contract expectations
   - Additional constraint informed:
     - command argument casing should be made explicit rather than relying on an unverified default

2. RFC 8259 JSON
   - Source: <https://www.rfc-editor.org/rfc/rfc8259>
   - Why consulted in second pass:
     - refine the first-pass precision analysis around JSON number interoperability
   - Additional constraint informed:
     - JSON alone does not protect large integer fidelity across JavaScript boundaries

3. MDN `Number.MAX_SAFE_INTEGER`
   - Source: <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MAX_SAFE_INTEGER>
   - Why consulted in second pass:
     - tighten the practical frontend precision boundary for `u64`-like transport
   - Additional constraint informed:
     - any `u64` larger than the JS safe integer range remains unsafe as a raw number

## Delta vs First Pass

## Refined Precision Finding

### 1. The precision issue is not limited to the preview response; `format_dynamic_strategy_ir_json` is in the same hazard zone

- Evidence:
  - `format_dynamic_strategy_ir_json(ir: StrategyIr)` still accepts a structured IR value over Tauri IPC
- Why this matters:
  - even if the preview response is fixed, the formatting command can still receive already-corrupted `constant_bits` values if the frontend hands Tauri a JS number outside the safe integer range
- Practical implication:
  - the safe fix should be endpoint-consistent:
    - either keep `constant_bits.bits` string-safe everywhere at the IPC boundary
    - or stop moving structured `StrategyIr` over Tauri where raw `u64` fidelity matters

## Refined Serialization Finding

### 2. `schema_compatibility` is still a weak wire contract, and the second pass did not find any reason to keep it as `Debug` text

- Evidence:
  - `format!("{schema_compatibility:?}")` still populates the response
- Why this matters:
  - this is now clearly a separate cleanup from the numeric precision issue
- Practical implication:
  - it can be fixed surgically with an explicit wire enum/string set without disturbing the broader DynamicDoubleMa preview flow

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_dynamic -- --nocapture`
- `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture --test-threads=1`
- `npm --prefix web run test -- src/services/optimizeService.test.ts src/strategies/dynamicDoubleMaBuilder.test.ts`

## Regression And Feature-Preservation Constraints

- Preserve DynamicDoubleMa preview semantics and frontend builder workflow
- Fix precision hazards without narrowing supported IR features
- Do not change command names or argument naming behavior without updating the frontend in lockstep

## Second-Pass Conclusion

`Bucket unchanged, but precision scope widened`

The first-pass `Small surgical refactor` bucket still stands. The main second-pass improvement is confirming that all IPC surfaces carrying structured `StrategyIr`, not just the preview response, need the same precision-safe transport discipline.
