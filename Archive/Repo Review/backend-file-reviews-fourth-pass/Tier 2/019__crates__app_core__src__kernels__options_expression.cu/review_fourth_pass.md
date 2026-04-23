# Fourth-Pass Review: `crates/app_core/src/kernels/options_expression.cu`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/kernels/options_expression.cu`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file deserved a fourth pass because the live source is compact enough to look “obviously done,” while the actual contract surface is broader than the file alone suggests. The current third-pass packet is directionally right on the main bucket: this is still a semantics-sensitive CUDA kernel surface where redesign would be riskier than the line count implies. But the third pass also flattened several file-specific details into a generic “ABI and validation” story and, in doing so, missed the sharper contract boundaries that the live caller code now makes visible.

Re-reading the live CUDA file together with `options_expression.rs` shows that the meaningful risks here are not just kernel names and raw output layout. They are also: the positive-premium invariant that is enforced upstream before flattening into GPU tensors; the fact that the combined kernel intentionally stops at search/holdout/conservative/harsher scoring and leaves extreme stress to host-side exactification; the fact that GPU aggregate metrics are intentionally retained and only median return is enriched later; and the fact that the combined kernel is only one runtime path because the legacy per-range path still exists behind an environment gate and launch fallback.

That means the third-pass bucket should stand, but the reasoning needs sharpening. The real safe-now hardening is not generic device-side “extra probes” and not any structural kernel rewrite. It is explicit contract documentation plus options-expression-specific verification that preserves both the combined and legacy scoring routes.

## Fourth-Pass Research Log

I re-opened the live CUDA source first, then validated its claims against the actual options-expression owner code instead of treating the earlier packets as authoritative.

The most important local contract reads were in `options_expression.rs`:

- GPU flat-score inputs flatten invalid rows as `outcome_valid = 0` and `outcome_premium = 0.0`, while valid rows carry the actual premium (`255-264`).
- Candidate selection rejects `premium_paid <= 0.0` before a valid trade can survive into summary generation (`1135-1137`).
- The CPU `robust_score(...)` function uses five conceptual lanes, but when `extreme_stress` is absent it falls back to the harsher-stress score (`4395-4419`).
- The combined GPU path is optional, environment-gated, and falls back to the legacy per-range GPU path on error (`2380-2388`, `3498-3524`).
- GPU shortlist ranking happens before exactification using device-produced robust scores (`3527-3558`).
- Exactification does **not** fully recompute retained GPU aggregate metrics; it preserves aggregate-based values and enriches medians plus exact extreme stress later (`2997-3064`, `3073-3122`).
- There is already options-expression-specific GPU coverage for combined aggregate parity, GPU-vs-CPU ranking parity, and exactify retention behavior (`7180-7329`, `7336-7397`, `8506-8734`).

I also rechecked `options_types.rs` to confirm that these metrics are part of persisted result state, not just transient scoring scratch. `OptionsExpressionMetrics` and `OptionsExpressionTupleResult` serialize the same fields that the GPU path reconstructs and then preserves (`471-546`).

Externally, I rechecked the current official CUDA C++ Programming Guide, the CUDA C++ Best Practices Guide, and the Rust Reference type-layout section to keep the ABI/performance framing grounded in current primary documentation. Those sources support the broad review posture here: preserve stable call contracts, avoid performance “cleanup” without measurement, and do not overstate cross-language layout claims when the actual interface is a flat scalar/pointer kernel signature rather than a shared struct ABI.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was correct to keep this file in `Small surgical refactor`. The kernel entry names, parameter ordering, scalar widths, and buffer layouts are still lockstep-coupled to the Rust launch side. It was also correct that structural redesign is not justified by the current evidence. This is a hot, strategy-local, CUDA-critical scoring surface, and there is no clear bug here that would justify widening scope.

It was also correct that caller-level verification matters more than file-local aesthetics. This file does not own the whole product contract by itself; ranking, exactification, sealed finalist handling, and artifact persistence are all decided by the surrounding options-expression pipeline.

### What the third-pass packet missed or underweighted

The biggest miss is that the live contract is not just “kernel ABI + validation.” It is a **mixed-precision scoring contract**.

Both kernels accumulate PnL, premium, return-on-premium sums, and squared-return sums in `float`, even though the host result surface stores metrics as `f64`. The host then reconstructs `OptionsExpressionMetrics` from those float-derived aggregates and later exactifies only selected fields. In particular, the exactify pass preserves GPU-derived expectancy / total-PnL / volatility inputs and only enriches `median_return_on_premium`, then adds exact extreme-stress metrics later. That means the kernel’s aggregate math is not throwaway shortlist scratch; it leaks into retained and serialized result metrics. The current packet did not call this out clearly enough.

The second miss is that the earlier packet treated the `0.45 / 0.25 / 0.15 / 0.15` blend as a somewhat opaque four-lane heuristic. The live CPU scorer shows something more precise: the combined kernel is intentionally modeling the current **four realized lanes** while folding the absent fifth `extreme_stress` lane into the harsher lane by fallback. In CPU code, missing `extreme_stress` defaults to `harsher_stress`, so the effective weight on harsher becomes `0.10 + 0.05 = 0.15`. That makes the kernel weighting less mysterious than the first packet suggested. The right documentation change is not just “these weights are policy.” It is “these weights intentionally match the current CPU fallback semantics when extreme stress is not GPU-scored here.”

The third miss is the runtime-path story. This file owns **two** relevant GPU scoring surfaces, not one: the legacy per-range aggregation kernel and the combined search/holdout/stress kernel. The combined path is only used when holdout events exist and the env flag allows it, and the caller can still fall back to the legacy route. Any verification advice that does not explicitly keep both paths honest is incomplete.

Finally, the earlier packet underweighted the fact that the positive-premium concern is already materially constrained upstream. Valid trades are rejected when `premium_paid <= 0.0`, and invalid flattened rows are explicitly marked invalid with zero premium. That does not make the precondition unimportant, but it does mean the review should frame zero-premium handling as a **document-the-invariant** issue, not as evidence that a device-side guard is probably missing.

### What the third-pass packet overstated

The third pass leaned too heavily on Rust type-layout / cross-language layout framing for this specific file. That concern is real in Rust/CUDA integrations that pass structs or depend on `repr(C)` layout, but this kernel surface is a flat list of `unsigned int`, pointer, `float`, `int`, and `double*` arguments. The dominant ABI risk here is argument order, element width, and output-lane interpretation, not Rust struct packing. The Rust Reference is still useful background, but it is not the highest-leverage framing for this file.

The third pass also recommended generic “extra probes/assertions” at a level that is less specific than the repo now justifies. The options-expression module already contains more focused coverage than the packet implied. The missing hardening is not broad, unspecific probing. It is explicitly documenting the mixed-precision/exactify boundary and adding parity coverage for the legacy fallback route, which the current tests do not appear to exercise directly.

## Implementation Priority

### Safe-now hardening

The current bucket should **stand** as `Small surgical refactor`.

The safest immediate hardening is documentation and narrow validation, not kernel math changes:

1. Add a short contract comment above each kernel or in the paired Rust launch owner stating the invariants that the live code already depends on:
   - `event_end >= event_start` for the split kernel.
   - `holdout_end >= search_end` for the combined kernel.
   - `outcome_valid != 0` implies a strictly positive premium.
   - the combined kernel intentionally scores search + baseline holdout + conservative holdout + harsher holdout only; exact extreme stress remains a host exactify step.
   - the harsher lane’s effective `0.15` weight intentionally matches the CPU scorer’s missing-extreme fallback semantics.

2. Document the precision contract explicitly:
   - per-lane aggregates are accumulated in `float` on the device,
   - reconstructed into `f64` metrics on the host,
   - and later exactified only for median return and extreme-stress enrichment.

   That comment matters because changing the accumulation type is not a cosmetic refactor; it would change shortlist ordering and persisted metrics.

3. If the paired Rust launch code is touched, prefer `debug_assert!`-style invariant checks there rather than device-side branches in the hot loop. That keeps behavior unchanged in release builds while making the range/premium assumptions explicit near the actual call boundary.

4. Strengthen verification in the options-expression test surface rather than in generic CUDA harnesses. The most valuable missing addition is a legacy-path parity test that disables combined scoring and compares legacy GPU results against CPU or combined-GPU results on the same small surface.

### Larger redesign still deferred

A few ideas may look tempting, but they are still redesign territory and should stay deferred:

- **Switching aggregate math to `double`** or adding compensated summation. This could reduce near-tie drift and volatility reconstruction error, but it is not a free cleanup. It would change shortlist order, reconstructed metrics, serialized artifacts, and possibly kernel throughput. That needs benchmarking and ranking-baseline review, not a small refactor.

- **Pushing extreme-stress or sealed-stage scoring into this kernel family.** The live pipeline intentionally exactifies extreme stress later and then runs sealed finalist evaluation afterward. Folding more of that into GPU kernels would widen memory, launch, and result-contract scope far beyond what this packet should authorize.

- **Micro-optimizations such as loop splitting, `__restrict__`, launch-shape retuning, or alias assumptions.** Those may or may not help, but the current official CUDA guidance still argues for measurement-first tuning, and this repo already has a fallback route for the combined scorer. There is no evidence here strong enough to justify those changes now.

## Verification If Changed

If this file changes, the verification should stay options-expression-specific and should cover both GPU routes.

Minimum local confidence:

1. Build the crate with the relevant CUDA features:
   - `cargo check -p ta_app_core --features cuda,cuda-backtest-kernel`

2. Re-run the existing combined-path parity tests that are already tightly coupled to this kernel contract:
   - `cargo test -p ta_app_core fused_gpu_search_holdout_stress_aggregates_match_exact_cpu_aggregates --features cuda,cuda-backtest-kernel -- --nocapture`
   - `cargo test -p ta_app_core gpu_combined_ranking_matches_cpu_topk_on_small_surface --features cuda,cuda-backtest-kernel -- --nocapture`
   - `cargo test -p ta_app_core exactify_ranked_results_reuses_gpu_summary_metrics_and_only_enriches_medians --features cuda,cuda-backtest-kernel -- --nocapture`

3. If verification is being improved as part of the same change, add and run a legacy-path regression with `VECTORGRID_OPTIONS_GPU_DISABLE_COMBINED_SCORING=1` so the per-range kernel path and the combined kernel path stay aligned.

4. If any ranking math, accumulator precision, or lane semantics change, re-run broader options-expression GPU workflow coverage rather than assuming the small-surface tests are enough. The real risk is shortlist stability and persisted metric drift on larger or near-tied candidate surfaces.

The third-pass packet’s generic `gpu_backtest_kernel` verification suggestion is weaker than the current repo warrants. The live tests that matter are already in `options_expression.rs`, and they are more directly coupled to this kernel family.

## Regression And Feature-Preservation Constraints

Preserve the current kernel entry names, parameter ordering, and lane/output layout unless the Rust launch side moves in lockstep.

Do not add a device-side premium guard as a casual “defensive fix.” The current pipeline already rejects non-positive premiums upstream and marks invalid rows explicitly. A device-side branch would change hot-path behavior and should only be considered if the upstream invariant is intentionally relaxed.

Do not reinterpret the combined kernel’s weight blend as an isolated four-lane policy knob. In the live code it is part of a broader CPU/GPU consistency contract where missing `extreme_stress` falls back to `harsher_stress`.

Do not assume exactification fully repairs GPU approximation. It does not. It enriches medians and adds exact extreme stress, but aggregate-derived expectancy/volatility inputs remain GPU-derived. That matters for persistence, artifact comparability, and ranking reproducibility.

Do not broaden this file’s scope into sealed-stage or full-stress redesign without revisiting the caller pipeline, because the live options-expression workflow intentionally stages those later.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: this file remains a `Small surgical refactor`, not a redesign candidate.

What the third-pass packet got right is the big picture: this is a hot, ABI-sensitive, semantics-sensitive CUDA scoring surface where broad cleanup would be reckless. What it still missed is the sharper contract that the live caller code now makes obvious. The most important live facts are that positive-premium validity is already enforced upstream, that the combined kernel intentionally folds the missing extreme-stress lane into the harsher weight, that GPU aggregate metrics survive into retained serialized results with only partial exactification later, and that the legacy GPU path still matters because the combined scorer is env-gated and fallible.

So the safe-now recommendation is narrower and better supported than the current packet: document the kernel invariants and the mixed-precision/exactify boundary, keep verification centered on the options-expression tests that already exist, and add legacy-path parity coverage if this area is being touched. Larger changes such as double-precision accumulation, GPU extreme-stress expansion, or performance retuning should stay deferred until there is measurement-backed evidence and an explicit willingness to re-baseline ranking and artifact semantics.
