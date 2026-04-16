# Fourth-Pass Review: `src-tauri/src/commands_reports_api.rs`

- Review date: `2026-04-15`
- Source file: `src-tauri/src/commands_reports_api.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet correctly noticed that this file is not “just helper glue,” but it narrowed the framing too far toward path-opening authority. The live source still shows a mixed-sensitivity public Tauri boundary:

- transport-facing commands that return report or CSV payloads (`build_report_xlsx`, `build_validation_shortlist_csv` at `src-tauri/src/commands_reports_api.rs:14-30`)
- persistence-affecting save commands, especially the artifact-backed save path (`src-tauri/src/commands_reports_api.rs:32-72`)
- shell-launch commands for report directories and arbitrary paths (`src-tauri/src/commands_reports_api.rs:74-119`)

That mix matters. The file is still thin, and nothing in the live source justifies escalating beyond `Small surgical refactor`, but the third-pass packet underweighted two things that are more actionable than another round of “capability boundary” framing:

1. this file is also a persistence boundary, not only a path-launch boundary
2. the highest-confidence file-local work is contract clarity and targeted negative-case hardening, not structural wrapper churn

## Fourth-Pass Research Log

I re-read the live source first, then compared it against the second- and third-pass packets, and only used adjacent repo context where it materially changed confidence.

Primary local evidence reviewed:

- `src-tauri/src/commands_reports_api.rs`
  - public command surface, spawn-blocking handoff, response DTO, and wrapper tests
- `src-tauri/src/commands.rs`
  - confirms these commands and `SaveReportXlsxResponse` are publicly re-exported
- `src-tauri/src/lib.rs`
  - confirms the commands are registered in the app invoke handler
- `src-tauri/src/run_artifacts.rs`
  - confirms the artifact layer exposes both a generic path-checked loader and separate run-kind-checked loaders for neighboring flows

Important local observations from the source:

- every command in this file is a thin `spawn_blocking` wrapper (`src-tauri/src/commands_reports_api.rs:14-119`)
- `open_reports_dir` computes an app-owned reports directory and then delegates to reveal behavior, rather than opening an arbitrary caller-supplied path (`src-tauri/src/commands_reports_api.rs:87-105`)
- `open_path` and `reveal_path` accept arbitrary strings and delegate validation/launch behavior downward (`src-tauri/src/commands_reports_api.rs:107-119`)
- the test suite is not trivial; it already covers async wrapper parity, writes under the app-local reports dir, artifact-backed save, app-local reports-dir creation, and missing-path failures (`src-tauri/src/commands_reports_api.rs:225-372`)
- the artifact-backed save path is persistence-affecting: the source test asserts that `report_xlsx_path` is written back into the artifact JSON and that `artifact_updated` is `true` (`src-tauri/src/commands_reports_api.rs:276-317`)

Official guidance reviewed for contract pressure-testing:

1. Tauri v2 docs, **Calling Rust from the Frontend**
   - commands can be `async`, return serializable data, and may return `Result<_, String>`
   - the `Returning Array Buffers` section explicitly recommends `tauri::ipc::Response` for large binary returns instead of JSON serialization
2. Tauri v2 docs, **Permissions** and **Capabilities**
   - commands are explicit privileges and can be part of the app’s exposed capability surface
3. Tokio docs, **spawn_blocking**
   - `spawn_blocking` is appropriate for bounded blocking work that completes on its own
   - started blocking tasks cannot be aborted, so the current wrapper shape is only appropriate because these operations are expected to be short-lived and bounded

One important confidence limit remains: `commands_reports.rs` is not part of this review bundle. That means earlier claims about the exact internal loader used by `save_report_xlsx_from_artifact_blocking` cannot be re-proved here. Where the fourth pass discusses that concern, it does so as an adjacent likely hardening target, not as a newly re-proven file-local defect.

## What Fourth Pass Sharpened

### 1. The current third-pass bucket should stand

The bucket should remain `Small surgical refactor`.

The live file is still intentionally thin. It does not own workbook construction, report-layout logic, or path-policy logic. The public API surface is stable, the wrapper pattern is consistent, and the source already carries targeted wrapper tests. There is no new file-local evidence for a redesign bucket.

What the third-pass packet got right:

- it was correct not to attack `spawn_blocking` on principle
- it was correct to treat `open_path` and `reveal_path` as sensitive public commands
- it was correct to warn against duplicating path validation in the command layer

### 2. The third pass overgeneralized the “capability boundary” framing

The third-pass packet is directionally right, but too coarse.

This file is not one uniform path-opening surface. Its commands split into three materially different risk classes:

- **transport contract**: `build_report_xlsx` and `build_validation_shortlist_csv`
- **persistence contract**: `save_report_xlsx` and especially `save_report_xlsx_from_artifact`
- **authority / shell-launch contract**: `open_reports_dir`, `reveal_path`, and `open_path`

That distinction matters because the safe-now hardening work is different for each class. The path-launch commands need validation discipline and bounded blast radius. The artifact-backed save path needs persistence and contract clarity. The fallback workbook builder needs transport clarity.

Treating the whole file mainly as a path-opening capability boundary obscures the more concrete, lower-risk improvements.

### 3. The third pass underweighted the persistence side effects of the artifact-backed save path

This is the biggest missed nuance in the current packet.

`save_report_xlsx_from_artifact` is not just “load artifact, export workbook.” The live test proves that the command can mutate the saved artifact by writing back `report_xlsx_path`, and the public DTO explicitly models that with `artifact_updated` and `artifact_update_warning` (`src-tauri/src/commands_reports_api.rs:6-12`, `53-72`, `276-317`).

That makes this file a **persistence-sensitive IPC boundary** in addition to a path-sensitive one. The practical consequence is that any hardening around artifact-backed export must preserve:

- when `artifact_updated` becomes `true`
- when `artifact_update_warning` is populated versus treated as fatal
- the write-back contract for `report_xlsx_path`

The third-pass packet did not call this out, and that omission matters more than another round of generic capability-language tightening.

### 4. The third pass underweighted existing source-level verification

The live source already has better wrapper coverage than the third-pass packet’s tone suggests.

The current tests prove:

- async wrapper parity for the in-memory workbook and CSV builders (`src-tauri/src/commands_reports_api.rs:225-245`)
- report-save placement under the app-local reports directory (`src-tauri/src/commands_reports_api.rs:247-274`)
- artifact-backed save happy-path behavior, including persistence mutation (`src-tauri/src/commands_reports_api.rs:276-317`)
- reports-dir creation behavior (`src-tauri/src/commands_reports_api.rs:320-356`)
- missing-path failures for `open_path` and `reveal_path` (`src-tauri/src/commands_reports_api.rs:358-372`)

So the next step is not “test the wrappers in general.” The next step is targeted negative-case hardening where contract risk still exists.

### 5. The earlier artifact-kind concern still looks plausible, but it is not fully re-proven by this bundle alone

The first-pass packet likely found the sharpest adjacent hardening candidate: artifact-backed export should reject unsupported artifact kinds before mutating saved state.

What the fourth pass can prove from the current bundle is narrower:

- `save_report_xlsx_from_artifact` accepts an arbitrary `String` path at the public boundary (`src-tauri/src/commands_reports_api.rs:53-72`)
- the artifact subsystem exposes a generic path-checked loader that does **not** enforce run-artifact kind (`src-tauri/src/run_artifacts.rs:2415-2419`)
- neighboring artifact flows do enforce run kind before continuing (`src-tauri/src/run_artifacts.rs:2422-2435`)

That is enough to keep the earlier concern alive. It is **not** enough, by itself, to restate as a freshly proven defect inside `commands_reports_api.rs`, because the actual blocking owner (`commands_reports.rs`) is not part of this bundle.

So the right fourth-pass framing is:

- this is still the most credible adjacent hardening target
- it remains worth a negative test if the adjacent owner is touched
- it should not be overstated as a re-proven file-local bug without reopening the blocking implementation

### 6. The clearest missed file-local hardening is still contract clarity

The source is operationally sound, but its public contract remains underexplained.

The current packet underweights how much behavior is hidden behind innocuous names and raw `String` / `Value` types:

- `build_report_xlsx` returns a `String`, but Tauri’s current docs make clear that large binary returns are normally better served by `tauri::ipc::Response`; the existing `String` shape therefore needs to be documented as a compatibility-oriented fallback transport, not a neutral design
- `open_reports_dir` is implemented in terms of reveal behavior (`src-tauri/src/commands_reports_api.rs:101-105`), so the name is broader than the actual UX contract
- `open_path` and `reveal_path` are shared shell utilities reused across multiple features, not report-only helpers
- `save_report_xlsx_from_artifact` may update persistence and may surface a warning separately from total failure, but that is not discoverable from the public command names alone

This is the narrowest safe-now improvement because it strengthens the public boundary without changing semantics.

### 7. Some broader refactor recommendations remain more dangerous than they first sound

The live context makes a few “cleanup” ideas look riskier than the third-pass packet implies.

- **Moving `open_path` / `reveal_path` now**: these commands are publicly re-exported from `commands.rs` and registered in `lib.rs`. Renaming or relocating them is public-surface churn, not internal tidying.
- **Switching to `tauri-plugin-opener` now**: the current app initialization in `lib.rs` does not initialize the opener plugin. Adopting it would mean plugin setup, permission/capability coordination, and likely frontend dependency churn. That is a broader design move, not a surgical cleanup.
- **Changing the `build_report_xlsx` return shape now**: Tauri’s docs support a more efficient binary return path, but this command is already a compatibility/fallback lane. Changing it is IPC- and frontend-sensitive.
- **Typing every `Value` payload now**: that is a cross-layer refactor spanning the report builder, invocation layer, and test surfaces. It is not justified by a bug shown in this file.

## Implementation Priority

### Safe-now hardening

1. **Add command-level contract documentation.**
   This is the strongest safe-now change.

   At minimum, document:

   - `build_report_xlsx` returns base64-encoded XLSX content as a fallback transport
   - `build_validation_shortlist_csv` returns CSV text, not a saved path
   - `save_report_xlsx_from_artifact` can update persisted artifact metadata and may surface a non-fatal warning via `artifact_update_warning`
   - `open_reports_dir` reveals the app-local reports directory through the shared launcher behavior
   - `open_path` and `reveal_path` are shared shell-launch utilities despite their current module home

2. **Optionally add command-specific context to join-failure strings.**
   Today every wrapper does `map_err(|e| e.to_string())`. Prefixing those with the command name would improve rare panic/join diagnostics without changing the success path.

   This is low-risk, but it is still a user-visible error-string change, so it should be treated as polish, not as a correctness fix.

3. **If the adjacent blocking owner is already being edited, add unsupported-artifact negative coverage.**
   The safest version of the earlier first-pass hardening remains:

   - ensure artifact-backed export rejects unsupported artifact kinds before persistence mutation
   - add a negative test proving that rejection

   But keep that scoped to the adjacent owner change, because the current bundle does not re-prove that internal path.

4. **If practical, add one targeted partial-success test for `artifact_update_warning`.**
   The DTO exists for a reason, but the current source tests only cover the empty-warning happy path. If the blocking owner already has an injectable seam for this behavior, one targeted test would make the persistence contract more explicit.

### Larger redesign still deferred

1. **Do not replace the fallback `String` transport with `tauri::ipc::Response` yet.**
   Tauri’s current guidance supports that redesign, but the change is IPC-sensitive and frontend-sensitive. It should wait for evidence that the fallback path is operationally important enough to justify coordinated churn.

2. **Do not convert the `Value` payloads to typed DTOs in isolation.**
   That is worthwhile only when the broader report contract is intentionally being tightened across frontend and backend together.

3. **Do not relocate `open_path` / `reveal_path` just to clean up module ownership.**
   The commands are already public, reused, and registered. Relocation is a compatibility exercise, not a local readability win.

4. **Do not swap to `tauri-plugin-opener` as a “small cleanup.”**
   That would require plugin initialization, capability/permission work, and consumer coordination. It is a deliberate cross-cutting redesign, not a safe-now refactor.

5. **Do not churn `SaveReportXlsxResponse` ownership unless adjacent report owners are already moving.**
   The type may not live in the ideal place, but changing that alone buys very little and can ripple into adjacent modules.

## Verification If Changed

Keep verification proportional to the actual edit.

For any code change in this file or its immediate owner path:

1. `cargo test -p ta_desktop_demo_app commands_reports_api -- --nocapture`
2. `cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests -- --nocapture`

If the change affects frontend invocation semantics, command names, or fallback workbook handling:

3. `npm --prefix ./web run test -- src/services/validationService.test.ts src/stores/commandActionStore.test.ts src/tauri/commands.test.ts`

If shell-launch semantics or path policy changes:

4. do one manual desktop check for:
   - `open_reports_dir`
   - `open_path` on a valid path
   - `reveal_path` on a valid path
   - rejected missing/out-of-policy paths

The third-pass packet slightly overstates manual smoke as a general requirement. It is only necessary when shell-launch behavior or path-policy behavior changes. It is not required for comment-only changes or DTO-local documentation.

## Regression And Feature-Preservation Constraints

- Preserve current command names and return shapes unless the frontend command surface is updated in lockstep. These commands are publicly re-exported in `src-tauri/src/commands.rs` and registered in `src-tauri/src/lib.rs`.
- Preserve the current persistence contract of `save_report_xlsx_from_artifact`, including the meaning of `artifact_updated`, `artifact_update_warning`, and the write-back of `report_xlsx_path`.
- Do not duplicate path validation in this wrapper layer; keep validation centralized in the shared launcher/report owner path.
- Do not widen what `open_path` or `reveal_path` will act on.
- Do not silently change `open_reports_dir` from reveal-style behavior to a different folder-open UX without coordinated consumer review.
- Treat `build_report_xlsx` as transport-sensitive IPC. Any change to its return type or encoding is a coordinated contract change, not an internal optimization.
- There is no GPU ABI risk here, and no reason to let this file accumulate GPU- or validation-engine-specific logic. Keep it a boundary layer.
- Keep `spawn_blocking` usage bounded. Do not let these wrappers absorb long-lived work that belongs in dedicated threads or deeper orchestration layers.

## Fourth-Pass Conclusion

The third-pass bucket should **stand**: `Small surgical refactor` is still the right action bucket.

What the current third-pass packet got right was the warning that `open_path` and `reveal_path` are genuine public capability surfaces and that validation should remain centralized. What it missed was that this file is also a **persistence-sensitive** and **transport-sensitive** command boundary. The live source proves that artifact-backed export mutates saved state and that the public DTO already encodes partial-success semantics. That deserves more weight than another round of generic capability framing.

The safest, highest-confidence next step is not a redesign. It is:

- make the public command contracts explicit in code comments/rustdoc
- keep path validation centralized
- if the adjacent blocking owner is touched, add targeted negative artifact-kind coverage there

The broader ideas — binary transport redesign, typed DTOs, module relocation, or opener-plugin adoption — remain deferred because they are compatibility-sensitive and broader than the live file justifies.
