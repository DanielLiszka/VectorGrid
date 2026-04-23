# Fourth-Pass Review: crates/app_core/src/strategy_dynamic.rs

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/strategy_dynamic.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Safe-now hardening; larger redesign still deferred`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file got a fourth pass because the current third-pass packet was directionally right about one thing and too defer-heavy about another. It was right that `strategy_dynamic.rs` is not a good candidate for a broad cleanup refactor yet. The lazy CPU fallback path, route-sensitive staging, and GPU/runtime coupling are still semantics-sensitive enough that a “make it cleaner” pass would be easy to get wrong (`strategy_dynamic.rs:70-166`, `352-495`, `963-1292`).

What changed on reopening the live source is the next-action call. The third-pass bucket should **change**, not stand. The file still has larger redesign debt, but there are now several bounded hardening moves that are safer and more urgent than waiting for a future extraction. The strongest ones are not cosmetic. They are contract-accuracy and failure-mode issues on exported or semi-public surfaces.

This pass also deliberately re-centered on the live code and the local context files rather than on prior packet wording. The earlier packet’s scheduler-sensitive test evidence may still be valid, but this bundle does not include the full workspace, so I did not treat those older test runs as fresh proof. The stronger fourth-pass findings are visible directly in the code.

## Fourth-Pass Research Log

1. Re-read the live `strategy_dynamic.rs` source with focus on four areas: preview/backend resolution, lazy CPU fallback, dynamic-indicator staging, and the two Double-MA CPU evaluators.
2. Cross-checked `strategy_plan.rs` to confirm what the compiled plan actually fingerprints and how indicator order and route shape are compiled (`context__02__crates__app_core__src__strategy_plan.rs:84-107`, `187-199`).
3. Cross-checked `strategy_gpu_program.rs` and `strategy_gpu_runtime.rs` to separate IR identity from compiled-plan identity and executable identity, and to verify what the runtime cache actually keys on (`context__03__crates__app_core__src__strategy_gpu_program.rs:191-226`, `413-425`; `context__04__crates__app_core__src__strategy_gpu_runtime.rs:85-117`, `121-142`).
4. Re-read the first-, second-, and third-pass packets only as history. Where the live source and the prior packet emphasis diverged, this review followed the live source.

## What Fourth Pass Sharpened

### 1. The current defer bucket is too strong; there is a bounded hardening slice now

The third pass treated this file as “larger refactor but defer” largely because the obvious debt is structural. That structural debt is real, but it is no longer the most actionable thing to say. There are smaller changes available now that do not require re-architecting the module: tighten one panic edge inside a fallible API, lock down evaluator parity, and correct public-facing naming around fingerprints and GPU eligibility.

That means the file should not be recategorized as “ready for broad refactor.” It should be recategorized as “do targeted hardening now; keep larger redesign deferred.”

### 2. `plan_fingerprint_hex` is a public-contract naming problem, not just an internal hash heuristic

`DynamicDoubleMaStrategyIrPreview` exports a field named `plan_fingerprint_hex` (`strategy_dynamic.rs:53-61`). The resolver populates it from `plan.ir_fingerprint_hex` (`strategy_dynamic.rs:412-418`). In `CompiledStrategyPlan`, that field is explicitly the **IR** fingerprint, not a fingerprint of the compiled route shape (`strategy_plan.rs:84-107`, `187-193`).

That distinction matters here because nearby runtime code already treats these identities as different things. The GPU program layer computes `compiled_plan_fingerprint_hex` from the serialized compiled plan and a separate `executable_fingerprint_hex` for the ABI/runtime shape (`strategy_gpu_program.rs:191-226`, `413-425`). The GPU runtime cache keys on those compiled/executable identities, not on the IR fingerprint alone (`strategy_gpu_runtime.rs:85-117`, `121-142`).

So the third-pass packet was right to be wary of hash-derived identity language, but it underweighted the most concrete issue: the exported preview field name suggests “resolved plan identity” while the code returns “IR identity.” A frontend or IPC consumer could reasonably assume that a GPU-to-CPU fallback or a route-shape change would alter `plan_fingerprint_hex`. In this file, it will not.

This is a safe-now hardening target. The safe move is additive, not breaking: keep `plan_fingerprint_hex` for compatibility, document it as an IR fingerprint alias, and add a clearly named field if a stronger compiled-plan identity is needed. Renaming or reinterpreting the existing field in place would be an avoidable IPC regression.

### 3. The file currently mixes “dynamic product v1” limits with much narrower runnable GPU-kernel eligibility

The third-pass packet noticed that GPU-v1 wording leaks into CPU-side validation. The sharper issue is broader than that.

`enforce_dynamic_double_ma_product_caps` is called before backend resolution (`strategy_dynamic.rs:352-360`). It rejects plans above 12 indicators, 24 conditions, 8 groups, or 8 conditions per group, and it also imposes a bidirectional short-entry binding rule (`strategy_dynamic.rs:169-257`). Those errors are all worded as “dynamic gpu v1” limits.

Later, the actual GPU-handle validator is far narrower. `validate_dynamic_double_ma_gpu_handle_v1` only accepts:

- `DoubleMa` profile
- no `halftrend_filter`
- `ExitMode::Cross`
- exactly **2** indicator slots
- only indicator/constant operands in the GPU ABI

See `strategy_dynamic.rs:298-349`.

That is not a minor wording wrinkle. It means the code currently uses one GPU-branded vocabulary for broad **product-surface** caps and another, much narrower check for what the GPU runtime can actually execute. The third-pass packet did not call this out strongly enough.

Two consequences matter:

First, CPU-requested previews are still fenced by GPU-branded product caps before any CPU fallback happens. Second, GPU-requested previews can pass the product-cap stage and then still fall back because the runnable GPU shape is much smaller than those earlier messages imply.

The safe-now move here is not to broaden CPU or GPU behavior. It is to make the contract honest. The caps need naming and tests that distinguish “product v1 surface” from “GPU-kernel v1 eligibility.” The risky move would be to loosen the caps or shift them later in the pipeline without coordinating the frontend builder, preview behavior, and downstream validation expectations.

### 4. `prepare_runnable_dynamic_double_ma_plan` still carries a hidden panic path inside a fallible API

`prepare_runnable_dynamic_double_ma_plan` returns `Result<_, String>` (`strategy_dynamic.rs:444-495`). But on the GPU-resolved path it still has a fallback that reconstructs a `StrategyHandle` with `expect("resolved GPU preview should produce a GPU handle")` (`strategy_dynamic.rs:477-482`).

That `expect` is effectively dead under the current resolver because `resolve_dynamic_double_ma_strategy_ir_preview` already builds and stores a handle on the accepted GPU path (`strategy_dynamic.rs:398-409`). Even so, leaving it in place is unnecessary contract debt. `StrategyHandle::from_plan` is fallible by design because GPU bundle compilation/cache work can fail (`strategy_gpu_runtime.rs:56-76`). A public function that already returns `Result` should not keep an internal panic edge for an invariant that can be checked explicitly.

This is a clean safe-now hardening item. Remove the reconstruction fallback, or turn the unexpected `None` into an `Err` with a clear invariant message. That keeps the public API fully fallible and avoids a future regression if the resolver/handle handoff changes.

### 5. Double-MA now has two CPU evaluators with different failure semantics, and the file does not lock them together

For `StrategyProfileKind::DoubleMa`, the file exposes two distinct CPU evaluation routes:

- `eval_dynamic_plan_metrics_cpu_with_candles` builds desired positions and feeds `eval_asr_metrics_from_desired` (`strategy_dynamic.rs:1615-1641`).
- `eval_dynamic_plan_metrics_cpu` routes Double-MA through `eval_dynamic_double_ma_metrics_cpu_generic` (`strategy_dynamic.rs:1664-1688`, `1896-2373`).

Those are not just two call wrappers over the same core evaluator.

The desired-position path uses error-returning loaders and group evaluators that surface missing slots, unsupported indicator kinds, and out-of-range bar access as `Err` (`strategy_dynamic.rs:1314-1488`, `1491-1613`). The generic evaluator uses `f64::NAN` sentinels and turns non-finite values into `false` conditions (`strategy_dynamic.rs:1707-1783`). In other words, the same structural mismatch can become either a hard error or a silent “no signal” depending on which public entrypoint the caller used.

The current test suite does not lock these two public entrypoints together for Double-MA. The file has a Double-MA parity test for `eval_dynamic_plan_metrics_cpu` against the static evaluator, and it has a `_with_candles` parity test for composite profiles, but it does not prove that the two Double-MA entrypoints agree on metrics or on failure behavior (`strategy_dynamic.rs:2576-2708`, `2921-3010`).

This is a stronger missed issue than the third-pass packet’s cache-key concern. A safe-now hardening step exists: add parity tests that compare the two Double-MA public entrypoints on the same plan and data, including trade-tail fields, before attempting any behavioral unification. The larger redesign—collapsing these into a single engine or a shared validation layer—should still be deferred.

### 6. The seeded-entry path assumes fast/slow are slots 0 and 1, but the rest of the file resolves by kind or ID

Both Double-MA evaluation routes seed the initial entry decision by directly reading slots `0` and `1` when the plan has at least two indicators (`strategy_dynamic.rs:1559-1575`, `2006-2019`). Elsewhere, the file is more careful: it resolves fast/slow by `IndicatorKind` or by canonical `indicator_id` names (`strategy_dynamic.rs:1337-1378`, `1719-1731`).

That mismatch matters because `compile_strategy_plan` preserves the input indicator order from the IR (`strategy_plan.rs:208-215`). Nothing in the live bundle shows a local guard in `strategy_dynamic.rs` that enforces “fast_ma must be slot 0 and slow_ma must be slot 1.” If `validate_strategy_ir` guarantees that elsewhere, the guarantee is off-file and currently implicit here. If it does not, the seeded-entry logic is a latent correctness bug for manually authored or externally reordered IR payloads.

This is another bounded hardening opportunity. The safe move is either to add a targeted test for reordered indicator order and then resolve the seed inputs by kind/name, or to add an explicit local assertion/validation that the compiled plan preserves the required slot order. The risky move would be to silently change seeded-entry behavior without first checking whether upstream validation already treats the order as part of the contract.

### 7. The third-pass packet slightly overstated the cache-key issue and underweighted the observability truth issue

The earlier packet was right that the staging cache uses heuristic hashes (`strategy_dynamic.rs:856-919`). But the current implementation’s slot cache is created inside each staging call (`strategy_dynamic.rs:969-972`, `1181-1183`). That makes cross-run or cross-dataset collision risk much less central today than the packet implied. The bigger present-day risk is misleading semantics on exported or diagnostic surfaces.

Two examples:

`DynamicIndicatorStagingProgressFn` reports the slot’s planned `IndicatorRouteKind` (`strategy_dynamic.rs:50-51`, `924-935`). In the CUDA path, the code can fall back to CPU because prepared inputs are unavailable or a CUDA compute attempt fails, yet the progress callback still receives the original planned GPU route (`strategy_dynamic.rs:1236-1288`). That is diagnostic drift, not a compute bug, but it matters more than the per-call cache fingerprint strength.

Similarly, `metrics_shadow_equivalent` only compares a subset of `Metrics` fields (`strategy_dynamic.rs:620-633`) even though this file now produces a much richer trade-tail metric surface (`strategy_dynamic.rs:2346-2372`). That is another observability blind spot the third-pass packet did not surface.

Neither of those should trigger a larger refactor right now. But both deserve more emphasis than the packet gave them.

## Implementation Priority

### Safe-now hardening

1. **Remove the dead panic edge in `prepare_runnable_dynamic_double_ma_plan`.** Keep the API fully fallible and do not leave a hidden `expect` in the GPU path.
2. **Add targeted tests before changing behavior.** The minimum useful additions are:
   - a Double-MA parity test between `eval_dynamic_plan_metrics_cpu` and `eval_dynamic_plan_metrics_cpu_with_candles`
   - a reordered-indicator test that proves or disproves the local slot-order assumption
   - a GPU-requested preview test showing what happens when a plan has extra indicator slots or non-indicator/non-constant operands
3. **Harden public contract naming without breaking it.** `DynamicDoubleMaStrategyIrPreview.plan_fingerprint_hex` should be documented as IR identity, not resolved-plan identity. If a stronger fingerprint is needed, add a new field rather than redefining the old one.
4. **Make the GPU messaging honest.** Distinguish product-surface caps from actual GPU-kernel eligibility in docs, tests, and error text. Do not broaden the accepted shape as part of that cleanup.
5. **Document diagnostic truth limits.** Today the staging progress callback reports planned route, not actual fallback route, and shadow comparison only covers a core metric subset.

### Larger redesign still deferred

The file still has a real structural redesign story, and that part of the third-pass packet was correct. The following should remain deferred until contracts are explicit and tests are in place:

- splitting preview/policy, staging/cache, telemetry, and evaluation into separate modules
- unifying the two Double-MA evaluators into one engine with one error policy
- redesigning the progress callback so it can report actual compute route and fallback cause
- separating product caps from GPU-kernel eligibility in a way that also updates the Tauri command surface, frontend preview consumers, and request-builder expectations together

## Verification If Changed

This fourth pass was a static bundle review. The full workspace was not included, so I did not re-run Cargo or frontend commands here.

If this file changes, re-run the same high-signal verification surfaces the earlier packet named, and add the new targeted checks that this pass exposed:

- `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture`
- `cargo test -p ta_app_core strategy_dynamic::tests:: -- --nocapture --test-threads=1`
- `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_dynamic -- --nocapture`

If `DynamicDoubleMaStrategyIrPreview` changes shape or field semantics, also run the frontend/command parity checks that cover the dynamic preview consumer path. If evaluator semantics change, add a regression test that compares the two Double-MA CPU entrypoints on the same data before and after the change.

## Regression And Feature-Preservation Constraints

Do not rename or remove `plan_fingerprint_hex` in place. It is exported and semantically adjacent to IPC/Tauri preview consumption. Any cleanup there should be additive first.

Do not broaden or reorder product-cap enforcement casually. The file currently applies those checks before backend fallback, and that behavior is part of the visible preview contract even if the wording is misleading.

Do not remove the lazy CPU fallback or eager-on-CPU preparation split. That part of the current design still looks intentional and performance-aware.

Treat `DynamicIndicatorStagingProgressFn` as a public contract. Reporting actual fallback route instead of planned route is probably a better design, but it is not a free internal cleanup because the callback type is exported.

If you tighten the generic Double-MA evaluator so that missing slots or unsupported series become hard errors instead of silent false conditions, treat that as a behavioral change. It is likely the right long-term direction, but it can expose latent caller bugs immediately.

If you fix the fast/slow slot-order assumption, first check whether upstream IR validation already guarantees that order. If it does, add a local assertion or test so the guarantee is visible here. If it does not, prefer a targeted correctness fix over a large evaluator rewrite.

## Fourth-Pass Conclusion

The third-pass packet got one big thing right: this file is still not ready for a broad refactor. But the third-pass **bucket** should change. The next step is not “wait for a larger redesign.” The next step is targeted hardening now.

The most important missed items were not generic technical-debt observations. They were concrete contract issues: a preview fingerprint whose name overstates what it identifies, GPU-v1 messaging that is broader than actual runnable GPU-kernel eligibility, a dead `expect` inside a fallible API, and two public Double-MA evaluators that are not yet locked together by parity tests or by consistent failure semantics.

So the correct fourth-pass call is: **do safe-now contract/test hardening now, and keep the larger module split deferred.**
