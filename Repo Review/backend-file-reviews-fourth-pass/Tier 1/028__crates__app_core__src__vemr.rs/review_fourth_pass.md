# Fourth-Pass Review: crates/app_core/src/vemr.rs

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/vemr.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/vemr.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/vemr.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/vemr.rs.review.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Targeted hardening now; larger redesign still deferred`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file got a fourth pass because the prior packets had converged a little too cleanly on “defer.” The architectural caution was reasonable, but the live source still needed a sharper answer to a narrower question: is there a bounded hardening slice that can be done now without reopening the bigger CPU/CUDA split?

After re-reading the live file, the answer is yes. The large redesign should still stay deferred, but the third-pass bucket is too coarse. It collapses low-risk hardening together with high-risk refactor work. In particular, the live file leaves the highest-value pure-Rust behavior mostly untested even though that behavior is local, deterministic, and cheap to lock down.

So the third-pass bucket should **change**. Not because the file is suddenly safe to restructure, but because “larger refactor but defer” understates a real safe-now hardening opportunity.

## Fourth-Pass Research Log

I re-read the live `vemr.rs` top to bottom, with special focus on five surfaces: serialized/defaulted strategy params (`34-205`), descriptor binding and one-time caching (`229-579`), CUDA sweep/device-pointer staging (`246-920`), CPU indicator materialization (`923-1119`), and the start-index / desired-position state machine (`1122-1298`). I also re-read all three prior packets and checked each high-confidence claim against the live file instead of assuming packet drift was impossible.

I used the repo guidance only to confirm contract sensitivity, not to expand scope. The repo map still places VEMR in the standalone-profile family with a GPU companion and CPU-preferred `Auto` posture, which is exactly the kind of environment where parity, route truthfulness, and silent semantic drift matter more than aesthetics.

I also rechecked the relevant official Rust and Serde docs before making any recommendations. Serde still treats missing fields with `#[serde(default = "...")]` as live deserialization behavior, so the omission-driven fields on `VemrStrategyParams` are active compatibility surface, not just convenience defaults. Rust’s `thread_local!` still provides per-thread owned state, which supports the current route-diagnostic and test-fallback design. And `OnceLock::get_or_init` still memoizes one initialization result for the lifetime of the process, which matters here because `vemr_indicator_ids()` caches a `Result`, not just a successful lookup.

## What Fourth Pass Sharpened

### What the current packet got right

The current third-pass packet is right about the broad architectural risk. This file is not “just a strategy config struct with helpers.” It owns serialized defaults, dynamic-indicator binding, host/device sweep surfaces, and the actual VEMR state machine. A broad cleanup that splits those pieces without moving parity and route coverage at the same time would be regression-prone.

It is also right that `VemrStrategyParams` is compatibility-sensitive. The field-level Serde defaults at `35-71`, the manual `Default` implementation at `184-205`, and the resolution logic at `380-402` together make the omission/mapping behavior part of the request contract. Because the params derive both `Serialize` and `Deserialize`, changing defaults or validation semantics is not just internal refactor work; it is request-shape and artifact-compatibility work.

The packet is also right to protect the CPU/CUDA boundary. The raw device-pointer ownership wrappers at `246-340` and the CUDA sweep helpers at `588-920` are GPU-companion-facing surfaces, not ordinary local helpers.

### What it missed or underweighted

The biggest miss is test coverage. The first-pass packet said default-build tests covered desired-position semantics, and the third-pass packet implicitly inherited that framing. The live tests do not support that claim. The test module at `1300-1384` covers only three things: period mapping, one invalid-parameter case, and one CUDA helper fallback diagnostic. The actual VEMR behavior engine lives in `compute_vemr_start_t` (`1122-1140`) and `build_vemr_desired_positions` (`1142-1298`), and those paths are currently untested in this file.

That matters because the state machine is where the real semantics live. Entry confirmation is streak-based when flat (`1190-1213`), but opposite-side signals while already in a position bypass confirmation and can trigger immediate exit or immediate flip (`1227-1239`, `1264-1276`). `max_trade_bars` is enforced off the live `bars_in_trade` counter after entry (`1240-1249`, `1277-1286`). `trade_on_next_bar` shifts the signal index with `saturating_sub` (`1156-1175`). Those are exactly the sorts of rules that change ranking, validation outcomes, and parity expectations when they drift, and they are the part of the file that can be hardened now without touching GPU execution.

The second miss is descriptor binding risk. The current packet talked about the `OnceLock` mostly as a cache that could use documentation. That is not the sharpest risk. The sharper risk is that `resolve_output_id()` is intentionally fail-open: if it cannot find an exact or fuzzy preferred output match, it returns the first declared output (`442-478`). By contrast, `resolve_param_key()` becomes strict when ambiguity remains and there is more than one parameter (`480-528`). That asymmetry means indicator-catalog drift can become a silent semantic change for outputs while still hard-failing for parameters. The safe immediate response is not a refactor; it is a regression test that locks the current resolved bindings for `zscore`, `bollinger_bands`, and `atr`.

The third miss is warmup/start-index ambiguity. `compute_vemr_fast_signals()` and `compute_vemr_risk_signals()` already derive `first_valid` from the first finite indicator outputs (`1077-1082`, `1116-1118`). `compute_vemr_start_t()` then takes the max of those indices and adds `max_period - 1` again (`1129-1138`). That may be intentional extra conservatism, or it may be double-counting warmup. What matters for this pass is that the current packet left this as generic “parity-aware cleanup” instead of calling it out as a semantics contract that should be explicitly locked before anyone changes it.

The fourth miss is silent length masking. Inside `build_vemr_desired_positions()`, every signal series access uses `.get(...).unwrap_or(f64::NAN)` (`1170-1175`). If any upstream indicator output becomes short or mismatched, the position engine degrades into NaN-gated no-op behavior instead of surfacing a clear invariant break. That is a real maintainability hazard, but it does **not** justify signature churn right now. It just means a debug assertion or targeted regression test would pull its weight immediately.

The fifth miss is that `vemr_indicator_ids()` is not only caching lookups; it is caching failure. Because the `OnceLock` stores `Result<VemrIndicatorIds, String>` (`530-580`), any first-call error becomes process-sticky. That is defensible if the dynamic-indicator registry is expected to be static after startup, but it is more than “cache construction or reuse” debt. The packet should have identified it as an intentional contract assumption rather than merely a documentation gap.

### What the current packet overstated or framed too weakly

The current packet overstated documentation as the main safe-now work. Documentation is worthwhile, but it is not the highest-leverage bounded hardening here. The highest-leverage bounded hardening is tests around the pure strategy semantics and descriptor bindings.

It also overstated current test confidence. Saying the file already had desired-position coverage was simply not supported by the live test module. That matters because the third-pass conclusion leaned on “test-backed” policy semantics without identifying that the most important semantics were the least tested.

It framed route diagnostics a bit too broadly. In live code, diagnostics are not a full route-truth surface. They only record attempted CUDA-helper fallback (`947-965`) or forced test fallback (`947-950`, `1325-1384`). They do not explain every reason GPU was skipped. That is fine, but it means the diagnostics are narrower than the packet implied.

## Implementation Priority

### Safe-now hardening

The first priority is to add unit tests for the behavior that actually defines VEMR. That means `compute_vemr_start_t()` and `build_vemr_desired_positions()`, not just config parsing. The minimum useful matrix should cover: flat-to-long entry, flat-to-short entry, confirmation streaks, `trade_on_next_bar`, mean-reversion exit, ATR stop exit, `allow_flip=true` immediate flip behavior, `allow_flip=false` immediate opposite-signal exit behavior, `long_only` short suppression, and `max_trade_bars`. These are all deterministic, local, and behavior-preserving to test.

The second priority is a descriptor-binding regression test around `vemr_indicator_ids()`. The live code relies on fuzzy string resolution and a fail-open output fallback. A small private-unit test that verifies the resolved keys/outputs for `zscore`, `bollinger_bands`, and `atr` would catch catalog drift without changing runtime behavior. That is a much better immediate guardrail than a comment alone.

The third priority is to add narrow internal invariants, not API churn. A `debug_assert_eq!` family at the top of `build_vemr_desired_positions()` to confirm `close.len()` matches the VEMR signal vectors would catch accidental caller drift in test/debug builds while preserving release behavior. Likewise, a short comment on `compute_vemr_start_t()` clarifying whether the extra `max_period - 1` is an intentional additional safety margin would reduce future “cleanup” risk.

A smaller fourth priority is validator hardening through tests, not behavior change. `validate_vemr_params()` should have table-style tests for `z_exit > z_entry`, empty `z_ma_type`, and the current latent rule that mapped periods still must be positive even when mapping flags are enabled. That last point is important because validation (`343-377`) and period resolution (`380-402`) are intentionally not the same thing right now.

### Larger redesign still deferred

A stricter descriptor-binding layer is still deferred. Replacing fuzzy string matching and first-output fallback with typed or centrally validated indicator bindings would be valuable, but that crosses the dynamic-indicator catalog contract and should not be done opportunistically inside `vemr.rs`.

Any attempt to remove the repeated Bollinger computation from the CPU fast-signal path is also still deferred. Right now the file computes upper, middle, and lower bands through three separate indicator calls (`1039-1068`). That looks like the clearest local performance smell, but fixing it safely probably requires multi-output adapter support or a companion helper that is shared consistently across CPU and CUDA paths. That is not a safe “cleanup-only” edit.

Unifying selector validation is also still deferred. `z_ma_type` is only checked for non-empty and `z_devtype` is not range-validated at all (`343-377`). A better long-term design would validate those fields against the indicator-catalog schema or a shared enum contract. But tightening that now would be an IPC/persistence decision because `VemrStrategyParams` is serialized public input.

The larger file split remains deferred as well. If this file is ever split, it should be done as a strategy-local move that keeps `vemr.rs`, `vemr_gpu_kernel.rs`, and the composite optimize/validate consumers aligned under the same parity review. The architectural instinct in the third-pass packet was correct there.

## Verification If Changed

If the safe-now hardening is limited to tests, comments, and debug assertions, the first verification pass should stay local and deterministic:

- `cargo test -p ta_app_core vemr::tests:: -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

If the change touches start-index semantics or desired-position logic, add explicit regression tests for the exact scenario being changed before running broader workflow coverage. The right order is to lock the behavior in unit tests first, then prove the broader workflow still agrees.

If anything touches CUDA sweep helpers, device-row pointer math, or `first_valid_for_period()`, verification needs to expand immediately. That is GPU-companion-facing behavior, so file-local tests are not enough. At that point at minimum re-run representative CUDA-enabled VEMR coverage such as:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel composite_cpu::tests::vemr_gpu_kernel_supports_standalone_shortlist_extras -- --nocapture`

If anything touches defaults, validation, or field semantics on `VemrStrategyParams`, treat that as request-contract work. The verification burden is not just “does it compile,” but also “does serialized input that used to work still deserialize and validate the same way.”

If anything touches the repeated indicator-compute path or CUDA staging path for performance reasons, this stops being “safe-now hardening” and should graduate to the repo’s normal perf-sensitive verification lane rather than relying on unit coverage alone.

## Regression And Feature-Preservation Constraints

Do not change `VemrStrategyParams` defaults, field names, or validation strictness casually. This struct is serialized input surface, so those edits can affect persisted artifacts, Tauri payloads, replayability, and any external caller using the public crate API.

Do not change `compute_vemr_start_t()` or the `build_vemr_desired_positions()` state machine until the current semantics are locked in tests. Those functions drive ranking-sensitive behavior, and several branches are subtle enough that a “cleanup” can easily become a strategy change.

Do not tighten `resolve_output_id()` or `resolve_param_key()` behavior in the same patch as unrelated cleanup. The current output side is fail-open and the parameter side is partially fail-closed; changing that is a compatibility choice, not just a readability improvement.

Do not touch `VemrDeviceMatrix`, `VemrPeriodSweepDevice::row_ptr_for_period()`, or `first_valid_for_period()` without reviewing the GPU companion path at the same time. Those are GPU-ABI-adjacent surfaces even though they live in this Rust module.

Do not broaden route diagnostics casually. The current diagnostics are intentionally narrow and thread-local. Making them more comprehensive may be worthwhile later, but it will change observability semantics and can easily create noisy or brittle tests.

Do not try to “optimize” the triple Bollinger computation without a parity and perf plan. That is the most obvious local performance debt, but it is also the easiest place to accidentally diverge CPU and CUDA behavior.

## Fourth-Pass Conclusion

The third-pass packet was directionally right about one thing: the big redesign should still stay deferred. This file remains too close to serialized defaults, dynamic-indicator bindings, and GPU-companion behavior to justify a casual architectural split.

But the third-pass **action bucket** should not stand as written. `Larger refactor but defer` is too passive for the live state of the file. It hides a meaningful, low-risk hardening slice that should happen now: tests for the start-index and desired-position semantics, regression tests for descriptor binding, and small internal invariants/comments that lock down the current behavior without changing it.

So the fourth-pass bucket is `Targeted hardening now; larger redesign still deferred`.

That is the conservative call. Preserve current behavior. Do not refactor the file structure yet. But do stop treating this file as “documentation only” debt, because the missed hardening here is not architecture. It is unpinned behavior.
