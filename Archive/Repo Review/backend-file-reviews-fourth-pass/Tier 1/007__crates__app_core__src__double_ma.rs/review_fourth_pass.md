# Fourth-Pass Review: crates/app_core/src/double_ma.rs

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/double_ma.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

`double_ma.rs` is still a high-sensitivity runtime owner: it exports public request/result DTOs, owns backend normalization, crosses CPU/GPU/hybrid routes, returns IPC-facing metadata, and now also carries several follow-up and cache hints that look more durable than they really are. The third-pass packet correctly narrowed attention to two contract issues, but a fourth pass was still warranted because some of the other suggested actions were more behavior-changing than the evidence justified, while a few public-surface mismatches were still undercalled.

The main fourth-pass conclusion is that the current bucket should stand, but the implementation priority should narrow further. The best next changes are still contract-truthfulness fixes and verification hardening, not speculative GPU cache or follow-up behavior tightening.

## Fourth-Pass Research Log

I re-read the live source around backend normalization, follow-up request/result shaping, follow-up cache token generation, public export scope, and dynamic GPU batch reuse. I also checked `crates/app_core/src/lib.rs` to confirm which surfaces are re-exported publicly and `crates/app_core/src/double_ma_validate.rs` to avoid recommending backend-shape changes that would silently desynchronize the validation side.

I also checked a small set of primary references to keep the review evidence-backed:

- Rust standard-library `HashMap` docs, which explicitly say `keys()` iterates in arbitrary order. That matters because this file currently calls the evicted key `oldest_key` even though the backing structure does not preserve insertion order.
- Rust standard-library thread-local storage docs, which clarify that TLS is per-thread and destroyed when the thread exits. That matters because the dynamic GPU batch cache is real retention risk, but it is not accurately described as a single obvious process-global leak.
- Rust API Guidelines documentation and future-proofing guidance, which reinforce that public items should be documented and that public fields/DTO shapes are real commitments.
- NVIDIA’s CUDA Best Practices Guide, which reinforces that host/device transfers are costly and that residency/materialization changes should be benchmark-gated rather than casually “cleaned up.”

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right to keep the bucket at `Small surgical refactor`, and it was right that explicit device intent is being weakened too early. `Backend::with_resolved_preferred_cuda_device` rewrites every GPU-bearing variant to `preferred_device_id.unwrap_or(0)` at `double_ma.rs:125-144`, and optimization still calls that helper before route resolution at `double_ma.rs:4988-4989`. That is a real contract bug for explicit `GpuOnly`, `GpuHybridKernel`, and `GpuKernel` callers.

The third pass was also directionally right that the follow-up DTOs over-promise self-sufficiency. Both follow-up request types duplicate MA parameter maps at top level even though `DoubleMaParamsResolved` already carries them at `double_ma.rs:440-490`. The implementations still resolve ALMA params and MA matrices from the top-level request fields, not the nested `params` copy, at `double_ma.rs:4591-4614`, `4621-4632`, `4797-4820`, and `4825-4834`. `LocalStabilityResult.base` then echoes the top-level copies back out again at `double_ma.rs:4902-4910`.

### What the current packet missed or underweighted

The explicit-device issue is broader than the third-pass packet stated. It is not only the optimization request path that gets rewritten. Follow-up execution also normalizes any supplied backend hint through `with_preferred_cuda_device()` inside `ma_matrix_with_backend_f64` at `double_ma.rs:10761-10764`. In practice that means a caller can lose an explicit follow-up `backend_hint` device selection during drilldown or local-stability work even if the original optimization path is untouched.

The third-pass packet also stopped one step short on the public `Auto` surface. The public enum shape still exposes `Auto { device_id: u32 }` at `double_ma.rs:105-121`, but that field is immediately overwritten by preferred-device resolution at `double_ma.rs:130-133` and then used only after normalization. So there are really two contract issues here, not one: non-`Auto` explicit device IDs are being clobbered incorrectly, and `Auto.device_id` itself is an underdocumented or misleading public field even if no behavior is changed right now.

The follow-up API-fidelity issue is stronger than the third-pass write-up implied because the file’s own tests already work around it manually. The shortlist-neighborhood test constructs a `DoubleMaLocalStabilityRequest` from `res.best_params.clone()` and still has to separately copy `res.best_params.fast_ma_params.clone()` and `res.best_params.slow_ma_params.clone()` into the top-level request fields at `double_ma.rs:24980-24995`. That is not just a theoretical footgun; the module’s own exact-path test scaffolding is already compensating for it.

A persistence/IPC nuance was also missed. `DoubleMaRequest.persist_followup_cache` and `BackendOptimizationResult.followup_cache_key` are public surface at `double_ma.rs:319-326` and `double_ma.rs:628-629`, and the result key can cross the IPC boundary. But the implementation is only a process-local `OnceLock<Mutex<HashMap<...>>>` cache at `double_ma.rs:43-44`, populated lazily at `double_ma.rs:7525-7536`, bounded to 16 entries at `double_ma.rs:9983-9991`, and silently treated as best-effort on load/store at `double_ma.rs:9972-9991`. That is not a correctness bug because follow-up work recomputes on cache miss, but it is a public contract and persistence-doc issue that deserves to be spelled out.

The public reset helper name is also too narrow. `reset_dynamic_gpu_preflight_cache()` clears both the preflight key cache and the thread-local uploaded dynamic GPU batch cache at `double_ma.rs:51-60`, and that helper is re-exported publicly from `lib.rs` at `lib.rs:103-105`. The current name makes it sound like a light preflight-only reset when it can also discard warmed uploaded batches.

### What the current packet overstated or should narrow

The prior packet’s recommendation to reject `fast_len >= slow_len` in drilldown should not be treated as a clear bug fix yet. Drilldown is a direct single-pair curve API: it validates MA kinds, period positivity, bounds, warmup, and then computes one fast/slow pair at `double_ma.rs:4482-4651`. It does not depend on the optimization manifold, matrix index, or neighborhood topology. Local stability is different: it explicitly models the optimizer neighborhood, rejects an invalid base point at `double_ma.rs:4678-4680`, and skips invalid neighborhood points at `double_ma.rs:4849-4852`. Tightening drilldown to the optimizer’s `fast < slow` convention would narrow a public diagnostic endpoint without clear evidence that current callers are wrong. If product/UI wants follow-up parity with optimization results, that is safer as request-builder/UI enforcement or documentation first, not an immediate backend hard error.

The follow-up cache identity concern is real, but the earlier “cheap identity signal” suggestion was too weak. The key currently omits direct candle identity and only uses `data_id`, candle count, source, MA kinds, strategy, and halftrend filter at `double_ma.rs:9840-9869`. That is enough to justify saying the cache key does not prove candle identity. It is not enough to justify a pseudo-fix like “first timestamp, last timestamp, and last close,” because those values can stay unchanged while interior bars change. The safe conclusion is narrower: either thread a real upstream dataset-version token into the key, or use a true content fingerprint if correctness matters enough to pay for it. Anything weaker should be treated as a heuristic, not a correctness fix.

The dynamic GPU batch cache concern also needs narrower framing. The cache is thread-local at `double_ma.rs:45-49`, cleared manually at `double_ma.rs:57-60`, and its key is materially stronger than the follow-up cache key: it includes the executable fingerprint, device ID, MA metadata, period sets, `first_valid`, candle length, close-buffer pointer, and first/last timestamps at `double_ma.rs:13280-13319`. Entries are inserted without eviction at `double_ma.rs:13340-13444`, so retention risk is real. But the right description is “missing eviction for potentially long-lived worker threads,” not “clearly global stale reuse” or “obvious monotonic leak across the whole app.” Any cap or per-run flush remains benchmark-sensitive because it may directly trade away intended VRAM-resident reuse.

Finally, some of the performance suggestions are still too speculative for the current bucket. Replacing follow-up cache payloads with shared ownership or rewriting the dynamic GPU finalize path to avoid materialization may be worthwhile, but both are ownership/perf-shape changes. The dynamic GPU path really does fully materialize filtered rows before pushing them back through `StreamAggregator` at `double_ma.rs:13577-13601`, yet that same materialized slice also feeds exact shortlist union/confidence/slice reranking later in the function at `double_ma.rs:13624-13760`. That makes the optimization real, but still benchmark-sensitive rather than safe-now hardening.

## Implementation Priority

### Safe-now hardening

The first priority should remain preserving explicit non-`Auto` device IDs. That change is small, semantics-focused, and locally testable: keep `GpuOnly`, `GpuHybridKernel`, and `GpuKernel` device IDs intact in `with_resolved_preferred_cuda_device`, then add a focused test that proves explicit IDs survive both optimization normalization and follow-up `backend_hint` materialization paths. That is the cleanest correctness win in this file.

The second safe-now hardening item is follow-up request truthfulness without changing the public DTO shape. The implementation should resolve MA params as “top-level request fields if present, otherwise `req.params.fast_ma_params` / `req.params.slow_ma_params`,” and `LocalStabilityResult.base` should echo the resolved values rather than the possibly-empty top-level copies. That keeps compatibility while making `DoubleMaParamsResolved` much closer to the self-contained shape it already appears to be.

The third safe-now hardening item is documentation and naming truthfulness for the public cache surfaces. `persist_followup_cache` and `followup_cache_key` should be documented as process-local, best-effort acceleration hints rather than durable persisted artifacts. `reset_dynamic_gpu_preflight_cache()` should at minimum document that it also clears the thread-local uploaded dynamic GPU batch cache. The `oldest_key` variable in `store_followup_cache_entry()` should either be renamed to reflect arbitrary eviction or moved to an actually ordered structure if chronological eviction is truly intended.

### Larger redesign still deferred

Redesigning `Backend::Auto` should be deferred. The public `Auto { device_id }` shape is currently misleading, but changing its semantics or removing that field is a public contract decision that can affect serde payloads, IPC expectations, and downstream callers. The safe-now move is documentation; the larger fix belongs in a deliberate API cleanup.

Strengthening follow-up cache identity should also stay deferred unless an upstream dataset-version token is already available. The current packet was right to distrust the present key, but this is not a place for weak fingerprints that only look rigorous. If the team wants correctness-grade cache identity, it should use a real dataset-version signal or a true content fingerprint and accept the coordination cost.

Dynamic GPU batch-cache eviction and dynamic-finalize materialization changes should remain deferred behind representative GPU perf gates. Both are plausible improvements, but both touch the VRAM-resident reuse story and can easily turn into benchmark regressions if they are “cleaned up” without measurement.

## Verification If Changed

If only the explicit device-preservation fix lands, add a focused unit test for explicit `GpuOnly`, `GpuHybridKernel`, and `GpuKernel` preservation through preferred-device normalization, and rerun the existing preferred-device error propagation test. Because follow-up paths also normalize backend hints, add one small drilldown or local-stability test that passes an explicit GPU backend hint and confirms the requested device survives normalization.

If the follow-up DTO hardening lands, add a round-trip test that builds follow-up requests from `DoubleMaParamsResolved` alone, without duplicating the MA param maps at top level, and prove that both drilldown and local stability still compute the same result. The existing smoke tests at `double_ma.rs:22914-23004` are good sanity checks to rerun after that change.

If cache naming/docs are updated without behavior changes, keep verification narrow. If follow-up cache-key semantics or eviction policy change, rerun the drilldown/local-stability smoke tests and add a direct test for cache miss/recompute behavior so the “best-effort, recompute on miss” contract stays intact.

If any dynamic GPU cache lifetime or materialization change is attempted anyway, treat it as perf-sensitive work rather than ordinary refactoring. That means rerunning the existing dynamic preflight cache reuse test at `double_ma.rs:17600-17635` and using a representative CUDA-enabled perf gate before calling the change an improvement.

## Regression And Feature-Preservation Constraints

Do not change the public `Backend` serde shape casually. Alias compatibility is already covered at `double_ma.rs:14712-14742`, and any behavioral cleanup around `Auto` has IPC and compatibility consequences beyond this file.

Do not harden drilldown by rejecting `fast_len >= slow_len` unless there is a deliberate product decision to narrow that API. The current source supports arbitrary single-pair curve inspection even though optimization and local stability intentionally stay on the `fast < slow` manifold.

Do not present `followup_cache_key` as durable persistence. Its current implementation is bounded, in-memory, and best-effort; preserve the current cache-miss recomputation behavior even if the docs are tightened.

Do not weaken route/fallback diagnostics or GPU residency reporting just to “simplify” the file. Those strings and summaries are load-bearing for debugging and public result interpretation.

Do not cap or auto-clear the dynamic GPU batch cache without a perf gate. That path is directly tied to VRAM-resident reuse and the current cache key intentionally binds entries to executable/device/candle identity details.

Do not use a weak candle-identity heuristic and then treat the result as a correctness fix. If follow-up cache identity is strengthened, it should be done with something that genuinely distinguishes dataset revisions.

## Fourth-Pass Conclusion

The third-pass action bucket should stand: `double_ma.rs` is still in `Small surgical refactor`, not rewrite territory. But the next pass should be narrower than the earlier packet suggested.

The current third-pass packet got the two most important themes right: explicit device selection is being weakened too early, and the follow-up DTOs are less truthful than they look. What this fourth pass adds is a stricter separation between safe-now hardening and larger redesign. The safe-now work is to preserve explicit non-`Auto` device IDs, make follow-up MA-param resolution truthful without breaking DTO shape, and document the real lifecycle of the follow-up and dynamic GPU caches. The larger redesigns—cleaning up `Auto`, hardening follow-up cache identity with a real version signal, and tuning dynamic GPU cache/materialization behavior—should remain deferred until they are approached as intentional API/perf work.

So the bucket is unchanged, but the emphasis should change. Fix the real contract bugs. Document the public cache semantics honestly. Defer the benchmark-sensitive GPU rewrites and the behavior-narrowing drilldown change until there is stronger product or measurement evidence.
