# Fourth-Pass Review: `crates/app_core/src/strategy_gpu_program.rs`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/strategy_gpu_program.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file was worth a fourth pass because the third-pass packet changed the emphasis from hash-stability/documentation toward mirrored Rust/CUDA ABI layout risk. That was a useful escalation in seriousness, but it also needed an adversarial check: was “add explicit size, offset, and layout assertions” actually the narrowest safe next move in the live code, or did it overreach and skip smaller contract hardening already justified by the current Rust-side implementation?

After rereading the live source and its immediate owners, the current third-pass bucket should **stand**. `strategy_gpu_program.rs` is still best classified as a `Small surgical refactor`, not a redesign candidate. But the third-pass packet’s **implementation priority should change**. The live file’s sharper safe-now risks are not hypothetical Rust-side offset drift alone. They are:

- untested raw numeric ABI encodings that already have downstream consumers,
- an `executable_fingerprint_hex` contract that is broader than the compact uploaded ABI,
- an unchecked `u32 -> i32` binding encoding with a sentinel,
- and a subtle serde-default / deserialized-handle integrity risk around fingerprint fields.

Those are narrower, more local, and easier to harden conservatively than a cross-language layout assertion project.

## Fourth-Pass Research Log

This pass re-read the live `strategy_gpu_program.rs` source first, then checked the adjacent owners that actually consume its outputs:

- `crates/app_core/src/strategy_gpu_program.rs`
  - numeric encoding helpers at lines `145-181`
  - fingerprint construction at lines `184-227`
  - compact ABI lowering at lines `229-284`
  - compilation/validation path at lines `286-427`
  - tests at lines `430-615`
- `crates/app_core/src/strategy_plan.rs`
  - verified that `CompiledStrategyPlan.ir_fingerprint_hex` is the upstream IR identity and that `IndicatorRouteKind` is currently `Cpu | GpuKernel | GpuSingle`
- `crates/app_core/src/strategy_gpu_runtime.rs`
  - verified how compiled bundles are cached, how `GpuStrategyExecutableCacheKey` is formed, and how uploaded executables are probed and reused
- `crates/app_core/src/gpu_backtest_kernel.rs`
  - verified that runtime code already consumes raw ABI operand-source numbers directly during dynamic GPU validation, and that the upload probe is narrower than full cross-language ABI validation

Primary standards checked during the pass were the Rust layout reference, the `cust::memory::DeviceCopy` contract, and the `DefaultHasher` docs. Those standards support three conservative conclusions:

1. `#[repr(C)]` plus Rust-side tests can lock Rust’s own field layout, but they do **not** by themselves prove a separately maintained CUDA-side consumer still agrees.
2. `DeviceCopy` remains a POD/bitwise-copy contract, so local safety justification is still warranted.
3. `DefaultHasher` remains the wrong thing to describe as a durable cross-release fingerprint contract.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right about the file’s sensitivity. This module is still the Rust-side owner of GPU predicate compilation, compact ABI shaping, and executable/cache identity. It was also right that the current upload/probe surface is narrow. In `strategy_gpu_runtime.rs`, `ensure_uploaded_gpu_strategy_executable_for_current_thread` only checks the uploaded probe’s ABI version, indicator-slot count, and condition count before accepting the uploaded executable (`strategy_gpu_runtime.rs:269-302`). That is useful sanity checking, but it is not a full proof of cross-language ABI agreement.

The third-pass packet was also right to keep the action bucket small. There is no clear behavior bug in the live file that justifies a schema change, ABI rewrite, or hash-algorithm migration.

### What it missed or underweighted

The biggest miss is that **numeric encoding drift is a more immediate live contract risk than speculative Rust/CUDA offset drift**. `strategy_gpu_program.rs` hard-codes the shipped numeric encodings for value sources, condition ops, group modes, routes, and the optional-group sentinel at lines `145-181`. That is not abstract metadata: `gpu_backtest_kernel.rs` already relies on raw operand-source values directly during dynamic-handle validation, accepting only `0 | 3` for indicator-or-constant operands (`gpu_backtest_kernel.rs:2587-2598`). The current test set in `strategy_gpu_program.rs` checks smoke, bounds rejection, ABI-shape preservation, and fingerprint reuse (`430-615`), but it does **not** lock those numeric encodings explicitly. That is a stronger safe-now hardening target than Rust-side offset assertions.

The second underweighted issue is that **`executable_fingerprint_hex` is broader than the compact uploaded device ABI**. The executable fingerprint includes full `GpuIndicatorSlot` entries (`strategy_gpu_program.rs:200-223`), and those slots carry `indicator_id`, `indicator_kind`, `source_indicator_id`, and `output_id` in addition to `indicator_index` and `route` (`51-60`). But the actual uploaded compact ABI keeps only `indicator_index` and `route` for indicator slots (`114-116`, `232-238`). Runtime cache identity then keys directly on `executable_fingerprint_hex` (`strategy_gpu_runtime.rs:104-118`, `304-339`). That means host-only descriptive metadata can invalidate executable/cache identity even when the compact uploaded ABI is unchanged. That is not obviously wrong, and it may even be intentional, but it is a real compatibility/performance nuance that the third-pass packet did not foreground.

A third underweighted issue is the **unchecked `u32 -> i32` cast in `encode_optional_group_binding`**. The function currently maps `Option<u32>` to `i32`, using `-1` as the sentinel and a plain `as i32` cast for present values (`strategy_gpu_program.rs:180-181`). `validate_group_binding` confirms only that a binding is `< group_count` (`322-332`); it does not locally prove that the value fits in signed 32-bit space. Upstream IR limits may make this theoretical, but the local file neither enforces nor documents that assumption.

A fourth missed point is a **serde-default / deserialized-handle integrity risk**. Both compiled-program structs default `compiled_plan_fingerprint_hex` and `executable_fingerprint_hex` during deserialize (`strategy_gpu_program.rs:68-71`, `123-126`). `StrategyHandle` is itself `Deserialize + Serialize` and carries both `gpu_program` and `gpu_program_abi` as optional serde fields (`strategy_gpu_runtime.rs:46-54`). `get_or_build_gpu_strategy_executable` then forms cache/upload identity directly from `handle.gpu_program_abi.executable_fingerprint_hex` (`strategy_gpu_runtime.rs:304-315`). In the reviewed files there is no local recompute-or-reject path for blank defaulted fingerprint fields, and there is no obvious assertion that deserialized `gpu_program` and `gpu_program_abi` still match if both are present. `StrategyHandle::from_plan` creates matched pairs (`56-76`), but deserialize does not. That is a narrower and more realistic persistence/IPC contract weakness than the third-pass packet emphasized.

There is also a smaller but worthwhile clarity point: `compile_gpu_predicate_program` stores `plan_fingerprint_hex` from `plan.ir_fingerprint_hex` (`strategy_gpu_program.rs:413-418`). That means the field is actually the **IR fingerprint**, not the compiled-plan fingerprint. The runtime surfaces also carry a separate `compiled_plan_fingerprint_hex` (`strategy_gpu_runtime.rs:121-149`). The current names are survivable, but they are easy to misread and should be documented before anyone tries to “clean them up” in a serialized contract.

### What the current third-pass packet overstated

The third-pass packet is too strong when it says the first safe-now hardening move should be to “add explicit size, offset, and layout validation between Rust-side ABI structs and the CUDA owner.” That statement reads cleaner than the live code actually supports.

In the reviewed files, there is no shared generated ABI manifest, no kernel-exported offset constants, and no other single source of truth that would let a Rust-only test prove CUDA-side agreement. A Rust-side `size_of` / `offset_of` test could still be useful as a **Rust-side drift tripwire**, especially against accidental field additions to the `DeviceCopy` structs. But it would not prove the CUDA owner still agrees. Treating that as the first hardening target risks giving maintainers a false sense that cross-language ABI safety is now “covered” when it would really only be partially checked on the Rust side.

So the third-pass packet got the **existence** of a cross-language ABI concern right, but it overstated the value of a local-only assertion layer. Real cross-owner ABI validation belongs in a larger coordinated change, not in a one-file-first recommendation.

## Implementation Priority

### Safe-now hardening

The safest immediate hardening remains small and local.

First, add explicit tests that lock the shipped numeric encodings and sentinel values from `strategy_gpu_program.rs:145-181`. That includes the `GpuValueSourceKind`, `ConditionOp`, `ConditionGroupMode`, `IndicatorRouteKind`, and the `None -> -1` behavior for optional group bindings. Those tests would protect a real live dependency that already reaches into `gpu_backtest_kernel.rs`.

Second, document the three fingerprint roles explicitly where they are produced:

- `plan_fingerprint_hex` is currently the upstream IR identity,
- `compiled_plan_fingerprint_hex` is the compiled-plan identity,
- `executable_fingerprint_hex` is the current executable/cache identity and is **broader than the compact uploaded ABI**.

That documentation should also explain when schema vs ABI version bumps are required, because this file is still the place a maintainer is most likely to touch during GPU predicate changes.

Third, either guard or document the signed-binding assumption in `encode_optional_group_binding`. A conservative version is to reject values above `i32::MAX` before encoding them. If upstream IR limits already make that impossible in practice, document the dependency explicitly in this file rather than leaving it implicit.

Fourth, add a small integrity check for reused/deserialized handles: when both `gpu_program` and `gpu_program_abi` are present, assert that their compiled-plan and executable fingerprints still match before caching/uploading them. Separately, add a targeted test for the legacy/defaulted fingerprint-field case so blank identifiers do not silently become cache keys.

Fifth, keep the earlier packet’s `DeviceCopy` hardening recommendation. The current `unsafe impl cust::memory::DeviceCopy` blocks at `134-143` are plausible, but they still lack adjacent proof comments. If maintainers want a slightly stronger local enforcement path later, exploring a derive-based `DeviceCopy` route could make sense, but the lowest-risk move today is still explicit safety justification and tests rather than changing the implementation mechanism casually.

### Larger redesign still deferred

A true cross-language ABI validation layer is still worth doing, but it is **not** a first local fix in this file. That would require a shared Rust/CUDA ABI manifest, generated bindings, or kernel-exported constants so Rust can validate more than its own self-consistency.

Likewise, splitting today’s broad `executable_fingerprint_hex` into two separate concepts — one “descriptive executable identity” and one compact ABI-only fingerprint — may eventually be the cleaner design. But that would affect runtime caches, serialized metadata, and probably existing tests. It should be treated as a compatibility-aware redesign, not a surgical refactor.

The same applies to a broader deserialize-time repair/validation story for `StrategyHandle`. The fourth pass supports adding narrow mismatch guards now, but a full persistence/IPC normalization pass is a larger follow-on change.

## Verification If Changed

If any safe-now hardening lands here, verification should stay close to the live owners of the contract:

1. `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`
2. `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`
3. `cargo test -p ta_app_core gpu_backtest_kernel -- --nocapture`

If numeric-encoding tests or handle-integrity guards are added, the most important extra coverage is a representative dynamic-GPU path, because that is where raw encoded ABI values are already consumed outside this file.

## Regression And Feature-Preservation Constraints

- The third-pass bucket should **stand**. Do not escalate this to a redesign unless a real behavior bug is demonstrated.
- Do **not** change any shipped numeric encoding or sentinel value without auditing every Rust-side and CUDA-side consumer first.
- Do **not** narrow `executable_fingerprint_hex` to an ABI-only identity casually. Runtime cache keys and tests already rely on the current broader scope.
- Do **not** rename serialized fingerprint fields casually, even where the names are imperfect. Document first; rename only with an intentional compatibility plan.
- Keep CPU-only builds working. Any `DeviceCopy` hardening must preserve the current CUDA feature gating.
- If Rust-side `size_of` / `offset_of` tests are added, describe them honestly as Rust-side drift tripwires, not proof of full Rust/CUDA ABI agreement.
- Preserve the current `GpuUnified` compilation gate and existing bounds checks unless a clear contract bug is being fixed.

## Fourth-Pass Conclusion

The current third-pass bucket is still correct: `strategy_gpu_program.rs` remains a `Small surgical refactor` file, not a redesign file.

What changed in the fourth pass is the priority. The third-pass packet correctly recognized that this module is ABI- and cache-sensitive, but it put too much weight on a local layout-assertion story that cannot, by itself, prove cross-language agreement. The safer and sharper next steps are smaller:

- lock the numeric ABI encodings with explicit tests,
- document the real fingerprint roles and version-bump rules,
- guard or document the signed binding sentinel assumption,
- and add handle/fingerprint integrity checks for deserialized or reused program/ABI pairs.

A true Rust/CUDA layout-validation layer is still desirable, but it should stay deferred until it can be done as a real cross-owner contract, not mistaken for a one-file proof.
