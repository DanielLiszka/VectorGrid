# Final Consolidated Review: `src-tauri/src/commands_system.rs`

- Review date: `2026-04-16`
- Source file: [commands_system.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_system.rs:1>)
- First-pass packet: [commands_system.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_system.rs.review.md:1>)
- Second-pass packet: [commands_system.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_system.rs.review.md:1>)
- Third-pass packet: [commands_system.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_system.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 3/010__src-tauri__src__commands_system.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Partially valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Safe-now hardening`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://docs.rs/window-vibrancy/latest/window_vibrancy/>

## Consolidated Judgment

The fourth pass correctly upgrades this file out of `Document only`, but one of its arguments does not survive contact with the current repo feature graph.

What is valid:

- [set_window_effect](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_system.rs:46>) clears the active effect before validating the requested effect string on both Windows and macOS.
- That means an unknown effect value can mutate window state and then return an error.
- This is a real, bounded public-command weakness and justifies a small hardening change.

What is not valid as written in the fourth-pass packet:

- the claimed `GpuStatus.cuda_available` ambiguity tied to a hypothetical kernel-only build;
- in this repo, `cuda-backtest-kernel` implies `cuda` in both [src-tauri/Cargo.toml](</C:/Rust Projects/ta_desktop_demo/src-tauri/Cargo.toml:15>) and [crates/app_core/Cargo.toml](</C:/Rust Projects/ta_desktop_demo/crates/app_core/Cargo.toml:9>);
- so `built_with_cuda_backtest_kernel == true` while `built_with_cuda == false` is not a reachable build shape here.

The final synthesis is therefore narrower than the fourth-pass packet:

- keep the bucket elevated to a small implementation pass;
- focus that pass on effect validation-before-side-effects;
- do not reshape the GPU DTO based on a feature-graph mismatch that is not real in the current workspace.

## Safe-Now Priority

- Validate and normalize the requested effect before any `clear_*` or `apply_*` side effects in [set_window_effect](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_system.rs:46>).
- If needed, extract a tiny pure planner/helper so unknown effect strings can be rejected without touching the OS window APIs.
- Add direct regression coverage proving an unknown effect is rejected before side effects are attempted.
- Keep [get_gpu_status](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_system.rs:24>) shape unchanged for this pass.

## Deferred Work

- Defer any broader `GpuStatus` DTO redesign unless the frontend and feature model move together.
- Do not widen the accepted effect string set without matching frontend filtering and manual verification.
- Do not churn [exit_application](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_system.rs:40>) return shape just to remove `Result`.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_system -- --nocapture`
- Manual desktop smoke for:
  - valid Windows effects: `mica`, `acrylic`, `blur`
  - valid macOS effect: `vibrancy`
  - unsupported platforms staying harmless
  - intentionally invalid effect input preserving the prior effect instead of clearing first

## Regression And Feature-Preservation Constraints

- Preserve current no-op behavior on unsupported platforms in [set_window_effect](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_system.rs:108>).
- Preserve the public command names and `GpuStatus` field names.
- Do not silently repurpose `cuda_available`; the fourth-pass GPU ambiguity argument is not currently supported by the workspace feature wiring.

## Final Conclusion

The fourth-pass packet is only partially valid. Its window-effect hardening point is correct and is the reason the final bucket moves to `Safe-now hardening`; its GPU-status ambiguity point is not supported by the current Cargo feature graph. No wrong fourth-pass file is present, and the implementation target should stay narrowly focused on validating effect strings before mutating window state.
