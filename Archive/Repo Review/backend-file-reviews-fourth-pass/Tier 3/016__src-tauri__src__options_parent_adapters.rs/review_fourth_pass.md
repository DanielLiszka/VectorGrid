# Fourth-Pass Review: `src-tauri/src/options_parent_adapters.rs`

- Review date: `2026-04-16`
- Source file: `src-tauri/src/options_parent_adapters.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file deserved a fourth pass because the current third-pass packet kept the right bucket but framed the file as a thin public Tauri IPC seam. The live source does not support that framing. `options_parent_adapters.rs` is a private support module under `commands.rs`, and the public command surface exported from `commands.rs` and registered in `lib.rs` consists of the actual options-expression commands, not these adapter helpers. The real risk here is persisted-artifact replay semantics, not command registration, cancellation scope, or runtime authority.

That distinction matters because the safer recommendations change with it. This file is still replay-semantics-sensitive and artifact-compat-sensitive, but it is not the place to optimize command ownership or do manual desktop smoke unless the command layer moved with it.

## Fourth-Pass Research Log

I re-read the live adapter source first, then checked the review history to see where the current packet had drifted. I used `commands.rs` and `lib.rs` to verify whether this module is actually part of the Tauri command surface, and I checked `options_types.rs`, `options_expression.rs`, and additional repo search hits for `DoubleMaRequest`, `DoubleMaValidationParams`, `DoubleMaParamsResolved`, and `DoubleMaDrilldownRequest` to validate how much schema this adapter is really coupling itself to.

I also rechecked a small set of external docs to ground the compatibility conclusions. The Rust API Guidelines checklist remains relevant because this is an adapter/conversion seam where failure modes and invariants should be explicit. The `serde_json::Value::pointer` docs matter because missing JSON Pointer paths return `None`, so this file’s fallback order is part of the real contract. The Serde container and field attribute docs matter because unknown JSON fields are ignored by default for self-describing formats like JSON, while missing fields only deserialize cleanly when defaults or aliases are explicitly provided. The Tauri “Calling Rust from the Frontend” docs were still useful, but mainly as a negative check: they reinforce that the public IPC boundary is the command layer, which is not what this file implements.

## What Fourth Pass Sharpened

### The current third-pass bucket should stand, but the current third-pass rationale should not

`Small surgical refactor` is still the right bucket. The file is small, semantically sensitive, and not a performance hotspot. The third-pass packet got that much right, and it was also right not to push a broad rewrite.

What it got wrong is the class of risk. The third-pass write-up treated this file as though it owned command registration, event identity, state ownership, and cancellation semantics. The code does not bear that out. In `commands.rs`, `options_parent_adapters` is a private module declaration beside `commands_options_expression`, not part of the public re-export surface. In `lib.rs`, the Tauri `invoke_handler` exposes `run_options_expression`, `analyze_options_expression_feasibility`, `inspect_options_expression_tuple`, and related command functions, not any helper from this adapter module. So the third-pass emphasis on “public IPC seams,” “registered command names,” and “manual desktop smoke for the command path this file owns” was materially overstated for this source file.

### `supported_options_parent_family_from_artifact` is only a kind allowlist prefilter, not a compatibility proof

The current packet underweighted a useful boundary: `resolve_strategy_kind` and `supported_options_parent_family_from_artifact` only inspect the serialized strategy-kind string. They do not validate that `/run/req` is present, that it still deserializes as a double-like request, or that validation candidates or `run.res.best_params` are replayable. Full replay compatibility is only established later when `build_double_like_parent_replay` deserializes the saved request and candidate payloads.

That means the helper name is slightly stronger than the behavior. In practice it is a classification gate, not a replay-success guarantee. I would not force a rename in this pass, but I would document that boundary explicitly and add tests that make it obvious a “supported family” classification can still fail later on malformed artifact shape.

### The file currently collapses “missing strategy kind” and “unsupported strategy kind” into the same path

`resolve_strategy_kind` returns an empty `String` when no non-empty string can be found at any of the probed JSON Pointer paths. That empty string then flows through `supported_options_parent_family_from_artifact` into the generic “supports X parent artifacts only” error. So a corrupted or partial artifact with no usable strategy kind is reported the same way as a well-formed artifact whose kind is simply not in the allowlist.

That is not a correctness bug, but it is an avoidable diagnostics blur. The safest hardening here is to keep the behavior the same while distinguishing:

- missing or malformed strategy-kind metadata
- known but unsupported strategy kind

That is a narrower and better-supported improvement than the third-pass packet’s command-boundary guidance.

### The strongest missed compatibility risk is over-deserializing shared DTOs

This was the main missed issue.

The adapter only needs a narrow slice of `DoubleMaRequest`: effectively `ma_source`, `strategy`, and `halftrend_filter` from `/run/req`. It only needs a narrow slice of `DoubleMaValidationParams`: the fast/slow lengths, MA kinds, optional MA params, strategy, and halftrend filter from the selected validation candidate. But the implementation deserializes the full shared structs.

That matters because Serde’s default JSON posture cuts both ways. Additive unknown fields are usually harmless because they are ignored unless `deny_unknown_fields` is enabled. So the earlier packet’s broad phrasing that “one family’s saved artifact shape diverges and replay silently stops being valid” was a little too broad. The narrower and more credible risk is this: if `DoubleMaRequest` or `DoubleMaValidationParams` gains a new non-defaulted required field that this adapter does not actually need, older persisted artifacts can become unreplayable here anyway, purely because this adapter is coupled to the full source DTO schema.

That is a real artifact-compatibility hazard. It is still not a reason for a big rewrite, but it is stronger than the third-pass packet’s IPC concerns and stronger than small prose dedupes like the support-list string.

### The replay fallback behavior is stricter, and safer, than earlier wording implied

The fallback contract is worth restating more precisely because a “cleanup” here could easily regress replay semantics.

- If `parent_candidate_index` is explicitly provided and the requested candidate is absent, the function errors immediately.
- If no explicit index is provided and candidate `0` is absent, the function falls back to `run.res.best_params`.
- If a candidate payload is present but fails to deserialize, the function errors; it does **not** silently fall back to `run.res.best_params`.

That fail-closed behavior is good. It avoids replaying the wrong parent candidate under the guise of being helpful. The first-pass packet gestured at the fallback order, but it underemphasized this distinction between “candidate missing” and “candidate present but malformed.” Any recommendation to make fallback more forgiving here would be regression-prone.

### There is a real invariance leak, but it belongs in deferred redesign, not in this pass

The deeper structural debt in this file is not command registration. It is that replay identity is split across loosely coupled fields.

`SupportedOptionsParentReplay` stores `trade_on_next_bar` separately even though that value is already present in `drilldown_request.strategy.trade_on_next_bar`. Then `build_supported_options_parent_signal_manifest` accepts `family` and `trade_on_next_bar` as separate free scalars rather than taking a typed replay context. On top of that, the downstream `DoubleMaDrilldownRequest` shape already duplicates MA params in both `params` and top-level `fast_ma_params` / `slow_ma_params`, so this adapter has to keep several mirrored values aligned.

That is the real redesign surface if the repo ever wants to harden this lane more aggressively. But it is cross-file and contract-sensitive. It should stay deferred unless the call sites are being redesigned together.

### Exact string matching should stay unless there is a deliberate artifact migration

I would also sharpen one earlier theme in a more conservative direction. The exact-string strategy-kind mapping is not a user-input normalizer. It is interpreting internal serialized artifact metadata. That means case-insensitive matching, loose aliases, or fuzzy acceptance would be riskier than they first sound, because they could silently accept stale or partially migrated artifacts that should instead fail clearly.

If the repo ever renames a strategy kind, the right place to carry compatibility is an explicit artifact migration or an intentional alias at the serialization boundary, not ad hoc broadening inside this adapter.

## Implementation Priority

### Safe-now hardening

The first safe-now step is to harden the adapter’s local contract tests rather than chase the third-pass packet’s command-boundary work. In particular, this file should gain direct fixture coverage for:

- missing strategy kind versus unsupported strategy kind
- explicit `parent_candidate_index` selecting a nonzero candidate
- explicit `parent_candidate_index` missing and failing closed
- implicit candidate-0 absence falling back to `run.res.best_params`
- candidate payload present but malformed, with no silent fallback
- supported-family classification succeeding even when later replay deserialization fails

The second safe-now step is a diagnostics pass. Keep current behavior, but separate “artifact missing usable strategy-kind metadata” from “artifact strategy kind is unsupported,” and include the resolved unsupported kind in the latter path when available.

The third safe-now step is documentation, not behavior change. Add short comments where they earn their keep:

- `supported_options_parent_family_from_artifact` is a kind allowlist prefilter, not a replay guarantee
- `parent_candidate_index` is fail-closed when explicitly requested
- candidate deserialization errors intentionally do not fall back to `run.res.best_params`
- `data_id = "artifact_replay"` and `bins = Some(256)` are compatibility/replay-shape choices, not casual defaults

The support-list prose duplication is real, but it is low leverage in a Tier-3 file. If this file is already being touched, either derive that string from a canonical list or add a tiny test that keeps the prose aligned. I would not treat that as the headline improvement.

### Larger redesign still deferred

The most defensible deferred redesign is to tighten the handoff so callers cannot drift `family`, `trade_on_next_bar`, and replay-derived drilldown state apart. That likely means pushing toward a single typed replay/manifest context instead of passing free scalars across helper boundaries. That work should stay deferred because it is cross-module and would need a careful call-site audit.

A second deferred lane is to reduce schema coupling by deserializing only the fields this adapter actually needs instead of the full shared `DoubleMaRequest` and `DoubleMaValidationParams` DTOs. I do think that is the most interesting long-term hardening opportunity that the earlier packets missed. But I would only do it after auditing the actual Serde field names, defaults, and aliases on those source DTOs. A rushed local DTO fork could create a second schema surface and introduce its own drift.

What should **not** be done under the banner of cleanup is loosening strategy-kind matching or widening replay fallback. Those changes are precisely the ones most likely to create silent artifact replay drift.

## Verification If Changed

The third-pass verification suggestion was too generic for this file. Manual desktop smoke is not the primary authority here unless the actual command layer changes. The right verification remains targeted backend coverage.

Keep the existing focused lanes referenced in the earlier packets if they still exist in the repo:

- `cargo test -p ta_desktop_demo_app options_parent_strategy_kind_support_matches_current_double_like_scope -- --nocapture`
- `cargo test -p ta_desktop_demo_app saved_artifact_replay_provenance -- --nocapture`
- `cargo test -p ta_app_core manifest_projection_uses_previous_bar_for_decision_when_trade_is_next_bar -- --nocapture`

If `bins` or replay-shape defaults move, also keep a direct drilldown smoke lane in scope, for example the existing `drilldown_smoke_has_bins_and_values` coverage in `ta_app_core`.

Most importantly, add local adapter tests for the candidate-selection and diagnostics cases listed above. Those are the gaps most directly tied to this file’s real contract.

## Regression And Feature-Preservation Constraints

Preserve the supported parent-family set unless product scope changes deliberately. This file is enforcing a narrow parent-artifact allowlist, and broadening that set without matching replay and manifest semantics would be unsafe.

Preserve exact-match strategy-kind handling unless there is an explicit artifact migration plan. Because this is interpreting persisted internal metadata rather than user-entered free text, permissive matching is more likely to hide drift than to improve compatibility.

Preserve the current fail-closed posture around validation candidate selection. An explicitly requested candidate index that is absent should remain an error, and a malformed candidate payload should not silently fall back to `run.res.best_params`.

Preserve `artifact_replay` and `bins: Some(256)` unless downstream replay consumers intentionally change their contract. There is no evidence in this file that either value is disposable.

Do not over-read `supported_options_parent_family_from_artifact` as a full compatibility check. It is a strategy-kind prefilter only. Any caller or future refactor that treats it as “artifact is replay-safe” would be smuggling in a stronger guarantee than the implementation actually makes.

Performance and GPU ABI risk are low here. The expensive work starts downstream in drilldown replay and options-expression evaluation. The meaningful risk in this file is artifact compatibility and validation/provenance drift, especially around candidate selection and strategy timing.

## Fourth-Pass Conclusion

The current third-pass action bucket should **stand**: `src-tauri/src/options_parent_adapters.rs` is still a `Small surgical refactor` file.

What should change is the explanation of why. The third-pass packet correctly resisted a broad rewrite, but it focused on the wrong problem class. This file is not a public command seam that primarily needs IPC, runtime-authority, or cancellation review. It is a persisted-artifact replay adapter that needs narrower contract hardening.

The strongest missed issue is unnecessary schema coupling from deserializing full shared `DoubleMaRequest` and `DoubleMaValidationParams` DTOs even though this adapter only consumes a subset of their fields. That is the clearest long-term compatibility risk, but it is still best handled conservatively: first with direct tests and clearer diagnostics, then with any narrower DTO redesign only after auditing the source Serde contract.

So the bucket stays put. Safe-now work is local tests, sharper malformed-vs-unsupported diagnostics, and explicit replay-contract comments. Larger redesign remains deferred, and any cleanup that broadens kind matching or softens candidate fallback should be treated as regression-prone.
