# Second-Pass Review: `src-tauri/src/metric_surface.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/metric_surface.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/metric_surface.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a compatibility-sensitive artifact-sidecar layer whose heuristics and fallback behavior should change only carefully.

## Stale Check

The live source still owns:

- metric-surface sidecar shaping
- fallback detection of surface kind
- `best_metrics_row` and `top_metric_rows` compatibility behavior

The first-pass packet remains current.

## Second-Pass Research Log

1. `serde_json::Value`
   - Source: <https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html>
   - Why consulted in second pass:
     - verify pointer and fallback behavior in surface reconstruction
   - Additional constraint informed:
     - save-time regeneration still depends on runtime artifact field presence and fallback ordering

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in second pass:
     - tighten the schema-compatibility reading around optional sidecar fields
   - Additional constraint informed:
     - omission-handling remains part of backward compatibility here

3. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - confirm why this sidecar should still be treated as desktop-facing contract
   - Additional constraint informed:
     - these sidecars feed history/detail payloads that surface through commands

## Delta vs First Pass

### 1. Save-time sidecar rebuilds still reinterpret history through `detect_surface_kind()` instead of trusting persisted metadata

- Why this matters:
  - a heuristic change can rewrite historical interpretation when an artifact is resaved
- Practical implication:
  - heuristic adjustments here are history-compatibility changes, not mere cleanup

### 2. `/run/res/metric_surface_kind` remains a stronger truth source than the persisted sidecar during regeneration

- Practical implication:
  - preserve that priority order unless a migration plan explicitly replaces it

## Verification If Changed

- `cargo test -p ta_desktop_demo_app metric_surface -- --nocapture`
- `cargo test -p ta_desktop_demo_app run_artifacts -- --nocapture`
- `cargo test -p ta_desktop_demo_app commands_meta -- --nocapture`
- history and workflow-artifact frontend tests

## Regression And Feature-Preservation Constraints

- Preserve backward compatibility for existing artifacts
- Preserve `best_metrics_row` and `top_metric_rows` unless a migration is planned
- Treat heuristic changes as historical-interpretation changes

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that save-time sidecar regeneration can reinterpret historical artifacts, which makes heuristic cleanup here more compatibility-sensitive than it first appears.
