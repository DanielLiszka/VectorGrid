# Artifact History Scale And Bootstrap Latency Plan

Last updated: 2026-04-10

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
- options-related artifacts or workflow history
- broad UI redesign unrelated to scale/latency
- replacing the existing first-tier artifact docs

## Current User-Facing Behavior

The current workflow-artifact sync path asks the runtime for the full run-artifact list, takes the newest row, and then loads a separate hydration payload for that path. That preserves truth, but it means the common “use the latest run” path still pays for full library enumeration first.

On the backend, artifact-library listing currently scans the runs directory, opens every JSON artifact, parses each one, and derives multiple summary families from the full scan. On the frontend, startup bootstrap restores shell state and kicks off startup data hydration while artifact/open-state restoration remains downstream of runtime/library calls.

This is functionally correct and centralized, but it is likely to degrade as the saved-run library grows.

## Current Code Surface

- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/HistoryContextDock.vue`

## Current Verification Surface

- `src-tauri/src/run_artifacts.rs`
  - hydration-payload and artifact-summary tests
- `web/src/components/HistoryWorkspacePanel.test.ts`
- `web/src/services/analysisService.ts`
  - current history/open follow-up behavior depends on artifact state rehydration
- `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- `node scripts/run_output_parity_check.mjs`

## Strengths Already Present

- artifact truth is already centralized in one Rust owner
- workflow-artifact rehydration has a clearly shaped payload contract
- the frontend already separates artifact-store state from higher-level analysis state
- retention, tags, and compatibility aliases already exist and should be preserved

## Weaknesses / Failure Modes

- full-library scans can dominate common “latest run” and history-entry flows as the artifact count grows
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
   - startup shell-interactive time
   - latest-run hydration time
   - history-library list/open time
   - artifact reopen time for representative small, medium, and large libraries
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

- [ ] Define the representative startup, latest-run, reopen, and history-library scale scenarios to measure.
- [ ] Record baseline timings before changing implementation.
- [ ] Identify the exact latest-run fast path that should avoid a full artifact-library scan.
- [ ] Decide whether a maintained artifact summary/index is warranted or whether a narrower command split is sufficient.
- [ ] Preserve a verified fallback to the current full-scan behavior for correctness and recovery.
- [ ] Define which bootstrap work is critical for first paint versus safe to defer.
- [ ] Identify the exact frontend stores, services, commands, and Rust file-IO paths that must change together.
- [ ] Keep artifact tags, validation hydration, dataset identity, and legacy artifact aliases behaviorally unchanged unless a narrower follow-up doc explicitly widens scope.
- [ ] Add or strengthen tests around latest-run hydration, history listing, and reopen behavior under larger fixture sets.
- [ ] Define a rollback trigger if the new fast path or index becomes stale or harder to reason about than the current scan.
- [ ] Record any required cross-doc sync in the first-tier artifact/history and shell docs if operator behavior changes.

## Verification Plan

- rerun the startup and history/open baseline scenarios after implementation and compare against the recorded baseline
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- rerun targeted artifact/history and hydration tests near the touched files
- rerun `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` if startup/open/reopen timing boundaries change
- rerun `node scripts/run_output_parity_check.mjs` if artifact payload or report-path behavior changes
- verify every changed surface against the repo minimum bar and update all linked docs together
- capture concrete acceptance evidence for closure, including saved baseline timings and any representative screenshots or trace artifacts used to validate the new path

## Rollout / Fallback / Compatibility Notes

- land the measurement harness first if the current timing boundaries are not already explicit enough
- prefer additive command splitting or summary indexes over schema rewrites
- keep the current full-scan path available as a fallback until the fast path proves trustworthy
- do not widen this epic into dense-table rendering or validation/report throughput beyond the points where they directly block artifact/history latency

## Related Docs

- `REPO_Improvement/06_artifacts/03_artifact_library_index_2026-04-09.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-09.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/28_workflow_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/43_output_parity_pack_2026-04-09.md`
