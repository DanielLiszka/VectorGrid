# Fourth-Pass Review: `crates/app_core/src/double_ma_validate.rs`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/double_ma_validate.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_validate.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_validate.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_validate.rs.review.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet got the main architectural call right: this file still exposes a public app-core validation API whose request and result DTOs advertise semantics that are only actually completed in wrapper layers. That remains the dominant design problem, and it still resists a safe one-file fix.

What justified a fourth pass was not overturning that conclusion. It was pressure-testing whether the current packet had become too all-or-nothing. After re-reading the live source, the answer is yes. The defer bucket should stand, but the packet still underweights a smaller local hardening slice that does not require a DTO split, and it overstates one of its secondary drivers.

## Fourth-Pass Research Log

1. **Rust API Guidelines — Checklist / Documentation / Dependability**
   - Why consulted in this pass:
     - reassess whether the public validation DTOs and entrypoints are under-documented relative to what they actually guarantee
     - validate whether silent acceptance/truncation behavior should be treated as a contract smell rather than a cosmetic issue
   - Fourth-pass constraint informed:
     - public functions should validate what they truly own and document failure/behavior boundaries clearly

2. **Serde docs — Container attributes**
   - Why consulted in this pass:
     - check how much protection a future DTO cleanup would actually buy at the JSON / IPC layer
   - Fourth-pass constraint informed:
     - without `deny_unknown_fields`, removing request fields from a deserialized struct does not itself create a strong runtime rejection boundary for old payloads

3. **Tauri v2 security docs — Trust Boundaries**
   - Why consulted in this pass:
     - validate whether this DTO split is only a library ergonomics issue or also an IPC-boundary issue
   - Fourth-pass constraint informed:
     - request-shape cleanup at this layer is not just internal refactoring; it sits on a trust boundary and needs explicit boundary semantics

## What Fourth Pass Sharpened

### 1. The current third-pass bucket should stand, but the packet was too close to “redesign or nothing”

The third pass was right about the big thing: `cross_ticker`, `validation_context`, and wrapper-shaped `runtime_ms` semantics are still cross-layer ownership problems, not safe local cleanup. That part should not change.

What it underweighted is that this file also has a smaller, purely local public-contract cleanup slice that is worth doing before any broader redesign. The safest next work is not just “wait for the larger split.” It is “tighten the docs and direct tests around the local semantics this file already owns, while deferring the cross-layer contract rewrite.”

### 2. The packet missed a local observability / contract issue around candidate truncation and `ValidationExecutionProfile`

`run_double_ma_validation_blocking_with_candles_with_aux_mask` silently rewrites `max_candidates == 0` to the default and truncates `req.candidates` before any work begins (`3434-3438`). Later, the GPU execution profile is built from the already-truncated candidate slice (`1362-1364`), and `ValidationExecutionProfile::requested_candidates` is populated from that post-truncation length (`233-266`).

That means `requested_candidates` does **not** currently mean “how many candidates the caller submitted.” It means “how many candidates survived pre-validation truncation and were handed to the GPU-prep path.” CPU results expose no structured signal for this at all because `execution_profile` is GPU-only in practice.

This is not a wrapper-layer redesign problem. It is a local naming / telemetry / documentation problem that the third-pass packet missed. At minimum, the field needs documentation. If the repo is willing to accept additive serialized-output drift, a safer bounded hardening step would be to add an explicit `submitted_candidates` or `evaluated_candidates` field rather than repurposing the existing one.

### 3. The packet also missed a local public-result contract: many aux fields are finalist-only, and one is SPA-coupled

The default public entrypoint always calls the aux-mask variant with `ValidationAuxWorkMask::full()` (`3406-3417`), which makes the result shape look like a “full analytics” result. But the implementation does not compute many optional analytics for all candidates.

Instead, hard-coded finalist caps are applied (`2117-2122`, `3733-3768`), and the optional fields are populated only on the leading candidates for those capped slices (`3790-3843`, `3846-3870`). `benchmark_slice_summary` is even narrower: it is only considered when SPA is requested (`3758-3761`, `3834-3842`).

So for fields like `regime_slice_robustness`, `friction_stress`, `entry_margin_fragility`, `underwater_duration`, `checkpoint_robustness`, and `benchmark_slice_summary`, `None` does not simply mean “feature absent.” It can mean:

- not requested by aux mask,
- not in the finalist subset,
- or, for benchmark slices, SPA was not requested.

That is a real public-result semantics issue entirely within this file. It should have been called out as a safe-now docs/test hardening target. It also has performance implications: broadening those computations to every candidate would be a CPU-cost change, so the safe fix is documentation and explicit tests, not widening the work casually.

### 4. One earlier “safe-now” idea was materially riskier than the packet acknowledged

The earlier recommendation to normalize semantically inactive `cross_ticker` payloads before GPU blocking is not a safe local cleanup anymore.

This file contains an explicit CUDA-gated unit test asserting that a direct `GpuKernel` validation request with `cross_ticker: Some(CrossTickerConfig { datasets: Vec::new(), max_datasets: 0, ... })` falls back to CPU and records a `cross_ticker` fallback reason (`5234-5286`). That makes the behavior deliberate and test-pinned for direct app-core callers.

Changing `gpu_validation_feature_blocker` to treat that payload as inactive would rewrite a currently tested app-core contract, alter route diagnostics, and change CPU/GPU route comparability for direct callers. The third pass was correct to keep the defer bucket, but it still should have called this regression risk out more explicitly.

### 5. The SPA point is weaker locally than the third pass made it sound

The third-pass packet treated SPA ambiguity as a co-equal reason to keep this file in defer. That is too strong based on the live source alone.

In this file, `req.spa == None` still has a clear meaning: SPA was not requested (`4014-4015`). When SPA is requested but there are not enough bars after warmup, the code returns `Some(SpaValidationStats { skipped_reason: Some("spa_skipped_not_enough_bars_after_warmup"), ... })` (`3999-4011`). So the strongest ambiguity claim is not demonstrated in this file’s own control flow.

That does **not** mean the SPA concern is false. It means the concern is mostly inherited from `validation_pipeline::compute_spa_validation_stats`, not directly shown here. In this packet, it should be treated as a secondary coordination note, not as a primary local driver on the same level as the request/result ownership split.

### 6. The redesign is harder than “move wrapper-owned fields out of the DTO”

The third pass correctly argued that wrapper-owned behavior should stop being advertised as core-owned behavior. But it still understated what that means operationally.

`DoubleMaValidationRequest` is a plain `Deserialize`-derived struct with field-level serde defaults and no container-level `deny_unknown_fields` guard (`1491-1535`). Serde’s container-attribute docs make clear that, for self-describing formats like JSON, unknown fields are ignored by default unless `deny_unknown_fields` is added. In a Tauri app, that matters because this request shape sits on an IPC trust boundary, and Tauri’s security guidance explicitly emphasizes strongly defining data passed between boundaries.

So a future cleanup that merely removes `cross_ticker` or `validation_context` from the app-core DTO may not actually harden runtime behavior for JSON / IPC callers. Old payloads can keep deserializing unless the wrapper layer also adds explicit rejection, preflight validation, versioned DTOs, or a deliberate `deny_unknown_fields` policy.

That is a meaningful fourth-pass sharpening: the larger redesign is not just “move some fields.” It is compatibility-sensitive IPC contract work.

## Implementation Priority

### Safe-now hardening

The safest bounded work is local and documentation-heavy.

First, add rustdoc on the public request/result DTOs and both public entrypoints stating, plainly, that app-core owns core fixed-candidate validation only; wrapper layers currently own `validation_context` enforcement, cross-ticker execution/population, and final end-to-end timing in some paths. That documentation should also state that `max_candidates` truncates the candidate list before evaluation, and that `execution_profile.requested_candidates` currently reflects the post-truncation set rather than the raw caller submission.

Second, add direct app-core boundary tests for the local semantics the current packets still leave implicit. The most valuable new tests are:

- a truncation-focused test that proves how `max_candidates` affects evaluated candidates and any emitted execution-profile counts,
- a finalist-only aux-analytics test with more candidates than the hard-coded finalist caps, so the partial-population behavior is explicit rather than accidental,
- and, only if the repo wants to freeze the current stopgap until redesign, a direct CPU-path test covering wrapper-owned fields so the no-op behavior is at least intentional and documented.

Third, if the repo wants one bounded code change now instead of docs-only hardening, the cleanest one is an **additive** telemetry field for submitted/evaluated candidate counts. Do not rename `requested_candidates` in place without a compatibility plan.

### Larger redesign still deferred

The larger redesign target is still the same: separate core validation semantics from orchestration semantics.

That means choosing one of two real end-states:

- app-core becomes the true owner of cross-ticker execution, validation-context enforcement, and timing semantics, or
- app-core exposes a smaller core DTO surface while Tauri/headless own an explicitly separate orchestrated DTO surface.

That redesign should also settle `runtime_ms` meaning across app-core, Tauri, and headless paths, and it should decide whether wrapper-owned inputs are tolerated no-ops, explicitly rejected, or fully executed by the layer that advertises them.

The fourth-pass addition is that this redesign should be planned as IPC compatibility work, not just type cleanup. If the team wants stronger runtime rejection at the boundary, a field-removal-only refactor is not enough.

## Verification If Changed

- `cargo test -p ta_app_core double_ma_validation -- --nocapture`
- add and run a targeted app-core test for `max_candidates` truncation / execution-profile semantics
- add and run a targeted app-core test that proves finalist-only aux-field population when candidate count exceeds the current caps
- if touching wrapper-owned field semantics, rerun:
  - `cargo test -p ta_desktop_demo_app validation_context_rejects_dataset_mismatch -- --nocapture`
  - `cargo test -p ta_desktop_demo_app cross_ticker_headless_double_ma_smoke_applies_stats -- --nocapture`
- if touching GPU blocker behavior, rerun:
  - `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel double_ma_validation_gpu_kernel_direct_cross_ticker_request_falls_back_to_cpu -- --nocapture`
- if changing serialized DTO output or execution-profile fields, rerun any report/artifact or headless validation snapshots that consume validation JSON, because this file’s DTOs are re-exported from `ta_app_core`

## Regression And Feature-Preservation Constraints

Do **not** treat inactive-`cross_ticker` normalization as a harmless cleanup. In this file it is now test-pinned behavior for direct GPU-kernel requests, so changing it affects route semantics and benchmark comparability.

Do **not** turn `cross_ticker` or `validation_context` into app-core hard errors without coordinated wrapper/headless work. The current codebase still passes these fields through shared DTOs, and a unilateral core rejection would be a contract-tightening change, not a refactor.

Do **not** widen finalist-only aux analytics to every candidate casually. That changes CPU cost and output semantics together.

Do **not** assume a DTO-field removal alone hardens the JSON / IPC boundary. With current serde posture, request-shape tightening is a separate compatibility decision.

Do preserve current public field names unless the repo is ready to treat the change as an artifact/reporting compatibility event. In particular, `requested_candidates` is misleading today, but a rename is more disruptive than an additive clarification.

## Fourth-Pass Conclusion

`Bucket stands.` The current third-pass `Larger refactor but defer` classification is still the right top-level bucket because the dominant issue remains the cross-layer ownership split around `cross_ticker`, `validation_context`, and timing semantics.

What the third pass got right was the architectural diagnosis and the caution against an uncoordinated hard-error fix.

What it missed was that there is a real, bounded hardening slice available **now** inside this file: document and test candidate truncation semantics, execution-profile meaning, and finalist-only aux-result behavior. Those are local, evidence-backed, and low-risk compared with DTO surgery.

What it overstated was the local weight of the SPA ambiguity, and what it underweighted was both the regression risk of changing `cross_ticker` GPU blocking and the IPC/serde compatibility complexity of any future DTO cleanup.

So the right fourth-pass posture is:

- keep the larger redesign deferred,
- land local docs/tests for the public semantics this file already has,
- and treat any true request/result contract cleanup as coordinated IPC and artifact compatibility work, not as a one-file tidy-up.
