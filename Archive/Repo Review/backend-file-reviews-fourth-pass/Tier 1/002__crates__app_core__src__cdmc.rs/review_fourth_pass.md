# Fourth-Pass Review: `crates/app_core/src/cdmc.rs`

- **Review date:** 2026-04-15
- **Source file:** `crates/app_core/src/cdmc.rs`
- **First-pass packet:** `review__first_pass.md`
- **Second-pass packet:** `review__second_pass.md`
- **Third-pass packet:** `review__third_pass.md`
- **Third-pass action bucket:** `Larger refactor but defer`
- **Fourth-pass action bucket:** `Larger refactor but defer`
- **Packet status:** `Reviewed`

## Why This File Got A Fourth Pass

The third pass made a useful challenge to the earlier packets by refusing to treat `cdmc.rs` as a purely local strategy helper. That part was right. This file sits on a public config surface, contains CPU/GPU signal builders, and exports device-sweep helpers that can shape downstream GPU behavior.

The reason a fourth pass was still justified is that the third pass appears to have over-promoted silent GPU fallback to the top issue without fully pressure-testing the other public helper contracts in the file. After re-reading the live source and the upstream indicator docs together, the sharper unaddressed problem is not only strategy semantics. It is that the file exposes a period-sweep helper API whose warmup contract cannot be exact for the indicators it already serves.

So the current defer bucket still stands, but it should stand for a different reason than the third-pass packet gave.

## Fourth-Pass Research Log

I re-read the live `cdmc.rs` first, then the third-pass packet, then the earlier review history to see where the bucket changed and why. I used `crates/app_core/src/lib.rs` only to confirm that `cdmc` remains a public module and that `CdmcStrategyParams` is still part of the public app-core surface.

I then checked the upstream indicator contracts that matter most for this file:

- VectorTA Correlation Cycle docs and docs.rs source, to verify what `state` means and when `angle` and `state` first become finite.
- VectorTA DPO docs, to verify DPO warmup semantics.
- VectorTA Decycler docs, to verify its minimum valid period and warmup semantics.
- VectorTA MAMA docs, to confirm warmup expectations and the CPU reference path.
- Rust API Guidelines and Serde docs, only for the narrower public-contract questions around validation, defaults, and documented error behavior.

One important boundary for this pass: the bundle did not include live copies of `cdmc_gpu_kernel.rs` or the CUDA kernel file, so I did not treat any unverified claim about those files as new evidence. Where parity risk obviously exists, I call it out as a constraint rather than pretending it was freshly re-verified.

## What Fourth Pass Sharpened

### Bucket assessment

**The third-pass bucket should stand, but the third-pass rationale should change.**

The file still has larger deferred work, but the strongest reason is no longer silent GPU downgrade. The stronger reason is that `CdmcPeriodSweepDevice` exposes a single public `first_valid_for_period()` contract even though this module exports DPO, Correlation Cycle, and Decycler sweeps with materially different warmup rules, and Correlation Cycle `angle` and `state` do not even share the same warmup.

That is a structural API problem, not just a missing log line.

### What the current third-pass packet got right

The third pass was right not to bless a blind semantics change to `cycle_state`. Upstream VectorTA still documents Correlation Cycle `state` as a trend/cycling classifier, with `+1.0` up, `-1.0` down, and `0.0` cycling or indeterminate, and its implementation still derives that from angle stability rather than from a native trend-vs-mean-reversion signal. The repo is clearly layering product behavior on top of that upstream output.

The third pass was also right that any change here is CPU/GPU-parity-sensitive. Even without reopening the kernel files in this pass, `cdmc.rs` is obviously not isolated: it defines the strategy-local signal interpretation that any parity-sensitive GPU path must match.

### What the current packet missed or underweighted

#### 1. `CdmcPeriodSweepDevice.first_valid_for_period()` is structurally under-specified for the helpers this file already exports

This is the clearest missed issue in the current packet.

The method is currently hardcoded to `period - 1`. That is not an exact warmup contract for the three sweep families already exposed here:

- DPO warmup is documented as at least `max(first_valid + period - 1, back)` where `back = period / 2 + 1`, so a blanket `period - 1` is not exact for all valid periods.
- Correlation Cycle documents `angle` as first finite at `first_valid + period`, while `state` starts at `first_valid + period + 1`.
- Decycler documents warmup as `first_valid + 2`, not `period - 1`.

The most important point is not merely that the formula is sometimes wrong. It is that the type cannot be right for all current use cases because `compute_cdmc_correlation_cycle_output_sweep_cuda_device()` can return either `angle` or `state`, yet `CdmcPeriodSweepDevice` carries no output-kind metadata at all. A single exact `first_valid_for_period()` implementation is impossible under the current type shape.

That is a larger redesign issue, and it is a better reason to keep the defer bucket than the third-pass emphasis on silent fallback.

#### 2. The live source already anchors the current `cycle_state` interpretation more than the earlier packets admitted

The local unit test does not merely exercise entry/exit flow. It effectively codifies the current branch semantics. In `build_cdmc_desired_positions_enters_and_exits_on_cycle_shift`, the asserted long entry at index `2` depends on `cycle_state = -1.0` selecting the current reversion branch.

That means the repo already has at least one strategy-local test anchoring the present sign-based interpretation. So while the upstream docs still justify skepticism, changing the mapping now would be an intentional product behavior change, not a low-noise cleanup.

This strengthens the case for deferring semantics changes until CPU/GPU parity and strategy expectations are reviewed together.

#### 3. `resolve_cdmc_periods()` is a hidden strategy-policy layer that the current packet barely discussed

`validate_cdmc_params()` accepts `dpo_period = 1`, `correlation_cycle_period = 1`, and `decycler_period = 1`, but `resolve_cdmc_periods()` silently rewrites those values upward with strategy-local clamps:

- `dpo` becomes `fast_len.max(cfg.dpo_period).max(2)`
- `correlation_cycle` becomes `slow_len.max(cfg.correlation_cycle_period).max(2)`
- `decycler` becomes `slow_len.max(cfg.decycler_period).max(2)`

This matters because the upstream indicator rules are not the same across those three indicators. DPO and Correlation Cycle both accept positive periods upstream, while Decycler requires `hp_period >= 2`. So the CDMC minimum-2 clamp is partly strategy policy, not merely upstream validity mirroring.

That does not make the current behavior wrong. But it does make it under-documented, and it means that a blanket validator-tightening change would be more behavior-changing than the earlier packets acknowledged.

#### 4. The output-resolution hardening should be narrower than the earlier packet suggested

The first pass suggested making `resolve_output_id()` fail closed. That broad recommendation was directionally reasonable, but too coarse.

A better bounded hardening step is to mirror the existing `resolve_param_key()` policy:

- if an indicator has exactly one output, fallback to that output is fine and keeps compatibility posture
- if an indicator has multiple outputs and no preferred output resolves, fail with a useful error listing the available outputs

That narrower rule is the safer fit here. It avoids the real danger surface — silently binding the wrong output on multi-output indicators like Correlation Cycle — without turning single-output compatibility glue into an unnecessary break.

### What the current packet overstated

#### 1. Silent GPU fallback is real debt, but not the sharpest bucket-driving issue in this file

The current file does silently fall back from preferred-GPU indicator work to CPU work. That is worth improving diagnostically.

But the third-pass framing overstates how much of the strict backend contract should be enforced here. `cdmc.rs` is a strategy-local signal builder with opportunistic GPU acceleration, not the only user-facing route-policy authority. Converting fallback into strict failure at this layer risks regressions in `Auto`, hybrid, validation, and other reuse paths unless the caller contract is audited first.

So the right near-term recommendation is lightweight diagnostic surfacing, not “make this helper enforce strict GPU semantics.” Treat fallback as observability debt unless and until the owning route layer says otherwise.

#### 2. Immediate hard rejection of negative `phase_threshold` is more compatibility-sensitive than the earlier packets allowed

The earlier packets were right that `phase_threshold` is effectively magnitude-only because the strategy uses `abs()`.

But this field is part of a serde-backed public config surface. Tightening validation from “finite” to “finite and non-negative” would change what persisted configs and saved artifacts can replay successfully. That may still be worth doing, but it is not a free cleanup. It needs migration intent or an explicit compatibility decision.

For a conservative pass, the safer immediate step is to document and test the magnitude-only contract first, then decide whether rejection, normalization, or migration is the right follow-up.

## Implementation Priority

### Safe-now hardening

The bounded hardening slice is real, and the current packet underweighted it.

First, document and test the strategy-local policy that already exists instead of implicitly re-litigating it every pass. That includes:

- the current sign-based use of `cycle_state`
- the fixed Correlation Cycle threshold of `9.0`
- the fact that effective CDMC periods are resolved through `fast_len` / `slow_len` lower bounds and a minimum of `2`
- the magnitude-only behavior of `phase_threshold`

Second, tighten `resolve_output_id()` only where ambiguity exists. Keep single-output fallback, but fail on unresolved multi-output descriptors. That is a conservative, high-value hardening change that reduces the chance of silent semantic drift on Correlation Cycle while preserving current compatibility posture for DPO and Decycler.

Third, improve GPU fallback observability without changing behavior. A route note, debug log, or first-fallback diagnostic is enough. Do not convert fallback into hard failure in this file unless the owning backend-policy layer is reviewed too.

Fourth, add an explicit warning around `CdmcPeriodSweepDevice.first_valid_for_period()`. If the public API cannot be redesigned yet, at least stop treating that helper as exact. A documentation warning or deprecation path is safer than silently changing the returned value and hoping downstream code was not compensating already.

### Larger redesign still deferred

Two larger redesign questions should stay deferred:

1. **`cycle_state` semantics.**
   Any attempt to reinterpret Correlation Cycle `state` as trend-vs-cycling in CDMC needs a coordinated review across CPU behavior, any GPU parity path, optimization ranking, and validation expectations. The live test suite already anchors the current behavior enough that this is now a product-policy review, not a drive-by bug fix.

2. **Exact warmup metadata for GPU sweep helpers.**
   `CdmcPeriodSweepDevice` needs either indicator/output-specific warmup metadata or split sweep types. The current generic helper cannot exactly represent DPO, Correlation Cycle angle, Correlation Cycle state, and Decycler at the same time. That redesign is worthwhile, but it is not a surgical change.

## Verification If Changed

Because this pass is review-only and the bundle did not include the full repo, I did not run verification commands. If any of the safe-now hardening changes land, the minimum verification set should be:

- `cargo test -p ta_app_core cdmc -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

If CUDA-facing sweep or fallback behavior changes, also run:

- `cargo test -p ta_app_core --features cuda cdmc -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel cdmc -- --nocapture`
- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_validate -- --nocapture`

The most useful new targeted tests would be:

- a unit test that locks the current `resolve_cdmc_periods()` normalization policy
- a unit test that makes the current `cycle_state` branch semantics explicit rather than incidental
- a unit test that proves unresolved multi-output output IDs now fail while single-output fallback still succeeds
- if a deprecation or replacement path is added for `first_valid_for_period()`, tests that pin the replacement semantics per indicator/output family

## Regression And Feature-Preservation Constraints

Any follow-up change should keep these constraints explicit:

- Preserve saved-config and saved-artifact replay unless a validation-tightening change is intentionally paired with migration work.
- Preserve current CPU/GPU behavioral parity unless the semantics review is deliberately reopened.
- Do not silently change `CdmcPeriodSweepDevice.first_valid_for_period()` semantics in place. Its current behavior is likely already depended on wherever it is used, even if the contract is weak.
- Do not convert preferred-GPU fallback into hard failure here without auditing `Auto`, hybrid, and validation callers.
- If output resolution is tightened, keep the change scoped to ambiguous multi-output indicators. Single-output fallback is the lower-risk compatibility behavior.
- Treat any change to `cycle_state` interpretation or the fixed `threshold = 9.0` as metrics-sensitive and ranking-sensitive, not as documentation cleanup.

## Fourth-Pass Conclusion

The **third-pass bucket should stand**, but the **third-pass explanation should not**.

What the current packet got right is the warning not to “fix” CDMC semantics casually. What it missed is that the file also exposes a **public sweep helper contract that cannot be exact for the indicator families it already serves**. That is the stronger structural reason the defer bucket still fits.

The best next step is therefore split in two:

- **safe now:** add narrow hardening and documentation around output resolution, effective period normalization, current `cycle_state` policy, and GPU fallback observability
- **still deferred:** redesign the sweep-helper warmup contract and reopen `cycle_state` semantics only as a coordinated CPU/GPU product review

So the fourth-pass verdict is not “accept the third pass as written.” It is: **keep the defer bucket, downgrade silent GPU fallback from “top issue” to “diagnostic debt,” and recognize the under-specified warmup helper as the sharper missed problem in the live file.**
