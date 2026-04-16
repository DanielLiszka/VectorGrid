# Final Consolidated Review: `src-tauri/src/metric_surface.rs`

- Review date: `2026-04-16`
- Source file: [metric_surface.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/metric_surface.rs:1>)
- First-pass packet: [metric_surface.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/metric_surface.rs.review.md:1>)
- Second-pass packet: [metric_surface.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/metric_surface.rs.review.md:1>)
- Third-pass packet: [metric_surface.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/metric_surface.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 3/015__src-tauri__src__metric_surface.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://docs.rs/serde_json/latest/serde_json/value/fn.to_value.html>
  - <https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html>

## Consolidated Judgment

The fourth-pass packet is valid and corrects the earlier misclassification. [metric_surface.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/metric_surface.rs:1>) is not a Tauri command seam. It is an internal compatibility layer for metric-surface inference, sidecar summary, and sidecar rebuild.

The highest-value issue remains the precedence split:

- [artifact_metric_surface_kind](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/metric_surface.rs:387>) can honor current schema-2 sidecar metadata when present;
- [build_optimization_metric_surface_artifact](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/metric_surface.rs:416>) rebuilds from [detect_surface_kind](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/metric_surface.rs:339>) instead of reusing a builder-specific precedence helper.

That creates two different notions of effective surface: one for reading/summarizing and another for rebuilding. The fourth pass also correctly sharpens two other points:

- `serde_json::to_value(...).ok()` here is low-risk defensive code compared with the more important row-content semantics;
- non-finite metric cells can serialize to JSON `null`, which is an implicit contract worth documenting and testing.

## Safe-Now Priority

- Keep the bucket at `Small surgical refactor`.
- Add a builder-specific precedence helper for rebuild that prefers:
  1. runtime hint;
  2. valid current sidecar metadata;
  3. exact-GPU heuristics;
  4. broad strategy fallback.
- Add direct tests for:
  - `RegimeHybrid` exact-GPU coverage;
  - invalid schema-2 `surface` labels;
  - accepted alias spellings in [parse_surface_label](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/metric_surface.rs:359>);
  - non-finite row values and serialized JSON shape.
- Add short comments for:
  - `mean = sharpe * volatility`;
  - non-finite row values becoming JSON `null`.

## Deferred Work

- Defer any typed artifact-model redesign or schema-shape change.
- Do not broaden heuristic support tables without upstream evidence.
- Do not rename or repurpose serialized summary fields casually.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app metric_surface -- --nocapture`
- Add regression cases for rebuild precedence disagreement and the alias / invalid-surface / non-finite-row cases above.
- If callers outside this module change, follow with the relevant artifact/report/meta tests in the owning modules.

## Regression And Feature-Preservation Constraints

- Do not let persisted sidecar metadata outrank fresh runtime hint during rebuild.
- Do not silently change how non-finite row cells serialize.
- Do not turn this pass into sidecar schema churn.
- Treat this file as persistence-compatibility code, not IPC-wrapper code.

## Final Conclusion

The fourth-pass packet is valid, no wrong fourth-pass file is present, and the final bucket remains `Small surgical refactor`. The implementation priority is a narrow rebuild-precedence hardening plus missing edge-case tests, not a broader artifact-model rewrite.
