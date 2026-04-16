# Fourth-Pass Review: src-tauri/src/commands_campaign.rs

- Review date: `2026-04-15`
- Source file: `src-tauri/src/commands_campaign.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_campaign.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_campaign.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_campaign.rs.review.md`
- Third-pass action bucket: `Larger refactor but defer`
- Fourth-pass action bucket: `Targeted hardening now; larger redesign still deferred`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file deserved a fourth pass because the third-pass packet narrowed the discussion to checkpoint durability, and that narrowing was directionally useful but too aggressive. The live source still clearly supports the “larger redesign still deferred” part of the prior packet: `commands_campaign.rs` remains a very large public orchestration surface, the command still runs through one long-lived `spawn_blocking` body, and broad control-flow extraction would still be regression-prone.

But the live source also shows several bounded correctness and contract-fidelity issues that do **not** require a large redesign to address. That changes the practical outcome. The right conclusion is no longer “defer everything except maybe persistence hardening later.” The right conclusion is “fix a small set of safe-now orchestration defects now, and still defer the broad module split.”

So the current third-pass bucket should **change**. Its large-refactor caution still stands, but its defer posture is now too strong for the live code.

## Fourth-Pass Research Log

I re-read the live `commands_campaign.rs` first, then checked only the adjacent context needed to validate contract and persistence assumptions.

The highest-signal local code reads were:

- `prepare_campaign_dataset`, because it is the point where the backend already learns the true dataset identity and timeframe.
- `existing_optimization_match` and `existing_validation_matches`, because the file’s skip/reuse semantics live there.
- `run_research_campaign_blocking_with_mode`, because that is where `validate_only`, `skip_existing`, checkpoint writes, and skipped-item summaries actually compose.
- `build_request_summary_value`, because the campaign summary is the stated retry/resume source of truth.
- the local test block, to see which semantics are already characterized versus still implicit.

I also checked the immediate neighboring backend context:

- `state.rs`, to verify that other dataset-loading paths already center source identity and normalize loaded data at the backend boundary.
- `run_artifacts.rs`, to verify how run summaries and config fingerprints are persisted and matched elsewhere.
- `REPO_MAP.md`, because it explicitly states that research-campaign request shape, progress payload shape, and saved summary artifacts are part of the product contract.

For external standards, I rechecked only primary references relevant to the claims in the earlier packets:

- Tauri’s current command guidance, which still prefers async commands for heavy work and confirms that this command boundary is supposed to keep UI-impacting work off the main thread.
- Tokio’s `spawn_blocking` guidance, which still makes the same point as the second pass: once the blocking task has started, aborting it does not cancel the work.
- Rust standard-library filesystem docs for `std::fs::write`, `std::fs::rename`, and `std::fs::File::sync_all`, because the third-pass packet’s persistence recommendation is only useful if it stays precise about what those primitives do and do not guarantee.

## What Fourth Pass Sharpened

### What the third-pass packet got right

The third-pass packet was right about three important things.

First, the file is still too central and semantics-sensitive for opportunistic decomposition. The main orchestration body at `run_research_campaign_blocking_with_mode` is still the highest-risk place to make broad structural edits. The public Tauri entrypoint is still a thin shell around a single blocking workflow, so the “bigger split later, not casually now” warning remains correct.

Second, the checkpoint durability concern is real. `save_campaign_summary_blocking` still rewrites the same summary file with `std::fs::write` at lines `1913-1927`, so the third-pass packet was correct to reject any claim that current summary writes are crash-safe.

Third, the second- and third-pass emphasis on cooperative cancellation still stands. The async command at lines `3746-3757` delegates into a single blocking task, so cancellation remains an internal polling property of the workflow, not a true external abort.

### Why the third-pass bucket is now too defer-heavy

The third-pass packet treated summary durability as the first safe-now hardening target and left the rest in the defer bucket. The live source now makes that priority too narrow.

The more important fourth-pass finding is that the file has a handful of **bounded orchestration correctness issues** that are safer to fix now than to defer. These are not “needs a large redesign” findings. They are direct control-plane mismatches between the file’s own request contract, persistence contract, and per-item summary contract.

### 1. `validate_only` is accidentally gated by `skip_existing`

This is the strongest live-source issue.

At lines `2142-2145`, the code only loads the existing run-artifact index when `req.skip_existing` is true. But at lines `2381-2427`, the `ValidateOnly` execution path unconditionally tries to find an existing optimization artifact in that preloaded list. That means `execution_mode = ValidateOnly` with `skip_existing = false` does not mean “validate existing optimization artifacts anyway.” It means “search an empty list, mark every dataset skipped, and continue.”

That is not just surprising; it is contract-shaped wrong. In `REPO_MAP.md`, `execution_mode` and `skip_existing` are separate campaign controls. The current implementation quietly makes `validate_only` operationally depend on `skip_existing`, even though the request surface does not say that.

The downstream effect is worse than a mere skip. At lines `3635-3644`, the final summary state only becomes `completed_with_failures` when `failed > 0`, and only becomes `completed_with_warnings` for `meta:` / `stage5:` warnings. A `validate_only` run that skipped every dataset because the existing-artifact list was never loaded can still finish with state `completed`.

That is a real correctness issue and a real product-surface issue. It should not be deferred behind a large refactor.

The bounded fix is straightforward: when `execution_mode == ValidateOnly`, always load the existing run-artifact summaries regardless of `skip_existing`, or reject the request up front with an explicit error if the product wants that coupling. The first option is the safer contract-preserving one.

### 2. Artifact reuse matching trusts request hints instead of the loaded source identity

The third-pass packet correctly said resume behavior should stay coupled to durable artifact truth, but it missed that the current reuse path already drifts from that principle even before any crash scenario.

At lines `969-1008`, `prepare_campaign_dataset` loads the file and learns the actual `source_identity`, then stores the true `source_size_bytes` and `source_mtime_utc_ms` into `dataset_meta`.

But at lines `1955-2006`, `existing_optimization_match` does **not** compare against that freshly learned identity. Instead, it compares against `prepared.input.source_size_bytes` and `prepared.input.source_mtime_utc_ms` at lines `1996-1999`, and only uses a content fingerprint if the caller provided one in the request.

That means reuse can fail even when the backend already knows the file matches:

- if the frontend omits `source_size_bytes` / `source_mtime_utc_ms`,
- if resume/retry reconstructs the request without those optional fields,
- or if the request carries stale hints but the freshly loaded file is actually the same dataset the artifact was built from.

That is not hypothetical. At lines `1839-1845`, `build_request_summary_value` writes only `csv_path`, `label`, and `timeframe` for each dataset into the saved campaign summary. It does **not** persist the optional identity hints. So the saved campaign summary is not a faithful round-trip input for a reuse path that depends on request-time size/mtime hints.

This is exactly the sort of bug the earlier packets were trying to avoid: the durable campaign artifact is supposed to be the retry/resume truth, but the matching logic is still biased toward ephemeral request decoration.

This is also a bounded safe-now fix. The matching helper should compare against the identity the backend just learned during preparation, not against optional caller hints. The caller-provided content fingerprint can remain a stricter override when present, but size/mtime matching should come from the prepared dataset, not the raw request.

### 3. Skipped validation items can report the wrong artifact path

At lines `1937-1952`, `existing_validation_matches` returns only a `bool`. It throws away the matching `RunArtifactSummary`.

That matters at lines `3187-3203`. When the workflow skips validation because a matching validated run already exists, the code records `artifact_path: Some(artifact_path)` in the item summary. At that point, `artifact_path` is the current optimization artifact path, not necessarily the already-matched validation artifact path.

So the summary can say, in effect, “this dataset was skipped because a validated artifact already exists,” while still pointing at the optimization artifact instead of the validated artifact that justified the skip.

That weakens reveal behavior, retry fidelity, and artifact-library consistency. It is a narrow persistence/reporting bug, not a design-level argument for splitting the file.

The safe-now fix is to return the matched validation `RunArtifactSummary` or at least its path, not just a boolean.

### 4. The durability recommendation needs more precision, and the file already exposes an instrumentation gap

The third-pass packet was right that `save_campaign_summary_blocking` is the key persistence weak point, but its proposed remedy was a little too loose.

The safe-now persistence hardening is not “just use an atomic rewrite” and move on. The practical bounded fix here is a same-directory temp-file rewrite path with explicit flush discipline before replacement. That matters because Rust’s docs distinguish what `write`, `rename`, and `sync_all` actually guarantee. A same-directory rename improves replacement semantics; it should not be oversold as complete crash-durability by itself.

There is also a missed observability clue in the live source. `ResearchCampaignItemDiagnostics` already reserves `checkpoint_save_ms` at lines `166-180`, and `build_item_diagnostics` accepts `checkpoint_save_ms` at lines `788-805`. But every current call site still passes `None`. The schema already tells you the file wanted checkpoint latency visibility; the implementation never finished it.

That makes the safe-now persistence hardening larger than the third-pass packet stated, but still bounded:

- harden the summary-write path, and
- actually populate `checkpoint_save_ms` so the artifact can tell you what the checkpoint path costs.

### 5. Input accounting is internally inconsistent when blank dataset paths appear

This is smaller than the reuse bugs, but it is still a real contract weakness the third pass did not call out.

At lines `2112-2126`, `counts.total` is derived from a filtered list of non-empty dataset paths. Later in the same function, `dataset_total` and the `items` vector are derived from `req.datasets` directly, not from the filtered list. If a request carries any blank `csv_path` entry, the workflow can count totals from one dataset universe while iterating, progressing, and failing items from another.

I would not treat this as redesign debt. It is a boundary-invariant problem. The safe-now fix is either to reject blank `csv_path` entries up front or to normalize the dataset list once and drive every later count, item, preload, and loop off that same normalized set.

### What the third-pass packet overstated

The third-pass packet’s main overstatement was not that the durability concern was wrong. It was that durability was the **first** safe-now hardening target and that the bucket therefore should stay unchanged.

The live source now shows at least three more urgent bounded fixes that affect actual campaign correctness:

- `validate_only` should not silently depend on `skip_existing`,
- dataset reuse should not depend on optional request identity hints when the backend has already loaded the file and knows its identity,
- and skipped validation summaries should point at the artifact that justified the skip.

Those are stronger “fix now” candidates than persistence polish alone.

## Implementation Priority

### Safe-now hardening

1. **Decouple `validate_only` from `skip_existing`.**
   Load the existing artifact index whenever `execution_mode == ValidateOnly`, and add a direct regression test for that path. This is the most important correctness fix because the current behavior can produce a misleadingly successful campaign summary.

2. **Match reuse against prepared source identity, not request hints.**
   Thread the prepared dataset’s actual size/mtime identity into reuse matching, and keep request-provided content fingerprint only as an optional stricter key. Add a regression test that omits request identity hints and still reuses the correct artifact.

3. **Return the matched validation artifact path on validation skips.**
   Replace `existing_validation_matches -> bool` with a helper that returns the matched summary or path. Add a test that validates the item summary points at the validated artifact, not the optimization artifact.

4. **Harden campaign summary rewrites and finally populate `checkpoint_save_ms`.**
   This is still worth doing now, but it is no longer the only immediate change that matters. Keep it additive and local: same-directory temp write, explicit flush, replacement, timing capture, and optional-field population.

5. **Make dataset accounting internally consistent.**
   Fail fast on blank dataset paths or normalize the request once and use that same normalized vector throughout the function.

### Larger redesign still deferred

The broad split is still justified, but still should not be mixed into the safe-now pass.

The deferred redesign candidates remain the same class of work the earlier packets already identified:

- extract DTOs and request/summary helpers,
- extract dataset-preload infrastructure,
- extract reuse/admission-policy helpers,
- extract summary persistence and checkpointing,
- and reduce the main orchestration body behind a service-style internal boundary.

That work still needs characterization coverage first. The fourth pass does **not** argue for doing the large split now. It argues for not using the large split as a reason to postpone narrower correctness fixes that are already clear.

## Verification If Changed

If this file changes, the minimum verification bar should now include the earlier coverage plus new targeted characterization for the missed paths.

Run at minimum:

- `cargo test -p ta_desktop_demo_app commands_campaign -- --nocapture`
- `cargo test --workspace`
- `npm --prefix web run test -- src/services/metaServiceCampaignStrategy.test.ts src/services/metaServiceCampaignResume.test.ts src/stores/taskStoreCampaignProgress.test.ts src/tauri/instrumentation.test.ts`

Add targeted Rust tests for the newly identified gaps:

- `validate_only` loads and uses existing optimization artifacts even when `skip_existing` is false.
- reuse matching succeeds when request identity hints are absent but the freshly prepared dataset identity matches the artifact.
- a skipped validation item records the validation artifact path, not the optimization artifact path.
- `checkpoint_save_ms` is populated when a summary checkpoint is written.
- blank `csv_path` handling is either rejected up front or normalized consistently.

If the summary writer changes, add a manual interruption/resume smoke that kills a campaign between summary rewrites and confirms the saved summary remains parseable.

## Regression And Feature-Preservation Constraints

Any fix here needs to stay deliberately small and preserve the existing desktop contract.

Keep these constraints explicit:

- Preserve the public `run_research_campaign` request/response schema and the `research_campaign_progress` event shape.
- Preserve cooperative cancellation semantics. Do not imply true task abortability while the workflow still runs inside a started `spawn_blocking` task.
- Preserve current optimization and validation engine selection. The safe-now pass should stay in orchestration, persistence, and matching logic, not in GPU/validation math.
- If checkpoint rewrites are hardened, keep temp files in the same directory as the final summary path. Do not introduce a cross-volume temp path that breaks rename replacement semantics.
- Do not widen `summary.state` semantics casually. The current file already uses `warnings` more broadly than `completed_with_warnings`; fixing that may be desirable, but it crosses frontend filtering and resume expectations and should be audited first.
- Do not change `auto_stage5` behavior without a frontend/meta-workflow audit. The current implementation still nests Stage 5 under the `auto_meta` branch, and changing that independently could alter shipped orchestration semantics.
- Preserve artifact fingerprint compatibility. The safe-now change is to use the backend’s freshly prepared dataset identity more faithfully, not to invent a new fingerprint scheme.

## Fourth-Pass Conclusion

The third-pass packet was right that `commands_campaign.rs` is still too risky for a casual module split and that summary durability is a real issue. That part stands.

But the third-pass bucket should **change**. The live source now shows several safe-now hardening steps that are more than “nice to have” and less than “major redesign”: `validate_only` is accidentally coupled to `skip_existing`, artifact reuse is matched against optional request hints instead of the loaded dataset identity, skipped validation items can point at the wrong artifact path, and checkpoint latency is exposed in schema but not implemented.

So the correct fourth-pass disposition is:

- **fix a small set of orchestration correctness issues now**, and
- **still defer the large structural refactor until characterization coverage is stronger**.

That is a meaningfully different answer from the current third-pass packet, and it is the stronger evidence-backed reading of the live source.
