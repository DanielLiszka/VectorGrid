# Fourth-Pass Review: src-tauri/src/commands_artifacts.rs

- Review date: `2026-04-16`
- Source file: `src-tauri/src/commands_artifacts.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_artifacts.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_artifacts.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_artifacts.rs.review.md`
- Third-pass action bucket: `Accept as-is`
- Fourth-pass action bucket: `Accept as-is`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file merited one more pass because it is still a public Tauri IPC seam even though it is intentionally thin. The live source remains only 21 command wrappers over `run_artifacts::*_blocking` calls (`src-tauri/src/commands_artifacts.rs:11-230`), but those wrappers sit directly on the trust boundary between the WebView and filesystem-adjacent artifact persistence. An `Accept as-is` packet is only correct here if the thinness is still buying the right kind of safety, and if prior packets did not blur the real risks.

After reopening the live source, cross-checking re-export and registration (`src-tauri/src/commands.rs:44-52`, `src-tauri/src/lib.rs:150-170`), and re-reading the repo routing guidance (`REPO_MAP.md:1414-1431`, `1450-1451`, `1477-1478`), I do not see a bucket-change-worthy defect in `commands_artifacts.rs` itself. The current third-pass bucket should stand.

The sharper fourth-pass conclusion is narrower than the earlier packet language, though: this file is acceptable not because it is generically “low risk,” but because it very deliberately refuses to own artifact semantics, path containment, retention, or replay shaping. Those concerns remain centralized in `run_artifacts.rs`, which is exactly where this repo says they belong.

## Fourth-Pass Research Log

I re-read the live `commands_artifacts.rs` source first, then validated that every exported wrapper is still re-exported from `commands.rs` and still present in the Tauri `generate_handler!` registration in `lib.rs`. I also re-read the repo guidance for the artifact subsystem. That guidance still states that persistence is centralized in `src-tauri/src/run_artifacts.rs`, that load/delete/tag-update operations are constrained to paths inside the app runs directory, that artifact families remain broader than plain run artifacts, and that Stage 4 / Stage 5 compatibility naming is still transitional rather than cleanly converged (`REPO_MAP.md:1414-1431`, `1450-1451`, `1477-1478`).

I then checked current official Tauri and Tokio guidance to make sure the earlier reasoning still matched the runtime model. Tauri’s current command docs still frame commands as the typed IPC surface and recommend async commands for heavy work; they also show that commands can take an `AppHandle` directly. Tauri’s security and runtime-authority docs still treat the command boundary as a trust boundary, with runtime authority deciding which window may invoke which command. Tauri’s command-scope docs also still put fine-grained scope enforcement in the command implementation, not in configuration alone. Tokio’s current `spawn_blocking` docs remain clear that it is appropriate for bounded blocking work, that started blocking tasks are not abortable, and that long-lived blocking work should not accumulate behind this API. Serde’s current docs still make the trade-off around `serde_json::Value` explicit: it is flexible but loosely typed, while typed conversion or raw-value optimization would require a broader transport decision rather than a file-local tweak.

That research does not uncover a new local bug. It does, however, sharpen where the real risks are and where earlier packets were a bit too generic.

## What Fourth Pass Sharpened

### What the third-pass packet got right

The third-pass packet was right to keep the bucket unchanged. The live file is still a thin, explicit façade. Each command takes owned IPC-safe arguments, forwards directly into `run_artifacts`, and keeps blocking filesystem work off the async command path with `spawn_blocking` (`src-tauri/src/commands_artifacts.rs:11-230`). It was also right to treat command names and payload shapes as compatibility surfaces, because all 21 wrappers are still wired into the public Tauri registry (`src-tauri/src/commands.rs:44-52`, `src-tauri/src/lib.rs:150-170`).

It was also right not to push for local abstraction. A macro or helper to collapse these wrappers would save little and make this public surface harder to scan. The current repetition is carrying useful explicitness.

### What it missed or underweighted

The biggest underweighted point is that this is path-sensitive IPC, but not scope-parameterized IPC. Most of the interesting commands accept raw `String` paths from the frontend (`update_artifact_tags`, `load_*`, `delete_run_artifact`). Tauri runtime authority can decide whether a window may call the command at all, but command scopes are only protective when the application defines and enforces them in the command implementation. This file does not define such a scope surface. That means path safety is not “helped a little bit” here; it is wholly delegated to `run_artifacts`. Earlier packets gestured at that, but fourth pass makes it the central reason the bucket still stands. Any wrapper-local path normalization, pre-validation, or convenience fallback would be a regression risk because it could diverge from the owner-layer containment policy.

The second underweighted point is performance shape. The earlier packets were mostly correct that this is not a compute hotspot, but “no local performance problem” was too absolute. `spawn_blocking` prevents executor stalls from disk I/O; it does not eliminate blocking-pool contention for many concurrent library scans, and it does not eliminate IPC serialization cost for large return values. That matters here because several commands return potentially heavy payloads by design: `ArtifactLibrarySnapshot`, `WorkflowArtifactsHydrationPayload`, replay responses, and raw `serde_json::Value`. The correct conclusion is still not “optimize this wrapper.” The correct conclusion is that any future scale work belongs in `run_artifacts` and the consuming flows as pagination, indexing, or narrower hydration responses, not as wrapper churn.

The third underweighted point is that join-error flattening is the only real file-local behavior. Every wrapper does the same thing after `await`: `.map_err(|e| e.to_string())?`. That keeps the public `Result<_, String>` convention uniform, but it also collapses blocking-task panics and join failures into generic strings. Earlier packets discussed error handling abstractly, but this is the concrete local debt. I still do not recommend changing it here in isolation, because changing error strings or error object shape on one command family would create inconsistent command-surface behavior and could easily regress frontend expectations. But it should be named accurately.

The fourth missed point is the Stage 4 / Stage 5 compatibility asymmetry already present in the surface. The command set exposes both `list_stage4_artifacts` and `list_stage5_artifacts`, but only `load_stage5_artifact_replay`. That asymmetry is not necessarily a bug. The repo map says Stage 4 and Stage 5 currently share the same backend implementation path, use the Stage-5-style artifact prefix, and still preserve Stage-4-style compatibility names in parts of the frontend and saved replay state (`REPO_MAP.md:1450-1451`, `1477-1478`). The implication is important: “narrow naming cleanup” is less safe than the third-pass packet implied. This surface is already carrying intentional migration debt, so local symmetry cleanups are more likely to cause regressions than to pay down debt.

### What the earlier packets overstated

The third-pass packet used language about “state-owner boundaries” that is too generic for this file. `commands_artifacts.rs` does not touch `State<AppState>` at all; it only accepts `AppHandle` and forwards into the artifact owner. The real boundary is not app state ownership. It is IPC contract ownership and artifact-path containment ownership.

The earlier packets also slightly overstated the value of manual desktop smoke as a primary regression net. For this file, manual smoke is secondary. The stronger evidence is command registration plus end-to-end artifact-flow tests that exercise history, latest-artifact hydration, replay, tagging, delete, and runs-dir reveal through the existing frontend and owner-layer code. The wrapper itself has so little logic that “manual smoke of the wrapper” is not where the meaningful confidence comes from.

## Implementation Priority

### Safe-now hardening

No change is required to justify a bucket change. `Accept as-is` is still the right call.

If this file is touched anyway, there are only two conservative hardening moves worth considering.

First, add a short module-level invariant comment documenting the rules this file already follows: all filesystem semantics live in `run_artifacts`, all heavy file work stays behind `spawn_blocking`, and no path policy should be duplicated here. That would improve future review quality without changing behavior.

Second, if a code change lands here, add one thin command-contract test rather than a broad local unit-test matrix. The highest-value slice would prove that a path-bearing command still routes failure through owner-layer validation and that the command remains registered and callable via the Tauri surface. Because the local logic is minimal, that is enough.

### Larger redesign still deferred

A typed artifact IPC redesign remains deferred. `save_run_artifact`, `save_batch_summary`, and `load_run_artifact` still use raw `serde_json::Value`, which is flexible but weakly typed at the trust boundary. A move to typed DTOs, generated IPC types, or a more deliberately versioned payload envelope could improve boundary clarity, but it would be a broad artifact-schema migration across many flows and compatibility layers. It is not safe as a file-local cleanup.

Likewise, any attempt to converge Stage 4 and Stage 5 naming should stay deferred until backend commands, frontend service layers, saved replay fields, and artifact naming are migrated together. The repo map is explicit that compatibility naming is still present today. Cleaning only this file would be churn with real regression risk.

If artifact-library scale becomes a genuine pain point, the right redesign target is owner-layer snapshot shape, pagination, indexing, or background refresh policy. That is where IPC payload mass and blocking-pool pressure should be addressed. It is not a good reason to rewrite these wrappers.

Finally, structured error transport is also broader than this file. Improving join-error observability would be reasonable, but only as part of a wider Tauri command-surface decision about whether the repo still wants `Result<_, String>` everywhere or wants structured, serialized error objects.

### Recommendations that would be risky if implemented now

Several seemingly tidy changes would be regression-prone if applied here in isolation: renaming Stage 4 / Stage 5 commands for symmetry, replacing `serde_json::Value` with typed structs only in this module, changing error text or error shape for join failures, removing `spawn_blocking`, or adding wrapper-local path normalization on top of `run_artifacts`. All of those change real contracts or risk bypassing the current owner-layer safety model.

## Verification If Changed

If `commands_artifacts.rs` changes, the most important verification is still cross-surface rather than purely local.

At minimum, run `cargo test --workspace` so the Tauri crate, the artifact owner, and adjacent backend consumers are covered together. For broader confidence, run the repo quality gate documented in the repo guidance.

Then re-run the frontend test slices that exercise artifact-driven workflows rather than only the wrapper: history flows, workflow-artifact hydration/bootstrap, meta replay, live-session artifact inspection, settings-driven runs-directory reveal, and any save-path producers whose response shape may have changed. The repo map points to the right consumers even when the source of the behavior lives in `run_artifacts`.

If command registration or naming changes, also re-check the command registry surfaces in `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, and the frontend command wiring layer. Manual desktop smoke is still useful as a final sanity pass, but it should not be the primary evidence for this file.

No special GPU ABI verification is required here. The local risks are IPC, persistence, and compatibility, not CUDA or validation-kernel behavior.

## Regression And Feature-Preservation Constraints

Any future change to this file should preserve the current command names, argument ownership model, and return-shape expectations unless the frontend and saved-artifact compatibility layers move in lockstep. The file should continue to accept `AppHandle` rather than pulling in unrelated `AppState` coupling.

It must also preserve the current off-thread boundary for filesystem-heavy work. `spawn_blocking` is not incidental here; it is part of the contract that keeps artifact I/O off the async command path.

Most importantly, this wrapper must not start owning path or retention policy. The repo guidance still centralizes persistence, retention, enrichment, and path containment in `run_artifacts` (`REPO_MAP.md:1414-1431`). That owner split is a feature, not an accident.

The existing Stage 4 / Stage 5 compatibility posture must also be preserved until there is a coordinated migration. The current asymmetry is already part of the shipped contract, and local “cleanup” would risk replay and saved-artifact regressions.

Finally, any future attempt to improve typedness or error richness at this boundary should be treated as a command-surface migration, not a wrapper polish task.

## Fourth-Pass Conclusion

The current third-pass bucket should stand. `src-tauri/src/commands_artifacts.rs` still belongs in `Accept as-is`.

What the third-pass packet got right is the core story: this is a deliberately thin public IPC façade, explicit wrappers are preferable to local abstraction churn, and the real artifact semantics belong in `run_artifacts` rather than here.

What fourth pass adds is a more precise framing of the remaining debt. The important risks are not generic “state-owner” issues. They are the path-sensitive trust boundary, the fact that fine-grained path enforcement is owner-layer rather than scope-parameterized in this command file, the fact that `spawn_blocking` solves blocking I/O but not payload-size or blocking-pool scale concerns, the fact that join-error flattening is the one meaningful local behavior, and the fact that Stage 4 / Stage 5 naming is still intentionally transitional.

None of that is enough to overturn the bucket. It is enough to narrow future advice. Safe-now work is limited to documentation and thin contract testing if the file changes anyway. Broader cleanup should stay deferred and coordinated, because the easy-looking changes here are exactly the ones most likely to regress persistence and IPC compatibility.
