# Final Consolidated Review: `crates/app_core/src/double_ma_mode.rs`

- Review date: `2026-04-15`
- Source file: [double_ma_mode.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/double_ma_mode.rs:1>)
- First-pass packet: [double_ma_mode.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/double_ma_mode.rs.review.md:1>)
- Second-pass packet: [double_ma_mode.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_mode.rs.review.md:1>)
- Third-pass packet: [double_ma_mode.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_mode.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/012__crates__app_core__src__double_ma_mode.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with one caveat`
- Canonical final action bucket: `Safe-now hardening`
- External research anchors:
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://doc.rust-lang.org/book/ch11-01-writing-tests.html>

## Consolidated Judgment

The fourth-pass packet correctly rejects the stale third-pass framing. This file is not a serde/default seam; it is a mode-selection policy helper whose decisions are externally visible through route behavior and serialized optimization results. The bucket lift from `Document only` to `Safe-now hardening` is justified.

The caveat is evidentiary, not directional. The most important issue is inferred from the live control flow rather than already backed by a reproducing test: Auto `GpuVdps` promotion currently keys off request/kernel compatibility, while executable GPU availability is computed later. That should be hardened, but it should be hardened with a reproducer first. No wrong `review_fourth_pass.md` is present in this folder.

## Safe-Now Priority

- Split "GPU-route compatible" from "GPU-route executable" in the mode-selection inputs.
- Make Auto `GpuVdps` promotion depend on an executable GPU route, not just request/kernel compatibility.
- Add focused tests for:
  - the exact `500_000` versus `500_001` threshold,
  - `BestEffort` non-promotion,
  - `PreferGpuResident` with no executable GPU route,
  - the defensive `export_all_csv_path` backstop.
- Name the large-search threshold as a constant so policy and tests stay aligned.

## Deferred Work

- Do not unify planner, backend resolution, and runtime fallback logic in this pass.
- Do not retune the `500_000` threshold without benchmark evidence.
- Do not collapse this threshold into later `GpuVdps` internal thresholds merely because both describe "large" searches.
- Do not change explicit manual `OptimizationMode::GpuVdps` rejection semantics in the same patch as Auto-mode hardening.

## Verification If Changed

- `cargo test -p ta_app_core double_ma_mode -- --nocapture`
- `cargo test -p ta_app_core auto_mode_only_adapts_when_grid_outputs_are_disabled -- --nocapture`
- `cargo test -p ta_app_core auto_mode_prefers_vdps_for_gpu_resident_large_runs_when_route_is_viable -- --nocapture`
- `cargo test -p ta_app_core gpu_vdps_rejects_cpu_backend -- --nocapture`

## Regression And Feature-Preservation Constraints

- Preserve the current `GpuVdps + CpuOnly` rejection for explicitly selected `GpuVdps`.
- Preserve `include_all`, `heatmap_bins`, and `export_all_csv_path` incompatibilities for `CoarseToFine` and `GpuVdps`.
- Keep the `500_000` Auto cutoff benchmark-sensitive.
- Treat `mode_used`, backend-route notes, and `vdps` result metadata as externally visible behavior, not private implementation details.

## Final Conclusion

The implementation-facing conclusion is:

- upgrade `double_ma_mode.rs` from `Document only` to `Safe-now hardening`
- accept the fourth-pass direction, but start with a reproducing test before changing policy
- harden Auto `GpuVdps` promotion around executable GPU availability, add the missing negative-path tests, and name the threshold constant
- defer planner unification, threshold retuning, and broader route-policy redesign
