# Fourth-Pass Review: crates/app_core/src/validation_shared.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/validation_shared.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/validation_shared.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/validation_shared.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/validation_shared.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file is still small, but it is not low-consequence. It is re-exported from `app_core`, it defines request/result/support DTOs that sit on validation-facing seams, and it owns the deterministic key helper that affects candidate identity. The earlier packets were right to classify it as a compatibility seam rather than a cleanup target, but that framing was still a little too generic for a file that mixes serde defaults, sentinel values, sparse JSON shape decisions, and helper semantics.

The current third-pass bucket should **stand**. I do **not** see evidence that this file needs a redesign bucket escalation. The right posture is still small, surgical hardening. What changed in this fourth pass is the priority within that bucket: the highest-value work is not broad “more docs” in the abstract, but a few very targeted invariant tests and one narrowly scoped default-path cleanup.

## Fourth-Pass Research Log

I reread the live `validation_shared.rs` source first, then checked the third-pass packet against the exported `lib.rs` surface and the nearby `validation_pipeline.rs` logic that consumes these configs. That cross-file read mattered because several behaviors that look like plain DTO defaults in this file are actually completed by sanitization or sentinel handling in the pipeline layer.

I also checked the relevant primary-source serde and Rust API guidance to calibrate which recommendations are truly safe. Serde’s field-default documentation confirms that omitted fields are filled from explicit default functions or `Default` implementations during deserialization. Serde’s container-attribute documentation also confirms that unknown fields are ignored for JSON unless `deny_unknown_fields` is added. The Rust API Guidelines’ dependability guidance remains a useful reminder that stricter validation is often desirable, but in this file that advice has to be balanced against IPC and artifact compatibility. Finally, RFC 8785 was useful as a calibration point for the canonical-key helper: the helper here is clearly a deterministic internal key normalizer, not a full canonical-JSON implementation.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right about the big picture. This file is more dangerous to “improve” than its size suggests, because changing defaults, enum spellings, omission behavior, or deterministic-key behavior can ripple into validation selection, saved artifacts, and Tauri-facing request/result contracts. It was also right to avoid pushing a redesign of `canonical_json_key()` or the auxiliary-work mask.

It was also correct to keep the bucket at `Small surgical refactor`. There is still no strong evidence that representation churn in this file would buy enough to justify compatibility risk.

### What it missed or underweighted

The biggest missed issue is that `CrossTickerGateConfig` currently has **two behavior-defining default paths** that must remain equivalent. Missing the whole nested `gate` object in `CrossTickerConfig` flows through `CrossTickerGateConfig::default()` (`validation_shared.rs:371-380`), while missing fields inside a present `gate` object flow through the field-level serde defaults on `CrossTickerGateConfig` itself (`validation_shared.rs:339-355`). Those two paths are manually kept in sync by the explicit `Default` impl (`validation_shared.rs:357-368`). That is a real drift surface. A future one-sided edit would make `gate: {}` behave differently from omitting `gate` entirely. None of the earlier packets called that out, and it is the clearest concrete hardening target in the file.

The second underweighted issue is that the most regression-prone behavior is **split across files**, not contained entirely in this module. Several public configs here are only half the contract; `validation_pipeline.rs` completes the semantics through clamping or sentinel interpretation. `WalkForwardConfigOptional` and `ReoptWalkForwardConfigOptional` accept raw values here, but the pipeline clamps `window_bars` to at least `64`, `step_bars` to at least `1`, and `top_n_candidates` to at least `1` (`validation_shared.rs:131-145`; `validation_pipeline.rs:677-679`, `843`, `851-852`). `SpaConfigOptional` similarly allows raw values here while the pipeline enforces minimum slices and block length at use time (`validation_shared.rs:156-164`; `validation_pipeline.rs:1532-1545`). `TradeBootstrapConfig` looks like a straightforward optional stationary-bootstrap parameter here, but `Some(0)` is actually a public sentinel meaning “IID-only mode; skip stationary stats” (`validation_shared.rs:291-297`; `validation_pipeline.rs:1261-1279`, plus the lock-in test at `validation_pipeline.rs:2139-2169`). `PerturbConfig` is even more subtle: bare `#[serde(default)]` on scalar fields means omitted JSON deserializes to all zeros, and the pipeline interprets `iterations == 0` as “disabled” (`validation_shared.rs:265-275`; `validation_pipeline.rs:1126-1129`). The earlier packets generally said “document defaults,” but that is not precise enough. The contract here is partly omission defaults and partly downstream sentinel behavior.

The third underweighted issue is **mixed sparse-shape serialization**. This file uses several different outward JSON shape policies at once. `ValidationArtifactContext` keeps `Option` fields as explicit `null` when absent (`validation_shared.rs:4-14`). `ValidationAdmissionProvenance` omits `heads` when empty but still serializes `note: null` when absent (`validation_shared.rs:38-58`). `ReoptWalkForwardStats` omits several optional efficiency fields and also omits empty vectors (`validation_shared.rs:209-245`). `TradeBootstrapStats` omits stationary fields when unavailable (`validation_shared.rs:299-323`). That asymmetry may be entirely acceptable, but it is not trivial. A “cleanup” pass that adds `skip_serializing_if` everywhere, removes explicit nulls, or normalizes empty strings into omitted fields would change persisted artifact shape and IPC payload shape even if the Rust-side meaning stayed the same. The earlier packets did not emphasize this enough.

The fourth underweighted issue is that the current advice about “serde/default round-trip” testing is too broad for the actual type split in this file. Many of the stats/result structs here are intentionally `Serialize`-only, not `Deserialize` (`WalkForwardStats`, `ReoptWalkForwardStats`, `SpaValidationStats`, `PerturbStats`, `TradeBootstrapStats`, `CrossTickerStats`, and related types). That means a generic call for round-trip tests can accidentally push the wrong change: adding `Deserialize` just to satisfy a review packet. I would avoid that. Input/default tests and JSON-shape stability tests are valuable here. Blanket round-trip expectations are not.

Finally, the earlier packets were directionally right about `canonical_json_key()`, but still too vague about what to lock down. The current tests only prove that map insertion order does not matter (`validation_shared.rs:490-507`). They do **not** prove nested-object stability, array order sensitivity, or parity between `canonical_json_key()` and `canonical_json_key_bytes()` (`validation_shared.rs:445-476`). Since this helper is tied to candidate identity, those are the next specific invariants worth pinning. The goal is still **not** to turn this into standards-grade canonical JSON; it is to make the intended narrower contract explicit and testable.

### Safe-now hardening

The highest-priority safe-now change is a targeted invariant around `CrossTickerGateConfig`. At minimum, add a test that deserializing `{}` into `CrossTickerGateConfig` yields the same field values as `CrossTickerGateConfig::default()`. That test would lock the two defaulting paths together and directly defend against the drift surface described above. If the repo wants a tiny code cleanup beyond that, the only one I would endorse here is using struct-level serde defaulting for `CrossTickerGateConfig` so the type’s `Default` impl becomes the single source of truth for missing-field behavior. Even that should land with the equivalence test first.

The next safe-now hardening target is not more generalized doc text, but a few **behavioral** tests aimed at the cross-file contract. The best candidates are: confirming that omitted `PerturbConfig` fields still produce the current zeroed no-op posture; confirming that `TradeBootstrapConfig { stationary_mean_block_len: Some(0) }` stays in IID-only mode; and confirming that low or zero walk-forward / reopt / SPA inputs continue to be sanitized in the pipeline rather than rejected or reinterpreted earlier. Those tests belong partly in `validation_shared.rs` and partly in `validation_pipeline.rs`, because that is where the effective contract actually lives.

The next safe-now hardening target is a couple of representative JSON-shape tests for persistence-sensitive DTOs. I would not try to snapshot every struct in this file. Two or three high-signal cases are enough: one sparse context/provenance shape, one stats shape with omitted optional fields, and one deterministic-key helper case with nested objects and arrays. That would give maintainers a concrete tripwire against accidental null-vs-omitted drift.

Rustdoc is still worthwhile, but now with more precision than the earlier packets suggested. The comments worth adding first are the ones that explain sentinel semantics and scope boundaries: `PerturbConfig` zero-default means disabled work unless explicitly requested; `TradeBootstrapConfig::stationary_mean_block_len = Some(0)` is the IID-only sentinel; `ValidationAuxWorkMask::preserved_finalists()` is an execution-cost policy; and `canonical_json_key()` is for deterministic internal keys, not standards-compliant canonical JSON.

### Larger redesign still deferred

I would still defer any attempt to make this surface “stricter” through broad serde policy tightening. In particular, adding `#[serde(deny_unknown_fields)]` across these shared structs is **not** safe-now hardening. It would tighten JSON acceptance semantics for request payloads and possibly saved artifacts, and Serde’s default JSON posture is currently permissive. If the product wants typo rejection or stricter request validation, that should happen through coordinated request-layer work or versioned wrappers, not by retrofitting this entire shared DTO layer in isolation.

I would also defer any broad JSON-shape harmonization. Converting all optional fields to omission-based serialization, changing empty strings into `Option`, or normalizing all result structs into a common sparse-output style would be outwardly visible on persistence and IPC surfaces. That kind of cleanup may be desirable someday, but it is no longer “small surgical” once artifacts or frontend expectations enter the picture.

I would also defer any effort to replace `canonical_json_key()` with a hashing layer or a fuller canonicalization scheme. The helper is currently narrow, local, and easy to reason about. Replacing it would require auditing every caller that depends on candidate-key equality, dedupe, or drift computation. Without evidence of collisions, performance trouble, or standards requirements, that is unnecessary risk.

Finally, I would defer any push to merge the required and optional config shapes or to add `Deserialize` to serialize-only stats types. Those are broader API-shape questions, not the kind of hardening this packet should trigger.

## Implementation Priority

1. **Keep the bucket unchanged.** The third-pass `Small surgical refactor` bucket is still correct.
2. **First hardening step:** add the `CrossTickerGateConfig` equivalence test between whole-object defaulting and empty-object deserialization.
3. **Second hardening step:** add targeted contract tests for sentinel/default behavior that is completed in `validation_pipeline.rs`, especially `PerturbConfig` and `TradeBootstrapConfig`.
4. **Third hardening step:** add a few representative JSON-shape and canonical-key invariants, then add terse rustdoc for the remaining non-obvious behaviors.
5. **Do not treat this packet as approval** for broad serde tightening, DTO reshaping, result-type round-tripping, or canonicalization replacement.

## Verification If Changed

If this file changes only in comments, the existing file-local tests are enough. If any code or tests are added around defaults, shape, or key behavior, I would verify at three levels.

First, run the local shared-module tests:

- `cargo test -p ta_app_core validation_shared::tests:: -- --nocapture`

Second, run the validation-pipeline tests because the effective semantics are split across modules:

- `cargo test -p ta_app_core validation_pipeline -- --nocapture`

Third, if any serde-visible behavior changes, run representative validation callers to catch downstream regressions:

- `cargo test -p ta_app_core double_ma_validate -- --nocapture`
- `cargo test -p ta_app_core composite_validate -- --nocapture`

If a change touches output shape rather than only docs/tests, I would also expect the owning artifact or report surfaces to be checked before merge, because the real risk here is persistence/IPC compatibility more than raw computation correctness.

## Regression And Feature-Preservation Constraints

Preserve the current bucket and keep behavior stable unless there is a clearly demonstrated bug. In practice that means several specific “do not” constraints.

Do not let the two `CrossTickerGateConfig` defaulting paths diverge. Missing `gate` and `gate: {}` should continue to mean the same thing unless there is an intentional contract revision.

Do not reinterpret sentinel values casually. `PerturbConfig`’s zero-default posture currently means “disabled unless explicitly requested,” and `TradeBootstrapConfig::stationary_mean_block_len = Some(0)` currently means “skip stationary bootstrap and keep IID-only stats.” Treat both as contract, not cleanup opportunities.

Do not normalize sparse serialization shape casually. Null-vs-omitted fields, omitted empty vectors, and explicit empty strings/zero counts are currently mixed in this file. That is a persistence and IPC surface, not just a style issue.

Do not add blanket `deny_unknown_fields` here without coordinated migration. The likely fallout is request rejection and legacy-artifact deserialization failure, not safer code in isolation.

Do not broaden this file’s scope into GPU work. There is **no direct GPU ABI surface here**. The meaningful risks are validation semantics, compatibility, IPC/persistence shape, and validation-cost changes if auxiliary work or default-on analysis is expanded.

Do not read “more tests” as “derive `Deserialize` everywhere.” Several result structs are intentionally output-only today, and forcing round-trip symmetry would be a larger API decision than this packet justifies.

## Fourth-Pass Conclusion

The third-pass bucket should **stand**. `validation_shared.rs` remains a `Small surgical refactor` file, not a redesign candidate.

What the earlier packet got right was the compatibility framing: this is a contract seam, and changing defaults, labels, or helper semantics is riskier than the file size suggests. What it still missed was where the real risk concentrates. The sharpest missed issue is the duplicated default-path contract in `CrossTickerGateConfig`. The most underweighted behavior is the cross-file sentinel/default contract completed in `validation_pipeline.rs`. And the most overstated recommendation is the broad idea of generic serde round-trip hardening, which does not fit a file containing many serialize-only result types.

So the safe-now recommendation is narrower and stronger than the prior packet: add a few pointed equivalence and contract tests, document the sentinel behaviors that matter, and leave the rest alone. Larger moves—strict unknown-field rejection, DTO shape harmonization, canonicalization replacement, or API-shape unification—should remain deferred because they carry compatibility, persistence, and validation-risk far beyond what this file alone justifies.
