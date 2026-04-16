# Fourth-Pass Review: `crates/app_core/src/double_ma_strategy.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/double_ma_strategy.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file needed a fourth pass because the live source and the current third-pass packet are materially out of sync.

The live module is a crate-local validation gate plus two optimization shims: `validate_strategy_cfg` spans essentially the whole file (`source__crates__app_core__src__double_ma_strategy.rs:3-376`), and the optimization helpers are direct pass-throughs (`379-384`). The current third-pass packet instead frames the file as a methodology or compatibility helper with tolerant parsing, heuristic selection, and approximation boundaries. That description does not match the code that is actually present.

That mismatch matters because it changes what is safe to do next. This is not a performance hotspot or a heuristic-heavy planner. It is a correctness boundary reused by the main optimization path (`context__02__crates__app_core__src__double_ma.rs:5039-5040`) and by the validation path (`context__03__crates__app_core__src__double_ma_validate.rs:1854`). A stale description here can push the review toward preserving behavior that may not actually be intentional boundary behavior.

The current third-pass bucket should still stand: this remains a `Small surgical refactor` file, not a redesign candidate. But the third-pass rationale should not stand as written.

## Fourth-Pass Research Log

I re-read the live source first, then checked the nearby caller and verification surfaces before deciding whether the bucket should move.

The local code review covered four concrete surfaces. First, I rechecked the validator and the identity wrappers in `double_ma_strategy.rs`. Second, I verified the optimization and validation call sites in `double_ma.rs` and `double_ma_validate.rs` to confirm that this module still sits on a shared contract boundary rather than on an isolated helper path. Third, I checked the existing test coverage in `double_ma.rs:22017-22089`, because the third-pass packet recommended a test target that does not line up with where the current tests actually live. Fourth, I reviewed `double_ma_validate.rs:3121-3231`, where `repair_params` intentionally sanitizes invalid values for internal perturbation-style work.

I also rechecked the review against the same primary Rust standards the earlier packets used, because they are still the right external frame for this file: the Rust API Guidelines dependability guidance on validating arguments, the naming guidance for wrapper semantics, and The Rust Book guidance on recoverable errors and test organization. Those sources still support a conservative reading: boundary validators should reject bad inputs predictably, wrapper names should not imply behavior they do not provide, and invariant tests are easiest to maintain near the owning module.

One important limit of this fourth pass is that the current bundle does **not** include the `ta_strategies::double_ma::StrategyConfig` accessor implementations that the first-pass packet cited when it argued that some `> 0` checks are dead due to accessor-side normalization. I therefore treat that first-pass finding as still credible, but not freshly re-proven from the files in this packet alone.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet got one big thing right: the action bucket does not need to change. This file is still small, semantics-sensitive, and low-throughput. The right next step is still targeted hardening, not a broader architecture rewrite.

It was also directionally right to warn against casual behavior changes. Tightening a validator used by optimization and validation flows can alter accepted inputs, returned error strings, and rerun behavior in ways that reach beyond this module.

### What it missed, underweighted, or overstated

The third-pass packet materially overstated a compatibility/tolerance framing that is not supported by the live file. There is no parsing logic here, no heuristic selection, and no explicit approximation boundary. The code is a straight-line validator with numeric checks, feature-gating checks, and pairwise incompatibility checks. Calling this a tolerant compatibility helper is not a faithful description of the source.

It also missed the current verification reality. The packet recommends `cargo test -p ta_app_core double_ma_strategy -- --nocapture`, but the current validator tests shown in the supplied context live in `double_ma.rs`, not in this module (`context__02__crates__app_core__src__double_ma.rs:22017-22089`). That makes the third-pass verification advice weaker than it sounds.

The packet also underweighted the fact that this validator returns `Result<(), String>` and those strings can propagate upward through optimization and validation entrypoints. Even though this module is `pub(crate)`, error-order and wording changes are still effectively IPC-visible once they bubble into the Tauri-facing workflow. That makes seemingly small “cleanup” changes more compatibility-sensitive than the third-pass write-up acknowledged.

Most importantly, the third-pass packet missed a useful distinction visible in the current context: this subsystem already has an intentionally tolerant internal repair surface in `double_ma_validate.rs::repair_params` (`3121-3231`). That code clamps or repairs invalid strategy values for internal validation perturbation flows. So the safe conclusion is **not** “all tolerant behavior in this subsystem is accidental.” The safer conclusion is narrower: boundary validation and internal repair are different responsibilities, and a fix to the validator should avoid collapsing them together.

### What earlier packets got slightly too strong or too weak

The first-pass packet still contains the strongest concrete correctness concern in the bundle: if `StrategyConfig` accessors normalize raw invalid values before the validator reads them, then the validator can claim to reject invalid input while actually accepting it. That concern remains plausible and important.

However, the first-pass implementation advice was slightly too broad for this packet’s available evidence. A sweeping rewrite from accessor-based checks to raw-field validation across the whole module would be harder to justify without first reproducing the issue in focused tests, especially because the current bundle does not include the accessor implementations and the wider subsystem clearly does contain intentional repair paths.

The safer fourth-pass refinement is: prove the bug with targeted regression tests first, then patch only the branches that are confirmed to be reading normalized values too early. That is narrower, better evidenced, and less likely to create unplanned compatibility fallout.

## Implementation Priority

### Safe-now hardening

Keep the bucket unchanged and stay narrow.

The first safe-now change is test-first hardening. Add a small local test block to `double_ma_strategy.rs` or a nearby dedicated validator test module, and reproduce representative raw-invalid cases that the current validator claims to reject. The point is not to blanket-test every field immediately; it is to pin the boundary on a few high-signal examples before changing behavior. At minimum, include one representative accessor-backed period/count case, one incompatible-profile case, and one numeric-threshold case that is expected to fail at the validator boundary.

If those tests confirm the earlier accessor-normalization concern, then patch only the affected branches by validating the raw submitted fields before any normalizing accessor is consulted. Keep the scope intentionally small. Do not combine that change with wrapper cleanup, broad validator decomposition, or typed-error work.

Add a short ownership note in this module explaining the intended split: this file is the boundary validator for submitted strategy configs, while internal stochastic or perturbation flows may still repair mutated configs elsewhere. That small note would make the current contract much easier to preserve.

Leave `validate_strategy_cfg_for_optimization` and `strategy_cfg_for_optimization` alone in the same change. They are identity shims today, and that is mildly awkward, but touching them now adds churn without reducing the real risk surface.

### Larger redesign still deferred

A larger redesign still belongs in the deferred bucket.

If the repo eventually wants stronger contract ownership, the more durable solution is to move raw boundary validation closer to the `StrategyConfig` owner in `ta_strategies`, or at least define an explicit raw-validation helper there. That would reduce the risk of app-core validators drifting away from accessor semantics.

Likewise, the pairwise incompatibility matrix near the bottom of this file is explicit but manual. A future cleanup could model the mutually exclusive profile families more directly instead of maintaining a long sequence of pairwise checks. But that is not necessary to safely harden the current file, and it would create more message-order churn than the current packet justifies.

A typed error surface is also still deferred. Returning strings is not elegant, but changing that boundary would ripple through callers, tests, and likely UI/reporting surfaces. There is no evidence in this packet that such churn is justified for this file right now.

## Verification If Changed

The third-pass verification advice should be tightened.

The immediately relevant existing signal in the supplied context is `cargo test -p ta_app_core validate_strategy_cfg -- --nocapture`, because the current validator tests shown in this bundle live under `double_ma.rs` rather than under a local `double_ma_strategy` test module. After adding local regression tests, run those exact tests as well, so the owning validator carries its own negative-path coverage instead of relying only on a giant neighboring file.

After any boundary-tightening change, rerun at least one caller-side optimization path and one caller-side validation path that pass through this validator. The goal is not performance validation; it is confirming that accepted valid configs still pass and that rejection text has not changed more than intended.

If the change specifically touches the accessor-versus-raw-field boundary, include a regression case that distinguishes “field omitted and defaulted” from “field explicitly set to an invalid raw value.” That is the compatibility line most likely to matter.

## Regression And Feature-Preservation Constraints

Preserve the current non-performance posture. I did not find a meaningful throughput, allocation, or GPU ABI issue in this file, so this review does **not** support bundling in performance work.

Preserve the current accepted strategy matrix unless a concrete bug is proven. In particular, keep the currently valid combinations that the code intentionally allows, including the shared trend-quality-style validation path for regime-hybrid settings (`source__crates__app_core__src__double_ma_strategy.rs:65-103` together with `264-289`).

Do not remove internal repair behavior that exists for internal validation perturbation work. `double_ma_validate.rs::repair_params` is evidence that some tolerant behavior is deliberate in that separate context. Boundary validation can be tightened without erasing that repair layer.

Treat error text and first-failure ordering as compatibility-sensitive. This module returns `String` errors, and caller-visible workflow behavior can change if the validator starts failing earlier or with different wording. That is an IPC-facing risk even though the module itself is not public API.

Be aware of persistence and rerun risk. If old saved configs or artifacts contain malformed raw values that were previously accepted through normalization, stricter boundary rejection could make those flows fail earlier. That does not mean the validator should stay wrong, but it does mean the change should be test-backed and intentionally staged.

## Fourth-Pass Conclusion

The current third-pass **bucket** should stand: `crates/app_core/src/double_ma_strategy.rs` still belongs in `Small surgical refactor`.

The current third-pass **packet**, however, should not be trusted as the main rationale for that bucket. It describes a tolerant heuristic/compatibility surface that the live file does not actually implement. The live code is a boundary validator plus two identity optimization shims.

What the third-pass packet got right was the overall caution level: do not turn this into a larger cleanup just because the file looks repetitive. What it missed was the actual risk boundary. The highest-value work here is still correctness and contract hardening, but the safer fourth-pass version is narrower than a sweeping rewrite: add focused regression tests first, confirm any accessor-side normalization leak on representative fields, patch only the confirmed branches, preserve internal repair paths, and avoid wrapper churn in the same change.

So the bucket stays the same, but the recommended posture is sharper: **test-first, branch-local hardening now; validator relocation, error typing, and profile-matrix redesign still deferred.**
