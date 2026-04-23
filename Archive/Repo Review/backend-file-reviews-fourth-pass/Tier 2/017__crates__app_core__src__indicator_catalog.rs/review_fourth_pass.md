# Fourth-Pass Review: `crates/app_core/src/indicator_catalog.rs`

| Field | Value |
| --- | --- |
| Review date | 2026-04-16 |
| Source file | `crates/app_core/src/indicator_catalog.rs` |
| First-pass packet | `review__first_pass.md` |
| Second-pass packet | `review__second_pass.md` |
| Third-pass packet | `review__third_pass.md` |
| Third-pass action bucket | `Small surgical refactor` |
| Fourth-pass action bucket | `Small surgical refactor` |
| Packet status | `Reviewed` |

## Why This File Got A Fourth Pass

The current third-pass bucket should stand, but the third-pass writeup was too generic for what the live file is actually doing. It correctly treated `indicator_catalog.rs` as a contract seam rather than a refactor playground, yet it flattened the remaining work into broad advice about “invariant tests” and “compatibility” without isolating the sharper live risks.

Re-reading the source shows that the real issue is not representation churn. It is uneven invariant enforcement across three different responsibilities in one module: raw registry projection (`145-194`), parameter normalization (`338-440`), and output selection (`443-493`). The public descriptor list can therefore look authoritative while still trusting registry metadata more than the later validation paths do. That is still a small-scope hardening problem, not a redesign bucket, but it is more specific than the current packet makes it sound.

## Fourth-Pass Research Log

I re-read the live source, the prior packets, and the local export context in `lib.rs` before reopening the external standards the earlier passes leaned on. The source re-check focused on the public DTO surface (`43-89`), descriptor projection (`145-194`), cached vs uncached catalog access (`306-336`), parameter validation (`338-440`), output validation (`443-493`), and the narrow current tests (`496-602`).

The external references that still matter here are narrow and unchanged. The Rust API Guidelines documentation chapter remains the right standard for public DTO/helper documentation and for explicitly describing error behavior on exported functions. The standard library `OnceLock` docs still support the current cache shape as a valid one-time static initialization pattern and do not justify cache churn here. RFC 1105 on Rust API evolution still matters because this file is re-exported, serde-facing, and IPC-adjacent: renaming fields, changing enum wire labels, or tightening accepted inputs is not “cleanup”, it is public-contract work.

That research did not change the bucket, but it did change the emphasis. The safest next work is documentation and self-consistency hardening around the existing contract, not field renames, typed-error conversion, or eager runtime failure on malformed registry metadata.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right to keep this file in `Small surgical refactor`. It was also right that broad internal cleanup is the wrong instinct here. This file is re-exported from `app_core`, participates in the Tauri-facing catalog surface, and influences plan/validation behavior indirectly through normalized metadata, so the compatibility seam framing is materially correct.

It was also right that tests and contract notes are higher value than representation churn. The live code still does not justify a larger redesign bucket.

### What it missed or underweighted

The missed issue is not primarily the redundant sort. The more important gap is that descriptor self-consistency is only partially defended.

At projection time, `normalize_dynamic_indicator_descriptor` copies registry ids, outputs, defaults, ranges, and notes almost verbatim into public descriptors (`145-194`). Later, `normalize_and_validate_indicator_params` validates caller-supplied values against parameter schema and can catch some descriptor drift indirectly (`231-303`, `338-440`). But there is no comparable invariant pass for top-level indicator ids or output ids, and there is no early guarantee that descriptor defaults are themselves valid against the descriptor kind/range/enum contract. `normalize_and_validate_indicator_output_ref` only validates the caller’s requested output id; it does not defend against duplicate normalized output ids or empty output ids already present in the descriptor (`443-493`).

That means the catalog can expose a public descriptor that is internally inconsistent and only fail later, or fail ambiguously, when a downstream caller tries to use it. The current test surface does not pin these invariants. It checks counts, basic non-empty id/label presence, simple lookup, and a few normalization cases (`500-602`), but not unique normalized indicator ids, unique normalized output ids per descriptor, or default-value self-validity.

The earlier packet also underweighted how observable error-order decisions are in this file. These helpers return `Result<_, String>`, not a typed internal error, so changing when an error is discovered can change user-visible diagnostics and test expectations even if the function still rejects bad input. That does not make earlier duplicate-schema pre-scan advice wrong, but it does mean the recommendation needed a stronger warning that validation-order changes are contract-adjacent here.

### Claims in the earlier packet that are too strong or too weak

The “enforce or document `step`” framing is too symmetrical for a safe-now packet. The source clearly exposes `step` publicly (`44-54`, `153-163`) and never enforces it during numeric normalization (`201-303`, `338-440`). That ambiguity is real. But strict enforcement is not equally safe to the documentation-only option. Enforcing `step` now would tighten accepted request payloads, and those payloads are part of a re-exported, serde-visible, IPC-adjacent interface. That can affect frontend request replay, saved artifacts/configs, and any test that currently assumes off-step numeric values normalize successfully. Documentation is safe-now; enforcement is coordination-required.

The second-pass snapshot framing was directionally useful but too broad. `list_dynamic_indicator_descriptors_cached` is a process-lifetime snapshot because it is backed by `OnceLock` (`318-324`). `list_dynamic_indicator_descriptors` is not; it is an uncached projection over the registry on each call (`306-315`). The file should document that distinction precisely rather than treating the whole catalog surface as one snapshot semantic.

The earlier sort-removal recommendation is correct but over-prioritized. The final sort over `normalized` (`417-430`) is redundant because values are already emitted in descriptor order during the schema walk (`377-408`). Removing it is safe if touched, but it is low leverage relative to the descriptor self-consistency gaps above. This should be opportunistic cleanup, not the headline of the next pass.

### Safe-now hardening

The safest improvement set is still small and local.

First, document the current contract instead of tightening it silently. `IndicatorParamDescriptor.step` should be described as metadata-only unless and until the project intentionally audits and tightens request compatibility. The same pass should document that `supports_gpu_kernel` and `prefers_gpu_by_speedup` are app-core route/policy hints layered onto the registry projection rather than raw registry capability facts, and it should document cached versus uncached catalog semantics separately.

Second, add file-local invariant tests that assert the public catalog is self-consistent before any caller uses it. The highest-value additions are: unique normalized descriptor ids, unique normalized output ids within each descriptor, and descriptor defaults that successfully normalize against their declared parameter schema. If the registry contract truly requires nonempty output ids and coherent `min <= max` bounds, those should be pinned too. This catches “catalog lies about itself” failures earlier without changing runtime IPC behavior.

Third, if touching `normalize_and_validate_indicator_params`, move duplicate-schema-key detection earlier than required/default resolution. That was a good finding in the first packet and still stands. But keep the change as narrow as possible and avoid gratuitous message churn, because error ordering is observable when the public return type is a `String`.

Fourth, treat the redundant sort as optional cleanup after the documentation/tests work, not before it.

### Larger redesign still deferred

A broader redesign is still not justified here.

Do not turn the cached catalog builder into a stealthy runtime panic or fallible load path just to catch malformed registry metadata sooner. That would change the behavior of a Tauri-facing discovery surface and expand the blast radius from a local descriptor bug into application startup or IPC failure. If the project eventually wants central registry self-validation, it should be introduced explicitly as a separate contract with clear consumer handling.

Do not rename DTO fields, change serde labels, or split the descriptor into multiple public types in this pass. This module exports serde-facing enums and structs directly, so field/variant reshaping is public-contract work.

Do not start a typed-error conversion campaign here in isolation. `Result<_, String>` is not ideal, but changing it locally would force wider caller coordination for limited benefit on this file.

## Implementation Priority

1. Keep the bucket unchanged and treat this as contract hardening, not refactoring.
2. Add rustdoc that makes three behaviors explicit: `step` is currently not enforced, GPU-kernel preference fields are app-core hints, and only the cached accessor is a process-lifetime snapshot.
3. Add focused invariant tests for descriptor self-consistency: normalized id uniqueness, output-id uniqueness, default-value validity, and any registry assumptions the team wants to claim publicly.
4. If touching parameter validation, pre-scan duplicate schema keys earlier, while minimizing message/order churn.
5. Only then consider removing the redundant sort if the file is already open.

## Verification If Changed

At minimum, keep the existing file-local coverage command from the earlier packets and expand it with new invariant tests rather than broader integration churn:

- `cargo test -p ta_app_core indicator_catalog -- --nocapture`

If the repo-local route tests named in the earlier packets are available in the working tree, they remain the right adjacent checks because this file can influence dynamic-plan route selection indirectly:

- `cargo test -p ta_app_core compile_gpu_plan_routes_dynamic_indicator_to_gpu_when_policy_prefers_gpu -- --nocapture`
- `cargo test -p ta_app_core compile_gpu_plan_routes_dynamic_indicator_to_cpu_when_policy_prefers_cpu -- --nocapture`
- `cargo test -p ta_desktop_demo_app list_dynamic_indicators_returns_only_dynamic_strategy_eligible_descriptors -- --nocapture`

New or tightened file-local tests should cover at least:

- duplicate normalized descriptor ids are rejected or at minimum detected in test assertions
- duplicate normalized output ids per descriptor are detected
- descriptor defaults normalize successfully against their declared schema
- duplicate schema-key drift is surfaced deterministically if that path is changed
- `step` behavior is pinned explicitly, whether the project keeps non-enforcement or deliberately tightens it later

## Regression And Feature-Preservation Constraints

Preserve the public wire surface. `IndicatorParamValue` uses explicit serde tagging (`kind` / `value`), and the public DTO fields are re-exported; renaming fields, changing enum labels, or removing redundant-but-public fields is not a casual cleanup.

Treat `step` enforcement as a behavior-tightening change, not a documentation task. Tightening it can affect frontend request payloads, artifact/config replay, and any user or test code that currently depends on min/max-only numeric validation.

Be careful with runtime-failure posture. Catching malformed registry metadata earlier is desirable, but converting the cached list path into a panic or fallible IPC boundary would be a larger product behavior change than this packet justifies.

Preserve GPU-route semantics unless consumers are updated together. This file does not define GPU ABI, but `supports_gpu_kernel` and `prefers_gpu_by_speedup` do influence how downstream code interprets route eligibility and preference. Clarify their meaning before renaming or repurposing them.

Keep message churn minimal where possible. Because the validation helpers expose plain strings, error wording and precedence are more observable than they would be with a fully internal typed error.

## Fourth-Pass Conclusion

The current third-pass bucket should stand: `indicator_catalog.rs` is still a `Small surgical refactor`, not a redesign candidate.

What the third-pass packet got right was the high-level framing: this is a public contract seam and broad cleanup would be riskier than helpful. What it missed was where the next pass should actually spend its budget. The highest-value work is not a redundant sort removal and it is not immediate `step` enforcement. The highest-value work is making the catalog self-consistent and clearly documented: pin the descriptor invariants the public surface relies on, document the few fields whose semantics are easy to overread, and keep any validation-order tweaks narrow and deliberate.

So the sharpened recommendation is: keep the bucket, keep behavior stable, harden the contract with docs and invariant tests now, and defer runtime self-validation, typed errors, and DTO redesign until there is an explicit cross-consumer plan.
