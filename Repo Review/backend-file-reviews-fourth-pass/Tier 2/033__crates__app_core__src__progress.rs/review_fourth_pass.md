# Fourth-Pass Review: crates/app_core/src/progress.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/progress.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/progress.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/progress.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/progress.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file is still tiny, but the live source confirms that it is not just a convenience helper. It exports a serialized progress payload (`progress.rs:5-8`), a public feedback enum (`progress.rs:12-16`), a public `Send + Sync` trait used as a trait object (`progress.rs:19-31`), and a thin forwarding helper that sits directly on the progress hot path (`progress.rs:35-46`). The module is also publicly exported from `crates/app_core/src/lib.rs:56`, so even small representation changes here are broader than the file size suggests.

The current third-pass bucket should **stand**. This is still a `Small surgical refactor` surface, but the fourth pass narrows what that should mean. The safe work is contract hardening, not representation cleanup. The third-pass packet was directionally correct about avoiding churn, but it stayed too generic and missed some live-source-specific constraints.

## Fourth-Pass Research Log

1. **Live source re-read of `progress.rs` and export confirmation in `lib.rs`.** This pass rechecked the file itself instead of inheriting the earlier packet framing. The important live facts are the all-public payload, the all-public feedback enum, the unsealed public trait, and the optional callback-based prefetch extension.

2. **Cargo SemVer Compatibility guidance.** I used the Cargo Book’s public API compatibility rules to re-evaluate earlier suggestions. For this file, the relevant parts are that adding fields to an all-public struct is a breaking change, adding enum variants or fields is a breaking change, changing trait item signatures is a breaking change, and even adding defaulted trait items is only “usually safe,” not free. That sharply limits how “small” a typed rewrite really is.

3. **Rust API Guidelines on object-safe traits, documentation, and future-proofing.** These guidelines were useful for separating what is good about the current shape from what is risky to change. `ProgressSink` is intentionally object-safe and used that way, but the same guidelines also reinforce that public traits and public fields are long-lived commitments unless they are sealed or hidden behind constructors/private fields.

4. **Tauri v2 frontend-calling guidance.** This was useful for pressure-testing IPC advice. The official docs explicitly say the event system is for small streamed payloads and is not the high-throughput path. That matters here because `double_ma_progress` is an event-driven frontend contract, so this file should stay lean and should not become the place where richer or chattier IPC semantics are introduced.

5. **Serde enum-representation guidance.** I checked this mainly to evaluate a future possibility that the current packet hints at: serializing `DoubleMaCampaignVramPrefetchFeedback` directly. The conclusion is that if that enum ever crosses IPC, its wire representation must be chosen deliberately rather than derived casually.

6. **`usize` and JavaScript numeric precision references.** I used the Rust standard library docs for `usize` and MDN’s `Number.MAX_SAFE_INTEGER` reference only to assess whether the payload counters hide an immediate IPC bug. My conclusion is that this is not an immediate correctness defect for the current desktop event use, but it is a real contract-shaping consideration if this payload ever becomes wider, versioned, persisted, or reused beyond today’s transient progress events.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right to keep the bucket unchanged. It was also right to treat this file as a compatibility seam rather than as “easy cleanup.” The current design is deliberately small, object-safe, and cheap to call. The helper is allocation-free, the hot-path payload is borrow-free except for a `'static` phase string, and the trait-object shape avoids generic monomorphization churn at every call site. Those are good properties to preserve.

It was also right that documentation is the highest-ROI improvement. The live source has no rustdoc at all, and the behavior that actually matters here is mostly semantic rather than algorithmic.

### What the third-pass packet missed or underweighted

The biggest miss is **public API evolution rigidity**. `DoubleMaProgressPayload` is an all-public struct (`progress.rs:5-8`), `DoubleMaCampaignVramPrefetchFeedback` is a public enum with public fields on the struct variants (`progress.rs:12-16`), and `ProgressSink` is a public unsealed trait (`progress.rs:19-31`). That means the surface is already much more frozen than the third-pass packet acknowledged. In practice, this makes “just add a field,” “just add a new feedback variant,” or “just reshape the trait a little” materially riskier than the earlier packet suggested.

The next miss is that `ProgressSink` is **not purely a sink**. The trait name and the earlier review language frame it as an output-only progress interface, but the live trait also contains an optional callback-based campaign-prefetch request hook (`progress.rs:22-25`) and a specialized feedback hook (`progress.rs:28-31`). That makes this a mixed progress-and-capability contract. The fourth pass therefore cares less about generic “progress DTO” concerns and more about clarifying that this trait now carries a narrow bidirectional extension point for campaign staging.

The third pass also underweighted **panic and blocking semantics**. `emit_double_ma_progress` is best-effort only in the narrow sense that `None` becomes a silent no-op (`progress.rs:41`). Once a sink exists, the helper does not catch panics, downgrade failures, or isolate blocking behavior. A sink implementation that panics will still unwind the caller. A sink implementation that blocks can still perturb hot loops. That should be documented explicitly. The earlier packets mentioned lossiness, but they did not draw this line cleanly enough.

Finally, the third pass underweighted the **frontend event contract**. `REPO_MAP.md` explicitly identifies `double_ma_progress` as a frontend-handled event channel and points to `taskStore.ts`, `taskStoreCampaignProgress.test.ts`, `taskStoreEventThrottle.test.ts`, and `optimizeService.test.ts` as current progress truth anchors (`REPO_MAP.md:824-845`). That means phase vocabulary and emission semantics are not just Rust-internal concerns. They are IPC-visible behavior with frontend aggregation and throttling consequences.

### What the third-pass packet overstated or weakly supported

The third-pass focus on **serde defaults, omission behavior, and round-trip testing** is too generic for the live file. `DoubleMaProgressPayload` derives `Serialize`, but not `Deserialize` (`progress.rs:4`), and the file has no serde field attributes, no serde container attributes, no `Default`, and no omission logic at all. On the live source, “default/omission round-trip coverage” is not the most specific or highest-value critique. That framing reads more like carry-over from other DTO-style files than a file-specific review.

Relatedly, the earlier packet’s suggestion that a **phase enum** or a **typed prefetch request struct** is the “smallest justified fix” is too aggressive. Converting `phase` from `&'static str` to an enum would change the public payload field type (`progress.rs:8`) and the helper signature (`progress.rs:39`). Replacing the callback signature with a new request type would change a public trait item signature (`progress.rs:22-25`). Those are not small representation tweaks on a public exported surface.

The verification guidance also needed sharpening. A generic `cargo test -p ta_app_core progress -- --nocapture` suggestion is too weak on its own because the live file currently has no local `progress.rs` unit tests to target. The existing downstream parity and campaign tests named in the earlier packet remain the real contract coverage, and any IPC-facing change should also hit the frontend progress aggregation tests named in `REPO_MAP.md`.

### One additional underweighted contract risk

The live payload serializes `processed_pairs` and `total_pairs` as `usize` (`progress.rs:6-7`). On today’s desktop app targets that is probably fine, and I would **not** change it in this bucket. But because `usize` is pointer-sized, it is not as future-stable as a fixed-width wire integer if this payload ever becomes versioned or reused more broadly. Combined with the fact that Tauri event payloads are JSON-facing and frontend JavaScript numbers only preserve integer precision up to `2^53 - 1`, this is a real long-horizon IPC consideration even though it is not today’s bug.

## Implementation Priority

### Safe-now hardening

The best immediate work is still documentation, but it should be more specific than the earlier packets implied.

First, add rustdoc that states the actual contract owned by this file:

- `emit_double_ma_progress` is a thin forwarder and a silent no-op when `sink` is `None`.
- `processed_pairs`, `total_pairs`, and `phase` are forwarded unchanged; caller-side code owns monotonicity, phase ordering, and any `processed_pairs <= total_pairs` invariant.
- `ProgressSink` implementations must be thread-safe because the trait is `Send + Sync`, and they should avoid blocking the optimization/validation hot path.
- Panics in sink implementations are not caught here and will propagate.
- `with_double_ma_campaign_vram_prefetch_request` is an optional capability hook, not a mandatory part of basic progress reporting.
- The prefetch callback borrows `Candles` and string slices, so it is a synchronous borrowed-use hook, not a handoff API.
- `on_double_ma_campaign_vram_prefetch_feedback` is a cold-path staging/reporting callback, not part of the hot per-pair emission path.

Second, if this file is touched at all, add **one tiny local unit test** that locks down the only behavior this module truly owns: `None` is a no-op, and a present sink receives the payload unchanged. Do **not** migrate monotonicity, phase-schedule, or validation-order assertions into this file. Those remain downstream behaviors owned by the callers that schedule progress.

Third, if you want a small readability improvement beyond docs, the upper safe limit is something like a callback type alias or clearer rustdoc naming for the five callback arguments. I would **not** spend this pass on new public wrapper types, trait split-outs, or field-type changes.

### Larger redesign still deferred

A larger cleanup is still deferred, and the fourth pass makes that deferral more explicit.

If this contract ever needs a real evolution window, the more coherent redesign would be:

- split the current mixed contract into a pure progress sink plus a separate optional campaign-prefetch provider surface;
- replace the all-public progress struct with a constructor-based or versioned wire DTO that can evolve without exposing public field literals forever;
- replace the positional prefetch callback signature with a typed request object only in a coordinated breaking window;
- define a dedicated wire DTO for campaign prefetch feedback if it ever needs to cross IPC, instead of deriving `Serialize` directly on the current internal enum;
- revisit counter widths with fixed-width integers if the payload becomes versioned, persisted, or shared beyond today’s transient desktop event flow;
- consider Tauri channels or a more deliberate throttled transport only if event volume becomes a measured bottleneck, because that is an IPC-architecture change, not a `progress.rs` cleanup.

Those are valid redesign ideas. They are just not “safe-now small refactor” work.

## Verification If Changed

I did **not** rerun repository tests in this fourth pass because the review bundle only included the target file and limited context, not the full workspace. The right verification remains contract-focused.

If this file changes beyond rustdoc, I would keep verification anchored to the existing downstream tests named in the earlier packet:

1. `cargo test -p ta_app_core cpu_dynamic_strategy_progress_parity_matches_legacy_results -- --nocapture`
2. `cargo test -p ta_app_core shared_validation_core_progress_phase_names_and_monotonic_counts -- --nocapture`
3. `cargo test -p ta_desktop_demo_app --lib commands::commands_campaign::tests::doublema_vram_prefetch_config_accepts_exact_kernel_and_learned_auto -- --nocapture`
4. `cargo test -p ta_desktop_demo_app --lib commands::commands_campaign::tests::doublema_vram_prefetch_config_rejects_non_plain_shapes -- --nocapture`

If payload fields, phase labels, or event cadence change, add the frontend-side progress aggregation surface as well. `REPO_MAP.md` points to the relevant truth anchors: `web/src/stores/taskStoreCampaignProgress.test.ts`, `web/src/stores/taskStoreEventThrottle.test.ts`, and `web/src/services/optimizeService.test.ts` (`REPO_MAP.md:840-845`).

If the only change is adding a tiny local forwarding/no-op test, that new test should supplement the existing downstream coverage rather than replace it.

## Regression And Feature-Preservation Constraints

- Preserve the current `DoubleMaProgressPayload` field names and the current phase-string vocabulary in a small pass. They are already IPC-visible compatibility surface.
- Do not convert `phase` to an enum in this bucket. That is a public field-type and helper-signature change, not a documentation cleanup.
- Do not add fields to `DoubleMaProgressPayload`, add variants or fields to `DoubleMaCampaignVramPrefetchFeedback`, or change `ProgressSink` method signatures in this bucket. The surface is public and unsealed.
- Do not try to “improve reliability” here by making the helper heavier, more validating, or panic-catching on the hot path. The file’s current value is that it stays thin.
- Do not broaden this into a Tauri transport redesign inside `progress.rs`. Official Tauri guidance already warns that events are not the high-throughput path; if that architecture changes, it should happen at the command/event layer, not by accreting logic into this helper module.
- There is **no direct GPU ABI issue** visible in this file. The real GPU-adjacent risk is contract coupling: changing the prefetch callback shape or feedback semantics would ripple into campaign staging/orchestration.
- There is **no visible persistence contract** in this file beyond `Serialize` on the transient progress payload. IPC risk matters here more than persistence risk.

## Fourth-Pass Conclusion

The third-pass bucket should **stand**: `crates/app_core/src/progress.rs` remains a `Small surgical refactor` target.

What changes in the fourth pass is the definition of “small.” The current packet was right to resist churn, but it was too generic about serde/default concerns and too casual about the cost of type reshaping. The live file is a public, IPC-adjacent coordination seam with an all-public struct, a public enum, and an unsealed trait-object contract. That makes docs, explicit behavioral notes, and at most tiny forwarding tests the right safe-now work.

The right fourth-pass call is therefore: **keep the bucket, narrow the scope, and defer any representation cleanup**. A phase enum, a typed prefetch request object, a trait split, or a broader event-transport rethink may all be reasonable later, but they are redesign work, not small-pass hardening.
