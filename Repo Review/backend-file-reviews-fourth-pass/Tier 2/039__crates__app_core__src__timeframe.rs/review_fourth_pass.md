# Fourth-Pass Review: `crates/app_core/src/timeframe.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/timeframe.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/timeframe.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/timeframe.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/timeframe.rs.review.md`
- Third-pass action bucket: `Document only`
- Fourth-pass action bucket: `Safe-now hardening`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file was worth a fourth pass because the live source does **not** fully support the prior packet’s framing anymore. The third-pass packet correctly treated `timeframe.rs` as a small but semantics-sensitive public utility, but it challenged the file through a mostly generic compatibility lens instead of this file’s actual risk surface.

The core reason the third-pass bucket should **change** is that the public parser and the public resampler are not fully aligned. `parse_timeframe_label` accepts arbitrary positive `m`, `h`, and `d` multipliers, including `2d`, `3d`, or `36h` (`timeframe.rs:27-85`). But `resample_candles` unconditionally flushes whenever the UTC day changes (`timeframe.rs:174-193`), then starts a new bucket lattice from the first timestamp seen in that new UTC day (`timeframe.rs:191-196`). That means intervals greater than one day cannot actually aggregate across multiple days. A `2d` label is accepted, but the live resampler still emits one bucket per UTC day. That is no longer just a documentation gap; it is a bounded contract mismatch.

The tier rationale still mostly holds: this is a shared contract/default surface, not a hot path or a GPU-critical module. But the action bucket is too soft. This file is publicly exported from `crates/app_core/src/lib.rs` (`lib.rs:73,101`), so a silent accepted-input mismatch is worth small corrective hardening rather than more documentation-only packets.

## Fourth-Pass Research Log

I reviewed the live source first, then pressure-tested the previous packets against the actual implementation.

1. **Live-source review of `timeframe.rs`**
   - Focused on:
     - parser acceptance and normalization rules (`27-85`)
     - day-reset and bucket-start calculation (`172-196`)
     - OHLCV aggregation and optional-field propagation (`227-313`)
     - current tests and what they do **not** cover (`320-467`)
   - Main outcome:
     - the real risk is the parser/resampler capability mismatch for intervals above one day, not generalized serde/default behavior.

2. **Public-export check in `crates/app_core/src/lib.rs`**
   - `timeframe` is a public module and `parse_timeframe_label`, `resample_candles`, and `TimeframeSpec` are all re-exported (`lib.rs:73,101`).
   - Main outcome:
     - label normalization and resample semantics are crate-level public behavior, not merely file-local helpers.

3. **Prior packet review**
   - The first and second passes were directionally right that anchoring semantics and optional-field behavior are the main contracts.
   - The third pass overfit a generic “compatibility seam / serde defaults” template and underfit the live code.

4. **External standards/research consulted**
   - **pandas `DataFrame.resample`**: documents `origin`, `label`, and fixed-frequency resampling semantics explicitly, which is a good benchmark for how much anchoring and bucket-label policy should be spelled out in public time-series helpers.
   - **Polars `DataFrame.group_by_dynamic`**: documents `every`, `period`, `label`, and `start_by`, including explicit first-datapoint anchoring. This is useful because the live Rust code is much closer to a “datapoint-anchored” window policy than to a calendar/session-aware one.
   - **Rust API Guidelines** and **Effective Rust** documentation guidance: reinforce that public interfaces should document the non-obvious semantics and invariants, especially when behavior is more surprising than the signature suggests.

Those external sources do **not** prove this file must mimic pandas or Polars. They do show that origin, label, and window-length semantics are first-class public contract choices in mainstream time-series APIs, which makes the live file’s current implicit behavior and `> 1d` mismatch more important to surface.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The current third-pass packet was right about three important things.

First, this file is **not** a performance-first refactor target. It is low-compute support code, and the meaningful risks are semantic and compatibility-oriented, not micro-optimizations.

Second, the packet was right that **representation churn would be lower value than invariant clarity**. The existing implementation is short, readable, and already test-backed for a few core behaviors. Broad cleanup would create motion without automatically improving correctness.

Third, it was right that **documentation and targeted invariant tests matter here**. That remains true, especially around bucket anchoring, timestamp labeling, and optional-field propagation.

### What the current packet missed or underweighted

The biggest miss is the accepted-label / actual-resample mismatch.

`parse_timeframe_label` accepts multi-day and supra-day fixed intervals (`62-71`), but `resample_candles` hard-resets on each UTC day boundary (`174-193`). Because of that reset, intervals strictly greater than `DAY_MS` never span multiple days. In practice:
- `1d` works as “one bucket per UTC day, timestamped at that day’s first observed bar.”
- `2d`, `3d`, `25h`, `36h`, and similar intervals do **not** work as their labels imply.

That is a real bounded hardening opportunity. The third-pass packet should have challenged whether “document only” was still defensible once that mismatch was visible in the live source. It is not.

The next underweighted issue is that the prior packets repeatedly described the resampler as “session-anchored” or “session-aware.” That is too strong. The live logic is not exchange-session-aware and does not know anything about a market calendar, a timezone, or a session boundary. It computes `day = ts.div_euclid(DAY_MS)` and resets when that integer changes (`173-191`). That is a **UTC-day-scoped, first-observed-bar-of-day anchor**, not a general trading-session model. If a day starts late because early bars are missing, the entire bucket lattice for that UTC day shifts to the first observed timestamp. That nuance matters.

The current packets also underweighted the parser’s **asymmetric normalization**. The file does not fully canonicalize equivalent durations. It lowercases and trims input, and it canonicalizes only a small legacy/convenience subset of second-based inputs (`34-50`). By contrast:
- `120s` becomes `2m`
- `7s` stays `7s`
- `60m` stays `60m`
- `24h` stays `24h`
- `1d` stays `1d`

So `TimeframeSpec.label` is not a universal canonical form; it is mostly “preserve caller form, except for some second aliases.” That is fine if intentional, but it is a real contract detail for any caller that persists or compares labels textually.

A smaller but still real miss: the optional-field contract is not only “preserve missing fields as `NaN`.” It also includes a **global field-flag downgrade** step (`288-309`). If the source marks `open/high/low/volume` as present but the resampler never sees a finite value for that field in any output bucket, the output flag is flipped to `false` and the entire output column is rewritten to `NaN`. That behavior looks reasonable, but it is more specific than the current packet acknowledged, and it deserves either docs or a targeted test.

Finally, the current tests do not cover the highest-value edge cases:
- interval greater than one day
- first observed bar arriving late in a UTC day
- source flags true but all optional values non-finite
- duplicate timestamps, which are currently allowed because the code rejects only `<`, not `<=` (`105-113`)

### Claims in the current packet that are too strong or too weakly supported

The third-pass packet’s serde framing is the clearest weak point.

This file has no serde derives, no serde container attributes, and no DTO shaping in the live source. Talking about “serde/default round-trip” and “DTO-visible behavior” is not strongly supported by the implementation. The real compatibility seam here is **textual timeframe normalization and resampling math**, not omission/default serialization behavior.

Likewise, the third-pass conclusion that the file “mostly confirms” documentation and invariant coverage are enough is too strong once the `2d`/`36h` mismatch is noticed. That mismatch is not just a hidden semantic choice. It is an accepted input family whose obvious meaning is not honored by the current algorithm.

## Implementation Priority

### Safe-now hardening

The safe-now work is small and bounded.

The highest-priority hardening is to make the resampler fail fast for intervals it does not actually implement. The most conservative change is inside `resample_candles`, not inside `parse_timeframe_label`: reject `interval_ms > DAY_MS` with an explicit error explaining that the current algorithm is UTC-day-scoped and does not support buckets spanning multiple days. That preserves current `1d` behavior, avoids redesigning anchoring policy now, and stops silently wrong output for `2d`/`36h`/similar requests.

The next safe-now hardening is documentation. Public rustdoc should state, explicitly:
- bucket timestamps are the **left edge / bucket start**
- the bucket lattice resets on **UTC-day** boundaries
- each UTC day is anchored to the **first observed timestamp of that UTC day**
- missing early bars therefore shift the bucket lattice for that day
- close is required and must be finite
- optional fields are aggregated only from finite values
- output field flags may be downgraded to `false` when no finite output values exist

The next safe-now hardening is test coverage:
- a regression test proving that `interval_ms > DAY_MS` errors
- a test showing first-observed-bar-of-day anchoring on a truncated day
- a test covering the global field-flag downgrade path when input flags are `true` but values are all non-finite

A very small clarity improvement is also justified: rename `session_origin` to something closer to `day_origin` or `first_bar_of_day_ts`. The current name overclaims session semantics the code does not actually implement.

### Larger redesign still deferred

The right long-term redesign is **not** a bigger version of the current ad hoc logic. If the product truly needs `2d`, `3d`, or exchange/session-aware aggregation, the file should eventually model anchoring policy explicitly rather than smuggling it through a UTC-day reset.

That larger redesign would likely include:
- a typed timeframe representation instead of a free-form normalized label string
- an explicit anchoring policy (`calendar`, `first_datapoint`, `session_open`, etc.)
- a timezone/session-calendar model if the app needs real market-session semantics
- an explicit decision on duplicate timestamps: reject, dedupe, or aggregate deterministically
- a clearer canonical-label policy, especially for equivalent durations like `24h` vs `1d`

That should stay deferred. It would touch public exports, downstream request/data-shaping behavior, and likely persisted label expectations. It is not a safe “small cleanup.”

## Verification If Changed

If only docs and variable names change, file-local tests are still the minimum:

- `cargo test -p ta_app_core timeframe::tests:: -- --nocapture`

If the safe-now hardening is implemented, I would add and run targeted tests for:
- `resample_rejects_interval_greater_than_one_day`
- first-observed-bar anchoring on a truncated UTC day
- optional-field flag downgrade when input flags are true but all values are non-finite

If a future redesign changes anchoring, duplicate handling, or label normalization, file-local tests are not enough. At that point, representative higher-level request/data-loading flows that consume these public exports should also be re-run because `TimeframeSpec.label` and resampled timestamps are the kinds of values that downstream code often persists, compares, or displays.

I did not execute the workspace tests in this bundle-only review environment, so the verification guidance above is recommended follow-up rather than a claim of execution.

## Regression And Feature-Preservation Constraints

Do not change `120s -> 2m` back-compat handling casually. Even though the file does not fully canonicalize all equivalent durations, this specific alias support is already tested and may be relied upon by existing request inputs.

Do not silently re-anchor from “first observed bar of UTC day” to wall-clock midnight, exchange open, or rolling-calendar windows inside a cleanup patch. Any such change will alter output timestamps and downstream derived series.

Do not “fix” the multi-day mismatch by inventing a partial spanning policy in-place unless the desired semantics are explicitly chosen first. A fail-fast guard for `> 1d` is the conservative near-term move. A silent algorithm change would be much riskier.

Do not normalize labels more aggressively without checking downstream textual expectations. Because the type is publicly exported, callers may compare or persist labels exactly as returned. Changing `24h` to `1d`, or `60m` to `1h`, could be a compatibility break even though the intervals are numerically equal.

Do not tighten duplicate-timestamp handling casually. The current code accepts nondecreasing timestamps and will aggregate duplicates into the same bucket. Rejecting or deduplicating them could change OHLCV outputs on dirty datasets. That is a policy decision, not a cleanup.

Performance, GPU ABI, and CUDA-route risk are all low here. This file is not on the repo’s main GPU/runtime-critical path. The real risks are semantic drift, textual compatibility, and downstream validation differences from changed timestamps or field flags.

## Fourth-Pass Conclusion

The third-pass packet was right that this is a small public utility whose main risk is semantics, not raw performance. It was also right that broad refactoring would be lower value than documenting and testing the contract.

But the third-pass bucket should **not** stand.

`Document only` is too weak because the live source currently accepts timeframe labels whose obvious meaning the resampler does not actually honor for intervals above one day. That is a real bounded hardening issue, not merely an under-documented behavior. The best conservative call is:

- change the action bucket to **`Safe-now hardening`**
- add a fail-fast guard for `interval_ms > DAY_MS`
- document the true anchoring contract as **UTC-day-scoped, first-observed-bar-of-day, left-edge labeled**
- add a small set of edge-case tests around that contract

Everything larger than that — true multi-day resampling, explicit session calendars, duplicate-timestamp policy, or a typed timeframe model — should remain deferred until there is an explicit product decision.
