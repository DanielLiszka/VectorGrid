# Fourth-Pass Review: `src-tauri/src/commands_optimize.rs`

- Review date: `2026-04-16`
- Source file: `src-tauri/src/commands_optimize.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_optimize.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_optimize.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_optimize.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet was directionally right, but it was intentionally thin. Reopening the live source against the current runtime state and the app-core optimization surface shows that the bucket should stand, while some of the earlier “safe-now” recommendations need to be narrowed.

This file is still a public Tauri IPC seam, not an algorithm owner. It normalizes product requests, acquires the single-run guard, bridges progress into the shared `double_ma_progress` event channel, and hands off real work to app-core through `spawn_blocking` (`src-tauri/src/commands_optimize.rs:58-134`, `266-290`). That means the real fourth-pass job is not to re-litigate app-core search logic, GPU routing, or validation math. It is to separate file-owned contract risks from app-core-owned engine risks, then tighten the packet around the former.

The current third-pass bucket should **stand**. This is still a `Small surgical refactor` file. The stronger fourth-pass conclusion is that the best hardening work is even narrower than parts of the earlier packet suggested.

## Fourth-Pass Research Log

This pass re-read the live `commands_optimize.rs` source first, then validated only the contract surfaces needed to challenge the existing packet:

- `src-tauri/src/commands_optimize.rs`
  - command registration surface, cancel aliases, progress bridges, mode-normalization helpers, and local tests
- `src-tauri/src/state.rs`
  - single-run ownership, cancel-token semantics, and background-task separation
- `src-tauri/src/commands.rs` and `src-tauri/src/lib.rs`
  - public command re-export and registration surface
- `crates/app_core/src/double_ma.rs`
  - result fields already returned to the frontend, stricter request validation, internal progress-offset behavior, and existing app-core tests for cancellation/progress invariants
- `REPO_MAP.md` and `AGENTS.md`
  - current routing truth, event-channel ownership, and repo preference for small surgical changes

This pass also rechecked the relevant official behavior notes behind the earlier packets:

- Tauri command-boundary guidance
  - reinforces that command signatures, return DTOs, and event names are public IPC contracts
- Tauri frontend-event guidance
  - reinforces that event names and payloads are compatibility-sensitive transport surfaces
- Tokio `spawn_blocking` and task error guidance
  - reinforces that cancellation here is cooperative once blocking work has started, and that `JoinError` classification exists even though this module currently flattens it to `String`

That research did not change the bucket, but it did sharpen which recommendations are actually worth making from this file.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right to keep the bucket unchanged and frame the main risk as **public-boundary drift rather than file-local redesign**. That remains the right top-level call. This module is thin, but it is not low-stakes glue: it owns command names, request-shaping behavior, progress transport, and run-slot participation (`src-tauri/src/commands_optimize.rs:10-134`, `266-290`; `src-tauri/src/lib.rs:120-125`, `183-184`).

It was also right to keep the cancellation model honest. The foreground cancel commands only flip the shared `AppState` atomic cancel flag (`src-tauri/src/commands_optimize.rs:16-25`; `src-tauri/src/state.rs:569-575`), while the actual optimize paths run inside `spawn_blocking` (`src-tauri/src/commands_optimize.rs:95-106`, `122-133`, `278-289`). That means cancellation is cooperative, not a hard stop. The earlier packet got that core semantics point right.

Finally, it was right not to collapse `cancel_background_task` into the foreground cancel path. `BackgroundTaskState` is a different state owner with its own running bit and cancel token (`src-tauri/src/commands_optimize.rs:10-13`; `src-tauri/src/state.rs:1413-1454`). Any “cleanup” that merged those would blur real ownership boundaries.

### What the current packet missed or underweighted

The biggest missed point is that the highest-value missing test is not a generic end-to-end optimize test. It is a **boundary-lifetime** test.

`run_double_ma_optimization`, `run_composite_cpu_optimization`, and `run_composite_mo_optimization` all acquire `_run_guard` before they call `get_candles` (`src-tauri/src/commands_optimize.rs:88-91`, `115-118`, `272-274`). `AppState::try_begin_run()` sets the shared running bit, and `RunGuard::drop` is what clears it (`src-tauri/src/state.rs:162-169`, `559-566`). That means any early error path after `try_begin_run()` but before the blocking task starts relies on guard drop to avoid wedging the app in a permanent “already running” state. The earlier packet talked about run-lock contention in general, but underweighted this more specific and more file-owned edge case.

The next underweighted point is that the wrapper-specific tests should target **wrapper-specific behavior**, not re-prove engine behavior app-core already owns. `crates/app_core/src/double_ma.rs` already tests cancellation returning `"cancelled"` and already tests monotonic progress/phase invariants (`crates/app_core/src/double_ma.rs:16286-16364`). The real Tauri-boundary-specific behaviors are narrower:

- request coercion before app-core sees the request (`src-tauri/src/commands_optimize.rs:58-80`, `88`, `115`)
- event-transport swallowing of `app.emit(...)` failures (`src-tauri/src/commands_optimize.rs:32-35`, `45-54`)
- static-phase remapping in `OffsetProgressSink` (`src-tauri/src/commands_optimize.rs:38-56`)

Those are the seams this file actually owns.

The current packet also underweighted how important the command-boundary normalization is as **compatibility glue**, not just documentation debt. `commands_optimize.rs` mutates `req.mode` before app-core resolves the request (`src-tauri/src/commands_optimize.rs:88`, `115`). That is not redundant. App-core already contains stricter checks that reject `export_all_csv_path` unless the incoming request mode is `Grid` (`crates/app_core/src/double_ma.rs:5132-5139`). So the Tauri wrapper is preserving a product contract here. If that normalization is removed or weakened casually, users do not get a more accurate system; they get new command-level failures.

A fourth underweighted point is the subtle semantic split between the two offset-progress helpers that now exist in the codebase. The Tauri-layer `OffsetProgressSink` rewrites `phase` to a fixed label supplied by the caller (`src-tauri/src/commands_optimize.rs:45-54`). The app-core internal `OffsetProgress` preserves `payload.phase` while only offsetting counts (`crates/app_core/src/double_ma.rs:5518-5531`). That makes “helper deduplication” more regression-prone than it sounds. A naïve unification could silently change validation/meta/campaign progress grouping even if the code looked mechanically cleaner.

One more point the earlier packet mostly skipped: `spawn_blocking(...).await.map_err(|e| e.to_string())?` is the only place this file can collapse a blocking-task panic or join failure into an undifferentiated user-facing string (`src-tauri/src/commands_optimize.rs:105-106`, `132-133`, `288-289`). I would still rank that below guard-lifetime and normalization coverage, but it is a real wrapper-local diagnostic seam.

### What the current packet overstated or framed too strongly

The earlier packet overstated the “visibility gap” around effective mode. `BackendOptimizationResult` already returns `mode_used`, `backend_route`, and `backend_fallback_reason` (`crates/app_core/src/double_ma.rs:587-625`). That means the stronger fourth-pass conclusion is not “return or record the effective mode later.” The stronger conclusion is: **the visibility is already present; the missing guardrail is a boundary test that proves command-side normalization keeps that result truthful**.

The earlier packet also treated moving shared progress bridges and normalization helpers into a neutral module as a plausible safe-now change. That is too strong for this bucket. Yes, `commands.rs` re-exports `TauriProgressSink` and `OffsetProgressSink`, and other command modules use them (`src-tauri/src/commands.rs:108-112`). But relocating them is naming cleanup, not hardening. It touches multiple command modules without materially reducing correctness risk. Under the repo guidance to prefer small surgical changes and avoid unrelated refactors, that should be treated as deferred (`AGENTS.md:42-46`).

Relatedly, the earlier packet’s “add real optimize command execution through the Tauri boundary” recommendation was too broad as stated. The missing tests should be narrowly aimed at file-owned behavior:

- early-failure run-guard release
- mode-coercion compatibility with existing app-core validation
- `OffsetProgressSink` count/phase remapping if that helper remains shared

Anything beyond that starts sliding into expensive Tauri-harness coverage that duplicates app-core confidence rather than improving this file’s packet.

### Small but important framing correction

Not all optimize entrypoints in this file share the same product policy. `run_double_ma_optimization` normalizes selectively, `run_composite_cpu_optimization` forces `Grid`, and `run_composite_mo_optimization` does no normalization at all (`src-tauri/src/commands_optimize.rs:58-80`, `82-134`, `266-290`). That is another reason to be careful about any “neutral shared normalization module” cleanup. The policy is not generic optimize infrastructure; it is product-specific command-boundary behavior.

## Implementation Priority

### Safe-now hardening

The best safe-now hardening is narrower than the current packet’s implementation staging.

First, add a targeted Rust-side boundary test that proves early failure does not strand the single-run flag. The cheapest version is a command-path test that calls one of the optimize commands with an unknown `data_id`, expects the current error, and then immediately proves a second run can still begin. That directly exercises the `try_begin_run()` plus `RunGuard::drop` lifetime that this file depends on (`src-tauri/src/commands_optimize.rs:88-91`; `src-tauri/src/state.rs:162-169`, `408-420`, `559-566`).

Second, add a command-boundary compatibility test for normalization. The most valuable case is Double-MA with a non-`Grid` requested mode plus a grid-only output request. The command layer should continue to coerce the request before app-core sees it, because app-core will otherwise reject `export_all_csv_path` for non-`Grid` modes (`src-tauri/src/commands_optimize.rs:58-74`, `88`; `crates/app_core/src/double_ma.rs:5132-5139`). This is the regression that matters most if someone “cleans up” the wrapper later.

Third, keep the documentation/comments extremely small but strategic. The file would benefit from brief local comments that say:

- `cancel_double_ma_optimization` is a compatibility alias for foreground-run cancellation, not a Double-MA-only cancel path
- mode normalization here is a product-compatibility shim, not the engine’s final route-resolution logic
- `double_ma_progress` is still a shared progress channel used outside the main Double-MA optimize flow

Fourth, emit-failure observability remains a valid hardening item, but only in a narrow form. If touched, log the **first** `app.emit(...)` failure per sink/run and keep compute success/failure independent from progress transport. Anything stronger risks making UI transport availability affect core execution.

Fifth, if `OffsetProgressSink` continues to be shared, add one small test that locks in its current semantics: offset counts, fixed total, and fixed phase label. That is the behavior a refactor is most likely to change accidentally.

A lower-priority safe-now diagnostic, if the file is already being edited, is to classify or log `JoinError` before stringifying it. That should remain internal diagnostics only. Changing the user-facing error contract here would be a broader API decision than this bucket justifies.

### Larger redesign still deferred

Several earlier ideas are still defensible as longer-range cleanup, but they do **not** belong in the current bucket.

Removing `cancel_double_ma_optimization` remains deferred. The alias is still registered in the Tauri command surface (`src-tauri/src/lib.rs:183-184`), and the compatibility cost of keeping it is lower than the coordination cost of removing it prematurely.

Renaming `double_ma_progress` remains deferred. `REPO_MAP.md` still identifies it as a named frontend event channel and a progress truth anchor for optimize/runtime behavior (`REPO_MAP.md:824-835`). That is a public IPC contract, not a local naming preference.

Relocating `TauriProgressSink`, `OffsetProgressSink`, or the normalization helpers into a neutral runtime module should also stay deferred. That work is more about ownership aesthetics than correctness, and it touches multiple consumers for modest payoff.

Most importantly, any attempt to unify the Tauri-layer and app-core offset-progress helpers should stay deferred until the caller semantics are reviewed together. One helper rewrites `phase`; the other preserves it. Treating them as interchangeable would be a regression risk, not a simplification.

Finally, broader end-to-end Tauri harness expansion is still deferred unless there is already an existing lightweight harness in `src-tauri/tests/` that makes the added coverage cheap. App-core already owns most engine-facing behavior here. The next tests should be boundary tests, not a second test framework for the optimization engine.

## Verification If Changed

If this file changes, the verification should stay narrow and boundary-focused.

Start with the existing local coverage:

1. `cargo test -p ta_desktop_demo_app commands_optimize -- --nocapture`
2. `cargo test -p ta_desktop_demo_app --test command_runtime_wiring -- --nocapture`

Then add and run the missing boundary coverage that this file actually needs:

3. a `src-tauri`-side test that proves an early failure after `try_begin_run()` releases the run slot
4. a `src-tauri`-side test that proves command-boundary normalization still preserves current Grid-only compatibility behavior
5. if `OffsetProgressSink` remains shared, a small test that locks in its count/total/phase remapping semantics

If command names, cancel wiring, or event behavior move, rerun the directly affected frontend surfaces as well. If the change stays limited to comments and Rust-side tests, full frontend retesting is less important than preserving the command/event contract itself.

For a broader confidence pass after any real code change, use the repo gate the project already treats as canonical:

6. `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck`

No dedicated perf gate is warranted for this file alone. The wrapper is already off the command thread via `spawn_blocking`, and the hot-path work lives in app-core, not here.

## Regression And Feature-Preservation Constraints

Preserve the public command names and argument shapes. `run_double_ma_optimization`, `run_composite_cpu_optimization`, `run_composite_mo_optimization`, `cancel_current_run`, and `cancel_double_ma_optimization` are all part of the live Tauri surface (`src-tauri/src/lib.rs:120-125`, `183-184`). A review packet should not treat them as private implementation details.

Preserve the `double_ma_progress` channel name and payload shape. `REPO_MAP.md` still documents it as a named frontend event channel and an optimize/runtime progress truth anchor (`REPO_MAP.md:824-835`). Any rename or payload drift is an IPC compatibility change.

Preserve the current run-guard lifetime. The guard is acquired before candle lookup and held across the full async wait on the blocking task (`src-tauri/src/commands_optimize.rs:88-106`, `115-133`, `272-289`). Moving `try_begin_run()` later, dropping the guard earlier, or acquiring it inside the blocking closure would materially change overlap semantics.

Preserve the distinction between foreground-run cancellation and background-task cancellation. They target different state owners (`src-tauri/src/commands_optimize.rs:10-25`; `src-tauri/src/state.rs:559-575`, `1413-1454`).

Do not remove or weaken the command-boundary normalization without coordinated frontend and campaign review. This wrapper is currently preventing app-core’s stricter validation from surfacing as new public failures in some Grid-only-output cases (`src-tauri/src/commands_optimize.rs:58-80`; `crates/app_core/src/double_ma.rs:5132-5139`).

Do not turn progress-transport failure into command failure. `app.emit(...)` is transport, not the optimization result itself. If observability is added, it should stay diagnostic and low-noise.

Do not assume helper deduplication is semantics-preserving. The wrapper and app-core offset-progress helpers are not behaviorally identical (`src-tauri/src/commands_optimize.rs:45-54`; `crates/app_core/src/double_ma.rs:5518-5531`).

Also keep scope discipline. This file is not a persistence owner, not a GPU ABI owner, and not the main validation-logic owner. It fronts those systems, but broad changes here will mostly create compatibility risk rather than produce deep backend wins.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**. `src-tauri/src/commands_optimize.rs` is still best classified as `Small surgical refactor`.

What the third-pass packet got right is the high-level framing: this is a thin but sensitive public IPC boundary, and the main danger is contract drift, not missing local algorithms. What the fourth pass changes is the **priority order**.

The earlier packet was too eager to treat helper relocation as safe-now hardening, and it overstated the visibility gap around effective mode even though the backend result already returns `mode_used`, `backend_route`, and `backend_fallback_reason`. The more important missing work is narrower:

- prove early-failure paths release the run guard
- prove command-boundary normalization still preserves today’s product compatibility
- keep progress-transport observability narrow and non-fatal
- avoid “mechanical” helper deduplication that would change phase semantics

So the fourth-pass conclusion is conservative but sharper: **keep the bucket, narrow the safe-now work, and demote the ownership-cleanup refactors back into deferred territory.** That is the best way to make this file look stronger in a public repo without creating regressions in command wiring, progress UI, or route-selection behavior.
