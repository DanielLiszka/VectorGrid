# Fourth-Pass Review: `src-tauri/src/lib.rs`

- Review date: `2026-04-16`
- Source file: `src-tauri/src/lib.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file is still small, but it is not trivial glue. It is the desktop bootstrap seam where plugin registration, managed state injection, manual window creation, startup restore/show policy, optional CUDA prewarm, and the full frontend-invokable command registry all converge (`src-tauri/src/lib.rs:15-194`). That makes it exactly the kind of Tier 3 file where earlier packets can be directionally right while still missing the sharpest concrete edge case.

The third-pass packet was right to keep the bucket conservative and to treat this file as a public boundary. The adversarial fourth pass mainly changes **what deserves the emphasis**. The most important issue in the live source is not another generic note about IPC sensitivity. It is the fact that startup window-state restore/show behavior currently appears to have **two owners**: the window-state plugin and the manual setup path.

This pass also matters because the repo prose is already stale on at least one live-source fact: `REPO_MAP.md` still says the Tauri app exposes `77` commands (`context__REPO_MAP.md:550-554`), while the current `generate_handler!` list in `lib.rs` contains `109` entries (`src-tauri/src/lib.rs:82-191`). That reinforces the earlier packets’ source-first framing, but it also means the review should keep challenging doc-driven assumptions.

## Fourth-Pass Research Log

1. I reread the live `lib.rs` and the third-pass packet first, then used `commands.rs` and `state.rs` only where needed to verify command-surface and state-ownership claims. The live file still does five distinct things in one place: plugin setup, state injection, manual window bootstrap, detached GPU prewarm, and the full command registry (`src-tauri/src/lib.rs:15-194`; `context__01__src-tauri__src__commands.rs:44-146`; `context__02__src-tauri__src__state.rs:559-574,1413-1452`).

2. I checked the official Tauri command and capability docs. Those confirm two things the earlier packets were broadly right about: the app-command surface is whatever is registered through the single `generate_handler!` call, and app commands are allowed by default unless `AppManifest::commands` is used to narrow them. That supports the public-IPC framing, but it does not by itself identify the highest-value fix for this file. See the official Tauri docs on Calling Rust from the Frontend and Capabilities.

3. I checked the official Tauri config reference. That matters because it weakens one of the earlier packets’ stronger claims: Tauri’s own `create: false` example manually creates a window from `app.config().app.windows[0]`, so the current `get(0)` lookup is order-sensitive but not inherently contrary to framework guidance. The same config docs also confirm that `dataDirectory` owns webview storage semantics such as localStorage and cache, and that Windows may require distinct data directories for windows with differing settings. See the official Tauri config reference.

4. I checked the official window-state plugin docs **and** the current plugin source. This is where the fourth pass sharpened the review materially. The plugin docs say tracked windows restore state automatically after creation, and the plugin source shows that `Builder::default().build()` restores initial state on `on_window_ready` unless that behavior is explicitly skipped. The same source also shows that `restore_state()` already uses the `VISIBLE` flag to decide whether it should call `show()`. See the official window-state plugin docs and the current plugin source.

5. I checked the Rust standard library docs for `std::thread::spawn`, `JoinHandle`, and `std::thread::Builder::spawn`. That matters because the live file uses a detached `std::thread::spawn`, not Tokio. The standard docs confirm that dropping the returned `JoinHandle` detaches the thread, and that `thread::spawn` panics if the OS cannot create the thread, while `Builder::spawn` returns an error instead. See the standard library docs for `std::thread::spawn`, `JoinHandle`, and `std::thread::Builder`.

## What Fourth Pass Sharpened

### Bucket assessment

The third-pass bucket should **stand**.

This file still fits `Small surgical refactor`. The fourth pass did not find evidence for a larger redesign local to `lib.rs`. The important opportunities are still narrow: clarify or de-conflict startup ownership, and tighten one best-effort background thread edge case. Anything broader would mostly reshuffle bootstrap code without reducing real risk.

### What the current third-pass packet got right

The third-pass packet was correct about three important things.

First, it was right to keep this file in a conservative bucket. The file is sensitive, but it is not an engine hotspot or architecture center of gravity. Second, it was right to treat `generate_handler!` as the compatibility surface that matters most for frontend IPC. The stale `77`-command count in `REPO_MAP.md` makes that even clearer: only the live `lib.rs` registry is trustworthy enough for command inventory (`context__REPO_MAP.md:552-554`; `src-tauri/src/lib.rs:82-191`). Third, it was right not to casually collapse state ownership. `AppState` and `BackgroundTaskState` really do represent separate cancellation/guard domains in local context (`context__02__src-tauri__src__state.rs:559-574,1413-1452`).

### What it missed or underweighted

#### 1. The sharpest file-local issue is overlapping startup ownership, not generic IPC commentary

The live file registers the window-state plugin with its default builder (`src-tauri/src/lib.rs:35`) and then, in `setup`, manually calls `window.restore_state(StateFlags::all())` followed by `window.show()` (`src-tauri/src/lib.rs:49-55`). That would be unremarkable if the plugin were passive. It is not.

The current window-state plugin restores initial state automatically for tracked windows on `on_window_ready` unless that initial restore is explicitly skipped, and its `restore_state()` implementation already decides whether to call `show()` based on the saved `VISIBLE` state. In other words, the live file currently combines plugin-managed initial restore with manual initial restore/show logic. Even if the current behavior happens to work, the ownership is ambiguous and the review should say that plainly.

That is a much stronger fourth-pass finding than the current packet’s generic “public IPC seams should stay honest about state ownership and cancellation scope” language. The generic warning is directionally fine. The overlapping restore/show policy is the concrete issue.

#### 2. `StateFlags::all()` plus unconditional `show()` is internally inconsistent

This is the clearest missed edge case.

`StateFlags::all()` includes `VISIBLE`, and the plugin’s own restore logic only calls `show()` when the saved visibility says the window should be shown. The current file restores `StateFlags::all()` and then unconditionally calls `window.show()` anyway (`src-tauri/src/lib.rs:49-55`). That means one of two things is true:

- either saved visibility is **not** part of the intended product contract, in which case restoring `VISIBLE` is misleading noise, or
- saved visibility **is** part of the contract, in which case unconditional `show()` overrides it.

The previous packet missed that direct contradiction. This is a better target for a small hardening change than more commentary near `generate_handler!`.

#### 3. The GPU prewarm path is best-effort in body, but not fully best-effort at spawn time

The prewarm work is clearly intended to be non-fatal and detached: it runs only after the main window is shown, logs warnings on device discovery or prewarm failure, and does not block startup (`src-tauri/src/lib.rs:53-78`). That overall policy is reasonable.

But the live code uses `std::thread::spawn`, drops the `JoinHandle`, and does not name the thread (`src-tauri/src/lib.rs:58-76`). The standard library semantics matter here:

- dropping the `JoinHandle` detaches the thread, so there is no later join or cancellation handle,
- and `std::thread::spawn` panics if the OS fails to create the thread, while `std::thread::Builder::spawn` would let this path log and continue instead.

For a path whose whole purpose is “warm things if possible, but do not make startup brittle,” the spawn-site failure behavior is slightly misaligned with the body’s existing warn-and-continue posture. The earlier packets did not call that out.

#### 4. The WebView data-directory concern is more about persistence policy than comment quality

The earlier packet was right that the `temp_dir()/vectorgrid/webview2` path is implicit (`src-tauri/src/lib.rs:43-47`), but it slightly underframed why that matters.

This path is not just a code-style question. Tauri’s config reference treats the data directory as the storage root for webview state such as localStorage and cache, and it also notes Windows compatibility constraints when windows with different settings use different data directories. So the real issue is product policy and future window/profile compatibility, not merely “this needs a comment.”

That said, this is still **not** a safe-now migration target. Changing it would alter persistence behavior and potentially invalidate or relocate existing webview state.

### What was overstated or weakly supported

#### 1. The third-pass Tokio research does not match the live source

The third-pass research log cites Tokio `spawn_blocking` guidance. The live file does not use Tokio here. It uses a detached `std::thread::spawn` (`src-tauri/src/lib.rs:58-76`). That does not invalidate the bucket, but it does make that piece of the packet weakly supported.

A fourth-pass review should call this out because the mismatch changes the real risks. The relevant questions are detach/join semantics and spawn-failure behavior, not blocking-runtime semantics.

#### 2. The earlier window-index claim was somewhat too strong

The current `app.config().app.windows.get(0)` lookup (`src-tauri/src/lib.rs:39-41`) is order-sensitive, so it is fair to call it less explicit than a label lookup. But the prior packet pushed it close to “correctness issue” territory.

That is stronger than the evidence in this bundle supports.

- The actual `tauri.conf.json` is not in the fourth-pass bundle.
- Tauri’s own config docs show manual creation from `app.config().app.windows[0]` for `create: false` windows.

So this should be reframed as a **future-proofing clarity improvement**, not the primary defect in the file.

#### 3. A broad cleanup of imports or helper extraction would be noisy and low-yield

`use commands::*;` is visually broad (`src-tauri/src/lib.rs:8`), but `commands.rs` re-exports far more than the frontend IPC surface, including headless-only helpers and DTOs (`context__01__src-tauri__src__commands.rs:53-60,113-130,132-145`). Replacing the wildcard with a 109-item explicit import list would mostly duplicate the handler registry and create another synchronization surface.

Likewise, splitting `run_tauri()` into many tiny helpers would not solve the actual ambiguities identified above. It would mostly move them around.

## Implementation Priority

### Safe-now hardening

1. **De-conflict initial window restore/show ownership.** This is the highest-value small hardening change.

   The code should have one clear owner for initial window restore/show semantics. A conservative implementation can go in either of two directions:

   - keep the manual restore path, but configure the window-state plugin to skip initial restore for that specific window so startup restore happens only once, or
   - trust the plugin’s default initial restore and remove the extra manual restore/show path after desktop smoke verification.

   In either case, make the visibility policy explicit. If the product wants the main window to always open visible, then do **not** restore `VISIBLE`. If saved visibility should matter, then do **not** unconditionally call `show()` afterwards.

2. **Make the detached CUDA prewarm spawn-site match the existing best-effort policy.**

   A very small, low-risk improvement would be to switch the prewarm spawn from `std::thread::spawn` to `std::thread::Builder::new().name(...).spawn(...)`, log failure to create the thread, and still drop the handle. That preserves current behavior where the prewarm remains detached and non-blocking, but it removes the “panic if OS thread creation fails” edge case and improves observability.

3. **Treat label-based window lookup as optional clarity hardening, not a must-fix.**

   If the repo already has a hard contract that the manually created window is labeled `main`, then label lookup is a reasonable cleanup. But this should stay behind the restore/show ownership fix and should not be sold as the primary correctness win from this pass.

### Larger redesign still deferred

1. **Changing the WebView data-directory policy** should stay deferred. Moving away from the current temp-based absolute path would change persistence behavior for webview cache and storage, and may also matter for future multi-window/profile behavior on Windows. That is a product-policy change, not a casual cleanup.

2. **Restricting app commands through `AppManifest::commands` or a tighter permission model** is also deferred. Tauri’s default app-command exposure can be narrowed, but that is a build/config contract change with real frontend and capability-file coordination cost. It is worth considering at the product boundary level, but it is not a `lib.rs`-only hardening pass.

3. **Large bootstrap decomposition** should remain deferred. The fourth pass did not find evidence that this file’s current shape is the problem. The problem is overlapping responsibility inside the current shape.

## Verification If Changed

Because the main fourth-pass finding is about startup ownership, verification should lean more heavily on **desktop behavior** than on unit tests alone.

1. Run the existing Rust-side smoke coverage that earlier packets already used:
   - `cargo test -p ta_desktop_demo_app --test mock_runtime_smoke -- --nocapture`
   - `cargo test -p ta_desktop_demo_app --test command_runtime_wiring -- --nocapture`

2. Re-run command-surface parity checks because `lib.rs` remains the canonical registry:
   - `npm --prefix ./web run check:commands`
   - `cd web; node ./scripts/check-tauri-parity.mjs`

3. Perform manual desktop verification focused on the startup path, not just command invocation:
   - first launch with no saved window state,
   - relaunch after move/resize/maximize,
   - relaunch after any intentionally supported hidden/minimized/visibility state,
   - check that startup produces one coherent show/restore sequence instead of a double-restore or focus tug-of-war,
   - if the prewarm path changed, verify a CUDA-enabled build still starts cleanly and that the app does not block on the prewarm thread.

4. If the data-directory policy changes at all, add explicit persistence checks for local webview state. That is a higher-risk change than anything else discussed in this packet.

## Regression And Feature-Preservation Constraints

- Do **not** rename, add, or remove commands casually. The live `generate_handler!` list is the actual frontend IPC contract, and repo prose is already stale on the command count (`context__REPO_MAP.md:552-554`; `src-tauri/src/lib.rs:82-191`).
- Do **not** merge `AppState` and `BackgroundTaskState` cancellation semantics in the name of cleanup. Local context still shows distinct ownership and guard behavior (`context__02__src-tauri__src__state.rs:559-574,1413-1452`).
- Do **not** keep both plugin-driven and manual startup restore/show ownership without an explicit reason. That is the clearest current ambiguity.
- Do **not** “fix” the prewarm thread by making it synchronous or join-on-startup. This file’s GPU-sensitive concern is cold-start behavior and first-use latency, not throughput. Joining or blocking here would move the code in the wrong direction.
- Do **not** change the WebView data-directory path casually. That is a persistence and compatibility change, not a cosmetic refactor.
- There is no evidence in this file of a GPU ABI or validation-algorithm problem. The GPU-sensitive risk here is only startup prewarm policy; validation risk only appears indirectly if command names or bootstrap behavior drift.

## Fourth-Pass Conclusion

The third-pass bucket should **remain** `Small surgical refactor`.

What changed in the fourth pass is not the bucket. It is the prioritization.

The current third-pass packet was right that `lib.rs` is the public command boundary and that changes here should stay conservative. But it underweighted the most concrete issue in the live source: the startup path appears to layer manual window-state restore/show on top of a plugin that already performs initial restore and visibility handling. That overlap is a better small hardening target than more commentary about IPC sensitivity or a broad bootstrap rewrite.

The best safe-now work is therefore:

- choose one owner for initial restore/show semantics and make `VISIBLE` policy explicit,
- make the detached CUDA prewarm thread creation itself best-effort,
- keep window-index cleanup secondary and modest.

Everything bigger than that — command-permission narrowing, data-directory migration, major bootstrap decomposition — should stay deferred because it carries more regression risk than this file currently justifies.
