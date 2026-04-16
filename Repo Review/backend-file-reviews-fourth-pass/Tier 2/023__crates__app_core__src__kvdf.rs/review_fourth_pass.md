# Fourth-Pass Review: crates/app_core/src/kvdf.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/kvdf.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet correctly treated `kvdf.rs` as a semantics-sensitive strategy file, but it also backed away from the concrete contract edges too quickly. The live source still deserves a fourth pass because there is a difference between “do not refactor this casually” and “the remaining issues are only abstract compatibility concerns.” They are not. One of them is a real normalization bug, and a few others are narrower than the earlier packets implied.

My fourth-pass conclusion is that the current third-pass bucket should **stand**. This is still a `Small surgical refactor`, not a redesign file. But the reason the bucket stands is more specific than the third-pass packet said: the highest-value work here is a small set of localized correctness, documentation, and observability fixes, while the larger contract changes proposed earlier remain too risky to call safe-now.

## Fourth-Pass Research Log

I reopened the live source first, then used the third-pass packet only as a challenge target, and used the earlier packets mainly to identify which claims had become too generic or too aggressive.

I also checked the current external contracts that this file is implicitly depending on:

1. **Rust API Guidelines** (`checklist`, `dependability`, `documentation`, `future-proofing`). These reinforce four constraints that matter here: public APIs should validate or clearly document preconditions; public behavior should have documented error/failure semantics; public fields and helper visibility are a compatibility commitment; and future-proofing cuts both ways, because once helper carriers/functions are public, tightening them later is itself a breaking change.
2. **Serde field/container attribute docs.** `KvdfStrategyParams` uses per-field defaults, which means its default values are not merely internal implementation details; they participate in the deserialization contract for omitted fields.
3. **VectorTA indicator docs for Keltner, VWAP, DevStop, DTI, and FVG Trailing Stop.** These were the most important external checks because this file resolves dynamic-indicator outputs and normalizes the VWAP anchor string.
   - Keltner documents `upper_band`, `middle_band`, and `lower_band` outputs.
   - VWAP documents default anchor `1d`, supports anchors with units `m`, `h`, `d`, and `M`, and explicitly says uppercase `H`/`D` are normalized.
   - DevStop and DTI are documented as single-output indicators (`values`).
   - FVG Trailing Stop documents `upper`, `lower`, `upper_ts`, and `lower_ts` outputs.

That last point matters because the live file is making output-selection choices and anchor-normalization choices that should match the indicator contract, not just compile.

## What Fourth Pass Sharpened

### The third-pass bucket should stand

`Small surgical refactor` is still the right bucket. The file is hot-path, strategy-local, and parity-sensitive, so broad cleanup is still the wrong move. The third-pass packet got that part right.

It also got right that:

- this file is not a good candidate for abstraction-first cleanup;
- CPU/GPU parity and route-truthfulness matter more than cosmetic simplification;
- the presence of route diagnostics already lowers the severity of some earlier observability concerns.

### What the third-pass packet missed or underweighted

#### 1. `normalize_kvdf_vwap_anchor()` is not merely permissive; it is wrong for month anchors

This is the strongest live issue in the file.

`default_kvdf_vwap_anchor()` returns `"session"` and `normalize_kvdf_vwap_anchor()` trims the string, lowercases it wholesale, and maps `"session"` to `"1d"` (`kvdf.rs:124-126`, `196-201`). That looked like a benign aliasing helper in earlier passes. It is not benign.

The current VWAP docs say the anchor must be a positive number plus one unit, and the valid units are `m`, `h`, `d`, and `M`; they also say uppercase `H` and `D` are normalized. They do **not** say uppercase `M` is normalized, because `M` means months while `m` means minutes. In the live code, `"1M"` becomes `"1m"`, which changes a monthly anchor into a one-minute anchor.

That means the fourth pass should treat this as a concrete correctness bug, not just an under-documented config contract. The earlier packets missed that distinction.

#### 2. Route diagnostics exist, but they are incomplete in one important fallback case

The file records a diagnostic only when a CUDA helper is actually attempted and returns an error (`compute_indicator_f64` at `374-433`, `compute_indicator_f32` at `436-488`). If a GPU-preferring backend reaches the helper with `prepared_cuda_inputs = None`, the code silently drops to CPU without recording why.

That is not a semantics bug, but it is a route-truthfulness gap. In a strategy file that already has dedicated route diagnostics, silent CPU fallback due to missing prepared inputs is exactly the kind of thing that should be visible. The earlier packets correctly noted that diagnostics exist, but they underweighted that they do not yet cover every meaningful GPU-to-CPU fallback path.

#### 3. The FVG contract is internally inconsistent, and the current packet history did not call that out clearly enough

`compute_kvdf_fast_signals()` errors if it cannot find any finite `fvg_upper_ts` and `fvg_lower_ts` values (`953-1005`, especially `996-999`). The CUDA smoke test explicitly injects gaps “so the FVG surface has finite output rows” (`1287-1295`), which is a clue that finite FVG rows are not guaranteed on ordinary trending data.

But `build_kvdf_desired_positions()` treats non-finite FVG trailing-stop values as permissive for entries and non-triggering for hard exits (`1138-1139`, `1152-1153`, `1171-1172`). In other words, the state machine itself is written to tolerate missing FVG values, while the upstream fast-signal builder currently rejects an all-`NaN` FVG surface.

That does **not** automatically mean the current rejection is wrong; relaxing it would broaden the strategy’s accepted input domain and could change optimization coverage. But it does mean the file contains an unresolved contract mismatch, and that mismatch deserves to be test-pinned before anyone changes behavior either way. The earlier packets focused on malformed vector tolerance; they underweighted this higher-value inconsistency.

#### 4. The output-resolution risk is narrower and more specific than the first pass implied

The earlier packet was directionally right to dislike `resolve_output_id()`’s fuzzy matching and fallback behavior (`336-371`), but the fourth pass sharpens where the real risk is.

The biggest ambiguity is not generic “catalog drift.” It is the **specific** overlap in FVG output names. The documented output set includes both `upper` and `upper_ts`, and both `lower` and `lower_ts`. If the exact-match phase ever stops matching, the current substring heuristic can misbind a trailing-stop request onto the boundary series because both share the same prefix. That is a real future-drift hazard.

By contrast, a blanket recommendation to make output resolution strict for **all** indicators is too broad. Keltner is a strong candidate for exact-id enforcement because the documented output names line up cleanly. FVG is a strong candidate because the fuzzy fallback is ambiguous. But DevStop, DTI, and VWAP are single-output indicators in the docs, and the adapter/catalog may legitimately normalize those single outputs under generic ids like `value`. The earlier packet’s broad “make required output mismatches fail closed” framing was a little too coarse.

#### 5. The validation gap is broader than one known caller, but that makes the “safe-now” fix less safe than it sounded

The first pass focused on one known internal caller that bypasses `validate_kvdf_params()`. That may still be true in the wider repo, but the live file itself already shows a broader issue: none of the public compute helpers call the validator. `compute_kvdf_signals()`, `compute_kvdf_slow_signals()`, `compute_kvdf_shared_signals()`, `compute_kvdf_dti_signal()`, and `compute_kvdf_fast_signals()` all accept `KvdfStrategyParams` directly and rely on downstream normalization or indicator-level failures rather than enforcing the file’s own validator first.

That matters because it changes the review recommendation. It means the problem is not just “one missed call site.” It is that validation is currently an external convention rather than a guaranteed property of the helper surface. Retrofitting validation into every public helper may be the right long-term design, but it is **not** obviously safe-now, because it would tighten behavior for callers that currently rely on permissive normalization such as `abs()` on `dti_threshold` or aliasing of `vwap_anchor`.

### Claims from the earlier packet history that are too strong or too weak

The fourth pass would narrow three earlier recommendations.

First, making `build_kvdf_desired_positions()` return `Result<Vec<i32>, String>` is **not** a safe-now refactor. The function is public, sits on a hot-path state machine, and almost certainly participates in CPU/GPU parity expectations. A checked sibling or stronger rustdoc is a safer first move than changing the current function’s return type.

Second, narrowing helper visibility from `pub` to `pub(crate)` is also **not** a safe-now call. The Rust API Guidelines are right that public helper fields/functions are a strong commitment, but that is exactly why visibility tightening is a compatibility change once those items are already exposed through a public module. The third-pass packet was right to lean conservative here, and the first-pass packet overstated the safety of this cleanup.

Third, the earlier packet history treated the VWAP-anchor issue mostly as a validation/doc problem. That was too weak. The month-unit corruption is a concrete bug and should move ahead of broader debates about visibility or fail-open desired-position handling.

## Implementation Priority

### Safe-now hardening

1. **Fix `normalize_kvdf_vwap_anchor()` so month anchors survive normalization.**

   Keep the `"session" -> "1d"` alias if the product/UI still depends on it, but stop lowercasing the entire anchor string. The normalization logic should preserve `M` for months while still normalizing known case-insensitive day/hour spellings. This is the clearest safe-now change in the file.

   Add focused tests for at least:

   - `"session" -> "1d"`
   - `"1d" -> "1d"`
   - `"1D" -> "1d"`
   - `"4H" -> "4h"`
   - `"1M" -> "1M"`
   - `"2M" -> "2M"`

2. **Improve route diagnostics for silent CPU fallback caused by missing prepared CUDA inputs.**

   If a GPU-preferring backend reaches the CUDA-capable helper path but `prepared_cuda_inputs` is absent, record a route diagnostic before falling through to CPU. That is a low-risk observability improvement and matches the file’s existing diagnostic intent.

3. **Add contract tests before changing output-resolution behavior.**

   The safest immediate hardening is not a broad runtime behavior change. It is a test that pins the currently expected output ids for the multi-output indicators KVDF relies on, especially Keltner and FVG Trailing Stop. If those expectations hold, a later cleanup can remove the ambiguous substring fallback more confidently.

4. **Document the public helper assumptions rather than silently tightening them.**

   The public helpers still need rustdoc that says what they assume: validated params, equal-length signal vectors, and the fact that `"session"` is an alias rather than the indicator’s native documented anchor spelling. This is especially important because `KvdfStrategyParams` is a serde-backed DTO and its defaults are part of the serialized contract.

5. **Add a decision test for the all-`NaN` FVG case before changing runtime behavior.**

   Right now the file is inconsistent: the signal-builder rejects an all-`NaN` FVG surface while the position builder tolerates missing FVG values. The safe-now step is to write the test that expresses which side is actually intended. The safe-now step is **not** to silently relax or tighten behavior without that decision.

### Larger redesign still deferred

1. **Uniform validation at the public helper boundary.**

   If the repo wants validation to be guaranteed rather than caller-owned, the durable solution is a validated config boundary or a checked helper layer. Doing that ad hoc inside each existing public helper is a compatibility change and should be coordinated.

2. **Fallible desired-position construction.**

   A checked sibling or a validated newtype for signal bundles could make malformed-vector handling explicit without immediately breaking the current public helper. Changing the existing function signature should stay deferred.

3. **Visibility narrowing of helper carriers and compute helpers.**

   This may still be architecturally desirable, but it is not a no-downside cleanup for a public module surface. Treat it as a semver-facing decision.

4. **Hot-path deduplication and caching.**

   The repeated output lookup, repeated descriptor lookup, and repeated parameter-vector construction are real but secondary. Any caching or templating refactor should wait for measurement and parity coverage.

## Verification If Changed

At minimum, rerun the file-local lane:

- `cargo test -p ta_app_core kvdf::tests:: -- --nocapture`

If the safe-now anchor fix is made, add and run targeted tests for monthly-anchor preservation and documented alias normalization.

If route diagnostics are touched, rerun the existing CUDA fallback-diagnostic coverage already present in this file, and keep the broader KVDF optimization/validation parity lanes in scope. The earlier packets already identified the right broader categories to rerun: current optimization smoke plus KVDF CPU/GPU validation-parity and route-diagnostic coverage.

If output-resolution behavior changes, add a focused test that the catalog still resolves the exact Keltner and FVG outputs KVDF expects, and then rerun the normal KVDF smoke coverage. Do **not** treat this as a docs-only change.

If anyone decides to change the all-`NaN` FVG behavior, that needs more than the file-local tests. It should rerun representative optimization and validation flows because it changes which datasets/configurations are accepted.

## Regression And Feature-Preservation Constraints

- Preserve the current third-pass bucket. This is still strategy-local hardening, not a refactor campaign.
- Do **not** bundle the month-anchor fix with a serialized default change from `"session"` to `"1d"` unless UI, persistence, and artifact expectations are reviewed together. Serde defaults make that a contract decision, not a tidy-up.
- Fixing the month-anchor bug will intentionally change results for any existing KVDF configuration that used `*M` anchors and was accidentally getting minute semantics. That result change is justified, but it is still user-visible and should be called out.
- Preserve current desired-position state-machine behavior unless a dedicated parity review says otherwise. The fourth pass does not support casual rewriting of the entry/exit logic.
- Do not convert the existing desired-position builder to a fallible API in the same patch as small correctness fixes.
- Do not tighten helper visibility in the same patch as behavior fixes; that mixes semver surface changes with runtime changes.
- Do not relax the all-`NaN` FVG rejection without first deciding whether the intended contract is “FVG evidence required” or “missing FVG is permissive.” The current file points in both directions.
- Preserve current CPU fallback behavior for indicator compute. The safe-now observability recommendation is to report missing CUDA prerequisites better, not to remove fallback.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: `kvdf.rs` remains a `Small surgical refactor` file.

What the third-pass packet got right is the overall posture. This file is still too semantics-sensitive for cleanup-driven redesign, and the larger API-shape changes proposed earlier remain deferred.

What the third-pass packet missed is that one live issue is sharper than “compatibility-sensitive cleanup”: `normalize_kvdf_vwap_anchor()` currently corrupts valid monthly VWAP anchors by lowercasing `M` into `m`. That is a real bug and should move to the front of the queue. The next best safe-now improvement is better route diagnostics for silent GPU-to-CPU fallback when CUDA-prepared inputs are missing.

By contrast, the earlier packet history was too aggressive about calling helper visibility tightening and fallible desired-position construction safe-now. Those are still plausible future directions, but they are not the right fourth-pass changes for a hot-path public strategy module.

So the fourth-pass recommendation is: **keep the bucket, fix the month-anchor normalization bug, add the missing route diagnostic, pin the output/FVG contracts with tests, and defer the public-surface and fail-open redesign questions until a coordinated parity-aware pass.**
