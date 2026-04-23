# Fourth-Pass Review: `crates/app_core/src/double_ma_backend.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/double_ma_backend.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_backend.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_backend.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_backend.rs.review.md`
- Third-pass action bucket: `Document only`
- Fourth-pass action bucket: `Safe-now hardening`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet treated this module as mostly a compatibility seam whose highest-value work was documentation and invariant preservation. That was directionally reasonable, but the live source and current route owners show a slightly stronger conclusion: this file is still small and low-throughput, yet it is not merely descriptive glue.

`double_ma_backend.rs` now sits on three active contract paths at once. First, it participates in backend selection and fallback messaging through `gpu_kernel_blockers_for_request` and `backend_kind_label` (`crates/app_core/src/double_ma_backend.rs:10-108`). Second, it directly drives DynamicDoubleMa GPU preflight through `build_strategy_capability_report_for_request`, which is called from `prepare_dynamic_gpu_execution_selection` in `double_ma.rs` (`crates/app_core/src/double_ma.rs:11669-11710`). Third, it emits route/residency labels that are serialized into optimization results and therefore can leak into UI, artifacts, or report surfaces through `indicator_route_plan` and `residency` (`crates/app_core/src/double_ma.rs:619-627`, `428-437`, `794-803`).

That combination means the current third-pass `Document only` bucket is slightly too weak. Broad refactor churn is still the wrong move, but there is at least one bounded hardening opportunity that is about behavior truth, not just comments.

## Fourth-Pass Research Log

I reread the live target file first, then checked the third-pass packet against the surrounding route owners rather than against the earlier packets. The important local findings were the unconditional `cpu_dynamic_supported: true` in `build_strategy_capability_report_for_request` (`double_ma_backend.rs:101-106`), the stale `#[allow(dead_code)]` on that same live helper (`81-82`), and the fact that the route summary emitted here is string-backed rather than enum-backed (`110-207`).

I then traced the current callers in `double_ma.rs` and `double_ma_validate.rs`. The main optimization path uses this module for kernel blocker planning, backend fallback labeling, strict GPU-residency gating, and result-shape summaries (`double_ma.rs:5218-5328`, `10000-10031`). Dynamic GPU preflight uses the capability report from this file to decide whether to stay on `GpuKernel`, fall back to `CpuOnly`, or hard-error (`11669-11710`). Validation also reuses the blocker helper when planning exact GPU candidate replay (`double_ma_validate.rs:930-975`).

To judge whether the current helper outputs were only internal or actually compatibility-sensitive, I checked the result types. `OptimizationResult` serializes both `indicator_route_plan` and `residency`, and those structures carry the string labels produced or enabled by this file (`double_ma.rs:619-627`, `428-437`, `794-803`). That makes label churn an IPC/persistence concern, not just internal cleanup.

I also checked the broader backend surface for the specific rule that matters most here: CPU support is modeled independently from kernel support in the live route owners. Auto/fallback backend resolution explicitly checks `supports_cpu_single` before choosing `CpuOnly` (`double_ma.rs:5222-5250`, `5306-5322`), the CPU optimizer hard-errors when either MA lacks CPU-single support (`11065-11077`), and hybrid kernel planning separately rejects CPU-staged routes when the required CPU implementation is missing (`12407-12416`).

Finally, I checked the current semantics against official Rust references. The `std::env::var` docs confirm that missing environment variables are returned as `Err`, which matches the current default-enabled behavior in `hybrid_backend_enabled`. The Rust dead-code lint docs confirm that `#[allow(dead_code)]` exists to suppress an unused-item lint, which makes the still-present suppression on a live helper a stale signal rather than an active compatibility requirement. The Rust API Guidelines’ predictability/dependability guidance supports keeping route labels and policy helpers explicit and test-backed when they feed larger runtime behavior.

## What Fourth Pass Sharpened

### What the third-pass packet got right

The third-pass packet was right about the general posture. This is not the place for a large refactor, and the file does deserve conservative handling because it owns policy vocabulary that other runtime surfaces depend on. It was also right to treat the `top_k <= 256` limit as a policy constant rather than a casual cleanup target; that limit feeds kernel admission logic and should not move without matching runtime evidence.

It was also correct that this file is not a throughput hotspot. None of the meaningful concerns here are about allocation, vectorization, or CUDA kernel speed. The risk is semantic drift: route truth, fallback truth, serialized labels, and validation messaging.

### What the third-pass packet missed or underweighted

The most important miss is that `build_strategy_capability_report_for_request` is not merely dead or document-like policy mirror code. The live source marks it with `#[allow(dead_code)]` (`double_ma_backend.rs:81-82`), but the current optimization owner calls it directly during DynamicDoubleMa GPU preflight (`double_ma.rs:11669-11710`). That means its fields are behavior-bearing today.

Within that live helper, `cpu_dynamic_supported` is hardcoded to `true` (`double_ma_backend.rs:101-106`). That is too strong for the surrounding codebase. Elsewhere, the route owners already model CPU support as contingent on both MAs exposing a CPU-single implementation (`double_ma.rs:5222-5250`, `5306-5322`, `11065-11077`). In other words, the repo already knows that “kernel-capable” and “CPU-capable” are separate facts. Because DynamicDoubleMa preflight uses this capability report to decide whether to emit a `CpuFallback` path (`double_ma.rs:11679-11710`), the current helper can over-advertise fallback viability and defer the real failure until the CPU executor. That is a bounded correctness/contract weakness, not a documentation-only issue.

The third-pass packet also underweighted how persistence-sensitive this module’s labels are. `backend_kind_label`, `map_hybrid_capability_label`, `map_hybrid_route_label`, `residency_tier`, and `staging_reason` all feed serializable result structures (`double_ma.rs:619-627`, `428-437`, `794-803`). Changing them is not just an internal code tidy-up; it can alter saved artifact shape, UI explanations, or report content.

A second underweighted issue is the cross-file tension around residency semantics for CPU-staged kernel-capable indicators. The repo has explicit test support for MAs that are both kernel-capable and hybrid-`CpuStaged` (`double_ma.rs:14691-14708`). The pure `GpuKernel` executor also has an explicit precomputed-input path (`double_ma.rs:14015-14045`), and there is a direct FRAMA test showing that path in use (`18223-18226`). Yet the residency summary builder later treats every resolved `GpuKernel` backend as fully GPU-resident regardless of `cpu_staged_indicators` (`10001-10028`). That mismatch is broader than this file alone, but this file’s route-summary output is one side of it. The third-pass packet talked about “defaults/omission behavior” in the abstract and missed this more concrete contract seam.

### What the third-pass packet overstated or weakly supported

The earlier packet leaned too hard on the idea that the unused `_objective` and `_score_mode` parameters are themselves a high-value problem. In the live code, that is a weak finding. The helper is only called through current route-preflight flows, and there is already a downstream test explicitly pinning that objective choice should not create a kernel-preflight blocker (`double_ma.rs:14754-14774`). Those parameters look more like compatibility-shape or call-site-symmetry placeholders than an urgent design smell. Removing them for cleanliness alone would create churn without solving the more important truth gap.

Similarly, describing `build_strategy_capability_report_for_request` as effectively dead/lightly used is no longer defensible against the live source. The stale `#[allow(dead_code)]` attribute is now evidence of drift in the review packet, not evidence that the function is unimportant.

## Implementation Priority

### Safe-now hardening

The third-pass bucket should change from `Document only` to `Safe-now hardening`.

The highest-value bounded hardening is to stop overstating CPU fallback availability in `build_strategy_capability_report_for_request`. The cleanest acceptable fix is whichever one preserves current successful fallbacks while preventing false-positive `CpuOnly` fallback claims:

Either compute `cpu_dynamic_supported` from the same CPU-single eligibility rule already used by the backend resolver and CPU executor, or move the CPU-fallback decision out of this helper and into the DynamicDoubleMa preflight caller where MA support can be checked directly. The important part is behavioral, not structural: do not return `CpuFallback` merely because GPU dynamic is unsupported if the corresponding CPU route is not actually viable.

The next safe-now hardening is smaller but still worthwhile: remove the stale `#[allow(dead_code)]` once the helper remains live. Leaving it in place is not a runtime bug, but it is a misleading maintenance signal in a route-sensitive module.

The test plan should also be tightened, but specifically around the hardening above rather than around generic cleanup. Add a focused preflight test that proves the dynamic capability report does not advertise CPU fallback when the selected MA pair lacks CPU-single support, or add the equivalent caller-level test if the fallback decision is moved outward. Keep the existing positive capability tests, and add an exact `top_k` boundary test for `256` versus `257` if that can be done cheaply in this module.

### Larger redesign still deferred

The broader residency-model issue should stay deferred. The current codebase has at least three overlapping concepts: hybrid route planning, actual execution backend, and actual host/device residency. This file emits one layer of that model, while `double_ma.rs` emits the final `GpuResidencySummary` and strict-residency enforcement. Any attempt to “fix” that by editing this file in isolation would be regression-prone.

In particular, do not rush to replace the string labels in this file with enums, and do not rename route/capability/residency values casually. Those strings are already serialized through `OptimizationResult`, so that kind of cleanup is IPC/persistence work and should be treated as a coordinated schema change.

Also defer any attempt to rework the `top_k` threshold or the `VECTORBT_ENABLE_GPU_HYBRID_KERNEL` parsing semantics. Both are current policy contracts, and neither was shown to be incorrect in the provided source set.

Finally, do not spend review capital on removing `_objective` or `_score_mode` unless a broader call-site simplification is already happening. That change is low-yield and carries unnecessary review churn for a module whose real risk is route truth.

## Verification If Changed

If the safe-now hardening is implemented, the minimum useful verification is targeted rather than broad.

Run the local `double_ma_backend` tests that already pin the current positive and blocker behavior, especially `capability_report_marks_gpu_supported_for_plain_sma_request`. Keep the existing downstream regression that proves objective choice is not itself a blocker (`kernel_preflight_allows_sortino_but_still_flags_large_topk` in `double_ma.rs`). Then add one new targeted regression covering the actual hardening: a DynamicDoubleMa or helper-level case where GPU dynamic is blocked and CPU fallback is only allowed when both MAs truly satisfy CPU-single eligibility.

Because validation reuses `gpu_kernel_blockers_for_request` (`double_ma_validate.rs:930-943`), any change that touches blocker semantics or blocker text should also run the relevant `double_ma_validate` tests or an exact-combo replay case. If only `cpu_dynamic_supported` truth is tightened and blocker strings stay unchanged, validation risk is lower but still worth a quick targeted check.

If any work touches route-summary or residency labels, verification must extend beyond unit tests. Those fields are serialized into results, so the change should be checked against saved-run/report consumers, not just compile-time behavior.

## Regression And Feature-Preservation Constraints

Preserve the current string vocabulary unless all consumers are updated together. `backend_kind_label`, hybrid capability/route labels, `residency_tier`, and `staging_reason` are not purely local implementation details anymore; they feed serializable output.

Preserve the current hybrid env toggle contract unless the project explicitly wants a breaking policy change. The current helper defaults to enabled when the environment variable is absent and only treats `"0"` or case-insensitive `"false"` as disable signals. That behavior is live and likely relied on by existing runs and tests.

Preserve the `top_k <= 256` kernel gate unless kernel/runtime evidence moves with it. That limit is part of route admission and validation messaging, not just a magic number inside a private helper.

Be careful not to “fix” CPU fallback truth by becoming too pessimistic. The right target is parity with the existing CPU eligibility rule, not blanket removal of CPU fallback behavior.

Do not attempt to solve the residency-model mismatch here by declaring every `GpuKernel` route non-resident. The provided source already shows a mixed world in which pure kernel paths may consume precomputed matrices. Untangling that requires coordinated work across `double_ma_backend.rs`, `double_ma.rs`, and likely downstream UI/report explanations.

Validation messaging is also a regression surface. `gpu_kernel_blockers_for_request` is reused in validation unsupported-context errors (`double_ma_validate.rs:930-943`), so any blocker-text changes can alter user-visible validation diagnostics even if optimization behavior stays the same.

## Fourth-Pass Conclusion

The current third-pass bucket should change.

`Document only` is no longer the best framing for `crates/app_core/src/double_ma_backend.rs`. The earlier packet was right that this file should be handled conservatively, that broad refactor churn would be counterproductive, and that the route/blocker vocabulary deserves stability. But it underweighted a live hardening target: `build_strategy_capability_report_for_request` is active preflight logic, not dead policy mirror code, and its unconditional `cpu_dynamic_supported: true` is stronger than the surrounding backend owners justify.

So the right fourth-pass call is `Safe-now hardening`, not because this file needs a redesign, but because one small behavior-truth fix is warranted now. Tighten CPU-fallback truth, remove the stale dead-code suppression, and add one or two focused contract tests. Leave the larger residency-model cleanup deferred, because that crosses file boundaries and touches serialized result semantics.
