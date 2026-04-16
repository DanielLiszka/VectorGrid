# Fourth-Pass Review: `crates/app_core/src/ml_signal.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/ml_signal.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass bucket should **stand**, but the third-pass write-up itself does **not**. The live file is not a vague “support and instrumentation seam.” It is the public backend owner for ML-signal dataset export, score-cache ingestion, validation, policy sweep, and model-bundle inspection, and that surface is re-exported from `crates/app_core/src/lib.rs:201-213`. `REPO_MAP.md` also routes ML Signal as its own workflow entry (`web/src/services/mlSignalService.ts` -> `src-tauri/src/commands_ml_signal.rs` -> `crates/app_core/src/ml_signal.rs`) and describes artifact/schema/validation-context expectations around this exact lane.

What the third-pass packet got right is narrow but important: this file still does **not** read like an architecture-redesign target. The next defensible pass is still local hardening, contract clarification, and better verification. What it missed is where the actual local risk sits: persistence truthfulness, score-cache truthfulness after timestamp alignment, export-manifest truthfulness, and a couple of public contract seams that look stronger than they really are.

## Fourth-Pass Research Log

This fourth pass was source-first, contract-first, and bundle-scoped.

1. I re-read the live source with line-level focus on the export path, score-cache build/load path, validation/sweep path, and bundle-listing path.
2. I cross-checked repo guidance to validate ownership and consumer expectations:
   - `crates/app_core/src/lib.rs:201-213` confirms this file is a public app-core surface, not a purely local helper module.
   - `REPO_MAP.md:365-371` confirms this file is the backend workflow owner for ML Signal.
   - `REPO_MAP.md:1504-1513` is especially relevant because it says the ML artifact surface treats split manifests, score manifests, feature schema path/version, and validation context as operational contract details.
3. I refreshed the Rust/std and crate docs that matter for the persistence recommendations:
   - `std::io::BufWriter`: dropping a `BufWriter` attempts a flush, but drop-time flush errors are ignored; explicit `flush()` is the reliable error surface.
   - `std::fs::File::create`: creating the destination file truncates any existing file immediately.
   - `csv::Writer`: the CSV writer is already buffered, so wrapping it in an extra `BufWriter` is unnecessary.
   - `std::fs::rename`: useful for same-directory staged publication, but not a magic multi-file transaction and not safe across mount points.
4. This pass did **not** rely on a live workspace execution run. The bundle only contained the review packet plus local file context, so the conclusions below are source-backed and contract-backed rather than benchmark-backed.

## What Fourth Pass Sharpened

### Bucket assessment

`Small surgical refactor` should remain the action bucket.

That is not because the third-pass packet was strong. It was not. It is because the live risks are still mostly local to this file:
- output publication/staging,
- load-time score-cache truthfulness,
- export-manifest truthfulness,
- targeted resilience guards,
- and missing tests around the export and model-bundle branches.

A module split, public-surface narrowing pass, or broader workflow redesign would be a larger change than the current source justifies.

### What the current third-pass packet got right

The third-pass packet was right about two things:

1. **Do not overreact into a broad refactor.**  
   The file is large and multi-responsibility, but the next useful pass is still local and behavior-preserving.

2. **Explicit contract behavior matters more than convenience cleanup.**  
   That instinct is correct; it just needed to be applied to the real contracts in the file instead of to an abstract “support seam” framing.

### What the current third-pass packet missed or underweighted

#### 1. The real persistence bug is broader than “sidecars can appear before the main CSV”

The earlier packet correctly noticed that sidecars are written before the export finishes, but it underweighted two sharper facts:

- `write_json_file` builds a `BufWriter` and returns without an explicit `flush()` (`ml_signal.rs:996-1000`).
- The score-cache JSON writer repeats the same pattern (`1668-1671`).
- The export path opens the **final** CSV path up front (`1065-1069`) and can return `"cancelled"` in the middle of the row loop (`1112-1115`).

That means the problem is not only “orphaned sidecars.” The file can also leave:
- a truncated **final-path CSV** on cancellation or row-write failure,
- truncated/replaced final-path JSON artifacts on write failure,
- and lost final flush errors on the JSON side because the `BufWriter` is never explicitly flushed.

That is a real persistence-integrity issue on a workflow that writes user-facing artifacts.

**Safe-now hardening:** stage the primary artifact to a sibling temp file, flush it explicitly, then rename into place; publish sidecars only after the primary artifact succeeds. The same explicit-flush fix should be applied to `write_json_file` and the score-cache JSON write path.

**Important constraint:** if a temp-file strategy is adopted, it must use a sibling temp path in the same directory. A cross-device temp directory plus `rename` is not a safe default.

#### 2. `load_score_cache` currently trusts file-level truth before timestamp alignment

This is the biggest missed correctness issue in the bundle.

In `load_score_cache`, the code counts probability presence before it even knows whether a row matches the current candles (`1758-1774`), and then it builds the returned summary from persisted file metadata (`1796-1808`) instead of recomputing from the rows that actually matched the current candle series.

That matters because `validate_score_cache_contract` trusts those loaded counts (`2142-2179`).

So a cache with:
- the same `data_id`,
- valid probability columns,
- but **zero actual timestamp overlap** with the current candles

can still satisfy the current validation contract and proceed into `validate_ml_signal_candidates`.

Once that happens, the desired-position path runs on an all-`None` score vector, producing a flat/no-signal validation run instead of rejecting the cache as incompatible.

This is exactly the kind of “looks accepted, but is actually a silent no-op” contract weakness the third-pass packet should have called out.

**Safe-now hardening:** after timestamp alignment, recompute:
- effective matched rows,
- effective matched probability rows,
- missing-long/missing-short probability counts on matched rows,
- and the returned summary fields.

Then reject the cache when there are zero actual matched probability rows against the loaded candles. That keeps legitimate `CustomPartial` coverage working while closing the current silent-mismatch hole.

#### 3. `track_record_bars` is currently clamped upward and can lie

`score_obs` is computed as `count().max(2)` in `validate_ml_signal_candidates` (`2015-2019`), and that clamped value is also written into `track_record_bars` (`2055`).

That means the returned result can report `track_record_bars = 2` even when the actual matched score coverage is `0` or `1`.

The minimum-of-two logic is understandable for probability-metric math, but it should not leak into a public truth field.

**Safe-now hardening:** split this into:
- `actual_score_obs` for the returned `track_record_bars`, and
- `metric_score_obs = actual_score_obs.max(2)` for `compute_probability_metrics`.

This is a small, local truthfulness fix.

#### 4. The export manifest likely misstates the exported time range

The export loop only writes rows from `frame.first_valid_bar` through `last_label_bar` (`1027-1030`, `1112-1115`), but the export manifest records:
- `first_timestamp = candles.timestamp.first()`
- `last_timestamp = candles.timestamp.last()`

at `1315-1316`.

Unless those fields were intentionally meant to describe the **source candle span** rather than the **exported dataset row span**, the manifest is overstating what the CSV actually contains by including the warmup prefix and the lookahead tail that were never exported.

The current field names read like exported-row bounds, not source-series bounds.

**Safe-now hardening:** either:
- change those fields to the first and last **exported** row timestamps, or
- if source-span was actually intended, rename/document that explicitly in the next versioned manifest contract.

I would not leave the current ambiguity in place.

#### 5. `feature_frame` trusts external indicator lengths and can turn adapter drift into a panic path

`feature_frame` computes `pattern_recognition`, `alma`, `adx`, and `atr` outputs (`680-760`) and then the export path indexes those series directly (`1155-1283`) without checking that they match the candle length.

If the dynamic-indicator adapter ever returns a shorter series or a column count that does not match the candle count, this file will panic instead of returning an actionable error.

That is not just a theoretical style concern: this file is precisely where external dynamic-indicator output becomes a user-facing persisted artifact.

**Safe-now hardening:** validate indicator output lengths against `candles.close.len()` inside `feature_frame` and return `Err(...)` early.

#### 6. The file-level test coverage is validation-heavy but export-light

There are 17 local tests, and they do a decent job covering:
- score parsing,
- probability vs raw-only gating,
- long-only vs two-sided requirements,
- sweep ranking,
- split-window generation,
- and ticker holdout stability.

What they do **not** cover is now more important than what they do cover:
- no direct `export_ml_signal_dataset_with_candles` artifact test,
- no manifest timestamp truth test,
- no cancellation/staging test,
- no zero-overlap cache load/validation test,
- and no model-bundle listing degradation test.

The current third-pass memo did not give enough weight to this asymmetry.

### What the current third-pass packet overstated or framed poorly

#### 1. Calling this file a “support and instrumentation seam” is materially wrong

That framing understates the blast radius.

This file owns:
- public request/result DTOs,
- persisted artifact shapes,
- score-cache wire semantics,
- validation/sweep behavior,
- and model-bundle inspection.

That is a workflow-critical public surface, not a support seam. The bucket can still stay small, but the review rationale has to match the actual ownership.

#### 2. Treating “helper truthfulness” as the main issue was too weak

Helper truthfulness matters, but the bigger problems are not generic helper hygiene. They are concrete contract mismatches:
- final-path publication before success,
- stale file-level score summaries surviving load-time mismatch,
- clamped `track_record_bars`,
- export-manifest timestamp ambiguity,
- and overlapping split windows being collapsed to a single `split_id`.

The third-pass packet never got specific enough.

#### 3. The broader “make it atomic with temp-file-plus-rename” advice needs to stay narrow

This is one place where the earlier, more detailed packet can be misunderstood if implemented too aggressively.

A temp-file-plus-rename pattern is a good local fix for individual files. It is **not** a true all-or-nothing transaction for:
- schema JSON,
- split manifest,
- dataset CSV,
- and export manifest

as one group.

So the safe claim is narrower:
- avoid final-path partials,
- stage each file carefully,
- and do not publish sidecars before the primary artifact succeeds.

Anything stronger risks overpromising correctness the filesystem is not actually providing.

### Safe-now hardening

The safest next patch set is still small and local:

1. **Harden writes and publication order**
   - explicit `flush()` for JSON writes,
   - same-directory staged temp files for the dataset CSV and score-cache JSON,
   - no sidecar publication before the primary artifact succeeds.

2. **Make load-time score-cache truthfulness depend on actual timestamp matches**
   - recompute effective matched coverage on load,
   - reject zero actual matched probability rows,
   - stop trusting stored coverage/row counts when returning the live `score_summary`.

3. **Make public result fields truthful**
   - decouple actual `track_record_bars` from the clamped count used for probability metrics,
   - fix or explicitly redefine dataset export `first_timestamp` / `last_timestamp`.

4. **Convert panic-risk assumptions into validation errors**
   - validate dynamic-indicator output lengths before indexing.

5. **Make bundle listing fail-open at the directory level**
   - one malformed `manifest.json` should not hide all healthy bundles.
   - whether the degraded row should be `"warning"` or a new `"error"` state is a cross-layer decision; the safe-now part is directory-level resilience.

### Larger redesign still deferred

Some of the real issues are not good “small surgical” edits and should stay deferred:

#### 1. Split-plan artifact semantics need a versioned redesign, not a stealth tweak

`build_ml_signal_split_windows` allows overlapping windows whenever `step_bars < window_size` (`961-993`), and export assigns only the **first** matching `split_id` (`1229-1233`).

That means:
- a row can logically belong to multiple roll windows,
- the split manifest indices are source-bar indices rather than exported-row offsets,
- and the row-level `split_id` does not fully encode membership.

This is the biggest *design* issue in the file, but it is **not** a safe-now behavioral tweak. It needs an explicit artifact-contract decision:
- keep `split_id` as a convenience hint and document it as such,
- or version the export format so row-level phase/multi-window membership is represented explicitly.

#### 2. Feature-schema identity is still weaker than the repo guidance implies

`REPO_MAP.md:1509-1510` says feature schema path/version is part of the operational ML artifact contract, but in this file:
- `MlSignalScoreCacheManifest` has `feature_schema_path`,
- the builder always writes it as `None` (`1673-1693`),
- and load-time validation only checks the integer `feature_schema_version`.

That is a real contract weakness, but fixing it cleanly likely means threading schema identity through the stage-2 request/build/load path, not a one-line patch.

This should stay deferred unless the command boundary and saved-artifact consumers move together.

#### 3. `validation_context` ownership needs an explicit boundary decision

The public request DTOs carry `validation_context` (`269-280`, `298-324`), but the app-core entry points in this file do not use it (`2182-2317`).

Because the repo guidance says validation context compatibility is enforced when present, the practical enforcement appears to live above this file.

That is acceptable if intentional, but it means the app-core public API is weaker than it looks to a direct caller.

Changing that is possible, but it is coordination work:
- Tauri command boundary,
- frontend expectations,
- and tests all need to agree on whether compatibility is owned above or inside app-core.

#### 4. Duplicate timestamp policy still needs a compatibility decision

The current behavior is last-write-wins during cache build (`1612-1633`), with only a duplicate count preserved in the result/manifest.

I would **not** casually change that in the same pass as the truthfulness fixes above. The safe-now action is documentation and explicit tests. A behavior change belongs in a coordinated compatibility pass.

#### 5. Public-surface narrowing and module splitting remain deferred

The first-pass packet was reasonable to notice the public surface is broad. It was not reasonable to treat narrowing it as a safe-now cleanup. This file is re-exported intentionally, and pruning that surface in a public-facing repo is not the same kind of change as fixing write staging or load-time summary truth.

## Implementation Priority

1. **First priority: persistence and load-time truthfulness**
   - explicit JSON flush,
   - staged publication,
   - actual-match score-cache accounting,
   - zero-overlap rejection.

   These are the most defensible fixes because they close real silent-failure paths without changing the model/sweep semantics.

2. **Second priority: exported/result metadata truthfulness**
   - actual `track_record_bars`,
   - correct export-manifest timestamp bounds,
   - indicator-length guard rails,
   - resilient bundle listing.

   These are still local and high-value, but slightly less urgent than preventing silent partials and silent mismatch acceptance.

3. **Third priority: documentation/tests that freeze current semantics where behavior is intentionally kept**
   - duplicate timestamp last-write-wins,
   - split-id first-match behavior if retained,
   - ticker-holdout hashing semantics.

4. **Deferred priority: versioned artifact-contract work**
   - split-plan redesign,
   - stronger schema identity handshake,
   - validation-context ownership shift,
   - public API narrowing or module split.

## Verification If Changed

At minimum, any patch in this file should re-run the existing ML Signal surfaces and add targeted tests for the newly identified edges.

Recommended verification set:

- `cargo test -p ta_app_core ml_signal::tests:: -- --nocapture`
- `cargo bench -p ta_app_core --bench ml_signal_workflow --no-run`
- `cargo bench -p ta_app_core --bench ml_signal_validation --no-run`
- `cargo test -p ta_desktop_demo_app --no-run`

Targeted tests I would add before or with the hardening work:

1. **Score cache zero-overlap compatibility**
   - same `data_id`,
   - valid probability rows in the cache file,
   - zero timestamp overlap with current candles,
   - must reject instead of silently validating an all-flat run.

2. **Truthful `track_record_bars`**
   - one matched probability row should not report `2` bars just because the probability-metric helper needs a minimum of two observations.

3. **Export-manifest timestamp truth**
   - exported manifest timestamps should match the first and last exported rows, or the test should explicitly lock in source-span semantics if that is the intended contract.

4. **Export cancellation/publication**
   - if staging is added, cancellation mid-export should not leave the final output path looking complete.

5. **Indicator-length guard**
   - a mismatched indicator series should return `Err(...)`, not panic.

6. **Model-bundle listing resilience**
   - one malformed bundle manifest should not suppress healthy bundle summaries.

7. **If split semantics are kept as-is, pin them**
   - overlapping windows plus first-match `split_id` should have an explicit regression test so future reviewers do not infer a stronger contract than the code provides.

If public DTOs, state strings, or manifest field meanings change, I would also add:

- `cargo test --workspace --no-run`

## Regression And Feature-Preservation Constraints

- Do **not** reject legitimate low-coverage `CustomPartial` score caches just because coverage is low. The safe-now rejection boundary is zero actual matched probability rows or missing required sides on the matched rows.
- Do **not** silently change duplicate timestamp resolution. Last-write-wins may be imperfect, but changing the winner policy is a persisted-artifact compatibility change.
- Do **not** reinterpret `split_id` as full split membership without versioning the artifact contract. The current single-column representation is already weaker than that.
- If staged writes are added, use sibling temp paths on the same filesystem and verify overwrite behavior on the supported Windows target set before assuming `rename` semantics are identical everywhere.
- If `first_timestamp` / `last_timestamp` are changed to exported-row bounds, verify the UI/report consumers that may currently be treating them as source-series bounds.
- Do **not** casually normalize ticker holdout differently. The current export path hashes `req.data_id` (`1060-1063`), so changing case normalization or identifier source can move holdout assignments.
- Do **not** tighten or loosen model-bundle path requirements until bundle shape is confirmed. The current classifier effectively expects both long and short model paths to exist (`2351-2356`).
- If validation-context enforcement is pulled into app-core, keep the command boundary and frontend copy aligned; otherwise the same request may start failing in one layer but not another.

## Fourth-Pass Conclusion

`Small surgical refactor` should remain the bucket, but the current third-pass packet should **not** stand as the rationale for that bucket.

The real fourth-pass conclusion is:

- the file is a workflow-critical public ML artifact surface, not a support seam;
- the most important safe-now work is persistence hardening and load-time truthfulness, not generic structural cleanup;
- the biggest still-deferred design issue is the split-plan/export contract, especially the gap between overlapping windows and the single `split_id` column;
- and a few small truth fields (`track_record_bars`, export-manifest timestamps, effective score-summary coverage) currently overstate what the live run actually used.

If this file changes next, the best version of that change is:
- explicit-flush and staged writes,
- actual-match validation of score caches,
- truthful metadata,
- and additive tests that lock down the now-visible contract edges.

That keeps the public behavior conservative while making the persisted artifacts and validation results much harder to misread.
