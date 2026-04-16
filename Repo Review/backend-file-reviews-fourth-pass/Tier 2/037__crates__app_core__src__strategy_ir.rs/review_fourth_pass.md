# Fourth-Pass Review: crates/app_core/src/strategy_ir.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/strategy_ir.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file still warrants a fourth pass because the live source is a contract surface, not just an implementation helper. `strategy_ir.rs` defines the serialized IR types, version gates, graph validation, and the static profile builder that feed preview, planning, and GPU program compilation. In that role, small contract mistakes propagate far beyond this file.

The current third-pass packet kept the right bucket, but it did not stay anchored on the live source. Its main discussion drifted into neighboring planner/runtime concerns such as `DefaultHasher`, `OnceLock`, and thread-local caches, which are primarily owned by `strategy_plan.rs`, `strategy_gpu_program.rs`, and `strategy_gpu_runtime.rs`, not by `strategy_ir.rs` itself. That matters because the real fourth-pass question here is not cache topology. It is whether this IR file is honestly expressing, validating, and documenting the contract it claims to own.

## Fourth-Pass Research Log

This pass re-read the live `strategy_ir.rs` source, then checked the immediate downstream consumers in `strategy_plan.rs`, `strategy_gpu_program.rs`, and `strategy_gpu_runtime.rs` to see which IR details actually propagate into compiled plans, GPU metadata, and executable identity.

It also revisited the repo routing docs and earlier review packets to separate source-backed findings from review drift. The most useful local cross-checks were:

- `strategy_ir.rs:288-468`, where the validator’s real scope is visible.
- `strategy_ir.rs:722-794`, where the static builder shows the stricter profile-specific shape the file itself considers canonical.
- `strategy_plan.rs:121-128`, `217-230`, and `187-193`, where indicator IDs are normalized for planning but the raw IR is still fingerprinted as serialized.
- `strategy_gpu_program.rs:413-418` and `strategy_gpu_runtime.rs:121-150`, where the raw IR fingerprint continues to surface in GPU-facing metadata.

Official documentation checked during this pass reinforced four points.

First, Serde defaults are part of the wire contract, not just an implementation convenience. `#[serde(default)]` and `default = "..."` make omission behavior observable to callers, so they need to be treated as contract decisions, not hidden internals.

Second, Serde’s `untagged` and field-attribute guidance confirms that `ConstantBits` is intentionally accepting more than one incoming shape today, and that changing serialization behavior would be a separate compatibility decision rather than a documentation-only cleanup.

Third, the Rust API Guidelines still support two conclusions that matter here: public contract types should be documented clearly, and validation should happen as early as practical when the API surface claims to validate inputs.

Fourth, the downstream identity discussion is real but narrower than the third-pass packet implied. Official Rust docs note that `HashMap` iteration is arbitrary and that `DefaultHasher` should not be treated as a stable cross-release hash. That strengthens the case for canonicalized downstream fingerprints if durable identity ever matters, but it does not make fingerprinting the central issue of `strategy_ir.rs` itself.

A separate cross-boundary research point also matters here: ECMAScript numbers only preserve integer precision up to `2^53 - 1`, and JSON numbers larger than that cannot be round-tripped losslessly through ordinary JavaScript `Number` values. That sharpens the `ConstantBits` risk because this IR crosses frontend/Tauri/backend boundaries.

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **stand**. This file still fits `Small surgical refactor`, because the immediate work is contract hardening, validator-scope clarity, and a few targeted tests. There is no evidence here for a sweeping redesign, no hot-path performance opportunity, and no need to destabilize the planner/runtime stack just to improve the IR layer.

What should **not** stand is the third-pass reasoning. The central issues in this file are validator scope, schema leniency, and canonicalization asymmetry. The cache and runtime-handle discussion is adjacent context, not the primary review frame.

### What the current third-pass packet got right

The third-pass packet was right to avoid a broad rewrite. It was also right to be cautious around fingerprint and identity semantics once the IR leaves this file, because `CompiledStrategyPlan` exposes `ir_fingerprint_hex` (`strategy_plan.rs:84-107`, `299-315`), `CompiledGpuPredicateProgram` copies that into `plan_fingerprint_hex` (`strategy_gpu_program.rs:413-418`), and `GpuStrategyExecutable` surfaces it again (`strategy_gpu_runtime.rs:121-150`). That is a real downstream contract.

It was also directionally right that identity/fingerprint changes should not be treated casually as stable public IDs. The problem is that this is downstream context, not the main thing the source file itself needs from a fourth pass.

### What it missed or underweighted

#### 1. `validate_strategy_ir` is much looser than the file’s own builder implies

The validator checks only schema support, two shallow moving-average field checks, a non-empty `ma_source`, and graph referential integrity (`strategy_ir.rs:460-468`). That is materially weaker than the canonical IR shape produced by `build_static_strategy_ir`, which normalizes `ma_source` and MA ids (`strategy_ir.rs:726-728`), sets profile-specific strategy flags (`731-756`), and fills composite-profile execution fields such as `halftrend`, `macz`, `ott`, and `signal_mode` (`772-790`).

Earlier packets focused mostly on execution-policy omissions, but the gap is broader than that. The live validator does not enforce:

- profile/program agreement,
- profile/strategy-flag agreement,
- profile/execution-policy agreement, or
- even basic moving-average/catalog legality for `fast_ma`, `slow_ma`, or `ma_source`.

That last point is easy to miss. Dynamic indicator nodes get real descriptor-backed validation through `find_dynamic_indicator_descriptor`, parameter normalization, and output-ref validation (`strategy_ir.rs:375-391`). By contrast, `fast_ma` and `slow_ma` only require non-empty `ma_type` strings and finite numeric params (`288-302`), and `ma_source` only has to be non-empty (`464-467`). So the file currently validates dynamic indicators more rigorously than it validates the two core moving averages that every strategy IR carries.

That is a stronger and more important finding than the third-pass cache discussion.

#### 2. The deserialization contract is more liberal than the earlier packets fully accounted for

The earlier reviews correctly noticed defaults, but they underweighted how much leniency is already baked into this wire format.

The public structs do not opt into `#[serde(deny_unknown_fields)]`, so extra or misspelled JSON keys are ignored by default for JSON-like formats. Combined with the many `#[serde(default)]` fields on `IndicatorNode`, `SignalProgramGraph`, `ExecutionPolicy`, and `StrategyIr` (`strategy_ir.rs:186-236`), that means this IR surface is intentionally or accidentally tolerant of omission and typo drift.

That is not automatically wrong, but it means that recommendations like “just tighten validation here” are riskier than they first sound. Some of the contract leniency is happening before `validate_strategy_ir` even runs.

#### 3. Canonicalization is asymmetric across builder, validator, and planner

This is the most important missed edge case.

The builder canonicalizes `ma_source`, `fast_ma_type`, and `slow_ma_type` to trimmed lowercase (`strategy_ir.rs:726-728`). The validator does not. It only rejects empty values (`460-467`). Dynamic indicator IDs are trimmed and lowercased in a temporary validation variable (`339-344`), and their params/output refs are normalized only for checking (`382-391`), but the original IR object is not rewritten.

Then `compile_strategy_plan` normalizes again when it matters for execution: it trims indicator IDs for indexing (`214-215`), lowercases `indicator_id` for descriptor lookup (`121-128`, `217-230`), and fingerprints the **raw serialized IR** separately (`187-193`).

The result is that semantically equivalent IR payloads can be accepted and compile to the same effective plan shape, while still preserving different raw serialized forms and therefore different `ir_fingerprint_hex` values.

That is a real persistence and diagnostics risk. It is also a much better explanation for why downstream identity needs care than the third-pass packet’s generic cache framing.

#### 4. `ConstantBits` is not just obscure; it is cross-language fragile

The earlier packet was correct that `ValueRef::ConstantBits` is underdocumented. The fourth pass sharpens the reason.

`ConstantBits.bits` is a `u64` (`strategy_ir.rs:114-117`). The file provides a custom **deserializer** that accepts either a numeric `u64` or a decimal string (`126-143`), but it does **not** provide a matching custom serializer. So Rust-side serialization will emit a JSON number by default.

That is risky on a frontend/Tauri boundary because ordinary JavaScript `Number` values cannot represent arbitrary 64-bit integers exactly. In practice, the safe wire form for this field is the decimal string form the deserializer already accepts, not the numeric form the serializer currently emits.

This is an IPC and persistence concern, not just a readability concern. But changing the serializer now would be a compatibility change, not a free cleanup.

### What the current packet overstated

The biggest overstatement is the idea that hash/cache semantics are the main fourth-pass story for `strategy_ir.rs`. They are not. This file supplies the raw material for that story, but the actual fingerprinting logic lives in `strategy_plan.rs` and `strategy_gpu_program.rs`.

A smaller but still important overstatement is the earlier implication that simply tightening `validate_strategy_ir` in place is the obvious next step. That is directionally appealing, but not automatically safe. Because the current serde layer is already permissive and because the source file now supports both legacy static indicators and descriptor-driven indicator nodes, global tightening can break saved JSON, preview payloads, or future builder flexibility unless the repo first distinguishes “well-formed IR” from “runnable constrained-profile IR.”

## Implementation Priority

### Safe-now hardening

1. **Document the real validator scope explicitly.**
   Add rustdoc on the public contract types and constants, especially `StrategyIr`, `ExecutionPolicy`, `IndicatorNode`, `SignalProgramGraph`, `ValueRef`, and the schema/version constants. The most important sentence is that `validate_strategy_ir` is currently a **well-formedness** validator, not a full runnable-profile validator.

2. **Document wire-format leniency and preferred forms.**
   The docs should make three things explicit:
   - omitted fields with serde defaults are part of the contract,
   - unknown JSON fields are currently ignored,
   - `ConstantBits.bits` should be treated as a decimal string when crossing JavaScript boundaries, even though deserialization remains backward-compatible with numeric input.

3. **Add tests that lock down the current leniency deliberately instead of leaving it accidental.**
   The current suite covers schema compatibility, dynamic-indicator validation, and builder success (`strategy_ir.rs:803-1110`), but it does not pin the boundary behavior that is now most risky. Add targeted tests for:
   - profile/execution mismatch,
   - profile/program mismatch,
   - invalid or unknown MA ids / `ma_source` behavior,
   - non-canonical casing/whitespace inputs,
   - unknown JSON fields,
   - `ConstantBits` string-vs-number transport expectations.

4. **If stricter validation is needed immediately, add a separate stricter helper instead of changing `validate_strategy_ir` semantics in place.**
   A narrowly named helper for preview/build surfaces is the safest small-step hardening. It preserves the current low-level compatibility surface while letting constrained callers ask for stronger guarantees.

### Larger redesign still deferred

1. **Split the validation layers.**
   The clean design is to distinguish:
   - serde/wire acceptance,
   - IR well-formedness,
   - canonicalization,
   - runnable profile validation.

   Right now those ideas are partially mixed and partially deferred.

2. **Move any durable-identity story onto a canonical representation.**
   If `ir_fingerprint_hex` ever needs to be stable or meaningfully comparable across semantically equivalent payloads, it should not be computed from the raw serialized `StrategyIr` object. It should come from a normalized representation, likely after ID normalization and deterministic map ordering. That change belongs to the planner/runtime identity layer, not as an opportunistic tweak inside `strategy_ir.rs`.

3. **Version any serializer change for `ConstantBits`.**
   The right long-term wire contract may be “always serialize as decimal string, continue deserializing both string and legacy numeric forms.” But that is a schema/persistence decision, not a same-day cleanup.

4. **Decide whether this file owns a constrained profile DSL or a generic boolean-signal IR.**
   Today it looks like both. The builder and repo docs imply constrained static profiles, while the validator accepts something much closer to a generic graph with a profile tag. That ambiguity should be resolved deliberately, not by incremental validator patches.

## Verification If Changed

If only documentation and tests change, the local verification surface remains small:

- `cargo test -p ta_app_core strategy_ir::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_plan::tests:: -- --nocapture`

If any normalization, fingerprint, or serializer behavior changes, expand verification immediately to the downstream identity surfaces:

- `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`

If validation semantics tighten, add the caller-boundary checks too:

- dynamic preview / command tests on the Tauri side,
- any saved-artifact replay coverage that consumes serialized strategy IR,
- frontend or request-builder tests if the accepted JSON surface changes.

No performance gate is justified for this file alone. This is build-time/validation-time contract code, not a hot-path runtime surface.

## Regression And Feature-Preservation Constraints

- Preserve schema v1 compatibility unless there is an explicit migration plan.
- Do not tighten deserialization or validation globally just because the stricter behavior looks cleaner. The live surface currently tolerates omitted fields, extra fields, and some non-canonical input forms.
- Do not change `ConstantBits` serialization format without coordinating frontend, Tauri, and any saved JSON artifact consumers.
- Do not silently canonicalize deserialized IR in place unless you have audited how raw serialized form is used for persistence, diagnostics, and exposed identity fields.
- If raw IR fingerprint behavior changes, expect fallout in `CompiledStrategyPlan.ir_fingerprint_hex`, `CompiledGpuPredicateProgram.plan_fingerprint_hex`, and `GpuStrategyExecutable.plan_fingerprint_hex`. That is more of a persistence/diagnostics contract than a GPU ABI-layout issue, but it is still externally visible.
- GPU executable cache keys are driven by compiled-plan/executable fingerprints, not directly by raw IR fingerprints (`strategy_gpu_runtime.rs:85-117`), so raw IR canonicalization changes are more likely to affect observability and artifact identity than executable-cache correctness. Still, verify both.
- Do not overcorrect into `#[serde(deny_unknown_fields)]` everywhere without deciding whether forward-compatibility and typo-detection should be handled at the serde layer or at a stricter caller-specific validation layer.

## Fourth-Pass Conclusion

The third-pass action bucket should **stand** as `Small surgical refactor`, but the third-pass packet itself is not the right explanation for why.

What the current packet got right was restraint: this file does not need a rewrite. What it missed was the actual contract tension in the live source. `strategy_ir.rs` currently mixes a constrained-profile builder with a comparatively loose serde/validation surface, and that gap shows up most clearly in four places: profile semantics, MA/`ma_source` legality, canonicalization asymmetry, and `ConstantBits` cross-language transport.

So the safe-now work is not “rewrite the IR” and not “jump straight into cache redesign.” It is to make the validator’s scope honest, make the wire contract explicit, and add tests around the permissive edges that are currently implicit. The larger redesign—separating wire acceptance, normalization, strict runnable validation, and durable identity—should remain deferred until it is coordinated across planner, GPU runtime, Tauri preview, and any persisted artifact flows.
