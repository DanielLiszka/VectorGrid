# Fourth-Pass Review: `src-tauri/src/commands_data.rs`

- Review date: `2026-04-15`
- Source file: `src-tauri/src/commands_data.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_data.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_data.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_data.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file remained worth a fourth pass because the third-pass packet narrowed too aggressively onto market-data-root migration aliasing and, in doing so, underweighted several more concrete contract defects that are already visible in the live source.

The current third-pass bucket should **stand**. This still does not look like a redesign file. It is a public Tauri/runtime boundary with a few sharp correctness and robustness problems, but those problems are still local enough to fix surgically without renaming commands, changing IPC payloads, or splitting the module.

The third-pass packet also got several things right:

- it kept the action bucket conservative instead of inflating this into a large refactor;
- it treated command names, event names, and payload shapes as compatibility-sensitive surfaces;
- it correctly recognized that market-data-root movement is path-sensitive and should not be “cleaned up” casually;
- it correctly left background Polygon cancellation semantics in the cooperative-cancel bucket instead of pretending `spawn_blocking` abort is available.

Where the third pass fell short was prioritization. The live file still contains smaller, clearer defects with tighter evidence than the proposed canonicalization-first patch.

## Fourth-Pass Research Log

I re-checked the live file against current primary documentation where the packet was making standards-sensitive claims.

Official references consulted:

- **Tauri v2 — “Calling Rust from the Frontend”**: async commands are preferred for heavy work; non-async commands run on the main thread; borrowed types such as `State<'_, T>` need workaround patterns in async commands.
- **Tokio — `spawn_blocking`**: blocking tasks cannot be aborted once they have started running.
- **Rust std — `Path::starts_with`**: prefix matching is component-based, not raw substring matching.
- **Rust std — `fs::canonicalize`**: canonicalization resolves symlinks and requires the path to exist.
- **Cargo Book — environment variables**: `CARGO_MANIFEST_DIR` is the manifest directory of the package being built.
- **Current Massive/Polygon stock minute-aggregate WebSocket docs**: `AM` minute aggregate payloads explicitly include `sym`, `o`, `h`, `l`, `c`, `s`, and `e` response attributes.
- **Microsoft Win32 docs — `SHBrowseForFolderW` / `CoInitializeEx`**: the current folder dialog surface is old but still intentional enough that I would not churn it in this pass.

That research changed the priority call in two ways.

First, the third-pass canonicalization idea is directionally reasonable, but it is not the obvious first patch the packet makes it sound like. `Path::starts_with` already matches only on whole path components, so the remaining bug class is aliasing/junction/case-variant identity, not a plain substring-prefix bug. And `canonicalize` is not a free drop-in because it requires the compared path to exist and resolves symlinks, which makes first-run and empty-destination behavior trickier than the packet suggests.

Second, the official Polygon schema makes the current live-bar normalization more permissive than it needs to be. The backend is already subscribed to a single channel and the docs still say `sym`, `s`, and `e` are part of the event payload, so silently fabricating `sym` from the requested symbol is weaker validation than the upstream contract requires.

## What Fourth Pass Sharpened

### 1. The third-pass “canonicalized move guard is the sharpest bug” claim is overstated

The risk category is real, but the priority is off.

At `src-tauri/src/commands_data.rs:508-527`, the move guard compares `old_root` and `new_root` lexically and uses `starts_with` on `old_cache` / `new_cache`. The third-pass packet treated this as if the current code were doing a raw-string prefix check. It is not. Rust’s `Path::starts_with` already matches only whole path components, so the packet overstates the weakness of the existing guard.

What remains is the narrower class of alias-path problems: symlinks/junctions, filesystem case folding, and equivalent-but-not-identical path spellings. That is worth hardening eventually, but it is not the cleanest immediate patch because `canonicalize` only works on existing paths and resolves symlinks. Here, `new_root` exists because the function creates it, but `new_cache` may not exist yet, and blindly canonicalizing the wrong level can distort first-run or empty-destination behavior.

My conclusion: the third-pass packet was right to flag the risk, but wrong to elevate it above more concrete bugs already present in this file.

### 2. There is a real silent live-stream failure path the third-pass packet missed

`run_polygon_live_candle_stream` has one loop exit that emits no status event and does not disarm the live runner.

At `src-tauri/src/commands_data.rs:955-958`, a failed `Pong` send on an inbound ping just `break`s out of the loop. Unlike the `Close`, read-error, and end-of-stream paths, that branch does **not**:

- emit `polygon_live_status`, and
- call `disarm_runner_after_nonmanual_feed_interruption(...)`.

That means a transport failure can terminate the task without producing the same interruption semantics as the other non-manual failure paths. `AppState::live_candle_stream_info()` will eventually notice the finished task lazily, but that requires a later status poll and does not replace the missing event emission or immediate runner-disarm path.

This is a sharper defect than the canonicalization item because it is a direct correctness gap in a current runtime branch, not a narrower path-alias edge case.

### 3. Live-bar normalization is too trusting for a public runtime boundary

`normalize_polygon_live_bar` at `src-tauri/src/commands_data.rs:719-754` does two things that are weaker than the surrounding codebase and weaker than the current upstream schema:

- it accepts a missing `sym` by silently substituting the requested symbol (`unwrap_or(symbol)` at line 730), even though the upstream minute-aggregate contract still defines `sym` as a response attribute; and
- it forwards OHLC/timestamp values without validating basic invariants before emitting the event and handing the bar to the live-execution runner.

This matters because `AppState::upsert_live_bar` already enforces the minimum bar contract elsewhere in the runtime: finite OHLC, `high >= low`, and positive timestamps (`src-tauri/src/state.rs:442-451`). The live-stream boundary should not be looser than the state-mutation boundary when both are consuming the same domain object class.

This is safe-now hardening, not redesign. The smallest defensible change is:

- require `sym` to be present in the vendor payload;
- require finite `open/high/low/close`;
- require `start > 0` and `end >= start`.

That preserves the current event names and payload field names while removing unnecessary permissiveness.

### 4. The writability probe in `set_market_data_dir_for_base` is destructive

The third-pass packet missed the most concrete filesystem-safety bug in the market-data-root code.

At `src-tauri/src/commands_data.rs:503-506`, writability is tested by writing a fixed filename:

- probe path: `.vta_write_probe`
- write mechanism: `std::fs::write`
- cleanup: best-effort `remove_file`

That will truncate an existing file with the same name if one is already present. The filename is obscure, but it is still user-controlled filesystem state, and this function is explicitly mutating arbitrary user-selected directories.

This should be hardened before canonicalization work. The safe patch is small:

- create a unique probe filename under the chosen root, or
- use a create-new/open-options probe so the check cannot clobber an existing file.

This is a much stronger “safe-now” recommendation than the third-pass canonicalization proposal because it avoids real data loss potential without changing any public contract.

### 5. The stale sample-data lookup remains more concrete than the third pass admitted

The sample-data bug is still live and is still easier to justify than the canonicalization-first patch.

`list_sample_csvs` still builds its root as `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")` at `src-tauri/src/commands_data.rs:263-266`, and `sample_csvs_from_root` still looks under `src/data/...` at `src-tauri/src/commands_data.rs:238-259`.

That has two problems:

- the relative walk goes above the workspace root when `CARGO_MANIFEST_DIR` is the `src-tauri` package directory; and
- the helper still assumes `src/data`, while the rest of the file’s repo-data discovery is built around a top-level `data/` directory.

The third-pass packet underweighted this by moving the spotlight away from it. I would not. This is a clearer defect with a smaller blast radius.

The safer fix is also narrower than the earlier review packets implied: do **not** invent another root-guessing rule. Reuse the existing repo/workspace discovery helper family and resolve curated samples from the actual repo `data/` surface.

### 6. Two prior recommendations need to be narrowed to avoid regressions

The third-pass packet inherited some advice from earlier passes that is too strong as written.

#### 6a. “Trim `csv_path` in validate/repair wrappers” is not clearly safe

I would **not** recommend automatically trimming filesystem paths in `polygon_validate_dataset` and `polygon_repair_dataset`.

Yes, `polygon_top_off_dataset` trims `req.csv_path` today. But broadening that policy at the backend boundary is still risky because exact path strings are the safer cross-platform contract. On Unix-like systems, leading and trailing spaces are legal filename characters. A backend-side trim can therefore turn a valid path into a different path.

This is precisely the kind of “helpful normalization” that causes hard-to-diagnose regressions in public command surfaces. If the frontend wants to trim user text input before calling a command, that is a frontend policy choice. The backend path contract should stay exact unless the repo explicitly declares otherwise.

#### 6b. “Convert `load_price_data` / `resample_price_data` to async `spawn_blocking` wrappers” is valid, but narrower than the packet suggests

The concern is real. Tauri’s current docs still say heavy work should prefer async commands, and non-async commands still run on the main thread. So the direct command wrappers at `src-tauri/src/commands_data.rs:20-37` are still worth revisiting.

But the packet makes the change sound more straightforward and more complete than it really is:

- Tauri’s async-command guidance still warns that borrowed types such as `State<'_, T>` need workaround patterns.
- Even if those two public wrappers move behind `spawn_blocking`, the same synchronous load/resample work still happens inside other async commands in this file (`polygon_download_1m_and_load` and `polygon_top_off_dataset`) after their blocking download work finishes.

So I agree with the direction, but not with overselling it as the immediate “fix the blocking posture” patch. The safer phrasing is: **worth doing for direct invocations, but it is not the whole blocking-work story for this module**.

### 7. One adjacent issue is probably real, but I would still defer it

`load_gpu_speedups_csv` is another workspace-root-dependent surface (`src-tauri/src/commands_data.rs:162-229`). Like the sample-data command, it depends on finding repo-local files instead of app-local state.

I suspect this is acceptable during local development and potentially brittle in packaged builds, but I would not change its runtime behavior blindly in this pass because:

- it is advisory UI metadata, not a core dataset/runtime command;
- unlike `list_sample_csvs`, I do not have the frontend fallback behavior in the provided context; and
- returning an empty list instead of an error could silently hide a developer regression if the service currently expects the CSV to exist during source-tree runs.

That makes it a good deferred note, not a safe-now patch.

## Implementation Priority

### Safe-now hardening

1. **Fix the silent ping-failure exit path** in `run_polygon_live_candle_stream` so a failed `Pong` send emits a terminal status event and goes through the same runner-disarm/finalization path as the other non-manual interruption branches.

2. **Harden `normalize_polygon_live_bar`** to require vendor-provided `sym`, enforce finite OHLC, and validate timestamp ordering before emitting `polygon_live_bar` or calling the live-execution runner.

3. **Replace the fixed `.vta_write_probe` writability check** with a non-destructive create-new or unique-temp probe so `set_market_data_dir_for_base` cannot overwrite an existing user file.

4. **Fix `list_sample_csvs` by reusing repo/workspace discovery rather than adding another path assumption.** The safe target is the live repo `data/` surface, not `src/data`, and the wrapper-level test should assert the real resolution rule rather than only testing the helper in isolation.

5. **Keep the earlier sync-command concern in scope, but as secondary cleanup.** If touched, move the direct `load_price_data` and `resample_price_data` command entries off the main-thread path using a Tauri-supported async/state-access pattern. Treat that as direct-command UX hardening, not as a complete solution to all blocking work in this module.

### Larger redesign still deferred

1. **Canonicalized or identity-based move-guard comparison** for market-data-root migration. The risk class is real, but the implementation needs more care than the third-pass packet suggests because `canonicalize` is not a drop-in for non-existent descendants and because the persisted/displayed path contract should not be rewritten casually.

2. **Windows dialog modernization** (`SHBrowseForFolder` → modern folder picker / `IFileDialog` style surface). Microsoft’s direction is clear, but that is a platform-surface redesign, not a small hardening patch.

3. **Workspace-root-dependent advisory assets** such as `gpu_speedups.csv`. If release packaging is expected to work without a source checkout, that surface should eventually be made explicit: bundled asset, generated artifact, or deliberate “dev-only” fallback. I would defer that until the frontend/error-path contract is verified.

## Verification If Changed

Minimum backend verification if any safe-now hardening lands:

- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
- `cargo test -p ta_desktop_demo_app --test settings_tests -- --nocapture`

Targeted additions I would expect in this file’s local test module:

- a wrapper-level sample-data resolution test that reflects the real repo layout (`data/`, not `src/data`);
- a live-bar normalization test that rejects missing `sym`;
- a live-bar normalization test that rejects invalid OHLC/timestamp combinations;
- a market-data writability test proving an existing `.vta_write_probe` file is not clobbered by the replacement probe logic.

If the live-stream loop changes, I would also want at least one targeted runtime test or extracted-helper test for the failed-ping path so the terminal-status/disarm behavior is not left implicit.

If direct `load_price_data` / `resample_price_data` signatures change to async, the frontend smoke surface should include:

- normal CSV load;
- resample after load;
- Polygon download-then-load;
- Polygon top-off-then-reload.

That last point matters because this file also invokes the same state-loading logic from inside already-async commands, and that path should not be broken while cleaning up the direct wrappers.

## Regression And Feature-Preservation Constraints

- Preserve current command names and current live event names. This file is already part of the public desktop/frontend IPC surface.
- Preserve existing `polygon_live_bar` field names. Hardening validation is fine; renaming payload fields is not.
- Do **not** broaden backend-side path trimming casually. Exact filesystem paths are the safer cross-platform contract.
- If the market-data-root move guard is hardened later, use canonicalization or equivalent identity checks for comparison only. Do not rewrite the persisted/displayed user path to its canonical form as a side effect.
- If `load_price_data` / `resample_price_data` are converted to async commands, use a Tauri-supported state-access workaround instead of a naïve borrowed-`State` async rewrite.
- Preserve cooperative cancellation semantics for Polygon download/top-off flows. `spawn_blocking` does not provide a real abort once the blocking work is running.
- Keep the current Windows dialog posture stable unless the repo intentionally chooses a broader platform/dialog redesign.

## Fourth-Pass Conclusion

The third-pass bucket should **stand**: this file is still in **`Small surgical refactor`**, not redesign.

But the third-pass packet’s priority call should **change**.

The packet was right that market-data-root movement has path-identity risk. It was wrong to treat that as the sharpest immediate bug. The current live source still contains more concrete and more local defects:

- a silent live-stream termination branch on failed `Pong` send;
- a too-permissive live-bar normalization boundary that fabricates `sym` and skips basic OHLC/timestamp validation;
- a destructive fixed-name writability probe that can clobber an existing file;
- a still-stale sample-data lookup that is easier to prove and easier to fix than the canonicalization item.

The safest next pass is therefore:

- fix the silent live-stream exit;
- harden live-bar validation;
- make the market-data writability probe non-destructive;
- repair `list_sample_csvs` using existing repo/workspace discovery;
- keep async command-boundary cleanup as secondary, with the correct Tauri workaround posture.

What I would explicitly **not** do from the current packet is blindly trim more filesystem paths at the backend boundary or rush a canonicalization patch that assumes `canonicalize` is a trivial drop-in. Those are the recommendations in the current packet most likely to cause cross-platform or first-run regressions if implemented too quickly.
