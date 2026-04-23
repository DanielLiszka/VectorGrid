# Fourth-Pass Review: `src-tauri/src/run_artifacts.rs`

- Review date: `2026-04-16`
- Source file: `src-tauri/src/run_artifacts.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/run_artifacts.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/run_artifacts.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/src-tauri/src/run_artifacts.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file still merits a fourth pass because the current third-pass packet narrowed the discussion to write durability, and that framing is directionally useful but not yet sharp enough for the live source. The durability concern is real, but it is not the only high-signal issue left in the file, and it is not obviously the first change that should land.

The current third-pass bucket should stand. The live source still looks like a persistence-sensitive compatibility hub that needs additive hardening, not a broad redesign. The remaining work is still local to `run_artifacts.rs`: path-contract tightening, normalization consistency, latest-artifact selection correctness, and more honest write-semantics wording. That is still `Small surgical refactor`, not a larger architecture move.

## Fourth-Pass Research Log

1. Rust `std::fs::write` documentation.
   - Why consulted: the third-pass packet centered its argument on whole-file replacement writes.
   - Constraint sharpened: `fs::write` is a convenience whole-file write API, not a durability contract by itself.

2. Rust `std::fs::rename` documentation.
   - Why consulted: to test whether “atomic replace” is precise enough wording for a safe-now recommendation.
   - Constraint sharpened: replacement behavior is platform- and filesystem-sensitive enough that a staged-write helper should be described conservatively, especially on Windows.

3. Rust `std::fs::File::sync_all` documentation.
   - Why consulted: to separate “best-effort staged replace” from “durable commit”.
   - Constraint sharpened: full durability has an explicit cost, so adding `sync_all` indiscriminately—especially to live-session append paths—would be a real performance regression risk.

4. Rust `Path::canonicalize` documentation.
   - Why consulted: the overwrite path validation question in this file depends on whether the full path or only the parent is canonicalized.
   - Constraint sharpened: the file’s safer path checks are the ones that canonicalize the full path; the overwrite helper currently does not.

5. Tauri v2 “Calling Rust from the Frontend” documentation.
   - Why consulted: to re-anchor these helpers as IPC-facing command behavior rather than internal-only utilities.
   - Constraint sharpened: save/load/update failures and compatibility quirks here are desktop contract issues, not just backend cleanup details.

6. Local source cross-checks.
   - `src-tauri/src/run_artifacts.rs` was re-read around overwrite targeting, latest-run selection, hydration/detail builders, live-session mutation, and retention helpers.
   - `src-tauri/src/lib.rs` was cross-checked to confirm that `save_run_artifact`, `save_batch_summary`, `load_latest_run_artifact_summary`, `load_latest_run_artifact_hydration`, `update_artifact_tags`, `load_run_artifact`, and related commands are on the public Tauri command surface (`src-tauri/src/lib.rs:150-170`).
   - `src-tauri/src/state.rs` was re-checked only for surrounding runtime expectations; nothing there changes the persistence conclusions.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The current third-pass packet is right about three important things.

First, the bucket should remain small. The file is still a persistence and compatibility hub, but the remaining changes do not require a library-model rewrite.

Second, the durability concern is real. `write_json_pretty`, `save_run_artifact_blocking`, `save_batch_summary_blocking`, and `update_artifact_tags_blocking` still end in plain whole-file writes, so the current implementation should not be described as crash-safe persistence.

Third, broad cleanup is still the wrong move. This file feeds run history, hydration, replay, tagging, meta reuse, policy-eval replay, and live-session inspection. Any recommendation that renames families, rewrites the stage4/stage5 compatibility lane, or replaces the JSON compatibility posture wholesale would be too risky for a safe-now pass.

### 1. The overwrite-path issue is stronger than the third-pass packet described

The third-pass packet correctly called out that `existing_path` is treated as best-effort instead of as a strict overwrite contract, but the live source shows a more specific problem than “silent fallback.” The overwrite helper canonicalizes only the parent directory (`src-tauri/src/run_artifacts.rs:539-558`), while the read/delete/tag-update path checker canonicalizes the full file path (`src-tauri/src/run_artifacts.rs:561-574`). That means overwrite validation is materially weaker than the rest of the file’s path-safety posture.

That difference matters because the overwrite path is not just ambiguous; it is also easier to misuse. If `existing_path` points at an existing path inside the runs directory whose parent is valid but whose file target is not the intended run artifact, the helper still accepts it. Because the file path itself is not canonicalized here, an existing symlinked file path is also validated more weakly than the read/delete/update surfaces validate it. The current code likewise does not verify that the overwrite target is actually a run-artifact file name, a run-artifact kind, or even an existing JSON file. In practice, that means the public `save_run_artifact` command can overwrite the wrong artifact family or the wrong existing path inside the runs directory while still passing the current parent-only check.

This is a more important safe-now target than the third-pass packet gave it credit for because it is an IPC-facing contract bug. The command is publicly exposed through the Tauri handler registry in `src-tauri/src/lib.rs:150-170`, so this is not just an internal helper mismatch.

### 2. Save-time enrichment coverage is overstated and narrower than the file’s own compatibility model

The first-pass packet said save-time enrichment is stronger than read-time normalization. The live source is less generous than that summary suggests.

`save_run_artifact_blocking` only starts dataset normalization and source-identity backfill from `/data/csv_path` (`src-tauri/src/run_artifacts.rs:1055-1108`). But elsewhere in the same file, CSV resolution intentionally accepts `/dataset_meta/csv_path` and validation-context CSV paths (`src-tauri/src/run_artifacts.rs:587-600`). The file’s own minimal test artifact also uses `dataset_meta.csv_path` without `data.csv_path` (`src-tauri/src/run_artifacts.rs:2942-2959`). So the current save path does not actually cover one of the artifact shapes that the same module already treats as valid.

That means the third-pass packet slightly overstates the current save-time hardening. For compatibility-shaped artifacts, schema/version/resample/source-identity backfill can still be skipped entirely even before the read-time asymmetry begins.

There is a second asymmetry here as well: save-time backfill writes `config_fingerprint_version`, but it does not compute a missing `config_fingerprint` (`src-tauri/src/run_artifacts.rs:1034-1052`). That can produce a “version present, fingerprint empty” state which is hard to justify as a coherent persisted contract.

### 3. Latest-run selection is weaker than the current packet acknowledged

The third-pass packet focused on scan omission and write durability, but `load_latest_run_artifact_entry_blocking` still has a correctness problem even when every file is readable.

The helper sorts candidate paths lexicographically by full path and returns the first supported run artifact (`src-tauri/src/run_artifacts.rs:1324-1375`). That is only safe if the filename prefix is constant and the timestamp segment is the only ordering signal. This file explicitly accepts both legacy and current run prefixes (`vectorbt_run_` and `vectorgrid_run_`), so full-path order is not equivalent to “newest timestamp.” A newer legacy artifact can lose to an older current-prefix artifact purely because the prefix sorts later. The fallback full-scan path repeats the same weakness for nonstandard file names by iterating reversed path order instead of comparing embedded artifact timestamps.

There is also a smaller same-timestamp edge case that the current tests do not cover: once suffixes reach two digits, simple lexicographic order mis-ranks `_10` relative to `_9`. The filename-order assumption also appears in the prefix-based retention helper (`src-tauri/src/run_artifacts.rs:1191-1196`).

This is a good example of something the third-pass packet underweighted: the compatibility posture in this file is broader than the fast-path ordering logic that is currently used to find “latest.”

### 4. The latest-hydration response can disagree with itself for the same artifact

The current third-pass packet correctly noted read-time normalization lag, but the live source shows a sharper version of the problem: `LatestRunArtifactHydrationResponse` can be internally inconsistent.

`load_latest_run_artifact_hydration_blocking` builds a summary and a hydration payload from the same raw `Value` (`src-tauri/src/run_artifacts.rs:2451-2466`). The summary path computes a missing `config_fingerprint` and infers `config_fingerprint_version` when needed (`src-tauri/src/run_artifacts.rs:2041-2057`). The hydration and detail builders do not; they return the persisted fingerprint fields as-is (`src-tauri/src/run_artifacts.rs:1761-1770`, `1808-1817`). Likewise, summary preserves an empty `ts` string, while hydration/detail invent `Utc::now()` when `ts` is missing (`src-tauri/src/run_artifacts.rs:1751-1757`, `1820-1823`, `2033-2040`).

So the same IPC response can legitimately say two different things about one artifact depending on whether the frontend looks at `summary` or `hydration.current_run`. That is a stronger and more concrete contract weakness than the third-pass packet captured.

### 5. The save boundary is too permissive for a public command surface

`save_run_artifact_blocking` accepts any `serde_json::Value`. If the root is not an object, it simply skips enrichment and still writes the payload under a run-artifact filename (`src-tauri/src/run_artifacts.rs:1001-1138`). It also never verifies that the artifact kind is actually a supported run-artifact kind before persisting.

That is notably looser than `create_live_session_artifact_blocking`, which sanitizes and stamps a concrete kind and timestamp on create (`src-tauri/src/run_artifacts.rs:789-825`, `2469-2479`). For a public Tauri command, that difference matters. Wrong-kind or malformed payloads can create files in the runs directory that later become invisible to summary builders, misclassified by tooling, or harder to reason about during latest-run selection and retention.

The third-pass packet did not call this out, and it should have. Contract validation at the save boundary is a cheaper and clearer hardening step than broad persistence-mechanics work.

### 6. The durability recommendation is still right in spirit, but too strong in wording and too early in priority

The third-pass packet’s main recommendation—“move canonical artifact rewrites to atomic replace semantics”—is directionally right, but it is too strong as written and slightly over-prioritized compared with the logic bugs above.

The Rust docs support a narrower statement. `std::fs::write` is just a convenience whole-file write API. `std::fs::rename` replacement behavior differs by platform and filesystem. `File::sync_all` is explicitly more expensive. So the safe-now recommendation should be a centralized staged-write helper with honest semantics, not a claim that the file can become crash-safe through a simple helper swap.

That wording matters even more because this file also rewrites live-session evidence during append/finalize paths (`src-tauri/src/run_artifacts.rs:844-860`, `2483-2557`). A blanket “make every write durable” implementation that adds synchronous flushes to each mutation would risk live-execution responsiveness. The third-pass packet was too history-centric here; if write semantics are tightened, the file needs a deliberate policy split between low-frequency canonical history writes and higher-frequency live-session mutations.

## Implementation Priority

### Safe-now hardening

1. Tighten `existing_path` into a true overwrite contract.

   Make `existing_path` strict instead of best-effort. Require the target file to exist, canonicalize the full path instead of just the parent, keep the path inside the runs directory, and reject overwrite targets that are not supported run artifacts. This is the highest-value fix because it closes both the silent-fallback problem and the weaker overwrite-path validation gap without changing artifact schema.

2. Add one shared read-normalization helper for run artifacts.

   Summary, detail, and hydration should not normalize the same artifact differently. A single helper should at least unify `config_fingerprint`, `config_fingerprint_version`, stable timestamp policy, and cheap dataset-meta normalization that can be derived from already-persisted fields. This is the safest way to remove the current summary-versus-hydration mismatch.

3. Drive save-time dataset backfill from the same CSV-path compatibility rules used elsewhere in the file.

   Do not rely only on `/data/csv_path`. Use the file’s existing compatibility resolution posture so artifacts that only carry `dataset_meta.csv_path` or validation-context metadata are not left half-normalized on save.

4. Fix latest-run selection to compare artifact time semantics, not whole file paths.

   The standard-prefix fast path and the nonstandard fallback should both select by parsed artifact timestamp with a deterministic tiebreaker for suffixes, not by full path order. Keep the fix scoped to latest-run helpers first; do not reorder all library listing APIs unless a real consumer needs it.

5. Validate the save boundary.

   Require an object root and supported run-artifact kind in `save_run_artifact_blocking`. If missing `ts` still needs compatibility support, stamp it once at save time rather than inventing a new value at read time.

6. After the semantic fixes above, centralize write mechanics behind a staged-write helper.

   This is still worthwhile, but it should land as best-effort replace hardening with precise wording, not as a blanket “atomic persistence” claim. If live-session mutation paths use the helper too, keep the implementation lightweight and measured.

### Larger redesign still deferred

1. Replacing raw `serde_json::Value` traffic with typed artifact enums/structs across all families.

   That would improve correctness long-term, but it is broader than this pass should carry because it touches compatibility, replay, and frontend IPC assumptions simultaneously.

2. Rich corruption diagnostics in library listings.

   The silent-skip behavior is still a real diagnostics gap, but surfacing structured scan-health data across history and library APIs is a separate product-contract decision.

3. Stage4/stage5 naming unification.

   The compatibility alias lane remains awkward, but this pass did not find evidence that renaming it now would be safer than leaving it alone. Keep it deferred until frontend and saved-artifact migration can move together.

4. Concurrency-safe filename reservation.

   `suggest_ts_file_path_with_limit` still relies on `exists()` checks before write, so true concurrent same-timestamp saves could race. That is real technical debt, but it is a different change from the current overwrite and latest-order correctness fixes.

5. Uniform retention policy across batch, campaign, live-session, and policy-eval families.

   The retention story remains uneven across artifact families, but changing that now is a policy decision, not a small hardening step.

## Verification If Changed

At minimum, keep the current focused run-artifact suite and add a few missing edge-case tests.

- `cargo test -p ta_desktop_demo_app run_artifacts -- --nocapture`
- `cargo test -p ta_desktop_demo_app save_run_artifact_blocking_overwrites_existing_path_when_provided -- --nocapture`
- `cargo test -p ta_desktop_demo_app load_latest_run_artifact_summary_prefers_latest_run_file_without_scanning_meta_history -- --nocapture`
- `cargo test -p ta_desktop_demo_app load_latest_run_artifact_hydration_prefers_latest_run_file_without_scanning_meta_history -- --nocapture`

Add targeted tests for the gaps the third-pass packet still missed:

- `existing_path` rejects missing paths, foreign artifact-family targets, and overwrite targets that are only parent-valid but not file-valid; add a symlink-target case where the platform/test environment allows it.
- save-time dataset-meta/source-identity backfill works when CSV path comes from `dataset_meta` or validation-context compatibility shapes rather than only `data.csv_path`.
- `LatestRunArtifactHydrationResponse` does not disagree internally on `ts`, `config_fingerprint`, or `config_fingerprint_version` for legacy/malformed artifacts.
- mixed `vectorbt_run_*` and `vectorgrid_run_*` libraries pick the true newest artifact by embedded timestamp.
- nonstandard-filename fallback also picks newest by embedded artifact timestamp.
- same-timestamp suffix ordering remains correct once suffixes exceed one digit.
- if write mechanics change, cover live-session append/finalize paths as well as run save/tag update.

Because these commands back desktop history hydration and report-backfill flows, frontend regression coverage is still warranted after any contract tightening:

- `npm --prefix ./web run test -- src/services/workflowArtifactsSync.test.ts src/services/historyService.test.ts src/services/validationService.test.ts`

## Regression And Feature-Preservation Constraints

- Preserve backward compatibility for already-saved legacy artifacts and accepted legacy filename prefixes. The latest-run fix should improve compatibility, not narrow it.
- Preserve the current JSON field names and stage4/stage5 compatibility lane in safe-now changes. This file is wired into history hydration, meta replay, saved-stage views, and validation/report backfill.
- Keep path sandboxing at least as strong as the read/delete/update helpers. The overwrite-path fix should move toward the stricter path model already present elsewhere in the file, not invent a looser one.
- Do not move full source hashing or content-fingerprint recomputation into hot library-list paths without measurement. Cheap structural normalization is safe; content hashing on every scan is not.
- Do not add `sync_all` or similarly heavy durability work to every live-session append/finalize write without measuring live-execution overhead first. This file has no direct GPU ABI risk, but it does sit on the live-execution evidence path, so write-latency regressions matter.
- If `save_run_artifact_blocking` begins rejecting malformed payloads, return explicit contract errors. This is an IPC-surface change, so silent behavior changes would be harder for the frontend to diagnose.
- Keep latest-run fixes narrowly scoped. List ordering changes can affect UI expectations; the clear bug here is selection correctness, not every sort order in the library.
- Validation and meta workflows depend on artifact continuity. Any normalization helper that mutates interpreted values must preserve the same replay-visible meaning of saved requests and results.

## Fourth-Pass Conclusion

`Bucket unchanged, but the implementation priority should change.`

The current third-pass packet was right to keep `src-tauri/src/run_artifacts.rs` in `Small surgical refactor`, and it was right to keep durability in view. But it over-centered generic write mechanics and underweighted cheaper, clearer correctness fixes that are already visible in the live source.

The highest-value safe-now work is not a generic “atomic write” pass. It is: tighten overwrite targeting, unify read normalization so one response cannot disagree with itself, drive save-time dataset backfill from the file’s own compatibility rules, and fix latest-run selection so accepted legacy artifacts are ordered by timestamp rather than by filename prefix. After that, a centralized staged-write helper is still worthwhile—but it should be described honestly as best-effort replace hardening, not as full crash-safe persistence.

So the bucket stands, but the third-pass packet’s priority order should not. The live file still wants a small, conservative hardening pass; it just wants a more semantic one before a more mechanical one.
