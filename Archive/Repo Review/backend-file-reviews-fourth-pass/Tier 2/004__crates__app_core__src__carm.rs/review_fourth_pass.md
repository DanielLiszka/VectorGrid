# Fourth-Pass Review: crates/app_core/src/carm.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/carm.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The current third-pass packet got the bucket right but became too abstract. Re-reading the live source shows that `carm.rs` is still not a broad cleanup target, yet the third pass underweights the concrete parts of the contract that actually matter here.

The main fourth-pass conclusion is that the third-pass bucket should **stand**. This file is still best treated as a parity-sensitive strategy-local surface where conservative hardening is safer than cleanup symmetry. But the live code still contains one clear safe-now correctness hole and several narrower contract details that the third pass blurred together as generic “policy.”

What changed in this pass is the precision of the critique. The real pressure points are not file size or abstraction shape. They are:

- `otto_threshold` still has an inconsistent runtime contract, and the problem is sharper than “just overloaded semantics.”
- the local OTTO test coverage is weaker than the names and assertion messages suggest,
- OTTO is recomputed twice per signal build, which is a real hot-path cost but not an obvious small-surgical fix,
- and some earlier stronger recommendations should be narrowed because they risk compatibility regressions.

## Fourth-Pass Research Log

This pass re-read the live `carm.rs` source first, then compared it against the first-, second-, and third-pass packets and the local `lib.rs` re-export surface.

The source review focused on these live areas:

- public config and validation: `CarmStrategyParams` and `validate_carm_params` (`carm.rs:18-40`, `173-203`)
- output binding and dynamic-indicator dispatch: `resolve_output_id` and `compute_indicator_*` (`284-436`)
- OTTO construction: `compute_carm_otto_signals*` (`762-949`)
- signal assembly and warmup: `compute_carm_signals*` and `compute_carm_start_t` (`951-1074`)
- desired-position semantics: `build_carm_desired_positions` (`1084-1187`)
- local tests: `tests` (`1189-1434`)

External docs consulted in this pass:

1. VectorTA OTTO documentation.
   - Why it mattered: it confirms that OTTO’s parameter is `ott_percent` with a documented default of `0.6`, and that one OTTO computation produces two outputs, `hott` and `lott`.
   - Why that matters here: `carm.rs` feeds `otto_threshold` into `ott_percent`, but later uses the same raw field as a normalized spread gate.

2. VectorTA AO / APO / Coppock / ROCR documentation.
   - Why it mattered: those pages document single-output surfaces as `values` outputs.
   - Why that matters here: `carm.rs` asks `resolve_output_id(..., &["value"])` for these indicators, which weakens the case for a blanket fail-closed resolver change.

3. Serde field-default documentation.
   - Why it mattered: `CarmStrategyParams` is a serde-defaulted public request/config surface.
   - Why that matters here: the meaning of `otto_threshold` is not internal cleanup trivia; it is active deserialize-time contract.

No new workspace execution evidence was collected in this pass. This bundle is sufficient for static review and contract analysis, but not for a fresh end-to-end cargo run.

## What Fourth Pass Sharpened

### The third-pass bucket should stand

`Small surgical refactor` is still the right action bucket.

The third pass was correct about the big picture: this file is strategy-local, behavior-sensitive, and close to CPU/GPU and validation parity surfaces. It should not be “improved” by generic decomposition or by cleanup that silently retunes behavior.

That part of the third pass should remain.

### What the third pass got right

The third pass correctly recognized three things:

- this file is closer to product policy than to generic utility code,
- any change here needs parity awareness rather than isolated refactor energy,
- and broad tightening of fail-open behavior or route semantics would be too casual for this surface.

Those judgments remain sound.

### What the third pass missed or underweighted

#### 1. `otto_threshold` is not just overloaded; it is a dual-unit contract with a live scale mismatch

The sharpest issue is at the OTTO boundary.

In OTTO construction, `carm.rs` passes:

- `cfg.otto_threshold.abs().max(0.6)` into `ott_percent` (`803`, `899`)

But in desired-position gating, the same raw field is compared against:

- `otto_spread = (lott - hott) / abs(close)` (`1076-1082`)
- `otto_spread >= cfg.otto_threshold` and `<= -cfg.otto_threshold` (`1131-1133`)

That means one public number is simultaneously used as:

- an OTTO indicator percent input,
- and a normalized spread threshold.

The mismatch is not cosmetic. It is numeric. A user value of `0.6` is documented OTTO input as `0.6%`, but the regime gate interprets the same value as a raw ratio of `0.6`, i.e. a `60%` spread requirement. Even before CPU/GPU parity is considered, non-default positive values are far harder to reason about than the current packet admits.

The negative-value case is even worse. The OTTO builder normalizes the field with `.abs()`, while the CPU gate preserves the sign. So a negative threshold is not merely confusing; it creates contradictory semantics inside the same file.

This is the clearest safe-now hardening item in the file, but the fourth-pass conclusion is narrower than a full semantic rewrite:

- **safe now:** reject negative `otto_threshold` values in `validate_carm_params`
- **defer:** do not silently reinterpret positive thresholds yet, because that would change the meaning of saved configs and candidate ranking for existing non-default users

The third pass should have kept this concrete issue in the foreground instead of collapsing it into a general “defaults and selector behavior” warning.

#### 2. The local OTTO threshold test overstates what it actually verifies

`build_carm_desired_positions_keeps_rocr_and_apo_thresholds_strict_but_otto_inclusive` (`1254-1286`) is presented as an inclusiveness/equality test, but the fixture does not actually hit equality.

The test uses default `otto_threshold = 0.0` and sets:

- `close = 100.0`
- `otto_hott = 100.0`
- `otto_lott = 160.0`

That produces an OTTO spread of `0.6`, not equality at the threshold. So the test name and assertion message claim more than the data proves.

This matters because exact-threshold behavior is subtle here. With the current inclusive comparisons, a spread exactly equal to the threshold is admitted. At threshold `0.0`, an exact zero spread would make both `otto_up` and `otto_down` true. That may be intended, but it is not what the current test actually pins.

This is a missed verification weakness, not a redesign request. The safe-now move is to repair the test so it either:

- truly tests equality, or
- is renamed to describe what it really covers.

#### 3. OTTO is recomputed twice per signal build

The OTTO hot path is also sharper than the third pass suggested.

`compute_carm_otto_signals*` computes the same indicator twice with the same params:

- once for `hott` (`823-834`, `919-928`)
- once for `lott` (`835-846`, `929-938`)

The OTTO docs describe one indicator output surface containing both `hott` and `lott`. So this is not just visual duplication; it is real duplicated work in both the `f64` and `f32` paths.

This is a valid performance concern for a hot strategy-local file, and the third pass missed it.

However, this is **not** a clear safe-now refactor. The dynamic-indicator adapter surfaces used here are single-output shaped, so deduplicating OTTO work likely requires either:

- multi-output dynamic-indicator support, or
- a strategy-local OTTO fast path that bypasses the current adapter abstraction.

Both options are larger than this bucket and carry CPU/GPU parity risk. So this belongs in **larger redesign still deferred**, not in the safe-now lane.

#### 4. OTTO is also coupled to other indicator periods in a way the public surface does not make obvious

The file hides more OTTO policy than just `otto_threshold`.

In OTTO construction:

- `fast_vidya_length` is derived from `periods.coppock_short.max(2)` (`806-808`, `902-904`)
- `slow_vidya_length` is derived from `periods.ao_slow.max(periods.otto)` (`810-812`, `906-908`)

So changing Coppock or AO periods can retune OTTO, even when the caller thinks they are only changing Coppock or AO behavior.

This may be intentional strategy design, but it is still contract debt. The public config surface under-describes how much cross-coupling exists between supposedly separate knobs.

The third pass should have called this out explicitly. The right near-term move is documentation. Exposing or decoupling those knobs is deferred work because it changes strategy semantics.

### Which earlier recommendations are too strong or too weak

#### `resolve_output_id` should not be fail-closed by default in this bucket

The earlier packet’s fail-closed recommendation is too strong as a safe-now change.

`resolve_output_id` currently tries:

1. exact output-id match,
2. substring/alias match,
3. first output fallback (`284-320`)

For OTTO, the exact `hott` / `lott` match is already the meaningful safety surface. For the single-output indicators, though, the external docs describe output fields as `values`, while `carm.rs` asks for `value`. That strongly suggests the alias-tolerant resolver is doing real compatibility work rather than just masking drift.

A blanket fail-closed change could therefore break currently functioning single-output binding without delivering a clearly proven safety win.

A narrower, safer hardening move would be a focused resolver test for the currently expected alias set. Tightening to hard failure should stay deferred until the descriptor/output-id contract is proven end to end.

#### Surfacing GPU fallback diagnostics is probably larger than the current bucket

The silent GPU-fallback concern is real, but the earlier packet’s “record the first CUDA error string or route note” suggestion reads smaller than it likely is.

Today the fallback mechanism is local:

- a failed CUDA attempt flips `cuda_failed = true` (`361-365`, `417-420`)
- later indicator calls in the same signal build stop retrying CUDA (`345-349`, `401-404`)

That means one build can become mixed execution: some earlier indicators may have already come from GPU, while later ones fall back to CPU. That is worth knowing. But this file’s current return types are just signal vectors plus `first_valid`; they are not a diagnostic surface.

So unless there is an existing low-risk logging channel already used in this module family, pushing structured route diagnostics out of `carm.rs` is probably not a true small-surgical change. It widens contracts across optimization, validation, and reporting layers.

The third pass was right to avoid making route-surface tightening a default move here.

## Implementation Priority

### Safe-now hardening

The current third-pass bucket should stand, but it should be made concrete with these priority items:

1. **Reject negative `otto_threshold` values in `validate_carm_params`.**
   - This is the clearest live correctness hole.
   - It fixes the signed CPU gate vs absolute-value OTTO-input inconsistency without silently retuning positive thresholds.

2. **Add a direct unit test for negative-threshold rejection.**
   - The current validation test only checks `INFINITY` rejection.
   - The negative case is the actual missing hole.

3. **Repair the OTTO inclusiveness test so it matches its own claim.**
   - Either make the fixture hit exact-threshold equality,
   - or rename the test if the goal is only “OTTO positive spread continues to permit entry once ROCR/APO clear their strict gates.”
   - If equality at `0.0` is intentional, add a small explicit test for that behavior.

4. **Document the current `otto_threshold` contract and OTTO derived-parameter coupling.**
   - Minimal doc comments are warranted because this is a public serde-defaulted config surface.
   - The doc should be explicit that the current field drives both OTTO construction and OTTO gating, and that OTTO’s inner VIDYA lengths are derived from other strategy periods.

5. **If additional hardening is desired, prefer a resolver test over a fail-closed resolver rewrite.**
   - The current packet should not treat fail-closed output binding as the default safe-now move.

### Larger redesign still deferred

These are valid issues, but they should remain deferred because they change semantics or widen contracts:

1. **Split the overloaded `otto_threshold` field.**
   - Preferred end state: separate fields for OTTO indicator percent and OTTO regime threshold.
   - Alternative end state: keep one field but align the gate to percent units explicitly.
   - Either way, this is a compatibility-sensitive change touching saved configs, optimizer ranking, validation parity, and likely GPU-kernel behavior.

2. **Avoid recomputing OTTO twice.**
   - This likely needs multi-output dynamic-indicator support or a dedicated strategy-local OTTO helper.
   - That is worthwhile, but it is not this bucket’s surgical hardening lane.

3. **Add structured GPU fallback observability only at the right layer.**
   - If the product needs route truthfulness here, add it through an existing logging or orchestration surface.
   - Do not widen `CarmSignals` or related public/internal result shapes casually.

## Verification If Changed

At minimum, if the safe-now changes above are made, verify with:

- `cargo test -p ta_app_core carm -- --nocapture`

Add or update targeted unit coverage for:

- negative `otto_threshold` rejection,
- true OTTO threshold-equality behavior,
- and any resolver alias expectations that are meant to stay supported.

If OTTO semantics are changed beyond negative-value rejection, also run:

- `cargo test -p ta_app_core composite_cpu -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

If any CUDA-path behavior, OTTO refactor, or fallback behavior changes, also run representative CUDA coverage:

- `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel carm -- --nocapture`
- representative CARM-dependent `composite_cpu` / `composite_validate` CUDA-path coverage

If OTTO computation is ever deduplicated, add explicit CPU/GPU parity checks for HOTT/LOTT warmup, NaN prefix behavior, and resulting desired-position parity.

## Regression And Feature-Preservation Constraints

Preserve these behaviors unless a larger approved redesign explicitly changes them:

- current `long_only`, `allow_flip`, and `trade_on_next_bar` semantics in `build_carm_desired_positions`
- current `start_t` handling and two-opposing-votes exit posture
- current strategy-local organization of the full CARM signal chain inside one file

Treat the following as compatibility-sensitive surfaces:

- any change to the meaning of **positive** `otto_threshold` values
  - this affects saved config semantics, optimization ranking, and validation comparability
- any change to OTTO output binding or multi-output handling
  - this affects hot-path performance and CPU/GPU parity
- any change to output-id fallback rules
  - this can break compatibility with live indicator-descriptor naming conventions
- any change to GPU fallback reporting that alters result shapes or public/internal orchestration contracts

One specific caution from this pass: do **not** implement a blanket fail-closed `resolve_output_id` rewrite without first proving the live output-id contract end to end. The external indicator docs strongly suggest that alias tolerance is not accidental for the single-output indicators used here.

## Fourth-Pass Conclusion

`Bucket unchanged.`

`carm.rs` should stay in `Small surgical refactor`.

The third-pass packet got the bucket right and correctly resisted broad cleanup, but it was too abstract for the live file. The fourth pass sharpens that into a narrower, evidence-backed recommendation:

- there is one clear safe-now correctness hardening item: reject negative `otto_threshold` values,
- there is one real test-quality gap: the OTTO inclusiveness test does not prove what it says it proves,
- and there are two meaningful but deferred issues: duplicated OTTO computation and a public OTTO contract that still hides too much coupling.

The earlier stronger moves should be narrowed. A fail-closed resolver rewrite is not clearly safe now, and structured GPU fallback diagnostics likely exceed this packet’s true scope unless there is already a low-risk logging surface ready to use.

So the right fourth-pass posture is conservative: fix the live correctness hole, improve the tests and docs that explain the current contract, and defer the semantic and performance redesigns until they can be coordinated across CPU, GPU, validation, and saved-config surfaces.
