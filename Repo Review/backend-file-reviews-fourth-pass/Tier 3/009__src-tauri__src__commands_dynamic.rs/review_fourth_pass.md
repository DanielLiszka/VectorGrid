# Fourth-Pass Review: `src-tauri/src/commands_dynamic.rs`

- Review date: `2026-04-16`
- Source file: `src-tauri/src/commands_dynamic.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file deserved a fourth pass because the third-pass packet kept the right bucket but drifted away from the live source. `commands_dynamic.rs` is a very small Tauri seam: three commands, two response DTOs, no managed state access, no event emission, no background-task orchestration, and no GPU/runtime ownership. That makes the third-pass focus on state-owner boundaries, cancellation semantics, and wrapper consolidation much less useful here than a direct review of the command boundary itself.

The live source still points to a narrower, more concrete problem set: JSON/IPC contract sharp edges, inconsistent treatment of strategy IR as text on ingress but structured data on egress, and thin local tests that do not exercise the risky path. The third pass correctly resisted a larger rewrite, but it underweighted the file-specific transport concerns that earlier passes had already surfaced and that the current source still supports.

So the fourth pass was not about changing the bucket. It was about re-anchoring the review on what this module actually owns, separating confirmed file-local risk from higher-layer assumptions, and trimming back recommendations that were broader than the evidence justifies.

## Fourth-Pass Research Log

I reread the live `commands_dynamic.rs` file first, then rechecked its public exposure in `src-tauri/src/lib.rs` and `src-tauri/src/commands.rs`. I also checked `src-tauri/src/state.rs` only to confirm the negative: this module does not participate in application state ownership, cancellation bookkeeping, or background-task coordination.

I then reread the prior packets to see where the review had become stronger and where it had drifted. The main pattern was clear: the first and second passes stayed close to the transport seam, while the third pass became more generic and less tied to what the file actually does.

For external standards, the relevant references remained the same ones that materially affect this file:

1. The official Tauri command documentation, to recheck command argument naming and the fact that ordinary command return values are serialized as JSON.
2. RFC 8259 and JavaScript safe-integer guidance, to recheck when JSON-number round-trips can stop being exact on the JS side.
3. Rust `f64::to_bits` documentation, to recheck that the constant-bit helper is deliberately bitwise rather than numeric.
4. Serde rename and container-attribute guidance, to recheck what can be stabilized at the DTO boundary without a larger redesign.

Those sources did not change the overall bucket. They did sharpen which earlier claims remain strong, which should be softened, and which safer immediate changes are actually narrower than the current packet suggests.

## What Fourth Pass Sharpened

### Bucket assessment: the third-pass bucket should stand

The current third-pass bucket should stand. This is still a `Small surgical refactor`, not because the file is trivial, but because the real risk is concentrated in a few narrow command-contract choices rather than in any large structural ownership problem.

The module is small, its responsibilities are already easy to locate, and the hot path is elsewhere. Even the meaningful fixes here are still seam fixes: safer IR transport discipline, explicit command-argument policy where it actually matters, more stable wire literals, and better tests. None of that requires a redesign of the module shape.

### What the third-pass packet got right

The third-pass packet was right on three important points.

First, it kept the action bucket unchanged. That remains the correct call.

Second, it treated this file as a public IPC seam instead of “just glue.” That is still the right mindset, because the command signatures and DTO fields are compatibility surfaces even if the file is operationally small.

Third, it warned against broad wrapper churn. That was also right. This file does not benefit from architectural cleanup for its own sake, and changing command names or response shapes casually would create more frontend risk than backend gain.

### What the third-pass packet missed or underweighted

The biggest miss is that the third-pass packet largely abandoned the file-specific issue that still matters most: `StrategyIr` crosses the Tauri boundary in structured form even though this module already treats IR as JSON text on input. That asymmetry is the core smell in the live source.

`preview_dynamic_double_ma_strategy_ir_json` accepts `ir_json: String`, parses it, and then returns both `ir: StrategyIr` and `pretty_json: String`. `format_dynamic_strategy_ir_json` goes the other direction and accepts `ir: StrategyIr` directly. That means the command surface is not following one stable transport discipline. It is switching between text-form IR and structured-object IR depending on direction and helper. That is precisely the sort of seam that becomes brittle when JS precision, nested serde shape, and frontend reuse semantics are involved.

The third pass also underweighted `format_dynamic_strategy_ir_json`. Earlier packets correctly noted that it shares the same precision hazard zone as the preview response. The fourth-pass refinement is stronger than that: this formatter is the most structurally fragile command in the file because it requires the frontend to build a fully deserializable `StrategyIr` object, with all nested field naming and numeric fidelity already intact, before Rust ever sees it. In other words, it is the command least aligned with the safer “opaque JSON text as canonical IR” pattern that the preview input already uses.

A second underweighted point is that only one of the three commands actually has a top-level argument-casing problem worth fixing. `preview_dynamic_double_ma_strategy_ir_json` has `ir_json`, `requested_backend`, `top_k`, and `include_all`, which differ between Tauri’s documented camelCase default and snake_case. By contrast, `format_dynamic_strategy_ir_json(ir: StrategyIr)` and `encode_dynamic_strategy_constant_bits(value: f64)` each take a single argument whose key is unchanged by casing policy. So the earlier recommendation to add `rename_all = "snake_case"` “to these commands” was directionally understandable, but too broad. The real casing contract question is concentrated in the preview command.

A third underweighted point is the local test gap. The current tests only cover a straightforward `DoubleMa` preview path and one ordinary finite float for the bit encoder. They do not exercise any IR carrying `constant_bits`, they do not exercise the formatter command at all, they do not prove command-argument naming at the real boundary, and they do not cover signed zero. Given that `to_bits` exists specifically to preserve exact bit patterns rather than numeric value, lack of a `-0.0` regression test is a real missed edge.

### What earlier packets overstated or left too weakly scoped

The earlier packet family was right to worry about precision, but one of its safer-looking remedies needs tighter wording: `pretty_json` is not automatically a precision-safe fallback merely because it is returned as a string.

That only holds if the frontend treats the JSON text as opaque text all the way back into Rust. If the frontend reparses that string into a JavaScript object before using it as the runtime source of truth, the same integer-fidelity problem returns unless the serialized representation itself keeps `constant_bits.bits` string-encoded. Because `pretty_json` is currently generated from the typed `StrategyIr`, not from raw input text, it may already have normalized any such field into a plain JSON number. So “use `pretty_json` as the canonical surface” is directionally right only when paired with one of two additional constraints: either the frontend keeps it as opaque text, or the serializer itself is made string-safe for the relevant fields.

There is also a narrower file-local option that earlier packets did not fully articulate: if the team wants `pretty_json` to preserve the incoming string-safe encoding without immediately changing app-core serializers, this command could parse the incoming `ir_json` twice — once as raw JSON value for pretty-printing, and once as `StrategyIr` for preview logic. But that narrower option has its own tradeoff: it would preserve unknown fields or other raw-input details that typed deserialization may ignore, so it should not be adopted casually under the label of “normalization.”

The argument-casing concern also needs slightly more careful severity language. Tauri’s docs make the documented default clear enough that the current preview signature is a genuine contract risk if the frontend is sending snake_case keys. But the live fourth-pass bundle does not include the actual frontend invoke site or generated bindings, so this remains a high-confidence compatibility risk rather than a fully re-proved production bug in this pass. The safe conclusion is still that the contract must be made explicit one way or the other.

Finally, the third-pass language about state ownership, cancellation scope, and not merging owner paths was simply too generic for this file. There is no evidence of those concerns in the source under review. Keeping that language in the current packet dilutes the useful review signal instead of sharpening it.

## Implementation Priority

### Safe-now hardening

The safest immediate hardening still fits the current bucket.

The first priority is to stop treating the structured `ir` field as the canonical cross-JS source of truth. If it must remain in the preview response for compatibility, it should be treated as transitional or display-oriented rather than as the payload the frontend trusts for runtime execution. The canonical round-trip representation should be text, not a JS object reconstructed from Tauri JSON.

The second priority is to tighten what “canonical text” means. If the frontend is going to rely on a text field, it must keep that field as opaque text until Rust parses it again. If the backend wants `pretty_json` itself to be safely round-trippable, then the command needs to preserve any string-safe constant-bit encoding rather than regenerating those values from typed `StrategyIr` serialization. Earlier packets were correct to point toward text transport, but not specific enough about this distinction.

The third priority is to stabilize `schema_compatibility` without causing unnecessary churn. The current use of `format!("{schema_compatibility:?}")` is weak because it freezes Rust `Debug` spelling as a wire contract. The narrow fix is not necessarily to expose a new IPC enum type. A safer immediate step is to keep the field as `String` but populate it from an explicit literal mapping that the frontend can rely on.

The fourth priority is to codify preview-command argument naming and test the real boundary. Because only the preview command has top-level argument names that differ between camelCase and snake_case, the safest immediate action is to make that single command explicit: either add `rename_all = "snake_case"` there if the callers truly use snake_case, or align the callers to documented camelCase and leave Rust unchanged. What should not remain is an implicit contract that is only “probably fine.”

The fifth priority is targeted regression coverage:

- a preview fixture that actually exercises constant-bit-carrying IR,
- a formatter test rather than leaving that command untested,
- a `-0.0` test for `encode_dynamic_strategy_constant_bits`,
- and one non-mocked boundary test or desktop smoke that proves the preview arg names and round-trip behavior.

These are narrow hardening steps, not redesign work.

### Larger redesign still deferred

The larger redesign question is whether `StrategyIr` should cross the Tauri boundary as a structured object at all.

That is still deferred.

The cleanest long-term shape is probably one of these two directions:

1. Dynamic IR stays textual at the Tauri seam, and this module becomes “text in / preview metadata out / text formatting helpers.”
2. `StrategyIr` becomes a deliberately JS-safe cross-language DTO with field-level serializers for precision-sensitive content.

Both directions are coherent. Neither needs to be decided in this pass.

What should stay deferred until there is actual consumer inventory is command pruning. `format_dynamic_strategy_ir_json` may be dead or low-value, and `encode_dynamic_strategy_constant_bits` may be redundant if the browser-side builder is already the canonical encoder. But removing either command without current caller proof would be a regression-prone cleanup, not safe-now hardening.

Likewise, pushing a serializer change down into app-core may be the best long-term answer if `StrategyIr` is meant to be shared widely across layers. But that reaches beyond this one Tauri seam and belongs in a coordinated cross-layer change, not in a small seam-only cleanup sold as purely local.

## Verification If Changed

If this file changes, the minimum local Rust verification should still include:

- `cargo test -p ta_desktop_demo_app commands_dynamic -- --nocapture`

But that is not enough if the contract is adjusted.

At least one real boundary check should be added for the preview command, because the risky issues here are not ordinary pure-Rust logic failures. They are IPC-shape failures: argument naming, JSON-number fidelity, and what the frontend treats as canonical after preview. A mocked invoke wrapper does not prove those.

If the runtime source of truth moves away from `response.ir`, then targeted frontend verification is required around the dynamic builder/preview/execute flow. Earlier packets named the likely owners (`optimizeService`, the dynamic builder helper, and related tests), and those are the right surfaces to recheck even though they were not re-opened in this bundle.

If the change is truly cross-layer, run the repository quality gate and the targeted frontend tests as part of the same coordinated change rather than validating the Rust seam in isolation.

## Regression And Feature-Preservation Constraints

Preserve the registered command names and the fact that this file remains a small strategy-local preview helper surface. The commands are registered in `lib.rs` and re-exported from `commands.rs`, so renaming or pruning them is not a purely local act.

Preserve exact constant-bit semantics. The current helper uses `f64::to_bits()` and rejects non-finite values. That is the right behavior. Do not “simplify” it into numeric casting, decimal re-encoding of the float value, or any other change that loses bit-level identity. Include signed zero in that preservation mindset even though the current tests do not.

Do not silently turn `schema_compatibility` from advisory metadata into a hard preview gate. Stabilizing the wire representation is safe. Changing the acceptance policy is a product/compatibility decision.

Be careful with any proposal that uses raw input JSON as the source of `pretty_json`. That can preserve string-safe constant-bit encoding, which is attractive, but it can also preserve unknown fields or other user-input shape that typed normalization may currently smooth over. That tradeoff is real and should be chosen deliberately.

Do not remove `format_dynamic_strategy_ir_json` or `encode_dynamic_strategy_constant_bits` merely because they look small or lightly used. Without current caller inventory, that is speculative cleanup.

Finally, avoid overclaiming performance, persistence, GPU ABI, or validation stakes. This file is not a hot path and does not own those surfaces. The real risk here is IPC contract correctness.

## Fourth-Pass Conclusion

The third-pass action bucket should stand: this file remains a `Small surgical refactor`.

But the third-pass packet itself did not stay focused enough on the live source. It got the bucket right and the anti-redesign posture right, yet it drifted into generic concerns that this module does not actually own. The real file-specific issues are still the transport ones: inconsistent text-vs-structured IR handling, an underweighted formatter seam, weak `schema_compatibility` serialization, and tests that do not hit the risky path.

The most useful safe-now hardening is therefore narrow and evidence-backed:

- make the preview command’s argument policy explicit,
- stop treating structured `ir` as the trustworthy cross-JS source of truth,
- treat text transport as safe only when it stays opaque or when the serializer is precision-safe,
- stabilize `schema_compatibility` with deliberate literals,
- and add risk-path tests, especially for constant bits and signed zero.

The larger redesign question — whether `StrategyIr` should ever cross this IPC boundary as a structured object — is still real, but it remains deferred. That question is broader than this tier, while the current contract hardening work is not.
