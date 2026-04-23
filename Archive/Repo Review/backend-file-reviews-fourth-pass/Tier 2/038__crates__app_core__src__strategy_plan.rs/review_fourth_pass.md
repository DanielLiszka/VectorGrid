# Fourth-Pass Review: crates/app_core/src/strategy_plan.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/strategy_plan.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass bucket should stand. `strategy_plan.rs` is still a small file with outsized semantic reach: it compiles validated `StrategyIr` into route-annotated plans, emits the first fingerprint in the dynamic-strategy chain, and feeds the GPU program/runtime layers exported from `app_core`. That still reads as a small-surgical surface, not a redesign trigger.

What justified a fourth pass was not a new structural concern. It was the need to challenge whether the earlier packets had found the right *bounded* changes. After reopening the live source and the adjacent consumers in `strategy_gpu_program.rs` and `strategy_gpu_runtime.rs`, the answer is: mostly yes on the bucket, but not on the priority ordering. The earlier packets were correct to avoid planner/runtime rewrites and to treat fingerprints and overrides as contract-sensitive. They underweighted one clearer bug-class in the current implementation and slightly overstated how confidently override strictness can be tightened without a caller audit.

## Fourth-Pass Research Log

This pass re-read the live planner and the two nearest downstream consumers rather than broadening into the rest of the repo.

First, `strategy_plan.rs` was reopened with emphasis on the fingerprint path and override path: `fingerprint_ir` (`strategy_plan.rs:187-193`), override application (`strategy_plan.rs:232-235`), and the file-local tests (`strategy_plan.rs:406-583`). That source review matters more than the earlier packets because the live code is the contract.

Second, the downstream fingerprint and cache surfaces were rechecked. `strategy_gpu_program.rs` computes a compiled-plan fingerprint from the entire `CompiledStrategyPlan` (`strategy_gpu_program.rs:191-195`), threads `plan.ir_fingerprint_hex` into the GPU program (`strategy_gpu_program.rs:413-426`), and has tests proving that route overrides intentionally change compiled/executable fingerprints while leaving the raw IR fingerprint alone (`strategy_gpu_program.rs:539-589`). `strategy_gpu_runtime.rs` then keys its compiled bundle and executable caches off those downstream fingerprints rather than off the raw IR identity alone (`strategy_gpu_runtime.rs:85-118`, `188-220`, `304-339`, `488-558`).

Third, targeted primary-source Rust and Serde docs were checked. Rust’s `DefaultHasher` docs state that the algorithm is unspecified and its hashes should not be relied upon over releases. Rust’s `Hash` docs add that hash input from standard-library types is not a portable stable format across compiler versions. Rust’s `HashMap` docs state that each map is randomly seeded, and its iterators visit elements in arbitrary order. Serde JSON’s `Value` docs state that JSON object maps are backed by a `BTreeMap` by default and can be recursively sorted. That combination matters here because `fingerprint_ir` hashes direct `serde_json::to_vec(ir)` bytes, while the IR contains `HashMap`-backed MA param maps (as shown by the `MovingAverageSpec { params: HashMap::new() }` construction in `strategy_plan.rs:359-365`).

No new verification command was run in this fourth pass. The bundle was reviewed as source and contract analysis, not as a runnable full workspace.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The current packet was right to keep the bucket at `Small surgical refactor`, right to avoid touching route-selection heuristics casually, and right to treat fingerprint behavior as contract-sensitive. It was also right that this file’s meaning is wider than its size because its outputs feed preview/runtime/GPU-adjacent flows.

It was also correct in spirit that unknown override keys are not harmless. Silent override misses are a real contract choice and deserve either documentation or explicit handling.

### What it missed: the sharper fingerprint problem is canonicality, not only `DefaultHasher`

The biggest miss is that the current fingerprint concern is not merely “`DefaultHasher` is not stable across releases.” The more immediate issue is that `fingerprint_ir` hashes the raw bytes from `serde_json::to_vec(ir)` (`strategy_plan.rs:187-193`), while the IR includes `HashMap`-backed MA parameter maps (`strategy_plan.rs:359-365`). That means the hash input can vary with map iteration order even before any cross-release `DefaultHasher` question is considered.

That matters more than the earlier packets suggested because it is a correctness-quality issue for “same semantic IR, same identity” within the planner’s own contract surface. It is also a narrower fix than a full hash-algorithm migration. Canonicalizing the serialized input first would address the strongest current weakness without changing route policy or widening the API.

This is still not a free change. Because the compiled-plan fingerprint is derived from serializing the entire `CompiledStrategyPlan` (`strategy_gpu_program.rs:191-195`), and because that plan includes `ir_fingerprint_hex`, any change to IR fingerprint generation will cascade into `compiled_plan_fingerprint_hex`, `executable_fingerprint_hex`, and the in-memory GPU executable cache keys. So the prior packets were directionally right that fingerprint changes have runtime consequences. The missed refinement is that the *reason* to touch fingerprinting now is canonicality of the input, not a premature push toward durable public IDs.

### What it underweighted: the fingerprint model is intentionally layered

The adjacent consumer files show a deliberate three-layer identity model:

- `strategy_plan.rs` emits `ir_fingerprint_hex` as the IR-level identity.
- `strategy_gpu_program.rs` derives `compiled_plan_fingerprint_hex` from the fully compiled plan and then derives `executable_fingerprint_hex` from the ABI-relevant GPU shape.
- `strategy_gpu_runtime.rs` keys its caches off the downstream compiled/executable identities, not off the raw IR token alone.

That layering is good. It means route overrides are allowed to preserve the IR identity while still changing compiled/executable cache identity, and the tests explicitly lock that behavior down (`strategy_gpu_program.rs:539-589`, `strategy_gpu_runtime.rs:488-558`).

The earlier packets discussed “fingerprint semantics” somewhat monolithically. The more precise review is that this layering should be preserved, and any hardening should respect it. In particular, replacing `ir_fingerprint_hex` with a “stable public ID” story would be overreach for this pass.

### What it underweighted: the more serious override risk is unchecked route promotion

The earlier packets focused on unknown override keys. That is real, but it is not the only override hazard, nor the one with the clearest GPU/runtime consequence.

`compile_strategy_plan` applies any supplied override route verbatim when the indicator slot id matches (`strategy_plan.rs:232-235`). There is no check that the override route is one the planner would ever select for that indicator. Downstream, `strategy_gpu_program.rs` copies that route directly into GPU indicator slots and ABI encoding (`strategy_gpu_program.rs:229-283`, `344-354`). The runtime then treats the resulting executable fingerprint and cache key as legitimate (`strategy_gpu_runtime.rs:304-339`).

So the current contract is not just “unknown keys are ignored.” It is also “known keys are trusted completely.” That means an override can ask for route shapes the default planner would never produce on its own. The bundle does not prove that every such promotion is supported end to end, especially for legacy indicators whose default route set is intentionally narrower (`strategy_plan.rs:109-119`). That is a more important risk to call out than the earlier packets did.

### What the earlier packet slightly overstated: unconditional unknown-key rejection is not obviously safe yet

The first-pass packet proposed validating that all override keys are consumed before returning a plan. That is a reasonable instinct, but it is not yet strongly evidenced as the safest immediate change.

This bundle does not include the non-test call sites for `route_overrides`. Without that caller inventory, hard-failing on unused override keys could regress internal flows that build a superset override map, reuse one override map across profile variants, or intentionally pass dormant experimental entries. In other words, the earlier packet was right to identify the ambiguity, but a blanket “make this an error” recommendation was slightly stronger than the evidence supports.

The safer bounded recommendation is to either document and test the permissive behavior explicitly, or add a separate strict-validation/helper path that higher-level callers can opt into once their behavior is audited. That keeps `compile_strategy_plan` conservative while still making the contract less surprising.

## Implementation Priority

### Safe-now hardening

The third-pass bucket should remain `Small surgical refactor`, but the priority order should change.

The strongest safe-now hardening is fingerprint-input canonicalization, not a planner redesign. Concretely, if this file is changed now, the most justified behavioral change is to canonicalize the IR’s JSON object ordering before hashing so that semantically equivalent IR payloads do not produce different fingerprints merely because of `HashMap` ordering. Because planning is setup-time work rather than a hot loop, the extra serialization/canonicalization cost is unlikely to be performance-sensitive in practice.

That change should be paired with an explicit regression test that constructs equivalent IRs with different MA-param insertion orders and asserts equal `ir_fingerprint_hex`. Without that test, the file would still be relying on an unstated property that the current code does not actually guarantee.

The next safe-now hardening is documentation and tests, not stronger control flow. `compile_strategy_plan` should document three points locally: that `ir_fingerprint_hex` is an opaque IR-level identity token rather than the sole executable cache key; that override keys target trimmed indicator slot ids, not dynamic source ids; and that overrides are trusted escape hatches whose values are not capability-checked in this function.

For override handling, the bounded improvement is to add tests that lock down the current semantics before changing them. One test should cover unused/unknown key behavior. Another should cover whether route overrides are intentionally trusted even when they request a non-default route shape. Those tests may reveal that a stricter helper is warranted later, but they should come before changing the contract.

### Larger redesign still deferred

A true stable public identity story is still deferred. If the product needs fingerprints that remain durable across compiler versions, crate versions, persisted artifacts, or external tooling, then this area needs a deliberately versioned digest contract rather than `DefaultHasher` over serialized Rust data. That is broader than this pass because it would alter every fingerprint layer and any external consumer that stores or compares them.

A more explicit override contract is also still deferred. If `route_overrides` is going to be promoted beyond an internal escape hatch, the better long-term shape is a typed validation/reporting step that can distinguish unused keys, illegal route promotions, and benign CPU-forcing overrides without turning the base planner into an opinionated policy gate.

Finally, if `CompiledStrategyPlan` is going to function as a persistence or IPC contract rather than a crate-exported in-memory data bag, it likely needs its own validation boundary. Today the structs are public and deserializable via `lib.rs` re-exports (`lib.rs:273-277`), while downstream GPU compilation mostly assumes planner-produced invariants. Tightening that properly is not a small-surgical change.

## Verification If Changed

This fourth pass did not execute workspace commands, so the items below are the required follow-up verification, not commands already run.

At minimum, rerun:

- `cargo test -p ta_app_core strategy_plan::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`

If fingerprinting changes, add and run a new regression test that proves equivalent IRs with different MA-param insertion orders yield the same `ir_fingerprint_hex`.

If override behavior changes, add and run explicit tests for:

- unused/unknown override keys
- override keys that differ only by surrounding whitespace
- override values that request a route shape the default planner would never select for that indicator

If the fingerprint output format or meaning changes, any preview/snapshot tests or artifact-shape tests that surface plan/program fingerprints should also be rerun, because the change will propagate into the compiled-plan and executable fingerprint layers.

## Regression And Feature-Preservation Constraints

Do not change default route-selection policy in this pass. The file-local concerns are about fingerprint soundness and override contract clarity, not about rebalancing CPU versus `GpuKernel` versus `GpuSingle`.

Do not collapse the existing layered identity model. The distinction between IR identity, compiled-plan identity, and executable identity is useful and already reflected in downstream tests. A hardening change should preserve that layering even if the raw hash inputs are canonicalized.

Treat any fingerprint-generation change as compatibility-visible. Even if the code change is small, it will alter emitted fingerprint strings in `CompiledStrategyPlan`, `CompiledGpuPredicateProgram`, `CompiledGpuPredicateProgramAbi`, and `GpuStrategyExecutable`, and it will alter the in-memory GPU cache keys derived from the downstream layers. That is acceptable only if callers treat those values as opaque and ephemeral.

Do not turn unknown override keys into hard errors without a caller audit. The current bundle does not show enough use-site evidence to prove that permissive behavior is accidental.

Do not assume route overrides are harmless just because they are internal. In the current code they are trusted enough to affect GPU ABI shape. If higher layers ever expose them through IPC or persisted manifests, validation must arrive before exposure.

## Fourth-Pass Conclusion

`Bucket unchanged.` `strategy_plan.rs` should remain in `Small surgical refactor`.

The current third-pass packet got the bucket right and was right to avoid a wider planner/runtime rewrite. Where it needs sharpening is priority. The most important missed issue is not generic documentation debt; it is that `fingerprint_ir` currently hashes non-canonical serialized input, which is a better-motivated target than changing route heuristics or broadening public-ID promises. The earlier packet also underweighted that route overrides are fully trusted on the value side, which is the larger correctness and GPU-ABI risk, while slightly overstating how safely unknown-key rejection can be imposed without a caller audit.

So the conservative fourth-pass recommendation is: keep the bucket, preserve route policy, preserve the layered fingerprint model, and focus any immediate code change on canonical fingerprint input plus tighter documentation/tests around trusted override semantics. Broader stable-ID work, override redesign, or compiled-plan validation should remain deferred.
