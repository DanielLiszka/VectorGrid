# Fourth-Pass Review: crates/app_core/src/asr.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/asr.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet was directionally right about compatibility and parity, but it stayed too high level for a file that owns both signal-state construction and final trade-metric emission. A fourth pass was warranted to challenge whether the live source still justified only abstract “be careful” guidance, or whether it contained a more concrete local defect that should outrank the earlier documentation and route-policy concerns.

The answer is that the bucket should stand, but for a sharper reason. This is still a small surgical refactor, not a redesign. The live file contains one clear trade-metrics correctness defect, one safe public-contract hardening gap, and a broader-than-advertised observability gap on CUDA fallback paths. Those are local fixes. The larger semantic questions are still real, but they should stay deferred until parity coverage exists.

## Fourth-Pass Research Log

This pass re-read the live `asr.rs` end to end, then checked the earlier packets only after the live-source review so the new conclusions stayed source-first rather than packet-first.

The external cross-checks in this pass were narrow and contract-focused: Rust API Guidelines dependability and validation guidance, Serde’s default-field behavior, and the current VectorTA Z-Score docs. Those checks matter here because `AsrStrategyParams` is a public serde-driven surface, and ASR delegates z-score behavior to an upstream indicator contract rather than defining a private enum of its own.

That research reinforced three points. Tightening `z_nbdev` validation is clearly safe. Tightening `z_ma_type` or `z_devtype` via a hand-written ASR allowlist is not clearly safe. And fallback behavior should be made more diagnosable before anyone tries to make it stricter.

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should stand: `Small surgical refactor`.

I would not escalate this to a redesign bucket. The strategy-local organization is still appropriate, and the file does not need decomposition to address its highest-value issues. The main blocker is no longer abstract policy cleanliness. It is a set of bounded correctness and observability problems inside otherwise serviceable strategy-local code.

### What the current third-pass packet got right

The third-pass packet was right to keep the file in a compatibility-sensitive lane, right to resist broad cleanup, and right to treat release-confirm behavior as parity-sensitive rather than casually “fixable.” It was also right that defaults and selector meanings should not be tightened casually on a public serde surface.

That conservative posture still holds. In particular, I would keep strategy-local CPU/GPU orchestration in this module, and I would not convert today’s fail-open GPU fallbacks into hard errors as part of this packet.

### What it missed or underweighted

The biggest miss is in `simulate_from_desired`, not in the earlier packet’s headline items. `trade_log_ret` is the value eventually written into `Trade.pnl` and therefore into `winning_trades`, `losing_trades`, `gross_profit`, and `gross_loss` (`2247`, `2304-2311`, `2323-2331`, `2343-2351`, `2401-2408`, `2497-2506`, `2658-2676`). But ordinary bar-return accumulation only happens behind `if risk_enabled && pos_bar != 0 && pos != 0` (`2419-2420`). When stop-loss and take-profit are both disabled, account-level `log_eq` and summary `pnl` still move (`2418`, `2630`), yet per-trade pnl can collapse to zero or commission-only values. On top of that, signal-based exits and flips add exit cost to `log_eq` but do not add that exit cost to `trade_log_ret` before the closing trade is pushed (`2288-2315`, `2319-2337`, `2339-2361`). That is a concrete trade-stat correctness bug, not a redesign question.

The local test surface underweights that bug. The only in-file test (`2861-2902`) checks that the added trade fields are finite and nonnegative, but it would still pass if every closed trade had zero pnl. For this file, that is too weak. A strategy module that emits trade-level metrics should have at least one regression test proving that trade records change with bar returns even when stop/take logic is off, and one proving that normal signal exits include exit-side commission in closed-trade pnl.

The third-pass packet also under-scoped the silent-fallback problem. It named `compute_indicator_f64` (`529-575`), and that concern is valid. But the same silence exists across most of the CUDA direct helpers: TTM squeeze (`688-705`), squeeze momentum (`735-752`), linear-reg angle (`778-785`), ER (`817-824`), z-score (`855-863`), and the Kaufman-stop host/device helpers (`1106-1126`, `1336-1390`). The issue is systemic. If route-truthfulness matters, diagnostics need to cover direct-device fallback, host fallback, and generic dispatch fallback, not just the preferred-host helper.

The release-confirm concern is also broader than the current packet described. The local code does not just allow entries before the first squeeze release because `release_confirm_left` starts at zero (`1844`, `1868-1873`). It also never disarms that condition when squeeze turns back on. Because trend entry only checks `release_confirmed` and not `!squeeze_on_now` (`1893-1912`), one historical release can effectively leave trend entry armed through later squeeze-on stretches. That makes the semantic issue more real than the third-pass packet suggested, but it also makes a one-line “fix” less safe than the first-pass writeup implied.

A final miss is CUDA warmup precision. `AsrPeriodSweepDevice::first_valid_for_period` returns only `period - 1` (`372-374`), and `compute_asr_kaufman_stops_cuda_device` hardcodes `prepared_inputs.high_low_first_valid() + period - 1` even when fallback dispatch paths discard an actual warmup value (`1355-1361`, `1370-1378`, `1400-1401`). I would not call that a proven live bug from this bundle alone, but it is an underweighted GPU parity risk. Any future cleanup here needs downstream GPU-kernel review, not a blind normalization pass.

### Claims that are too weak or too strong

The earlier review family was too weak on trade-metric correctness. That should now be the lead finding. It is more concrete and safer to fix than the release-confirm state machine.

The earlier review family was a bit too strong on local selector tightening as a default remedy. `z_nbdev` absolutely deserves validation in `validate_asr_params` (`377-429`). But a narrow ASR-local allowlist for `z_ma_type` or `z_devtype` would be riskier than it sounds. The live file already routes some valid-but-slower z-score combinations through generic dispatch instead of the direct CUDA path (`1538-1596`), and the upstream Z-Score docs still present `ma_type` as a general moving-average choice and `devtype` as a deviation-mode selector rather than an ASR-private enum. Tighten selector validation only if it is sourced from the dynamic indicator contract, not from a hard-coded local guess.

The earlier release-confirm finding is directionally correct but still too strong if translated straight into implementation. The local evidence is enough to justify tests and documentation. It is not enough to justify immediate CPU/GPU semantic convergence without checking the owning GPU path.

## Implementation Priority

### Safe-now hardening

First, fix the trade-record path in `simulate_from_desired`. `trade_log_ret` should accumulate ordinary bar returns for any open trade, not only when stop/take risk management is enabled. Signal-based exits and flips should also book the exit-side commission into the closing trade before that trade is pushed. This is the highest-priority change in the file because it is a clear correctness issue and stays local to trade-stat emission. Based on the live code, this should not change account-level `pnl`, Sharpe, Sortino, or drawdown because those are derived from `log_eq`; it should change the trade-derived fields only. It will still change persisted trade-level metrics and report values, so that output churn should be treated as intentional correctness repair.

Second, add the missing `z_nbdev` validation. Requiring it to be finite and `> 0.0` is a safe public-contract hardening step on a serde-driven config surface. That change is narrow, testable, and well aligned with the upstream indicator semantics.

Third, expand ASR route diagnostics across all silent CUDA fallback sites while preserving today’s fallback behavior. The safe-now change is observability, not stricter routing. This means logging when preferred CUDA host evaluation falls back to CPU, and when direct device-input helpers fall back to host arrays or generic dispatch. That preserves compatibility while making route reports honest.

### Larger redesign still deferred

The release-confirm state machine should still be deferred until parity coverage exists. The right redesign probably involves explicit “release observed / release armed / squeeze re-entered” state rather than the current countdown-only latch, but changing that now would move actual entry timing and candidate rankings.

CUDA warmup unification should also stay deferred unless the ASR GPU path is already being touched. The current first-valid approximations may be acceptable to present callers, but changing them blindly risks an ABI-like break between this module and `asr_gpu_kernel.rs`.

Selector tightening beyond `z_nbdev` should remain deferred as well. If the project later wants stronger local validation for `z_ma_type` or `z_devtype`, it should derive those accepted values from the indicator catalog or descriptor layer, not from an ASR-local allowlist.

## Verification If Changed

At minimum, any safe-now patch here should run:

- `cargo test -p ta_app_core asr -- --nocapture`
- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

The new in-file regression coverage should include a no-risk trade-stat test proving that closed-trade pnl is nonzero when bar returns are nonzero, and a commission test proving that normal signal exits include the closing cost in trade pnl. The existing `2861-2902` test is not sufficient for that surface.

If CUDA fallback diagnostics or warmup handling changes, also run:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel asr -- --nocapture`

If the trade-stat fix lands, expect downstream artifact and report values that expose `gross_profit`, `gross_loss`, `winning_trades`, or `losing_trades` to change. That is not a reason to avoid the fix; it is a reason to treat parity baselines and saved-output expectations as intentionally updated.

## Regression And Feature-Preservation Constraints

Do not reinterpret the file as needing architectural breakup. The safest work here is still local hardening inside the current module.

Do not change `Metrics.trades` semantics in the same patch as the trade-pnl fix. The clear bug is in closed-trade pnl accounting and the fields derived from the `trades` vector. Re-defining transaction-count semantics at the same time would blur the review surface and create avoidable report churn.

Do not convert today’s GPU fallbacks into hard errors as part of this pass. That would create route-policy and workflow regressions that are not justified by the current evidence. Add diagnostics first.

Do not hard-code a narrow allowlist for `z_ma_type` or `z_devtype` on the ASR surface unless that list comes from the dynamic indicator contract. This struct is public and serde-driven, so over-tightening it is both an IPC compatibility risk and a persisted-config risk.

Do not change release-confirm entry semantics without CPU/GPU parity review. That path is ranking-sensitive and validation-sensitive.

Do not change device pointer layout or first-valid conventions in `AsrPeriodSweepDevice` or `AsrKaufmanStopsDevice` without checking the downstream GPU consumer. Those are not ordinary refactor details; they are part of the CUDA handoff contract.

## Fourth-Pass Conclusion

The third-pass bucket should remain `Small surgical refactor`, but the rationale needs to be sharper than the current packet gives it.

What the third-pass packet got right is the conservative posture: keep the file strategy-local, avoid broad cleanup, and do not casually change ASR semantics on a public CPU/GPU-sensitive surface.

What it still missed is the most actionable problem in the file: `simulate_from_desired` currently misstates closed-trade pnl and the trade-derived metrics whenever stop/take logic is off, and it also misses exit-side commission on ordinary signal exits. That is a concrete, local correctness defect. The other safe-now items are narrower: validate `z_nbdev > 0` and make silent CUDA fallbacks diagnosable across the whole helper family.

The larger redesign work remains deferred. Release-confirm semantics, selector tightening beyond `z_nbdev`, and device warmup unification all need broader parity review. The right fourth-pass call is therefore not a bucket change. It is a more evidence-backed `Small surgical refactor` packet with the trade-metrics fix moved to the top of the queue.
