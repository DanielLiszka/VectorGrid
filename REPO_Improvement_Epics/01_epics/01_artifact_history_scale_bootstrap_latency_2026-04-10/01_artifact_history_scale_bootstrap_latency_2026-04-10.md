# Artifact History Scale And Bootstrap Latency Plan

Last updated: 2026-04-11

## Purpose

Define a measured, additive epic to reduce startup/bootstrap latency, latest-run hydration cost, artifact-library listing cost, reopen latency, and history-library scale pain without weakening artifact truth, replay fidelity, or saved-run compatibility.

## Scope Boundaries

In scope:

- artifact-library listing and hydration cost
- latest-run reopen and workflow-artifact sync latency
- startup/bootstrap critical-path versus deferred-path splitting
- history/library surfaces that depend on artifact enumeration and reopen speed
- baseline measurements, success criteria, rollback notes, and regression gates for these flows

Out of scope:

- changing artifact schema semantics unless a narrower follow-up doc proves it is required
- options-related artifacts or workflow history unless a shared artifact-library listing path is intentionally touched; if shared listing code changes, non-run families remain compatibility-frozen
- broad UI redesign unrelated to scale/latency
- replacing the existing first-tier artifact docs

## Current User-Facing Behavior

The current latest-run reopen path is already narrower than a full-library refresh. The frontend asks the runtime for `load_latest_run_artifact_summary`, then hydrates only that returned path with `load_run_artifact_hydration`. That preserves truth while avoiding the common-case “enumerate everything first” pattern on the normal latest-run sync path.

Full library browsing is still broader. On the backend, artifact-library listing still scans the runs directory, opens every JSON artifact, parses each one, and derives multiple summary families from the full scan. The latest-run summary path also keeps a correctness fallback to a full scan for nonstandard filenames.

On the frontend, startup bootstrap restores shell state, starts startup data hydration, and then chains artifact/open-state restoration through runtime/library calls. This is functionally correct and centralized, but it is likely to degrade as the saved-run library grows.

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/stores/historyStore.ts`
- `web/src/services/historyService.ts`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/HistoryContextDock.vue`

## Current Verification Surface

- `src-tauri/src/run_artifacts.rs`
  - hydration-payload and artifact-summary tests
- `web/src/services/workflowArtifactsSync.test.ts`
- `web/src/services/workflowBootstrap.test.ts`
- `web/src/stores/historyStore.actions.test.ts`
- `web/src/stores/historyStore.search.test.ts`
- `web/src/services/historyService.test.ts`
- `web/src/components/HistoryWorkspacePanel.test.ts`
- `web/src/services/analysisService.ts`
  - current history/open follow-up behavior depends on artifact state rehydration
- `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1`
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `node scripts/run_output_parity_check.mjs`

## Artifact Truth And Freshness Contract

- hydration/detail payloads remain authoritative; any summary or maintained index is derivative only
- artifact identity is not `config_fingerprint`; that field remains comparison/provenance metadata only
- the compatibility freeze covers `config_fingerprint`, artifact versioning, dataset/resample/source identity, route provenance, lineage, optimization metric surface, ML fields, validation payload presence, req/res plus validation req/res behavior, tags, and legacy aliases
- if a fast path skips hydration, it must do so only with a trustworthy freshness token or an explicit artifact-immutability rule plus a safe hydration fallback
- the current latest-run fallback for nonstandard filenames remains part of the correctness contract, not an optional optimization detail

## Strengths Already Present

- artifact truth is already centralized in one Rust owner
- workflow-artifact rehydration has a clearly shaped payload contract
- the frontend already separates artifact-store state from higher-level analysis state
- retention, tags, and compatibility aliases already exist and should be preserved

## Weaknesses / Failure Modes

- full-library scans can still dominate history-entry flows and shared library browsing as the artifact count grows
- startup responsiveness can degrade when artifact/open state depends on non-critical hydration work
- history/library surfaces can look slow even when the underlying analysis logic is fine
- repeated parse/list/hydrate work can stack unnecessarily across startup, library, and reopen flows

## Regressions To Avoid

- breaking saved-run compatibility, tags, or legacy artifact aliases
- making startup feel faster by silently dropping artifact truth or validation context
- introducing stale artifact indexes without a trustworthy fallback path
- widening scope into schema rewrites or history-UI redesign before baseline evidence exists
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [State Management | Tauri](https://v2.tauri.app/develop/state-management/)
  - relevant because artifact-library and startup improvements should keep state ownership explicit instead of spreading ad hoc caches across unrelated commands
- [Calling Rust from the Frontend | Tauri](https://v2.tauri.app/develop/calling-rust/)
  - relevant because the hot path here is frontend-to-command orchestration, not just local in-memory rendering
- [content-visibility | MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/content-visibility)
  - relevant because some startup/history wins may come from deferring non-critical UI work after the shell becomes interactive

## Proposed Improvement Approach

1. Establish the actual baseline:
   - shell restore / first-interactive time
   - startup data hydration time
   - latest-run summary time
   - latest-run hydration time
   - analysis refresh after hydration
   - history-library list/open time
   - artifact reopen time for representative branch cases rather than size-only tiers
2. Split the current path into two explicit lanes:
   - latest-run fast path
   - full-library browsing/index path
3. Introduce a repo-truth-preserving summary/index mechanism only if it measurably reduces directory-scan cost and keeps a safe fallback to the current full scan.
4. Separate bootstrap-critical work from deferred work so the shell can become interactive before non-critical artifact/history refreshes complete.
5. Keep success criteria explicit:
   - measurable improvement on the baseline scenarios
   - no artifact truth regressions
   - no reopen/history compatibility regressions

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Define the representative startup, latest-run, reopen, and history-library scenarios to measure, including standard-name latest run, nonstandard-filename latest run, validation/no-validation reopen, ML-signal reopen, and mixed-family library scans where shared listing code is touched.
- [x] Record baseline timings before changing implementation.
- [x] Identify the exact latest-run fast path that should avoid a full artifact-library scan and record the existing nonstandard-filename fallback contract.
- [x] Decide whether a maintained artifact summary/index is warranted or whether a narrower command split is sufficient.
- [x] Preserve a verified fallback to the current full-scan behavior for correctness and recovery.
- [x] Define which bootstrap work is critical for first paint versus safe to defer, with named milestones rather than one coarse startup number.
- [x] Identify the exact frontend stores, services, commands, and Rust file-IO paths that must change together.
- [x] Keep artifact truth behaviorally unchanged for tags, validation hydration, dataset identity, route provenance, lineage, optimization metric surface, ML fields, req/res, validation req/res, and legacy aliases unless a narrower follow-up doc explicitly widens scope.
- [x] Add or strengthen tests around latest-run hydration, history listing, and reopen behavior under larger fixture sets.
- [x] Define a rollback trigger if the new fast path or index becomes stale or harder to reason about than the current scan.
- [x] Record any required cross-doc sync in the first-tier artifact/history and shell docs if operator behavior changes or shared artifact-family compatibility assumptions are narrowed.

## Verification Plan

- rerun the startup and history/open baseline scenarios after implementation and compare against the recorded baseline
- run `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1` first for broad non-options changes, or record the narrower justified substitute when scope stays local
- rerun targeted artifact/history and hydration tests near the touched files
- rerun `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` if startup/open/reopen timing boundaries change
- rerun `node scripts/run_output_parity_check.mjs` if artifact payload or report-path behavior changes
- verify every changed surface against the repo minimum bar and update all linked docs together
- capture concrete acceptance evidence for closure, including saved milestone timings and any representative screenshots or trace artifacts used to validate the new path

## Rollout / Fallback / Compatibility Notes

- land the measurement harness first if the current timing boundaries are not already explicit enough
- prefer additive command splitting or summary indexes over schema rewrites
- keep the current full-scan path available as a fallback until the fast path proves trustworthy
- keep hydration/detail authoritative even if a maintained summary/index is introduced
- if shared artifact-library code is touched, preserve non-run families as compatibility-frozen during this epic
- do not widen this epic into dense-table rendering or validation/report throughput beyond the points where they directly block artifact/history latency

## Execution Notes

- Closed on the narrower command-split path rather than a maintained artifact index:
  - the backend now exposes `load_latest_run_artifact_hydration` so empty-store bootstrap can load the latest saved run in one command without first enumerating the full library
  - `list_run_artifacts_blocking` now derives run summaries directly from the scanned artifact files instead of building the full mixed-family library snapshot first
  - the latest-run summary helper still preserves the full-scan fallback for nonstandard filenames, and targeted Rust tests prove both the standard-name fast path and the fallback path
- Bootstrap and history timing ownership is now explicit instead of implied:
  - `main.ts` records `main.firstPaintWaitMs`, `main.desktopInitializationMs`, and `main.bootToReadyMs`
  - `workflowBootstrap.ts` records `workflowBootstrap.restoreShellStateMs`, `workflowBootstrap.startupDataHydrationMs`, and `workflowBootstrap.latestRunSyncMs`
  - `workflowArtifactsSync.ts` records `workflowArtifactsSync.latestRunFastPathMs`, `workflowArtifactsSync.latestRunSummaryCheckMs`, and `workflowArtifactsSync.latestRunRehydrationMs`
  - `historyService.ts` records `historyService.refreshSavedRunsMs`, `historyService.loadSavedRunHydrationMs`, and `historyService.loadSavedRunDetailMs`
  - the new perf helper remains additive and read-only; it keeps a bounded in-memory ring buffer plus a browser debug bridge through `window.__VTA_ARTIFACT_HISTORY_PERF__`
- The representative measured scenarios for this epic were intentionally limited to the surfaces the code actually changed:
  - standard-name latest run with no current artifact loaded
  - nonstandard-filename latest run with fallback scan retained
  - mixed-family library listing with run-only summary extraction
  - boot with no restored shell state in mocked-Tauri browser mode
  - history library refresh and saved-run detail/hydration service calls in Vitest
- Current-state mocked-browser timing capture after the measurement harness landed:
  - `main.firstPaintWaitMs`: about `42.0 ms`
  - `workflowBootstrap.restoreShellStateMs`: about `0.3 ms` with `restoredState=false`
  - `workflowBootstrap.startupDataHydrationMs`: about `1.5 ms`
  - `workflowArtifactsSync.latestRunFastPathMs`: about `0.6 ms`
  - `workflowBootstrap.latestRunSyncMs`: about `0.8 ms`
  - `main.desktopInitializationMs`: about `2.1 ms`
  - `main.bootToReadyMs`: about `47.8 ms`
  - these timings are mocked-browser baseline evidence for this epic’s new milestones, not native-desktop perf claims
- Decision for this epic:
  - a narrower command split is sufficient for the current artifact/history lane
  - a maintained summary/index remains deferred until future evidence shows the run-only list path and latest-run fast path are still insufficient
- Cross-doc sync was not required in the first-tier tree because operator-visible artifact semantics did not change and non-run families remained compatibility-frozen
- Acceptance evidence:
  - `cargo test -p ta_desktop_demo_app --lib load_latest_run_artifact_`
  - `cargo test -p ta_desktop_demo_app --lib list_run_artifacts_blocking_returns_only_run_summaries_from_mixed_library`
  - `npm.cmd --prefix web run test -- src/services/artifactHistoryPerf.test.ts src/main.test.ts src/services/workflowBootstrap.test.ts src/services/workflowArtifactsSync.test.ts src/services/historyService.test.ts`
  - `npm.cmd --prefix web run typecheck`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run build`
  - `npm.cmd exec -- playwright test -c ./playwright.config.ts ./e2e/browser/artifact-history-bootstrap-perf.spec.ts ./e2e/browser/shell-ux.spec.ts ./e2e/browser/command-palette-overlays.spec.ts`

## Related Docs

- `REPO_Improvement/06_artifacts/03_artifact_library_index_2026-04-09.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-09.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/28_workflow_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/43_output_parity_pack_2026-04-09.md`
