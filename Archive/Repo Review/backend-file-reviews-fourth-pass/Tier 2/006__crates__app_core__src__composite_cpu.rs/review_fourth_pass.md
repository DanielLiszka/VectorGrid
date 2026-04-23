# Fourth-Pass Review: crates/app_core/src/composite_cpu.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/composite_cpu.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The current third-pass packet is directionally right, but too abstract for the live source. It correctly keeps this file out of a broad cleanup pass: `composite_cpu.rs` still mixes public request/result types, route selection, GPU-assisted helper paths, multiple strategy-local owners, and hot-loop optimization code, so parity and compatibility matter more than symmetry. That part should stand.

What the third-pass packet underweights is that the live file still contains a few bounded, evidence-backed guardrail issues that are safer to fix than the larger route-policy questions it keeps deferring. The most important ones are not architectural; they are contract and correctness edges in already-public surfaces. In other words, the bucket stays the same, but the implementation priority inside that bucket should change.

This fourth pass therefore keeps the current bucket, explicitly says the third-pass bucket should stand, and narrows the recommendation surface to changes that are both local and materially safer than the broader cleanup ideas left hanging from earlier packets.

## Fourth-Pass Research Log

I re-read the live source with the third-pass packet first, then checked earlier packets only where the live code suggested that a recommendation might now be too weak or too strong. The most important live regions were the public request structs (`371-548`), the base GPU-kernel route and CUDA helper binding (`1949-2148`, `12148-12300`), the main base-composite orchestrator (`10873-11576`), the base drilldown/local-stability entry points (`11579-12122`), the repeated strategy-local local-stability implementations (`5071-5914`), and the route-semantics tests (`14015-14040`, `16605-16683`, `18687-18764`).

I also checked the repo guidance to keep route-policy conclusions aligned with the current product surface rather than with older packet language. `REPO_MAP.md` and `AGENTS.md` both reinforce that this file is still a public, benchmark-sensitive composite-family owner whose route semantics should not be casually rewritten.

For external standards, the useful references were the Rust API Guidelines on dependability and naming, Rayon’s `ParallelIterator` / `try_fold` / `try_reduce` docs, and Serde’s field/container attribute docs. Those sources do not justify a broad redesign here; they mainly reinforce three narrower conclusions: validate public inputs when practical, do not over-promise semantics in names, and treat defaulted/deserialized fields as compatibility-sensitive behavior once callers rely on them.

## What Fourth Pass Sharpened

### The third-pass bucket should stand, but the implementation priority inside it should change

The third-pass `Small surgical refactor` bucket is still the right bucket. The file is too policy-heavy and too performance-sensitive for a larger cleanup pass. The third-pass packet is also right that parity and route semantics matter more than cosmetic abstraction work.

Where it falls short is that it leaves the remaining work framed mainly as “be conservative” instead of ranking the concrete live defects. There are now at least three small, local hardening changes that are safer and more important than another round of abstract route-contract discussion.

### The strongest missed correctness issue is the local-stability base-point hole

All five implemented local-stability paths build their neighborhoods by starting at `-radius`, incrementing by `step`, and stopping at `+radius`, then later derive `summary.base` by searching the evaluated points and falling back to `0.0` if the exact base pair was never present. That pattern exists in ASR (`5124-5267`), VCC (`5356-5484`), VEMR (`5573-5704`), ROCB (`5791-5914`), and the base CompositeCpu path (`11974-12089`).

That means `step` can legally skip zero. When that happens, the exact base pair is never evaluated, but the function still succeeds and reports `summary.base = 0.0`. This is a real correctness issue, not just a documentation gap. It is also more important than most of the route-label complaints in the earlier packets because it can silently falsify the main summary number for a valid public request.

This is the highest-priority safe-now fix in the file. The narrow repair is to force the base pair into the candidate neighborhood regardless of `step`, then add targeted tests using a non-unit step that skips zero, such as `radius=2, step=3` or `radius=3, step=2`.

### Base CompositeCpu drilldown still has two public-surface guardrail gaps

The base drilldown path at `11579-11760` still accepts a public pair without enforcing `fast_len < slow_len`. Strategy-local drilldown implementations do reject that shape (`4877-4878`, `4989-4990`), and base local stability does too (`11919-11920`). The base drilldown path is therefore the outlier, even though it lives on the same public analysis surface.

It also leaves `bins` effectively unclamped: `let bins = req.bins.unwrap_or(1024);` at `11733`. By contrast, the implemented strategy-local drilldown paths already clamp to `1..=8192` (`4953`, `5054`). I cannot prove from this file alone that `bins=0` is currently crashing downstream, because the curve builder lives in `ta_strategies`, but the inconsistency is still a contract weakness on a public API. At minimum it leaves the base path with a broader allocation and shape surface than the strategy-local paths already deemed safe enough to clamp.

Those two changes are safer than the broader backend-semantics debate. They are local, behavior-preserving for valid requests, and they align the base analysis path with the stricter checks already used elsewhere in the same file.

### The earlier “make route labels fully truthful” recommendation needs narrowing, not expansion

The first-pass packet argued for stricter route-truthfulness. The live tests show that recommendation needs to be narrowed before it is implemented. The file already distinguishes between three different concepts:

- `backend_used` is the actual CPU/GPU execution authority.
- `backend_route` is the requested/resolved route label.
- `route_diagnostics` and `backend_fallback_reason` carry sub-route degradation details.

That split is not hypothetical; the tests pin it. In `base_composite_gpu_kernel_request_falls_back_for_entry_overlay_mode` (`14015-14040`), the result intentionally reports `backend_used == Cpu` while keeping `backend_route == "GpuKernel->CompositeCpu"` and surfacing the actual downgrade through `backend_fallback_reason`. In `kqsr_gpu_kernel_request_reports_route_diagnostics` (`16605-16683`) and `vcet_gpu_kernel_request_reports_host_stage_route_diagnostics` (`18687-18764`), partial helper/host-stage degradations are preserved in `route_diagnostics` while the main route label stays stable.

That means a fourth-pass fix should not relabel `backend_route` to chase sub-stage truth. That would fight the live tests and likely regress UI/reporting semantics. The safer pattern is already in the file: keep `backend_route` stable, keep `backend_used` authoritative, and surface helper-level degradations in diagnostics. If base CompositeCpu gets extra observability, it should follow that pattern.

### The current packet underweights how much safer that diagnostics pattern already is

Relatedly, the third-pass packet talks about parity and route-truthfulness in general terms, but the live file now already contains the better implementation model for several strategy-local paths. KQSR, KVDF, and VCET are not merely “diagnosed”; they are diagnosed in a way that preserves the route-label contract while still reporting helper fallback. That is the concrete pattern worth extending.

The underweighted implication is that the next bounded improvement is not “rewrite route semantics.” It is “bring the base CompositeCpu helper path and the remaining standalone families up to the same diagnostics contract where feasible.” That is still not a one-file-only change for ROCB/MKCS/CARM/CDMC, so it stays behind the smaller hardening items above, but it is the right direction.

### The earlier recommendation to make `resolve_output_id` fail hard is under-supported

`resolve_output_id` (`2113-2148`) is intentionally permissive: it tries exact preferred ids, then substring-style alias matching, then falls back to the first declared output. That is clearly not the strictest possible behavior, but it is also clearly deliberate compatibility code.

Without a frozen indicator-catalog contract or tests that prove the exact output ids are stable across the supported `vector-ta` surface, switching this to exact-only matching is not a safe-now cleanup. It would turn an internal compatibility hedge into a user-visible breaking change on the CUDA-assisted path. The current packet lineage therefore overstates the safety of “fail fast on drift.”

The narrower safe-now move is to add targeted tests that pin today’s acceptable HalfTrend/MACZ/OTT output ids and, if needed, record a diagnostic when the resolver falls through to fuzzy or first-output matching. Hard failure should wait until the catalog contract is explicitly frozen.

### Early validation should move earlier, but only in the narrow places that do not rewrite error precedence

The first-pass packet was right that some cheap validation is late. The live code still prepares CUDA inputs at `10982-11021` before checking the most basic MA-count and MA-kind requirements at `11061-11071`. It also delays the `include_all` row-cap guard until after HalfTrend, MACZ, and OTT have already been computed (`11367-11373`), even though the strategy-local ASR CPU path already performs that cap check immediately after pair counting (`2872-2876`).

That said, the safe change is narrower than “move validation earlier” in the abstract. Reordering everything above backend normalization could change which error wins first for bad device selection versus bad request shape. The safer patch is to keep backend normalization where it is, then move only static MA-shape checks ahead of CUDA input preparation, and move the `include_all > 2_000_000` guard to immediately after prescan has finalized `total_pairs`. That keeps the main error ordering mostly intact while still removing wasted work.

### Analysis-backend semantics are still ambiguous, but that is not the best safe-now refactor

The base drilldown and base local-stability requests still expose a public `backend` field (`442-480`, `507-548`), but the base paths compute HalfTrend/MACZ/OTT through the CPU-only helpers (`11667-11689`, `11946-11968`). At the same time, several strategy-local analysis paths do use `backend` to select helper behavior. So the ambiguity is real, but it is not uniform.

That makes one earlier recommendation too strong: “just reject non-`CpuOnly` on the base analysis path” may be honest, but it is still a behavior change on a public request type, and I do not have the frontend/request-builder surface here to prove that callers never send the selected optimization backend back into analysis. Conversely, “just honor GPU” is also not safe-now because the result types do not carry backend route metadata, so it would introduce more silent mixed-route behavior.

The safest fourth-pass position is to keep this in the deferred bucket unless the request-builder/IPC surface is updated at the same time. It is a real contract weakness, but not the highest-value local patch.

## Implementation Priority

### Safe-now hardening

The first change I would actually make is the repeated local-stability base-point fix. Ensure the exact base pair is always evaluated before computing `summary.base`, across ASR, VCC, VEMR, ROCB, and base CompositeCpu. This is the clearest correctness issue in the live file and it has a straightforward targeted-test story.

The second change is to harden the base CompositeCpu drilldown surface, not to redesign it. Add the missing `fast_len < slow_len` validation and align `bins` normalization with the already-clamped ASR/VEMR drilldown paths. Clamping to the existing `1..=8192` envelope is safer than introducing a new error for `bins=0`, because it matches behavior already present elsewhere in this same file.

The third change is to move only the obviously late, cheap rejections. Keep backend normalization intact, but move the base MA-count/empty/kind checks ahead of CUDA input preparation, and move the `include_all` row-cap check to right after prescan has finalized `total_pairs`.

A fourth safe-now improvement exists, but I would rank it behind the three above: if a small helper-plumbing change can surface base CompositeCpu HalfTrend/MACZ/OTT CUDA-helper fallback into `route_diagnostics` without changing `backend_route` strings or widening the public result contract, that would be worthwhile. If it requires signature churn or new cross-module diagnostics plumbing, it stops being a same-pass hardening item.

### Larger redesign still deferred

Route-diagnostic unification across ROCB, MKCS, CARM, CDMC, and base CompositeCpu is still worth doing, but it is no longer a one-file cleanup. The better pattern already exists in other strategy-local modules, so this should be treated as a multi-file diagnostics-contract pass, not bundled into the smaller correctness fixes.

Freezing the dynamic-indicator output-id contract is also deferred. Until the repo is ready to state exact output ids as part of the supported catalog ABI, `resolve_output_id` should remain compatibility-biased, with tests and diagnostics added before strict failure is considered.

The base analysis `backend` contract also stays deferred. If the product wants `backend` to matter for base drilldown/local stability, that should be reflected in the request-builder and probably in the result shapes. If the product wants those routes to be CPU-only, the request surface should say so explicitly. Either direction is broader than a local patch.

Finally, the host-precompute-heavy base `GpuKernel` route remains a larger redesign item. The live code still computes MA matrices on host and uploads the regime filter before calling the GPU kernel (`1949-2077`). That is a real performance opportunity, but it is not small-surgical work and it should not be mixed with the guardrail fixes above.

## Verification If Changed

If the local-stability base-point logic changes, the existing smoke `composite_cpu_local_stability_smoke_has_base_point_and_summary` should still pass, but it is not enough. Add at least one new test on the base CompositeCpu path and one on a strategy-local path where `step` skips zero, then assert that the exact base pair is still present and that `summary.base` comes from that point rather than from the `0.0` fallback.

If the base drilldown guards change, keep `composite_cpu_drilldown_smoke_has_bins_and_values` and add two explicit checks: one that `fast_len >= slow_len` is rejected on the base path, and one that `bins=0` normalizes or rejects in the same way the chosen contract intends. Given the existing ASR/VEMR clamp posture, normalization to a minimum of `1` is the less risky behavior.

If any helper-level route diagnostics are added for base CompositeCpu, re-run the route-semantics anchors that already pin the current contract: `base_composite_gpu_kernel_request_falls_back_for_entry_overlay_mode`, `kqsr_gpu_kernel_request_reports_route_diagnostics`, `kvdf_gpu_kernel_request_reports_route_diagnostics`, and `vcet_gpu_kernel_request_reports_host_stage_route_diagnostics`. The important preservation rule is that `backend_route` and `backend_used` semantics do not drift while diagnostics become richer.

If any hot-path control flow changes beyond the early-fail movement above, re-run the existing composite CPU tests and then use the repo’s CPU perf gate. If any GPU-route plumbing changes, also use the GPU perf gate when hardware is available.

## Regression And Feature-Preservation Constraints

Do not relabel `backend_route` strings as part of this pass. The live tests already pin the distinction between `backend_route`, `backend_used`, and fallback diagnostics. Breaking that split would be a public-contract regression, not just an internal cleanup.

Do not convert `resolve_output_id` to exact-only matching in this pass. The current permissive behavior is deliberate, and there is not enough evidence in this bundle to treat exact ids as a frozen ABI.

Do not rewrite prescan behavior, `include_all` semantics, or the finalist exactification logic while addressing the smaller hardening items. Those paths are benchmark-sensitive and already called out correctly by the earlier packets.

If the local-stability base-point bug is fixed, remember that some requests will now return a nonzero `summary.base` where they previously returned `0.0`. That is a user-visible behavior change, but it is a correctness repair, not a regression. The change should be deliberate and test-backed.

If the base drilldown `bins` contract is tightened, prefer alignment with existing in-file behavior over a new error mode. The strategy-local drilldown paths already clamp; matching that is less risky than introducing a fresh rejection path on a public API.

For the analysis `backend` ambiguity, do not silently expand GPU usage on the base path without adding matching observability or tightening the public contract elsewhere. This review does not have enough request-builder/IPC context to justify that change safely.

## Fourth-Pass Conclusion

The current third-pass bucket should stand: this file remains a `Small surgical refactor`, not a larger redesign.

The third-pass packet got the broad caution right. This is still a public, parity-sensitive, benchmark-sensitive owner, and broad route cleanup would be riskier than it looks. But the current packet underweights three concrete live issues that are safer and more important than another round of abstract cleanup: the repeated local-stability base-point omission, the base drilldown guard inconsistencies, and the late `include_all` / MA-shape rejections in the base CompositeCpu orchestrator.

The earlier packet lineage also needs narrowing in two places. First, route-truthfulness should be improved through diagnostics, not by rewriting `backend_route` semantics that the tests already freeze. Second, strict `resolve_output_id` failure is not safe-now without a frozen catalog contract.

So the fourth-pass recommendation is: keep the bucket, fix the base-point hole first, align base drilldown validation/clamping next, move only the clearly late cheap rejections, and leave broader route-diagnostic unification, analysis-backend contract cleanup, output-id strictness, and deeper GPU redesign deferred.
