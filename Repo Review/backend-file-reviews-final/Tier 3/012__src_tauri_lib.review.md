# Final Consolidated Review: `src-tauri/src/lib.rs`

- Review date: `2026-04-16`
- Source file: [lib.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/lib.rs:1>)
- First-pass packet: [lib.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/lib.rs.review.md:1>)
- Second-pass packet: [lib.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/lib.rs.review.md:1>)
- Third-pass packet: [lib.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/lib.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 3/012__src-tauri__src__lib.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://docs.rs/tauri-plugin-window-state/latest/tauri_plugin_window_state/>
  - <https://doc.rust-lang.org/std/thread/fn.spawn.html>
  - <https://doc.rust-lang.org/std/thread/struct.Builder.html>

## Consolidated Judgment

The fourth-pass packet is the right synthesis. [lib.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/lib.rs:1>) is the desktop bootstrap seam where plugin registration, managed state, window creation, startup restore/show policy, optional CUDA prewarm, and the full Tauri command registry converge.

The strongest file-local issue is not generic IPC commentary. It is overlapping startup ownership:

- the app installs the window-state plugin at [lib.rs:35](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/lib.rs:35>);
- then `setup` manually creates the window, calls [restore_state(StateFlags::all())](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/lib.rs:49>), and unconditionally calls [show()](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/lib.rs:53>).

That overlaps with the plugin’s own initial-restore behavior and makes `VISIBLE` policy ambiguous. The fourth pass also correctly points out that the detached prewarm path at [lib.rs:58](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/lib.rs:58>) uses `std::thread::spawn`, so the real spawn-site risk is thread-creation panic, not Tokio blocking semantics.

## Safe-Now Priority

- Choose one owner for initial main-window restore/show semantics.
- If manual restore stays, explicitly skip plugin initial restore for that window.
- If plugin-driven restore stays, remove the extra manual restore/show path after desktop verification.
- Make visibility policy explicit:
  - if the product always wants the main window visible, stop restoring `VISIBLE`;
  - if saved visibility should matter, stop unconditionally calling `show()` afterward.
- Consider switching the detached CUDA prewarm spawn to `std::thread::Builder::new().name(...).spawn(...)` so thread-creation failure can be logged instead of panicking.

## Deferred Work

- Defer any WebView `data_directory` migration; that is a persistence-policy change, not a cleanup.
- Defer command-permission narrowing and broader bootstrap decomposition.
- Treat label-based window lookup as optional clarity hardening, not the main issue.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app --test mock_runtime_smoke -- --nocapture`
- `cargo test -p ta_desktop_demo_app --test command_runtime_wiring -- --nocapture`
- Re-run parity checks because [lib.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/lib.rs:82>) is the canonical command registry:
  - `npm --prefix ./web run check:commands`
  - `cd web && node ./scripts/check-tauri-parity.mjs`
- Manual desktop verification should focus on startup restore/show behavior and first-launch / relaunch behavior.

## Regression And Feature-Preservation Constraints

- Preserve command names and registration order unless frontend contract work moves with them.
- Do not merge `AppState` and `BackgroundTaskState` semantics here.
- Do not block startup on CUDA prewarm.
- Do not change WebView storage location casually.

## Final Conclusion

The fourth-pass packet is valid, no wrong fourth-pass file is present, and the final bucket remains `Small surgical refactor`. The implementation focus should be narrow: de-conflict startup window ownership and make the detached prewarm spawn itself best-effort.
