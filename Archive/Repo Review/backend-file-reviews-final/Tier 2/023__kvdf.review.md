# Final Consolidated Review: `crates/app_core/src/kvdf.rs`

- Review date: `2026-04-16`
- Source file: [kvdf.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/kvdf.rs:1>)
- First-pass packet: [kvdf.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/kvdf.rs.review.md:1>)
- Second-pass packet: [kvdf.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/kvdf.rs.review.md:1>)
- Third-pass packet: [kvdf.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/kvdf.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/023__crates__app_core__src__kvdf.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://vectoralpha.dev/projects/ta/indicators/vwap/>
  - <https://serde.rs/field-attrs.html>
  - <https://rust-lang.github.io/api-guidelines/documentation.html>

## Consolidated Judgment

The fourth-pass packet is the correct implementation-facing frame for this file. The earlier passes were directionally useful, but the fourth pass isolated the one concrete correctness defect that should move to the front of the queue: `normalize_kvdf_vwap_anchor()` lowercases the entire anchor string, which turns valid month anchors such as `1M` into minute anchors such as `1m`. The current VectorTA VWAP docs explicitly distinguish units `m`, `h`, `d`, and `M`, and only call out uppercase `H` and `D` normalization.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live file and is materially better than the third-pass abstraction-heavy framing.

## Safe-Now Priority

- Fix `normalize_kvdf_vwap_anchor()` so it preserves monthly anchors while still keeping the current `"session" -> "1d"` alias if product behavior depends on it.
- Add focused tests for:
  - `"session" -> "1d"`
  - `"1d" -> "1d"`
  - `"1D" -> "1d"`
  - `"4H" -> "4h"`
  - `"1M" -> "1M"`
  - `"2M" -> "2M"`
- Improve route diagnostics when a GPU-preferring backend falls back to CPU because prepared CUDA inputs are absent. Today the file only records diagnostics when a CUDA helper is attempted and returns an error.
- Add or tighten tests that pin current output-selection expectations for the Keltner / VWAP / DTI / FVG surfaces before changing output-resolution behavior.
- Add rustdoc on the public helpers that explains:
  - `KvdfStrategyParams` defaults participate in the serde contract
  - `"session"` is a compatibility alias, not the indicator’s native anchor spelling
  - desired-position construction assumes coherent, equal-length signal vectors

## Deferred Work

- Do not convert `build_kvdf_desired_positions()` to a fallible public API in the same pass.
- Do not broadly tighten output resolution or validation behavior across all public helpers without a compatibility review.
- Do not change the current all-`NaN` FVG acceptance/rejection split until the intended contract is made explicit and test-backed.
- Do not narrow public helper visibility in the same patch as behavior fixes.

## Verification If Changed

- `cargo test -p ta_app_core kvdf::tests:: -- --nocapture`
- Add and run focused file-local tests for monthly-anchor preservation and current alias normalization.
- If GPU-route diagnostics are changed:
  - `cargo test -p ta_app_core compute_kvdf_signals_records_route_diagnostics_on_cuda_cpu_fallback --features cuda -- --nocapture`
  - `cargo test -p ta_app_core compute_kvdf_signals_gpu_smoke_runs_with_all_indicator_surfaces --features cuda -- --nocapture`
- If output-resolution behavior changes:
  - `cargo test -p ta_app_core kvdf_optimization_smoke_runs -- --nocapture`
  - representative KVDF CPU/GPU parity tests under the existing CUDA lanes

## Regression And Feature-Preservation Constraints

- Preserve current entry/exit state-machine behavior unless a dedicated parity review says otherwise.
- Call out that fixing the month-anchor bug intentionally changes behavior for any existing config that used `*M` anchors and was accidentally getting minute semantics.
- Preserve current CPU fallback behavior for indicator compute; the safe-now diagnostic change is observability, not route removal.
- Treat validation tightening and public-surface narrowing as compatibility work, not incidental cleanup.

## Final Conclusion

Keep `kvdf.rs` in `Small surgical refactor`, treat the fourth pass as valid, and prioritize the VWAP month-anchor bug plus narrow diagnostics/docs/tests. Public-surface tightening, broader validation enforcement, and FVG-contract redesign stay deferred.
