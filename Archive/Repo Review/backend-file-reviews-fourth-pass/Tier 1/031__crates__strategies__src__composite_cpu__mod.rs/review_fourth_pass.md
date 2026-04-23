# Fourth-Pass Review: `crates/strategies/src/composite_cpu/mod.rs`

- Review date: `2026-04-15`
- Source file: `crates/strategies/src/composite_cpu/mod.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Targeted hardening now; larger redesign still deferred`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet was right to treat this module as semantics-sensitive shared strategy code, but it collapsed the next step too aggressively into “document it and defer the rest.” After rereading the live source and the adjacent `ta_app_core` consumers, the more important conclusion is narrower and more actionable: this file does not need a large structural rewrite now, but it does have a few bounded hardening opportunities that are more valuable than additional prose.

The main reason is that the file owns more than local CPU math. `CompositeSignalMode` is a serialized wire/default surface in the strategy crate itself (`source__crates__strategies__src__composite_cpu__mod.rs:14-21`), and app-core request/candidate types deserialize it with `#[serde(default)]` in optimize, drilldown, local-stability, and validation paths (`context__02__crates__app_core__src__composite_cpu.rs:370-391`, `441-459`, `507-524`; `context__03__crates__app_core__src__composite_validate.rs:226-260`). The same module also carries a hidden caller contract around `start_t`, duplicates its position-transition engine across metrics and curves (`source__...:519-718` and `845-1116`), and participates in a HardGate-only CPU/GPU parity contract via app-core’s GPU regime builder and fallback tests (`context__02__...:1912-1944`, `1968-2004`, `13852-13891`; `context__03__...:5848-5861`, `14551-14627`).

That combination means the right fourth-pass question is not “should this file be split now?” It is “did the current packet miss a safer, bounded hardening step?” The answer is yes.

## Fourth-Pass Research Log

I reread the live module, especially the public enum surface, the overlay helpers, the metrics loop, the curves loop, and the local tests (`source__crates__strategies__src__composite_cpu__mod.rs:14-1436`).

I then checked the immediate app-core consumers to confirm which behaviors are load-bearing outside this crate. The highest-signal findings were:

- `CompositeSignalMode` is deserialized with `#[serde(default)]` across optimize, drilldown, local-stability, and validation candidate request types, which makes both the variant names and the default-to-`HardGate` behavior part of the request surface (`context__02__crates__app_core__src__composite_cpu.rs:370-391`, `441-459`, `507-524`; `context__03__crates__app_core__src__composite_validate.rs:226-260`).
- Base-composite GPU optimization and GPU fixed-candidate validation are intentionally `HardGate`-only today, and both have explicit fallback tests when `signal_mode` is anything else (`context__02__crates__app_core__src__composite_cpu.rs:1977-1985`, `14022-14040`; `context__03__crates__app_core__src__composite_validate.rs:5848-5861`, `14551-14627`).
- App-core already depends on exact HardGate parity between this module and `build_base_composite_hard_gate_regime_f32`, including strict inequality / non-finite handling (`context__02__crates__app_core__src__composite_cpu.rs:1912-1944`, `13852-13891`).

I also checked the current official guidance that is actually relevant to the recommendations here:

- Rust API Guidelines, Documentation: public items should document behavior and failure considerations, which supports documenting `start_t` and sentinel behavior rather than only adding higher-level prose.
- Rust API Guidelines, Dependability: Rust APIs should validate inputs when practical, which matters here because the file currently mixes silent sentinel handling with asymmetric defensive checks.
- Serde container attributes: `rename_all = "snake_case"` fixes the serialized enum spellings, so `hard_gate`, `entry_core_exit_overlay`, and `trend_pullback_reversion` are concrete request-shape behavior, not just internal naming.
- Rust Reference, code generation / `inline`: `#[inline(always)]` is only a hint and can hurt as well as help, so any attempt to “clean up” the hot loop by abstraction or inlining churn should remain evidence-driven and benchmark-gated.

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **change**.

`Larger refactor but defer` is still the right answer for any extraction of the metrics/curves engine into shared generic machinery, or for any API redesign around validated config/start indices. But it is the wrong overall bucket because there are low-blast-radius hardening steps available now that the third-pass packet did not elevate.

The more accurate bucket is: **do targeted hardening now, and still defer the larger redesign**.

### What the current third-pass packet got right

It correctly recognized that this module is a shared contract surface and that structural cleanup is not free. That remains true. The public enum, the duplicated hot-path logic, and the app-core consumers all make broad refactors easy to get wrong.

It was also right that `CompositeSignalMode` is not cosmetic. The variant semantics materially change entry/exit behavior (`source__...:53-189`), and those semantics are consumed outside this crate.

### What it missed or underweighted

The packet underweighted the fact that `CompositeSignalMode` is not just a behavioral enum; it is a serialized defaulted request field. Because the enum derives `Serialize`, `Deserialize`, and `Default` with `#[serde(rename_all = "snake_case")]` (`source__...:14-21`), and because app-core request/candidate types use `#[serde(default)]` for `signal_mode`, two contracts already exist today and should be tested explicitly: the wire spellings and the omitted-field default to `HardGate` (`context__02__...:370-391`, `441-459`, `507-524`; `context__03__...:226-260`). The third-pass packet gestured at “serde-visible regression tests,” but too vaguely. This is the concrete serde surface.

It also missed a hidden but important caller contract: the public evaluation functions accept `start_t` as a raw index, while the implementation still computes `t_sig = t.saturating_sub(trade_offset)` inside the loop (`source__...:521-535`, `847-861`). App-core callers already compensate by folding both warmup and `trade_on_next_bar` into `start_t` before they call into this module (`context__02__...:1289-1316`, `13848-13850`). That is more load-bearing than another section comment about “ownership split.” If this pre-offset contract is not documented, future direct callers can silently shift signal timing by one bar.

The third-pass packet also overstated the strength of the local tests. The current suite is useful, but it is not a full guardrail for the duplicated transition engine. It covers basic mode behavior and a couple of TP/SL outcomes (`source__...:1135-1435`), but it does **not** directly lock the highest-risk asymmetries: signal persistence delaying non-zero transitions but not flat exits (`source__...:223-259`), cooldown blocking only flat re-entry (`262-275`), `allow_flip == false` behavior in non-`HardGate` modes (`537-565`, `863-931`), or caller-visible parity between the metrics loop and the curves/trade-record loop.

### What it overstated

The third-pass packet’s suggestion that permissive / zeroed fallback behavior is already an intentional shared contract is too strong for the live code.

The metrics path explicitly returns all-zero `Metrics` for malformed lengths, out-of-range `start_t`, and invalid commission (`source__...:392-470`). But the curves path only mirrors the malformed-length and `start_t` sentinels; it does **not** mirror the invalid-commission guard (`source__...:762-788`, then `791-805` with no equivalent rejection). That asymmetry is enough evidence that the sentinel behavior is observable but not yet consistently hardened. It should not be treated as a settled contract without either aligning the behavior or testing/documenting the difference.

The packet also under-called the GPU/runtime coupling. For `HardGate`, the exact long/short/flat semantics in this file are mirrored into `build_base_composite_hard_gate_regime_f32` for GPU regime construction (`context__02__...:1912-1944`), and app-core already has a parity test asserting equivalent metrics (`context__02__...:13852-13891`). So touching equality handling, non-finite handling, or default mode behavior here is not just “CPU behavior drift”; it is also a CPU/GPU route-parity risk.

## Implementation Priority

### Safe-now hardening

First, add a focused serde/default regression test for `CompositeSignalMode`. The value here is not generic documentation polish. The value is freezing the concrete request behavior that already exists: the `snake_case` variant names and the default-to-`HardGate` behavior when the field is omitted. That is a real IPC/persistence boundary because optimize, drilldown, local-stability, and validation candidate types all deserialize the enum with `#[serde(default)]`.

Second, add targeted tests for the private overlay helper semantics, or equivalent end-to-end tests that hit the same branches. The most important cases are the ones not currently locked down: persistence delaying entry/reversal but not flat exits, cooldown blocking re-entry only after a close, and `allow_flip == false` behavior when non-`HardGate` modes want to reverse. Those are low-risk test additions with much better value than another round of descriptive comments.

Third, document the `start_t` contract on the public evaluation entry points. The docs should say plainly that callers are responsible for passing a `start_t` that already incorporates warmup and any `trade_on_next_bar` offset, because the evaluator still uses `trade_offset` to select the signal bar internally. That is the behavior the current app-core callers already rely on.

Fourth, harden the invalid-commission sentinel mismatch. The safest bounded code change here is to make `eval_composite_curves_from_log_returns_and_mas_f64_with_mode` return the same empty/sentinel output that the metrics path already returns when commission is invalid, or at minimum add a regression test and an explicit comment if the asymmetric behavior is truly intended. This recommendation is conservative because it changes only already-invalid inputs and reduces divergence between two public entry points that otherwise represent the same strategy engine.

### Larger redesign still deferred

A true shared transition engine for metrics and curves is still deferred. The duplication is real, and it is the long-term maintainability problem in this file, but collapsing both loops into a common abstraction changes the hot-path shape, the trade-record emission path, and the place where CPU/GPU parity assumptions are easiest to reason about. That is not a casual cleanup.

Likewise, changing the public API from silent sentinels to `Result`, or introducing a typed validated `start_t` / validated strategy wrapper, is a broader cross-crate contract change. It could be a good future direction, but it is not a safe-now patch for a public-facing shared crate.

Finally, enabling non-`HardGate` base-composite GPU optimization/validation is not a follow-on from this file alone. App-core’s current route matrix deliberately falls back to CPU for those modes (`context__02__...:1977-1985`, `14022-14040`; `context__03__...:5848-5861`, `14551-14627`). Any change there belongs to a dedicated backend/runtime project, not to a local cleanup pass in `ta_strategies`.

## Verification If Changed

At minimum, rerun the strategy-local suite and the two closest app-core consumers:

- `cargo test -p ta_strategies composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

If the change touches `CompositeSignalMode`, omitted-field defaults, or `HardGate` semantics, also rerun the feature-enabled route/fallback coverage when CUDA builds are available:

- `cargo test -p ta_app_core composite_cpu --features cuda,cuda-backtest-kernel -- --nocapture`
- `cargo test -p ta_app_core composite_validate --features cuda,cuda-backtest-kernel -- --nocapture`

If the hot loop shape, helper extraction, or `#[inline(always)]` decisions change, treat this as perf-sensitive backend work and run the CPU perf gate. If the `HardGate` GPU regime or fallback matrix changes and GPU is available, run the GPU perf gate as well.

## Regression And Feature-Preservation Constraints

Preserve the exact serialized/default `CompositeSignalMode` contract unless every caller and any persisted payload format move together. In practice that means preserving the current `snake_case` spellings and the omitted-field default to `HardGate`.

Preserve the caller-owned `start_t` semantics unless app-core callers are changed in lockstep. Right now the app-core code already pre-applies warmup and `trade_on_next_bar` offset before invoking these public functions.

Preserve `HardGate` equality and non-finite semantics unless the matching GPU regime builder and its parity tests are updated at the same time. This is a real cross-crate CPU/GPU consistency constraint, not a local style issue.

Preserve the current overlay asymmetry unless behavior migration is intentional and explicitly tested. Signal persistence currently delays non-zero transitions but does not delay a flat exit; cooldown currently blocks flat re-entry after a close rather than every signal change.

Do not “fix” the duplicated loops by introducing generic closure-heavy abstractions without perf evidence. This module sits on a hot path, and the Rust reference makes clear that inlining attributes are only hints, not guarantees. A maintainability-motivated refactor here still needs performance verification.

## Fourth-Pass Conclusion

The third-pass packet was directionally right about one thing: this file is not a good candidate for a broad cleanup refactor. But it was too conservative about the next step.

The bucket should change from `Larger refactor but defer` to **`Targeted hardening now; larger redesign still deferred`**.

What the third pass got right was the sensitivity of the behavior and the need to avoid casual structural churn. What it missed was that there are already a few concrete, bounded hardening steps available now: lock the serialized/default `CompositeSignalMode` contract, document the hidden `start_t` contract, add sharper overlay-branch tests, and remove or explicitly codify the current invalid-commission mismatch between metrics and curves.

That is the highest-value, lowest-regression next move. The deeper transition-engine unification and API redesign questions should still wait.
