# Fourth-Pass Review: `src-tauri/src/commands_options_expression.rs`

- Review date: `2026-04-15`
- Source file: `src-tauri/src/commands_options_expression.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_options_expression.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_options_expression.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_options_expression.rs.review.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Larger refactor but defer`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file needed a fourth pass because the third-pass packet was directionally correct about the overall bucket, but its “pull these two fixes forward now” recommendation still mixed together one genuinely bounded hardening item with one change that is much more likely to regress legitimate product behavior.

The live source still concentrates too many contract-sensitive responsibilities in one place: persisted replay caches, artifact creation and listing, saved-artifact inspection, CSV export, replay reconstruction from parent run artifacts, and the public Tauri command surface itself (`src-tauri/src/commands_options_expression.rs:1069-1095`, `1199-1383`, `1427-1709`, `1826-2144`). That concentration means the fourth pass needed to answer two narrower questions:

1. Does the third-pass defer bucket still stand?
2. If yes, which safe-now hardening steps are actually the right ones to pull forward?

The answer is that the defer bucket should stand, but the immediate priorities should change.

## Fourth-Pass Research Log

This pass re-read the live file first and then checked only the surrounding contracts needed to pressure-test the third-pass recommendations.

1. **Live command file re-read**
   - Focused on artifact summary/list/load/export/inspection and replay-cache code:
     - `resolve_options_parent_strategy_kind_from_saved_artifact` (`src-tauri/src/commands_options_expression.rs:453-471`)
     - directory/path helpers (`src-tauri/src/commands_options_expression.rs:652-789`)
     - persisted signal-manifest cache (`src-tauri/src/commands_options_expression.rs:791-843`)
     - artifact summary generation (`src-tauri/src/commands_options_expression.rs:1199-1383`)
     - replay reconstruction and provenance checks (`src-tauri/src/commands_options_expression.rs:1394-1709`)
     - list/load/export/inspect command entrypoints (`src-tauri/src/commands_options_expression.rs:1826-2144`)

2. **App-core inspection and feasibility re-read**
   - Re-read `inspect_options_expression_tuple_blocking` and `inspect_options_expression_event_path_blocking` in `crates/app_core/src/options_expression.rs` to confirm that inspection still rebuilds the current prepared run, re-ranks tuples live, and selects by `tuple_index` (`crates/app_core/src/options_expression.rs:5630-5763`).
   - Re-read `analyze_options_expression_feasibility_blocking` to confirm that feasibility preview is not a lightweight metadata call; it performs full prepare work, CPU ranking, and optional GPU planning / GPU ranking probes (`crates/app_core/src/options_expression.rs:5261-5345`).

3. **Artifact contract re-read**
   - Re-read `OptionsExpressionArtifactV1` in `crates/app_core/src/options_types.rs:1101-1125`.
   - The artifact stores `parent_artifact_id`, `options_dataset_hash`, `price_semantics`, the request, diagnostics, and ranked tuple results.
   - It does **not** store a parent-artifact content fingerprint, a signal-manifest fingerprint, or a stable tuple identifier beyond tuple ordering inside `selection_metrics`.

4. **Run-guard contract re-read**
   - Re-read `AppState::try_begin_run` in `src-tauri/src/state.rs:559-567`.
   - Compared that with the wrapper layer:
     - `run_options_expression` acquires the run guard (`src-tauri/src/commands_options_expression.rs:1933-1947`)
     - `analyze_options_expression_feasibility` does not (`src-tauri/src/commands_options_expression.rs:1949-1962`)

5. **External standards check**
   - Re-checked current Tauri v2 docs for runtime authority, permissions, and command scopes.
   - Re-checked Rust std docs for `Path::is_file` and `canonicalize`.
   - Re-checked current bincode docs.
   - Those sources matter here because they confirm:
     - Tauri command access and command-specific scopes are explicit API/security contracts, not incidental implementation details.
     - `Path::is_file` follows symlinks, so directory enumeration by itself is not a containment boundary.
     - `canonicalize` resolves symlinks and requires the target path to exist, which matters when evaluating whether export-path “scoping” is actually a tiny patch or a larger contract change.
     - bincode is now explicitly unmaintained, which turns any persisted-format cleanup into deliberate debt management rather than casual churn.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet got three important things right.

First, the overall bucket should still remain **`Larger refactor but defer`**. The file is still too contract-heavy for “one more small cleanup” to solve its deeper problems cleanly.

Second, it was correct that artifact listing has a real filesystem-boundary weakness around re-opening `artifact.request.parent_artifact_path` during summary generation (`src-tauri/src/commands_options_expression.rs:453-471`, `1199-1205`, `1826-1860`).

Third, it was correct that saved-artifact inspection is still too coupled to live replay behavior rather than a self-contained artifact identity. The app-core inspection path still rebuilds the current prepared run and re-selects tuples by rank position (`crates/app_core/src/options_expression.rs:5630-5763`).

### What the current third-pass packet missed or underweighted

#### 1. Artifact listing has a **first-hop** containment gap before the parent-artifact reopen ever happens

The third-pass packet correctly focused on `parent_artifact_path`, but it missed an earlier out-of-root read edge.

`list_options_expression_artifacts_blocking_headless` enumerates `read_dir(runs_dir)` and reads any matching file immediately after `path.is_file()` succeeds (`src-tauri/src/commands_options_expression.rs:1826-1860`). Unlike the load and inspect paths, it never calls `ensure_path_is_within_dir(&path, &runs_dir)` on the artifact entry itself before reading it.

That matters because `Path::is_file` follows symlinks. A symlink placed inside `runs/` can therefore point outside the allowed tree and still pass the current check. In other words, the listing path is weaker than the explicit load / inspect paths even before summary generation reopens the parent artifact.

This is a **bounded safe-now hardening item**. It is smaller and better supported than export-path scoping.

#### 2. The final replay-drift check in `inspect_options_expression_event_path` is currently ineffective

The tuple inspection path does a meaningful post-replay check:

- it clones the saved tuple at `req.tuple_index`
- recomputes the tuple inspection
- and rejects if the recomputed tuple does not match the saved tuple
  (`src-tauri/src/commands_options_expression.rs:2029-2059`)

The event-path inspection path does not. It clones the saved tuple into `expected_tuple`, but the final check compares that saved tuple back to the saved tuple again:

- `expected_tuple` is derived from `artifact.selection_metrics.get(req.tuple_index)?.tuple.clone()` (`src-tauri/src/commands_options_expression.rs:2097-2113`)
- the final guard is `if artifact.selection_metrics[req.tuple_index].tuple != expected_tuple` (`src-tauri/src/commands_options_expression.rs:2135-2139`)

That condition is tautological in the current flow. It does not validate the recomputed replay result at all.

This is a real missed defect in the current packet. It also makes the inspection-contract problem more immediate than the third-pass write-up suggested, because one of the two inspection commands already lost its intended replay-drift guard.

#### 3. The deeper inspection problem is not just tuple-rank coupling; it is missing parent-content provenance

The third-pass packet was right to call out tuple-index coupling, but that is not the whole problem.

`validate_saved_artifact_replay_provenance` only checks:

- `parent_artifact_id`
- normalized `options_dataset_id`
- `options_dataset_hash`
- `price_semantics`
  (`src-tauri/src/commands_options_expression.rs:1394-1424`)

But `parent_artifact_id` is derived from the parent artifact **path file stem**, not a content fingerprint (`src-tauri/src/commands_options_expression.rs:1481-1486`). And `OptionsExpressionArtifactV1` stores no parent-artifact content hash or signal-manifest fingerprint (`crates/app_core/src/options_types.rs:1101-1125`).

So if a parent run artifact changes in place while keeping the same filename, replay provenance can still pass. Stable tuple identity by itself would not fully solve that. The missing contract is stronger: historical inspection is not bound to the exact parent-content provenance that originally produced the saved artifact.

That is the strongest reason the defer bucket should stand.

#### 4. The signal-manifest cache problem is larger than “corruption currently hard-fails”

The earlier packets were right that corrupt signal-manifest cache files are handled too strictly (`src-tauri/src/commands_options_expression.rs:791-817`). But the fourth pass sharpens the problem further:

- dataset replay cache has an explicit versioned wrapper (`PersistedOptionsDatasetReplayCacheV1`) and can fall back from corrupt binary cache to legacy JSON (`src-tauri/src/commands_options_expression.rs:845-948`)
- signal-manifest replay cache is persisted as raw pretty-printed JSON with no cache wrapper, no version envelope, and no tolerant deserialize path (`src-tauri/src/commands_options_expression.rs:791-843`)

So the inconsistency is not just one harsh error path. The two persisted caches already follow different format, migration, and failure-policy rules. That strengthens the case for a deferred cache-policy unification pass, even though the immediate “corrupt signal cache should degrade to cache miss” fix is still safe-now.

#### 5. Feasibility preview is unguarded even though it is heavyweight and can hit CPU and GPU paths

The wrapper inconsistency is real:

- `run_options_expression` acquires the app’s single-run guard (`src-tauri/src/commands_options_expression.rs:1933-1947`)
- `analyze_options_expression_feasibility` does not (`src-tauri/src/commands_options_expression.rs:1949-1962`)

That would be less notable if feasibility preview were lightweight, but app-core feasibility preview performs full prepare work, CPU ranking, and optional GPU planner / GPU ranking probes (`crates/app_core/src/options_expression.rs:5261-5345`).

This is a genuine performance and cancellation-contract question that the current packet did not surface. I do **not** think the answer is “just add `try_begin_run()` immediately,” because that changes UX and concurrency semantics. But it is real enough to belong in the deferred-design bucket or at least in measurement-first hardening.

### What the current third-pass packet overstated or framed too aggressively

#### 1. `save_options_csv` path scoping is not well-supported as a safe-now change

The third-pass packet is too aggressive in elevating `save_options_csv` path scoping into the small-now set.

Yes, `save_options_csv` is a frontend-reachable write primitive (`src-tauri/src/commands_options_expression.rs:1896-1931`, `1990-1997`). But the broader Tauri surface in this same app also exposes native file-picker flows such as `pick_save_csv` (`src-tauri/src/lib.rs:117-119`) alongside `save_options_csv` itself (`src-tauri/src/lib.rs:171-177`). That is a strong local signal that user-chosen export destinations are part of current product behavior, not an accidental side channel.

The file also does not currently consume a Tauri command scope object or any other explicit path-scope contract. Tauri supports command-level permissions and scopes, but that means real export hardening is a **contract design** question, not a one-line “force it under `runs/`” patch.

So the safe-now recommendation should **not** be “scope CSV writes into `runs/` or app-local data.” That would likely regress legitimate export UX while still failing to express the real intended policy.

#### 2. “Decouple inspection from current tuple rank” is directionally right, but incomplete on its own

The third-pass packet was right about rank coupling, but the fourth pass shows that tuple identity alone is not enough. The file also needs parent-content provenance strong enough to bind inspection to the original saved parent run, not merely to the parent artifact’s filename and the imported options dataset hash.

That means “stable tuple hash” belongs in the deferred design discussion, but it should not be described as the full fix by itself.

## Implementation Priority

### Safe-now hardening

1. **Fix the artifact summary maxima bug**
   - `best_robust_score` and `best_selection_expectancy_per_premium` still use `fold(0.0_f64, f64::max)` (`src-tauri/src/commands_options_expression.rs:1206-1215`).
   - That can report impossible `0.0` maxima for all-negative result sets.
   - This remains the clearest correctness bug in the file and is smaller than the third-pass export-path proposal.

2. **Make corrupt signal-manifest cache loads degrade to cache miss**
   - `load_signal_manifest_replay_cache` should not fail the workflow for a corrupt persisted accelerator (`src-tauri/src/commands_options_expression.rs:791-817`).
   - Treat deserialize/read failure as `Ok(None)`, and optionally remove the bad cache file.

3. **Contain artifact listing before the first read**
   - In `list_options_expression_artifacts_blocking_headless`, validate each enumerated artifact entry against `runs_dir` before calling `read_artifact_value` (`src-tauri/src/commands_options_expression.rs:1826-1860`).
   - That closes the symlink-following gap that the third-pass packet missed.

4. **If parent strategy kind is still resolved by reopening the parent artifact during listing, apply the same boundary discipline there**
   - Either pass `runs_dir` into summary generation and refuse out-of-scope parent paths, or return an empty `parent_strategy_kind` for out-of-scope parents rather than reading them (`src-tauri/src/commands_options_expression.rs:453-471`, `1199-1205`).

5. **Replace the dead event-path replay-drift guard with a real comparison**
   - The current event-path post-check does not validate recomputed replay state (`src-tauri/src/commands_options_expression.rs:2097-2139`).
   - The bounded fix is to compare against an actual recomputed tuple selection, not the saved tuple that was already cloned from the artifact.

6. **Add explicit tests for the above**
   - all-negative maxima
   - corrupt signal-manifest cache fallback
   - symlinked/out-of-root artifact listing rejection
   - event-path inspection drift check actually exercising recomputed state

### Larger redesign still deferred

1. **Strengthen historical inspection provenance**
   - The artifact needs a parent-content-level provenance field (or equivalent signal-manifest fingerprint), not just `parent_artifact_id`.
   - Stable tuple identity should be considered together with parent-content provenance, not as a substitute for it.

2. **Unify persisted replay-cache policy**
   - Signal-manifest and dataset caches should eventually share the same design posture:
     - explicit version envelope
     - tolerant load behavior
     - migration/legacy policy
     - temp-file + rename write discipline where appropriate

3. **Revisit feasibility preview concurrency/cancellation**
   - Preview is heavyweight enough that its current unguarded concurrency model deserves an explicit decision.
   - That should be handled with product intent in mind, not by silently bolting the existing optimization run guard onto preview.

4. **Revisit export-path policy only through an explicit contract**
   - If the product eventually wants tighter export control, do it as a real command-scope / picker-token / export-policy contract.
   - Do not hard-code `save_options_csv` into `runs/` or app-local data as an unreviewed “hardening” step.

## Verification If Changed

If any of the safe-now changes are implemented, the minimum verification set should be:

1. `cargo test -p ta_desktop_demo_app commands_options_expression -- --nocapture`

2. Add targeted unit coverage for:
   - all-negative `best_robust_score`
   - all-negative `best_selection_expectancy_per_premium`
   - corrupt signal-manifest replay cache falling back to miss/rebuild
   - listing rejecting or skipping a symlinked matching artifact that resolves outside `runs/`
   - event-path inspection failing when the recomputed tuple no longer matches the saved artifact tuple identity

3. If parent-path containment logic changes during listing:
   - add a test where `artifact.request.parent_artifact_path` points outside the allowed run-artifact tree
   - verify listing remains tolerant and deterministic rather than aborting the whole library read

4. If preview/run concurrency semantics change:
   - verify frontend/store behavior for “preview while run active”
   - verify cancellation semantics and error strings remain coherent

5. Re-run frontend consumers that rely on the current DTOs:
   - `web/src/services/optionsExpressionService.test.ts`
   - `web/src/stores/optionsExpressionStore.test.ts`
   - `web/src/components/OptionsExpressionPanel.test.ts`

## Regression And Feature-Preservation Constraints

Any changes in this file should preserve the following unless there is an explicit cross-file product decision.

- Preserve `vectorgrid_options_expression_v1` artifact-kind enforcement and existing artifact-library compatibility for already-saved artifacts.
- Preserve the current legitimate export UX. Do not silently narrow `save_options_csv` to `runs/`, app-local data, or another fixed root unless the product explicitly wants that change and the frontend flow is updated in lockstep.
- Keep replay caches as accelerators only. Corrupt or stale cache data should not become authoritative.
- Do not present a tuple-hash or tuple-identity change as sufficient by itself for historical inspection correctness; parent-content provenance is part of the same problem.
- If you harden listing against malformed artifacts, keep tolerant library enumeration. One bad artifact should not blank the whole list.
- If you add stronger inspection validation, be careful not to break historical inspection solely because of benign formatting-only or verdict-refresh-only changes. The target is replay provenance and tuple identity, not gratuitous brittleness.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: this file still belongs in **`Larger refactor but defer`**.

However, the third-pass packet’s immediate-priority list should change.

What it got right:
- the file is still too contract-heavy to reclassify as a purely small-cleanup target
- parent-artifact reopening during listing is a real weak edge
- saved-artifact inspection is still too dependent on current replay behavior

What it missed or underweighted:
- artifact listing has a first-hop containment gap on the artifact entry itself
- `inspect_options_expression_event_path` currently ends with an ineffective tuple-drift guard
- the deeper inspection problem is missing parent-content provenance, not only tuple-rank coupling
- signal-manifest cache design is weaker than the dataset cache in both failure policy and format/versioning
- feasibility preview is heavy enough that its unguarded concurrency is a real runtime-contract question

What it overstated:
- `save_options_csv` path scoping is not well-supported as a safe-now hardening step and is more likely to regress legitimate export behavior than to solve the real policy problem

So the best fourth-pass disposition is:

- **Keep the defer bucket**
- **Pull forward a different safe-now set**
  - summary maxima correctness
  - signal-manifest cache miss-tolerance
  - artifact-list containment on the artifact entry itself
  - a real replay-drift check for event-path inspection
- **Leave export-path redesign, parent-content provenance, and cache-policy unification in the deferred bucket**

That is the most conservative, evidence-backed reading of the live file.
