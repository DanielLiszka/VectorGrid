# Follow-Up Performance Implementation Pass

Date: 2026-05-07
Status: implemented and targeted-verification complete

## Scope

This pass implemented the next low-risk opportunities identified after the third exhaustive review. It stayed within existing feature behavior and focused on reducing repeated work in hot orchestration paths and improving frontend performance evidence capture.

## Implemented Changes

### Artifact Catalog Family Reads

Files:

- `src-tauri/src/run_artifacts.rs`

Changes:

- Added a reusable validated catalog connection helper.
- Added a family-filtered catalog loader:
  - `SELECT summary_json FROM artifact_index WHERE family = ?1 ORDER BY ts DESC, path ASC`
- Routed family-specific list calls through that path before falling back to the full library scan:
  - run artifacts
  - meta artifacts
  - stage4/stage5 artifacts
  - ML policy evaluation artifacts
- Kept the full `list_artifact_library_blocking` API unchanged for callers that need the whole snapshot.
- Preserved stale/missing/catalog-error fallback to existing JSON scanning.

Performance effect:

- Valid catalog hits no longer deserialize unrelated artifact family summaries.
- Repeated desktop history/list calls scale with the requested family instead of the entire artifact library.

Regression guard:

- Catalog schema is unchanged.
- Unknown/stale/unreadable catalog state still falls back to the previous full scan path.
- New tests prove family reads ignore unrelated invalid catalog JSON.

### Campaign Candidate Admission

Files:

- `src-tauri/src/commands_campaign.rs`

Changes:

- Scalar-union admission now ranks row indices instead of cloned rows.
- Parameter JSON keys are cached on first use during interleaved dedupe.
- Full provenance counts are preserved; the implementation does not early-stop before dedupe accounting.
- Pareto-frontier admission now trims by indices and clones only selected rows.
- CompositeMo candidate selection skips the second JSON stringify/dedupe pass because admission rows are already deduped and candidate construction is a direct params clone for that family.

Performance effect:

- Fewer row clones in scalar-union and Pareto admission.
- Less repeated `serde_json::to_string` work during scalar-union dedupe.
- Less redundant candidate serialization for CompositeMo validation admission.

Regression guard:

- Existing ordering semantics are preserved:
  - objective score descending;
  - `input_index` ascending tie-break;
  - scalar-union round-robin objective interleave;
  - Pareto feasible-before-infeasible diversity trim.
- New scalar-union duplicate-param test verifies full dedupe counts and candidate order.

### Cross-Ticker Finalist Preservation

Files:

- `src-tauri/src/commands_validation.rs`

Changes:

- Cross-ticker preserved-finalist selection now scans candidates once and collects approved/survivor indices.
- Clones happen only for the returned bucket.
- Existing behavior is intentionally preserved:
  - any approved candidates return only the approved subset;
  - otherwise hard-gate survivors are used;
  - otherwise the first bounded finalists are returned.

Performance effect:

- Avoids multiple candidate scans and broad temporary clones on cross-ticker validation response shaping.

### Frontend Performance Evidence Capture

Files:

- `web/scripts/support/mockedTauriHarness.mjs`
- `web/scripts/capture-baseline-ui.mjs`
- `web/scripts/check-ui-perf-budget.mjs`

Changes:

- Added optional mocked Tauri command delays and timestamped invoke logs to the baseline UI harness.
- Extended baseline UI capture to harvest:
  - `__VTA_ARTIFACT_HISTORY_PERF__`
  - `__VTA_VALIDATION_REPORT_PERF__`
  - `__VTA_TEST_INVOKE_LOG`
- Added flattened observations to the UI perf budget report:
  - `main_boot_to_ready_ms`
  - `main_desktop_initialization_ms`
  - `workflow_startup_data_hydration_ms`
  - `workflow_latest_run_sync_ms`
  - `latest_run_fast_path_ms`
  - mocked invoke count/total/max/delay totals
  - artifact-history and validation-report entry counts

Performance effect:

- No product behavior change.
- The existing UI budget artifact now captures frontend timing bridge data needed to attribute startup/history latency instead of relying only on coarse cold-start totals.

## Verification

Rust:

```powershell
cargo fmt
cargo test -p ta_desktop_demo_app list_run_artifacts_blocking_uses_valid_catalog_snapshot --lib
cargo test -p ta_desktop_demo_app list_meta_artifacts_blocking_uses_valid_catalog_snapshot --lib
cargo test -p ta_desktop_demo_app campaign_scalar_union_admission --lib
cargo test -p ta_desktop_demo_app campaign_composite_mo --lib
cargo test -p ta_desktop_demo_app cross_ticker_selects_bounded_finalists --lib
```

Frontend scripts:

```powershell
node --check web/scripts/support/mockedTauriHarness.mjs
node --check web/scripts/capture-baseline-ui.mjs
node --check web/scripts/check-ui-perf-budget.mjs
npm --prefix web run baseline:ui:built
npm --prefix web run perf:ui:budget
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_campaign_perf_gate.ps1 -Mode candidate -OutPath output/benchmarks/ta_campaign_perf_followup.json
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_support_perf_gate.ps1 -Mode candidate -Case artifacts -OutPath output/benchmarks/support_perf_artifacts_followup.json
git diff --check
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck
```

Observed UI budget output from this pass:

- `app_cold_start_ms`: 80 / 2500
- `time_to_interactive_ms`: 637 / 8000
- `capture_runtime_ms`: 8396 / 60000
- `chart_live_feed_avg_ms_per_event`: 0.0015 / 2
- `main_boot_to_ready_ms`: 185.5
- `workflow_latest_run_sync_ms`: 13.6
- `latest_run_fast_path_ms`: 13.4
- `mock_tauri_invoke_count`: 22

Observed performance gate outputs:

- campaign follow-up `mean_total_wall_ms`: 27.9351
- campaign follow-up `mean_campaign_runtime_ms`: 25.0
- support artifact follow-up `artifacts.library_hydration_replay_repair`: 41.9294 ms
- support artifact follow-up `rebuild_artifact_catalog`: 25.733 ms
- support artifact follow-up `load_run_artifact_hydration`: 1.3526 ms

Repo quality gate result:

- Rust format/check/clippy passed.
- Web lint/typecheck passed.
- Desktop harness typecheck passed.

## Notes

- No baselines were intentionally refreshed in this pass.
- The UI capture command rewrites local generated screenshot/metrics artifacts under the existing baseline capture path.
- The backend changes are additive/refactoring-only with preserved public command contracts.
