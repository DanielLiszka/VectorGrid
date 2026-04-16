# Fourth-Pass Review: `crates/app_core/src/mkcs.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/mkcs.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The live third-pass packet was directionally correct, but it had become too abstract for a file that still carries several concrete, public, strategy-local contracts. `mkcs.rs` is exported as a public module and `MkcsStrategyParams` is re-exported from the crate root (`context__01__crates__app_core__src__lib.rs:43,125`), so the review standard here should stay closer to “public contract hardening” than to “generic parity caution.”

The third-pass packet got the big-picture call right: this file is not generic cleanup territory, defaults and fail-open behavior are compatibility-sensitive, and CPU/GPU parity matters more than aesthetic refactoring. That bucket should stand. What it missed was where the sharper edges actually are now that the live source is back on the table.

## Fourth-Pass Research Log

I re-read the live `mkcs.rs` source, the prior three review packets, the crate export surface in `lib.rs`, and the repo routing guidance in `AGENTS.md` and `REPO_MAP.md`. That was enough to verify that this file still owns MKCS parameter validation, signal generation, desired-position construction, and the CUDA sweep carriers used by the strategy-local GPU path.

I also refreshed the relevant standards surface from primary documentation:

- Rust API Guidelines, especially Dependability, Documentation, and Type Safety. The useful constraints here are that public functions should validate arguments whenever practical, public items should document important contracts, and newtypes only help when they actually encode a meaningful invariant.
- Serde field/container attribute documentation. The important reminder is that field defaults are part of the public compatibility contract, and unknown fields are ignored by default for self-describing formats unless `deny_unknown_fields` is added.
- `std::sync::OnceLock` documentation. In this file, the practical takeaway is that `OnceLock<Result<...>>` intentionally makes the first initialization outcome sticky for the process lifetime.
- NVIDIA’s CUDA best-practices guidance. The relevant constraint is unchanged: reusable intermediate GPU data should stay resident on device when downstream work can consume it there.

Those sources reinforce two conclusions that matter for this pass: keep the device-resident sweep posture, and prefer narrowly targeted validation, diagnostics, and contract tests over broad API churn.

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **stand**.

This is still a `Small surgical refactor` file, not because it is trivial, but because the high-value work is narrow and evidence-backed: earlier validation, sharper tests, and clearer contract documentation. I do **not** see enough evidence in the provided bundle to justify escalating this to a redesign packet.

The third-pass packet was right about three things.

First, defaults and selector behavior in this file are product policy, not cleanup fodder. `MkcsStrategyParams` and the period-resolution path are serialized, reused, and strategy-defining (`mkcs.rs:21-67`, `199-216`). Second, any change that touches CPU/GPU route truth has to be parity-aware. Third, the strategy-local organization is still a strength; there is no case here for pushing MKCS logic into a generic abstraction layer.

### What the third-pass packet missed or underweighted

#### 1. The GPU fallback contract is broader, and murkier, than the current packet says

The third-pass packet correctly warned against casually tightening fail-open behavior, but it described the risk too generically. In the live file, `compute_indicator_f64` does not just fall back to CPU after a CUDA execution error. It also falls back whenever the backend is anything other than `CpuOnly` **and** prepared CUDA inputs are absent (`mkcs.rs:520-568`). In practice that means the helper treats route intent as “GPU-preferred if convenient,” not “GPU-required.”

That matters because the fallback is per-call, not per-request. `compute_mkcs_breakout_signals` computes Donchian upper and lower in separate calls (`mkcs.rs:601-648`), and `compute_mkcs_compression_signals` is yet another separate call (`mkcs.rs:571-599`). A single MKCS signal set can therefore become mixed-provenance: one indicator output from CUDA, another from CPU, all without any surfaced diagnostic. The existing CUDA smoke test only proves that a `GpuOnly`-labeled request succeeds; it does **not** prove that the signal path stayed on GPU throughout (`mkcs.rs:843-881`).

That is a more precise and more important observation than the existing “CUDA failure silently falls back” framing. The safe-now move is not a hard semantic flip. It is to document this explicitly, add targeted tests, and add a low-friction diagnostic seam if the surrounding runtime has one. A hard “`GpuOnly` must error if CUDA/prepared inputs are unavailable” policy is still a larger redesign decision and should stay deferred.

#### 2. The CUDA sweep helpers silently normalize invalid input ranges

`compute_indicator_period_sweep_cuda_device` clamps invalid inputs instead of rejecting them: `period_start.max(1)`, `period_end.max(start)`, and `period_step.max(1)` (`mkcs.rs:384-430`). That means an invalid request such as `start=20, end=10, step=0` is silently rewritten into a one-row sweep rather than surfacing as an error.

That is a real contract issue because the public wrappers already return `Result`, so callers have an obvious channel for “bad sweep request.” Silent clamping changes the search surface while presenting it as a valid result. The third-pass packet did not call this out.

I would still keep the bucket unchanged because changing that behavior may affect existing callers. But this is exactly the kind of edge case that deserves either explicit documentation and test pinning, or a coordinated switch to hard errors after caller audit. It is more concrete than the packet’s generalized “fail-open behavior” warning.

#### 3. The sweep builders also trust returned device-matrix shape too much

The same helper returns `MkcsPeriodSweepDevice` without checking that the returned matrix row count matches the requested sweep cardinality (`mkcs.rs:413-429`). If the dispatch layer ever returns fewer rows than `1 + ((end - start) / step)`, the metadata says the full sweep exists while `row_index_for_period` later rejects some periods (`mkcs.rs:152-176`).

This is a good candidate for safe-now hardening because it is already on a `Result` path and a row-count mismatch would indicate an upstream dispatch or ABI contract break, not an alternative valid outcome. The current packet missed this entirely.

#### 4. The file validates signal lengths too late and in the wrong place

`compute_mkcs_compression_signals` checks only for “all non-finite,” not for output length matching the candle input (`mkcs.rs:571-599`). `compute_mkcs_breakout_signals` checks upper/lower length equality, but still does not validate either output against the input candle length (`mkcs.rs:601-648`). The first place input-length drift is handled is `build_mkcs_desired_positions`, which returns an all-flat vector on mismatch (`mkcs.rs:679-775`).

That means a likely upstream indicator-contract bug is converted into a silent strategy result instead of being surfaced at the indicator boundary where the mismatch first becomes knowable.

This is a stronger safe-now recommendation than the earlier suggestion to redesign `build_mkcs_desired_positions` around `Result`. There is no need to redesign the desired-position API first. The narrower fix is to return an error immediately from the signal-computation helpers when the series length differs from `candles.close.len()`.

#### 5. The Donchian output-resolution path has a strategy-specific invariant that is not enforced

Earlier review packets were right to be suspicious of permissive output resolution, but the sharper strategy-local issue is this: `mkcs_indicator_ids()` resolves upper and lower outputs independently via `resolve_output_id` (`mkcs.rs:342-376`), and `resolve_output_id` falls back to the first declared output on failure (`mkcs.rs:254-290`). If catalog drift ever makes both lookups miss their intended names, upper and lower can collapse to the same output ID without error.

That is materially worse than “the resolver is permissive.” It means a future descriptor change can quietly convert MKCS breakout logic into a broken but still finite computation.

A low-risk hardening step here is to assert or error when the resolved Donchian upper and lower outputs are identical. That is more surgical and more defensible than a wholesale redesign of the output-resolution helper.

Relatedly, `compute_mkcs_donchian_output_sweep_cuda_device` defaults to the upper band unless the caller passes one of a small set of lower-band aliases (`mkcs.rs:489-517`). Unknown or misspelled preferences do not error; they silently choose upper. The third-pass packet did not mention this public helper at all.

#### 6. The warmup contract is duplicated and unverified locally

`MkcsCompressionSignals` and `MkcsBreakoutSignals` both carry `first_valid` (`mkcs.rs:76-87`), but `build_mkcs_desired_positions` ignores those fields and trusts a caller-supplied `start_t` instead (`mkcs.rs:679-775`). That is not automatically wrong, but it means the file stores warmup metadata that its own most sensitive consumer does not use.

In other words, the correctness contract is split across two APIs: signal production computes `first_valid`, while desired-position generation depends on the caller to reassemble that into `start_t` correctly. That gap is exactly the kind of thing third-pass parity language can hide.

I would not redesign this right now. A checked variant or internal recomputation may be reasonable later. For this pass, `debug_assert!` coverage and targeted tests around too-early `start_t` are the right scope.

#### 7. Validation and period resolution disagree about inactive fields

`validate_mkcs_params` always rejects `stddev_period == 0` and `breakout_lookback == 0` (`mkcs.rs:180-197`), even when `map_fast_to_stddev_period` or `map_slow_to_breakout_lookback` makes those fields inactive. `resolve_mkcs_periods`, meanwhile, clamps both mapped and unmapped periods to `>= 1` (`mkcs.rs:199-216`).

That is a real contract inconsistency the third-pass packet did not mention. But it is **not** safe to “fix” casually. These fields are serialized public config, and changing validation semantics may affect persisted artifacts or frontend request assumptions. This belongs in the deferred bucket unless the wider config surface is reviewed together.

### What earlier packets overstated or weakly supported

#### 1. The `OnceLock<Result<...>>` point is real, but it is not one of the top priorities from this bundle

Yes, `mkcs_indicator_ids()` uses `OnceLock<Result<MkcsIndicatorIds, String>>`, so the first initialization failure becomes process-lifetime sticky (`mkcs.rs:342-376`). That is a meaningful policy choice.

But with the evidence available here, it is not yet a leading problem. The indicator-descriptor registry appears process-global and effectively static from this file’s perspective, so caching the first failure may be an acceptable “fail fast and stay failed” stance. I would keep this as a documentation note, not as a front-of-queue refactor item.

#### 2. “Use a stronger wrapper than `u64`” is too strong as a fourth-pass recommendation

The earlier packet’s instinct was understandable because `MkcsDeviceMatrix` and `MkcsPeriodSweepDevice` expose raw device-pointer-style information on a public surface (`mkcs.rs:113-176`). But recommending a wrapper such as `NonZeroU64` or a thin newtype is too hand-wavy unless the wrapper actually enforces a meaningful invariant.

A newtype around `u64` does **not** solve the real hazards here: pointer lifetime, ownership, layout, or device affinity. It can improve readability, but it can also create API churn with very little safety gain. The more honest near-term options are:

- document the pointer-lifetime and ownership contract clearly;
- audit whether these types can be narrowed to `pub(crate)`; or
- defer a truly opaque-handle redesign until the GPU-side caller packet is reviewed together.

So the public-surface concern is valid, but the suggested remedy was too strong for this pass.

#### 3. The stale MKCS bench is useful, but it is not the first safe-now priority

The earlier packet was right that a stale bench surface weakens performance confidence. But given the live code, the higher-value immediate work is earlier error surfacing and better contract tests. Bench repair should happen if performance-sensitive MKCS work is actually being changed; it should not outrank the correctness and observability hardening above.

## Implementation Priority

### Safe-now hardening

The current third-pass bucket should remain in place, but the implementation priority should be tighter than the previous packet implied.

The first safe-now item is earlier contract validation at the boundaries that already return `Result`. Specifically, validate that single-indicator MKCS outputs match the candle input length, and validate that CUDA period sweeps return the expected row count for the normalized period range. Those changes are narrow, behaviorally honest, and easy to verify.

The second safe-now item is strategy-specific output invariants and tests. `mkcs_indicator_ids()` should not allow Donchian upper and lower to resolve to the same output ID without surfacing an error. `compute_mkcs_donchian_output_sweep_cuda_device` should have explicit tests documenting its current “unknown means upper” behavior if that behavior is intentionally preserved.

The third safe-now item is observability and contract documentation rather than semantic inversion. Document that `compute_indicator_f64` is GPU-preferred but not GPU-guaranteed, that sweep ranges are currently normalized rather than rejected, and that `build_mkcs_desired_positions` expects a caller-validated `start_t`. Add targeted tests so those behaviors are either consciously preserved or consciously changed.

The final safe-now item is low-risk hygiene: keep the CPU-only warning cleanup from the earlier packet, because it is cheap and helps the real issues stay visible.

### Larger redesign still deferred

The more disruptive questions should stay deferred.

That includes whether a non-`CpuOnly` backend — especially `GpuOnly` — should be allowed to fall back to CPU at all; whether desired-position generation should return `Result` or derive its own start index from signal structs; whether the CUDA sweep carriers should become `pub(crate)` or fully opaque; and whether the serialized config contract should be tightened around inactive fields or unknown-field handling.

Those are real design questions, but they are bigger than this packet and easier to get wrong than the safe-now hardening above.

## Verification If Changed

At minimum, keep the current local verification surface:

- `cargo test -p ta_app_core mkcs::tests:: -- --nocapture`
- `cargo test -p ta_app_core mkcs_optimization_smoke_runs -- --nocapture`
- `cargo test -p ta_app_core mkcs::tests::compute_mkcs_signals_gpu_smoke_runs_with_stddev_and_donchian_surfaces --features cuda -- --nocapture`

If the safe-now hardening above is implemented, I would add targeted tests for:

- output-length mismatch surfacing in `compute_mkcs_compression_signals` and `compute_mkcs_breakout_signals`;
- Donchian upper/lower output-resolution distinctness;
- sweep row-count validation against the requested period range;
- documented fallback behavior when a non-`CpuOnly` backend lacks prepared CUDA inputs;
- too-early `start_t` behavior for `build_mkcs_desired_positions`.

If fallback semantics, CUDA sweep metadata, or public CUDA carrier visibility change, then the higher-cost CPU/GPU parity checks from the earlier packet should still be run, because those changes are route-sensitive rather than purely local.

I would only prioritize `cargo bench -p ta_app_core --bench mkcs_opt --no-run` in this pass if the bench itself or the performance-sensitive MKCS route is actually touched.

## Regression And Feature-Preservation Constraints

Do not regress the current device-resident sweep posture. The CUDA best-practice direction still points the same way, and nothing in this pass justifies host-staging intermediate MKCS sweeps.

Do not silently convert the current GPU-preferred signal path into a hard-error path. If `GpuOnly` or other non-CPU backends are going to become strict requirements instead of preferences, that should be treated as an explicit route-policy change, not hidden inside “cleanup.”

Do not casually tighten the Serde surface. Adding `deny_unknown_fields`, changing defaults, or altering validation of inactive mapped fields could break persisted configs or IPC payloads in ways that this bundle does not let me safely audit.

Do not treat thin pointer wrappers as a free safety win. If public CUDA carrier exposure changes, make it because visibility was actually too broad or because a new abstraction truly enforces ownership and lifetime, not because a renamed `u64` looks cleaner.

Keep the strategy-local layout. Nothing in the live file suggests that generic abstraction would reduce risk here.

## Fourth-Pass Conclusion

The third-pass bucket should **stand**: `mkcs.rs` remains a `Small surgical refactor` file.

What the current third-pass packet got right was the high-level posture: this file is compatibility-sensitive, route-sensitive, and not a candidate for broad cleanup. What it missed was where the live source is still concretely weak.

The most useful fourth-pass sharpening is not a redesign. It is a narrower implementation queue:

1. validate signal lengths earlier;
2. validate CUDA sweep shape against the requested range;
3. enforce or at least test the Donchian upper/lower distinct-output invariant;
4. document and test the actual fail-open behavior around GPU preference and `start_t` preconditions; and
5. keep the lightweight warning cleanup.

Larger redesign questions — strict GPU-only semantics, public CUDA carrier narrowing or opaque-handle redesign, and serialized-config policy changes — are still deferred. Acting on them now would risk regressions that are not justified by the evidence in this bundle.
