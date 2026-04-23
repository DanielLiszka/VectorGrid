# Fourth-Pass Review: src-tauri/src/commands_system.rs

- Review date: `2026-04-16`
- Source file: `src-tauri/src/commands_system.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_system.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_system.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_system.rs.review.md`
- Third-pass action bucket: `Document only`
- Fourth-pass action bucket: `Safe-now hardening`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

`commands_system.rs` is still a small file, but it is not private glue. It is a public Tauri IPC seam that is re-exported from `src-tauri/src/commands.rs:140` and registered into the live invoke surface in `src-tauri/src/lib.rs:134-188`. That makes even tiny semantic mismatches more important than the file size alone suggests.

The third-pass packet was right to treat this as a non-hot, low-churn desktop boundary rather than a performance target. What justified a fourth pass was whether that correct high-level framing had hidden a real bounded hardening opportunity. After reopening the live source, the answer is yes: there is one clear file-local command-contract weakness and one underweighted GPU-status ambiguity that make `Document only` too passive.

## Fourth-Pass Research Log

This pass re-read the live source first, then checked only the nearby routing and contract files needed to challenge the current bucket.

1. The live source was re-read against the prior packets, with particular attention to the actual side-effect ordering in `set_window_effect` and the exact DTO shape emitted by `get_gpu_status`.
2. `src-tauri/src/lib.rs` and `src-tauri/src/commands.rs` were re-checked to confirm that these commands are part of the public invoke surface, not dead support code.
3. `REPO_MAP.md` was used to verify the repo’s current build matrix and settings/bootstrap ownership. Two lines matter a lot here: the repo explicitly documents a separate GPU-kernel build variant at `REPO_MAP.md:242-246`, and it documents settings bootstrap + settings service as the owners of restoring/applying effect settings at `REPO_MAP.md:847-860`.
4. Current Tauri command documentation was rechecked to confirm that commands in separate modules are public IPC entrypoints, and that `AppHandle` / `WebviewWindow` injection is part of the intended contract surface.
5. Current `window-vibrancy` / Tauri docs were rechecked for platform support. One important review note is version alignment: current Tauri 2.10.x docs still surface `window-vibrancy ^0.6` as the dependency family, so exact API-shape reasoning should stay version-aware rather than leaning too hard on latest-crate signatures.

## What Fourth Pass Sharpened

### Bucket assessment

The current third-pass bucket should **change**.

`Document only` is no longer the best call because the file has a real local hardening target: an invalid `set_window_effect` request can mutate window state before the command reports failure. That is not just a documentation gap. It is small, bounded, and fixable without redesigning the module.

### What the current third-pass packet got right

The current packet correctly recognized three things.

First, this file is still small and not worth a broad refactor. Nothing here justifies “clean architecture” churn, async reshaping, or any performance-motivated rewrite.

Second, the packet was right to treat command names and DTO shapes as compatibility surfaces. That caution matters here because `get_gpu_status`, `exit_application`, and `set_window_effect` are frontend-invokable commands, not internal helpers.

Third, the packet was right that unsupported-platform `set_window_effect` behavior should stay conservative. The current Linux/other-platform no-op branch at `src-tauri/src/commands_system.rs:109-113` is consistent with the repo’s desktop-settings flow and should not be casually turned into a hard error.

### What it missed or underweighted

#### 1. Unknown window-effect values are not side-effect free

This is the biggest missed item.

On Windows, the command clears the active effect first (`clear_mica`, `clear_acrylic`, `clear_blur` at `src-tauri/src/commands_system.rs:67-69`) and only then validates the requested effect string in the `match` that can return `Err("Unknown window effect: ...")` at `:77-83`. macOS does the same pattern with `clear_vibrancy` at `:94` before the `match` that may error at `:95-105`.

That means an invalid effect request is not a clean rejection. It is “clear current state, then fail.” For a public IPC surface, that is a contract weakness. It is also not just a hostile-input edge case. The repo map explicitly says effect settings are restored through `settingsBootstrap` and applied through `settingsService` (`REPO_MAP.md:855-860`), so a stale persisted value, a downgrade mismatch, or a future enum migration bug can realistically route through this path during normal app startup.

The third-pass packet discussed hidden clear failures, but it underweighted the more important behavior: the command’s error path is already state-mutating.

#### 2. `GpuStatus.cuda_available` is semantically narrower than the DTO suggests

The GPU-status payload currently mixes two different notions of “GPU readiness.”

`cuda_available_now()` is gated only on the `cuda` feature (`src-tauri/src/commands_system.rs:16-23`). But the same DTO also reports `built_with_cuda_backtest_kernel`, and the repo explicitly documents `--features cuda-backtest-kernel` as a supported GPU-kernel build variant distinct from `--features cuda` (`REPO_MAP.md:242-246`). In other words, the repo’s own release matrix allows a build where `built_with_cuda_backtest_kernel` can be true while `cuda_available` is forced to false by construction.

That does not prove a runtime bug by itself, but it does prove an IPC truth problem. The field name reads like a general CUDA-availability bit, while the implementation only answers a narrower question tied to the `cuda` feature path. The ambiguity is reinforced by the fact that `get_gpu_status` also emits preferred-device diagnostics from `ta_app_core::preferred_cuda_device()` at `src-tauri/src/commands_system.rs:27-41`, which is broader than the `cuda_available_now()` helper suggests.

The current unit test locks this ambiguity in. The assertion at `src-tauri/src/commands_system.rs:130-132` requires `cuda_available == false` whenever `built_with_cuda == false`, which is exactly the wrong invariant if the product treats kernel-only builds as first-class GPU builds.

The third-pass packet did not call this out. It should have.

#### 3. The third-pass packet emphasized the wrong risk surface

The third-pass writeup talked about state ownership, cancellation scope, and blocking-task boundaries. That caution is good in the abstract, but it is weakly grounded for this file. `commands_system.rs` does not touch `AppState`, `BackgroundTaskState`, run cancellation, event channels, or blocking orchestration.

The sharper risks here are more concrete and more local: side effects before validation in `set_window_effect`, and a GPU-status DTO whose semantics do not cleanly match the documented release matrix.

### What was overstated or too weakly supported

Two earlier recommendations should be downweighted.

The first is the suggestion that `get_gpu_status` or `exit_application` should probably stop returning `Result`. That is not a compelling cleanup. These are public Tauri commands, and changing return shapes creates frontend coordination churn for very little gain. There is no correctness win here large enough to justify the IPC churn.

The second is the idea that this file merely needs a few more direct Rust tests for `exit_application` or OS-level window-effect behavior. That overstates how cheap those tests are. `exit_application` is not a particularly valuable unit-test target, and end-to-end effect behavior is inherently platform/GUI bound. If testing is added, the right move is to extract a tiny pure parser/decision helper and test that logic, not to pretend OS window APIs are easy unit-test terrain.

## Implementation Priority

### Safe-now hardening

1. **Validate the requested window effect before any clear/apply side effects.**

   The smallest worthwhile fix is to parse the normalized effect string into a private, platform-aware decision first, and only then call the clear/apply functions. Unknown values should return `Err(...)` without clearing the current effect. That is a local change, preserves current valid-effect behavior, and removes the most concrete contract weakness in the file.

2. **Add narrow regression coverage around the effect decision path.**

   The useful test is not “does Windows blur visually render in CI.” The useful test is “does normalization + validation accept the supported names and reject unsupported names without entering the side-effect path.” If this requires extracting a tiny helper enum or planner function, that is justified.

3. **Document the current scope of `GpuStatus.cuda_available` in code, not just in a review packet.**

   Even if the DTO is not redesigned immediately, the field’s present meaning is narrower than its name suggests. A short comment next to the field or helper is warranted so future contributors do not treat it as the general GPU-kernel readiness bit.

### Larger redesign still deferred

1. **Rationalize the GPU-status DTO into clearer, separately named signals.**

   The cleaner long-term model is to separate:
   - build flags (`built_with_cuda`, `built_with_cuda_backtest_kernel`)
   - vector-ta sweep-runtime availability
   - kernel/device-probe availability
   - preferred-device metadata/error reporting

   That redesign should be additive first. Do not silently repurpose or rename `cuda_available` in place unless the frontend, tests, and any saved diagnostics assumptions move together.

2. **Revisit window-effect ownership only if multi-window support becomes real product scope.**

   Right now the Tauri-injected `WebviewWindow` is a good fit for a single-window app. If the desktop shell ever grows secondary windows while effect settings stay global, then a central “which window owns effect application?” policy becomes worth addressing. That is not today’s problem.

## Verification If Changed

If the safe-now hardening above is implemented, the verification should stay tight and contract-oriented.

1. Run the local Rust target for this module:
   - `cargo test -p ta_desktop_demo_app commands_system -- --nocapture`
2. Re-run the frontend consumers already called out by the earlier packets:
   - `npm --prefix ./web run test -- src/components/DesktopTitlebar.test.ts src/services/settingsBootstrap.test.ts src/services/settingsService.test.ts src/services/optimizeService.test.ts src/tauri/commands.test.ts`
3. If `GpuStatus` fields or semantics change, also run frontend typecheck:
   - `npm --prefix ./web run typecheck`
4. Manual desktop smoke if `set_window_effect` changes:
   - on Windows, verify `mica`, `acrylic`, and `blur`
   - on macOS, verify `vibrancy`
   - on unsupported platforms, confirm the command remains harmless
   - on supported platforms, intentionally send an unknown effect and verify the existing effect is preserved rather than cleared before rejection
5. If the GPU DTO is touched beyond comments, verify the documented build matrix explicitly:
   - CPU-only
   - `--features cuda`
   - `--features cuda-backtest-kernel`

## Regression And Feature-Preservation Constraints

Do not change unsupported-platform behavior for `set_window_effect` from no-op to hard failure without coordinated settings/bootstrap review. Persisted desktop settings are a real path here, and harmless replay matters more than strict parity on unsupported platforms.

Do not change `GpuStatus.cuda_available` semantics in place and assume the frontend will “just figure it out.” It is a public IPC field. If the repo wants a more truthful status model, add a better-named field first and migrate consumers deliberately.

Do not treat the availability of extra upstream `window-vibrancy` effects as permission to widen this command surface. New effect strings require frontend filtering, copy updates, platform gating, and manual verification.

Do not rewrite `exit_application` into some softer window-close path without a product-level decision. `app.exit(0)` is explicit, and changing it would be behavior change, not cleanup.

There is no performance, allocation, persistence-layer, GPU-ABI, or validation-engine work to chase inside this file. Any change should remain narrowly about IPC truth and desktop command semantics.

## Fourth-Pass Conclusion

The third-pass packet was right about the scale of this file, but wrong to leave it at `Document only`.

`src-tauri/src/commands_system.rs` has a real, bounded hardening opportunity: `set_window_effect` currently mutates window state before rejecting an invalid effect string. That is a concrete public-command weakness, and the repo’s own settings-bootstrap flow makes it realistic enough to fix rather than merely document.

The packet also underweighted a second issue: `GpuStatus.cuda_available` does not read like the narrower signal it actually is, especially against the repo’s documented kernel-only build variant. That part is best handled conservatively through additive DTO cleanup or at minimum in-code clarification, not by a silent semantic flip.

So the right fourth-pass call is:

- keep the file small
- avoid broad refactors
- preserve current cross-platform no-op behavior on unsupported systems
- but change the bucket to **`Safe-now hardening`** because one small local fix is clearly justified and the GPU-status contract deserves sharper treatment than the third pass gave it
