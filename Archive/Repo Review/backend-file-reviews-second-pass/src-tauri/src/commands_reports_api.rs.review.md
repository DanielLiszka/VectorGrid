# Second-Pass Review: `src-tauri/src/commands_reports_api.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_reports_api.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_reports_api.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass treated this file as a narrow but contract-sensitive Tauri wrapper around the real report-export owners and kept the recommended work small.

## Stale Check

The live source still functions as:

- the Tauri-facing report command layer
- a base64 fallback transport for workbook bytes
- the home of app-wide `open_path` / `reveal_path` shell helpers

No source drift weakened the first-pass packet.

## Second-Pass Research Log

1. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - verify the command typing and binary-return posture at the Tauri IPC boundary
   - Additional constraint informed:
     - the base64 path is a transport workaround, not a neutral API design choice

2. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in second pass:
     - confirm the runtime contract for these filesystem and shell wrappers
   - Additional constraint informed:
     - they should remain bounded wrappers, not become long-lived workflow execution paths

3. Tauri state-management guidance
   - Source: <https://v2.tauri.app/develop/state-management/>
   - Why consulted in second pass:
     - verify whether the `AppHandle`-mediated context here implies any broader architectural issue
   - Additional constraint informed:
     - the current wrapper role remains appropriate

## Delta vs First Pass

## Refined API Finding

### 1. `build_report_xlsx` should continue to be framed as a fallback transport, not the preferred report API shape

- Evidence:
  - the path returns base64 workbook data rather than a persisted report contract
- Why this matters:
  - treating it as primary would entrench an IPC-heavy payload shape that is only justified as fallback
- Practical implication:
  - preserve the current DTO and fallback semantics unless report transport is intentionally redesigned

## Refined Boundary Finding

### 2. `open_path` and `reveal_path` are already app-wide shell utilities, not merely report-local helpers

- Evidence:
  - other features rely on them outside report export
- Why this matters:
  - removing or renaming them during "report cleanup" would have a wider blast radius than the file name suggests
- Practical implication:
  - keep them stable until all consumers move together

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_reports_api -- --nocapture`
- frontend validation-service fallback tests
- latest-report open/reveal flows

## Regression And Feature-Preservation Constraints

- Keep command names and DTO shapes aligned with `web/src/tauri/commands.ts`
- Preserve `SaveReportXlsxResponse`
- Preserve `build_report_xlsx` as fallback-only unless transport is intentionally redesigned
- Keep `open_path` and `reveal_path` stable until unrelated consumers are migrated

## Second-Pass Conclusion

`Bucket unchanged, but the fallback and shell-helper boundaries are sharper`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that the base64 workbook command is a fallback transport and that the open/reveal helpers have already escaped report-local scope.
