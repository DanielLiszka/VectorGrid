# Fourth-Pass Review: `crates/app_core/src/options_types.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/options_types.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet was directionally right that `options_types.rs` is a compatibility seam, not a refactor playground. It was also right to keep the bucket small. What it did not do was challenge the live source hard enough. This file is not just a bag of DTOs with some missing docs; it is a mixed contract surface that combines persisted manifests, Tauri-adjacent request/result payloads, inspection DTOs, and additive feasibility telemetry in one place. That mixture makes some hardening ideas safe and high-value, while making others much riskier than the earlier packet implied.

The current third-pass bucket should **stand**. The file still does not justify a redesign-first response. But the safe-now center of gravity is narrower and more concrete than “write docs and tighten invariant coverage.” The fourth pass found one especially important wire-contract edge case, plus a few places where earlier packets were too broad about serde strictness and enum migration.

## Fourth-Pass Research Log

This pass reopened the live source first, then cross-checked only the nearest contract consumer in `crates/app_core/src/options_expression.rs` to verify whether suspicious fields were just inert schema baggage or actual live policy inputs.

I focused the review on four questions.

First, I rechecked the serde behavior around whole-field defaults versus nested-field defaults, because `OptionsExpressionRunRequest` and `OptionsStaticFilterBundle` both rely on omission semantics. The official Serde field/container attribute documentation was useful here because the risk in this file is not serialization syntax by itself, but what a missing key means once the payload becomes persisted JSON and IPC input.

Second, I rechecked public-API expectations for documented defaults and compatibility-sensitive types using the Rust API Guidelines documentation guidance. That mattered because these types are re-exported from `ta_app_core` in `crates/app_core/src/lib.rs:230-244`, so the missing rustdoc is not just local ergonomics debt.

Third, I traced which apparently “small” fields actually feed runtime behavior. In particular, I checked filter application, structural cache keys, backend label production, and verdict refresh logic in `options_expression.rs` rather than assuming these were passive transport fields.

Fourth, I explicitly looked for places where earlier packets may have overgeneralized. The two main areas were blanket serde strictness recommendations and broad enum/newtype migration suggestions.

## What Fourth Pass Sharpened

### The third-pass packet got the bucket right, but it stayed too abstract

The third pass correctly identified the file as compatibility-sensitive and correctly resisted broad refactor churn. It also correctly emphasized preserving default values, omission behavior, and exported labels unless consumers are updated together. That still holds.

What it underweighted is that this file contains a few **behaviorally active** defaults and duplicated summary fields that deserve more targeted scrutiny than the third-pass packet gave them.

### The biggest missed finding is the `filters` default asymmetry

`OptionsStaticFilterBundle` has a custom `Default` implementation that sets `max_entry_lag_minutes` to `Some(5)` at `options_types.rs:301-310`. `OptionsExpressionRunRequest.filters` then uses `#[serde(default)]` at `options_types.rs:1064-1065`, so omitting the entire `filters` field gives the caller that bundle default.

But inside `OptionsStaticFilterBundle`, each field uses field-level `#[serde(default)]` at `options_types.rs:289-298`. That means a payload with `"filters": {}` or with a present `filters` object that simply omits `max_entry_lag_minutes` does **not** deserialize to `Some(5)` for that field. It deserializes to `None`.

That is not a cosmetic difference. `max_entry_lag_minutes` directly gates candidate eligibility and can produce `EntryPastCutoff` in `options_expression.rs:1024-1028` and `1058-1062`. The filter bundle also participates in the structural candidate cache key at `options_expression.rs:374-409`. So this one omission-shape difference can change runtime results, cache identity, and persisted request semantics.

This is a sharper and safer target than the earlier packet’s broader serde concerns. The immediate action is **not** to change behavior blindly. The immediate action is to add a regression test that proves the current omission matrix and then make an explicit product decision about whether this asymmetry is intentional. Right now it is too easy for two JSON payloads that look semantically equivalent to behave differently.

### The file has more drift-prone duplicated state than the third pass acknowledged

Earlier review history rightly called out `validation_verdict` as derived state. The fourth pass confirms that concern, but it should not be treated as the only one.

`validation_verdict` is persisted at `options_types.rs:543-544`, inferred at `options_types.rs:584-601`, and refreshed producer-side in `options_expression.rs:2908-2910`. But the same general pattern exists elsewhere.

`robust_score` is also persisted at `options_types.rs:545-546` and recomputed producer-side in `options_expression.rs:2914-2917` and `3122`. `OptionsExpressionRunResult` stores `tuple_count` separately from `ranked_results` at `options_types.rs:1087` and `1096-1097`. `OptionsExpressionEventPathInspectionResult` stores `point_count` separately from `points` at `options_types.rs:729-731`. The dataset manifest stores `row_count` and `contract_count` separately from the actual `contracts` vector at `options_types.rs:344-345` and `390-391`, and the engine does consume manifest counts in places like `options_expression.rs:390-393`.

I did **not** verify a load-side refresh path for every one of these duplicated fields from the provided bundle, so I am not claiming all of them are currently broken. But the fourth-pass conclusion is that the third pass underweighted this broader class of contract drift. The safe-now response is targeted invariant coverage and loader-edge sanity checks, not field removal.

### Blanket serde strictness remains too aggressive for this file

The first-pass packet’s concern about permissive deserialization was reasonable, but the stronger suggestions around container strictness still read too broad after reopening the live source.

This file does not expose one uniform category of payload. `OptionsDatasetManifestV1`, `OptionsSignalEventManifestV1`, `OptionsExpressionRunRequest`, `OptionsExpressionRunResult`, and `OptionsExpressionArtifactV1` are the highest-risk surfaces. But `OptionsExpressionFeasibilityTimingStats`, `OptionsExpressionFeasibilityCacheStats`, overlap summaries, coverage summaries, and inspection payloads are additive telemetry/reporting surfaces where permissive defaults are materially less dangerous and are often exactly what keeps older artifacts readable.

That matters because this file is not only persistence-facing; it is also Tauri-boundary-adjacent. Blanket `deny_unknown_fields` on request/result/report DTOs would not just harden disk loads. It would also make additive UI/backend skew fail harder at runtime. The right framing is per-type and versioned, not “tighten serde on major structs” as a general cleanup task.

### The enum migration advice needs narrower scope than earlier packets suggested

The earlier packet was right to notice several raw-string discriminators. But the fourth pass does not support a broad “convert string fields to enums/newtypes” recommendation as a small surgical follow-up.

Some of these strings look locally owned and narrow enough that constants or eventual typed wrappers would make sense: `backend_used` at `options_types.rs:1083` and `1110`, `best_available_backend` at `998`, and `recommended_backend` at `987`. The engine currently emits a very small label set for them in `options_expression.rs:2291-2300`, `2362-2365`, and `3807-3835`.

Others are riskier and probably not locally owned in the same way: `parent_run_kind` at `398` and `437`, `adjustment_mode` at `405` and `444`, plus provider/reference provenance fields in the dataset manifest. Those values are not just display labels; some of them feed manifest cache hashing at `options_expression.rs:333-369`. Treating all of them as local closed enums would be a broader cross-subsystem migration, not small hardening.

So the safer fourth-pass position is: narrow constant centralization is fair game for clearly local backend/artifact labels, but sweeping enumization across provenance-bearing strings is still deferred.

### `inferred_validation_verdict()` has a real edge-case contract gap

`OptionsExpressionTupleResult::inferred_validation_verdict()` promotes a row to `SealedFinalist` or `StressFragile` whenever supported sealed metrics exist at `options_types.rs:584-599`. It does **not** require `sealed_finalist == true`.

Today that may be harmless because producer-side code appears to keep the flag and the sealed metrics aligned. But the helper itself does not enforce that relationship, and verdict refresh re-applies the helper in `options_expression.rs:2908-2910`. That means a stale or hand-edited artifact with sealed metrics but `sealed_finalist = false` would still come back as a sealed-style verdict.

I would not change that behavior casually in this pass because it is exactly the kind of seemingly reasonable cleanup that can alter how older artifacts display. The safer action is to add an invariant test that makes the intended relationship explicit and then decide separately whether verdict inference should require both the flag and the metrics.

### One public field name is more misleading than the earlier packets acknowledged

`OptionsExpressionArtifactV1.selection_metrics` at `options_types.rs:1124` is not just a metrics vector. Its type is `Vec<OptionsExpressionTupleResult>`, so it is the retained ranked tuple payload for the artifact, not merely selection metrics.

That is not a reason to rename it in place. Renaming would be a persisted-schema migration. But it is worth calling out because the field name is materially more confusing in a public repo than the earlier packets suggested. Safe-now hardening here is documentation. A rename belongs in a versioned contract cleanup, not in a small patch.

## Implementation Priority

### Safe-now hardening

1. **Add a serde contract test for `filters` omission shape first.** Cover at least three request payloads: omitted `filters`, `"filters": {}`, and `"filters": {"max_entry_lag_minutes": null}`. That test should assert the actual deserialized `OptionsStaticFilterBundle` values. If the intended product contract is “5-minute lag unless explicitly disabled,” then a follow-up change is justified. If the current asymmetry is intentional, it needs to be documented as such.

2. **Add one downstream engine test proving the behavioral consequence of that filter contract.** The tightest version is a test that shows a late entry is rejected or accepted depending on the deserialized filter state, or a test that proves the structural cache key changes across the two request shapes.

3. **Expand invariant coverage around duplicated fields rather than deleting them.** The highest-value additions are: sealed metrics versus `sealed_finalist` alignment, `validation_verdict` refresh behavior, `tuple_count` versus `ranked_results.len()`, and `point_count` versus `points.len()`. For `robust_score`, this pass supports at least auditing whether artifact-load paths recompute it; if not, add a targeted follow-up issue rather than guessing.

4. **Add rustdoc to the highest-blast-radius public types.** The priority list is still `OptionsDatasetManifestV1`, `OptionsSignalEventManifestV1`, `OptionsExpressionRunRequest`, `OptionsExpressionRunResult`, `OptionsExpressionArtifactV1`, and `OptionsExpressionTupleResult`. The docs should explicitly label: active defaults, backward-compatibility defaults, derived fields, duplicated counts, and provenance-bearing strings.

5. **Narrow the string hardening to clearly local labels.** Centralizing backend label constants is reasonable. Broad enum conversion is not yet. If this file is touched, keep that change limited to values this module obviously owns.

6. **Add one small helper test for `allowed_fill_models_label()`.** This is not a top-tier issue, but it is cheap protection for an error-message helper that is manually synchronized with `supports_fill_model()`.

### Larger redesign still deferred

A module split is still deferred. The file would be cleaner as separate manifest, run-contract, inspection, and report submodules, but that is organizational debt, not the most valuable next patch.

A versioned schema cleanup is also still deferred. That future pass is where it would make sense to rename `selection_metrics`, decide whether `validation_verdict` and possibly `robust_score` should remain stored, revisit zero-versus-missing numeric compatibility fields, and selectively replace locally owned raw-string labels with typed wire forms.

Any deserialize strictness increase beyond targeted loader validation should also stay deferred until a versioned migration story exists. The current file is too persistence-sensitive and too IPC-adjacent for opportunistic strictness changes.

## Verification If Changed

At minimum, any change in this file should rerun the existing file-local contract coverage and the nearest engine tests:

- `cargo test -p ta_app_core options_types::tests:: -- --nocapture`
- `cargo check -p ta_app_core --tests`
- `cargo test -p ta_app_core options_expression::tests::refresh_validation_verdicts_requires_supported_validation_lanes -- --nocapture`

If request-field defaults or artifact-visible fields change, rerun the focused replay/provenance tests already identified in the third-pass packet as well:

- `cargo test -p ta_desktop_demo_app --lib commands_options_expression::tests::signal_manifest_replay_cache_round_trips_to_disk -- --nocapture`
- `cargo test -p ta_desktop_demo_app --lib commands_options_expression::tests::options_dataset_replay_cache_round_trips_to_disk -- --nocapture`
- `cargo test -p ta_desktop_demo_app --lib commands_options_expression::tests::saved_artifact_replay_provenance_accepts_matching_dataset_hash_and_semantics -- --nocapture`

If the `filters` contract changes specifically, add a new targeted regression test that compares omitted `filters` versus an empty `filters` object before landing the change. That is the new verification gap this pass found.

## Regression And Feature-Preservation Constraints

The third-pass bucket should stand, but only if the implementation stays conservative.

Do **not** blanket-add `deny_unknown_fields` to the major DTOs in this file. That risks breaking persisted artifacts and additive Tauri request/result evolution in one step.

Do **not** remove collection defaults wholesale. Several empty collections in this file are plausibly valid payload states, especially in inspection and report DTOs. Earlier review history overreached a bit by treating too many of them as obviously malformed.

Do **not** change the `filters` omission semantics casually. That one change can affect candidate eligibility, invalid-reason tallies, structural cache keys, replayed request behavior, and downstream research reproducibility.

Do **not** rename string labels or persisted field names like `selection_metrics` without a migration path. That is persistence and IPC contract churn, not cleanup.

Do **not** tighten `inferred_validation_verdict()` in place unless you first audit how existing artifacts are loaded and refreshed. The helper’s current behavior is imperfect, but an unreviewed semantic change there is still a validation compatibility risk.

There is no meaningful GPU ABI risk or benchmark-sensitive optimization opportunity in this file itself. The real risk surface is persistence, replay, validation semantics, cache identity, and request/result IPC drift.

## Fourth-Pass Conclusion

`Bucket unchanged.` The third-pass `Small surgical refactor` bucket remains the right call.

What the third-pass packet got right was the overall posture: this file is a contract surface, not a performance target, and its highest-value work is additive hardening rather than structural churn.

What it missed was the most actionable edge case in the file: the `filters` omission asymmetry created by combining a whole-struct default with field-level defaults inside `OptionsStaticFilterBundle`. That is a live behavior seam, not just documentation debt. The third pass also underweighted the broader class of duplicated summary fields, overstated how broadly serde strictness could be tightened, and treated string-field enumization more uniformly than the live ownership boundaries support.

So the safe fourth-pass recommendation is narrow and evidence-backed: keep the bucket, add the missing serde/default regression coverage, document the public contract more explicitly, add a few targeted invariants for duplicated fields, and defer schema cleanup, strictness changes, and broad enum migration until there is a versioned contract plan.
