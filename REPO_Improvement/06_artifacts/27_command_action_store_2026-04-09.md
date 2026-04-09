# Command Action Store Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the shared command-action store that bridges command palette, shortcuts, titlebar/footer actions, context docks, and latest-report opening behavior across the non-options shell.

## Scope Boundaries

In scope:

- `web/src/stores/commandActionStore.ts`
- `web/src/stores/commandActionStore.test.ts`
- components that consume the store:
  - `web/src/components/CommandPaletteOverlay.vue`
  - `web/src/components/GlobalShortcutBridge.vue`
  - `web/src/components/DesktopTitlebar.vue`
  - `web/src/components/RunFooterPanel.vue`
  - `web/src/components/OptimizeContextDock.vue`
  - `web/src/components/AnalysisContextDock.vue`

Out of scope:

- redesigning the shell command system from scratch
- options-related quick actions
- changing the underlying optimize/data/analysis/validation services beyond their current command surface

## Current User-Facing Behavior

The command-action store already acts as a shared action router for multiple entry points:

- command palette actions
- global shortcut handlers
- titlebar run/cancel affordances
- run-footer actions
- optimize and analysis context-dock actions
- open-latest-report and open-latest-meta-report affordances

It also carries path precedence logic for latest reports, including fallback to workflow artifact state and legacy runtime snapshots.

## Current Code Surface

- `web/src/stores/commandActionStore.ts`
- `web/src/stores/commandActionStore.test.ts`
- consuming components:
  - `CommandPaletteOverlay.vue`
  - `GlobalShortcutBridge.vue`
  - `DesktopTitlebar.vue`
  - `RunFooterPanel.vue`
  - `OptimizeContextDock.vue`
  - `AnalysisContextDock.vue`
- related stores:
  - `workflowArtifactsStore.ts`
  - `optimizeStore.ts`
  - `dataStore.ts`
  - `analysisStore.ts`
  - `validationStore.ts`

## Current Verification Surface

- `web/src/stores/commandActionStore.test.ts`
- `web/src/components/GlobalShortcutBridge.test.ts`
- `web/src/components/DesktopTitlebar.test.ts`
- `web/src/components/RunFooterPanel.test.ts`

## Strengths Already Present

- action fan-in is already centralized instead of duplicated in every component
- latest-report path precedence is explicit in one store
- the store keeps runtime-running guards around open-report actions
- tests already cover dispatch and latest-report fallback behavior
- multiple UI surfaces already depend on the same store, which encourages consistency

## Weaknesses / Failure Modes

- the store is a high-fan-out surface, so a small change can affect many UI entry points
- latest-report and latest-meta-report path semantics rely on overlapping state sources
- legacy snapshot support and Vue artifact fallback can drift if not tested together
- component affordances can diverge from command-store capabilities if the action list changes casually

## Regressions To Avoid

- breaking one action surface while another still appears to work
- changing latest-report precedence without rechecking all callers
- allowing open-report actions while the runtime is still running
- duplicating action-routing logic back into components
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the command-action store is a classic shared state/action router rather than component-local logic.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because the store sits at the center of many UI entry points and needs strong isolated tests.
- [Tauri event namespace](https://v2.tauri.app/es/reference/javascript/api/namespaceevent/) is relevant because legacy quick-action snapshots and runtime-driven updates are event-shaped integration points in the desktop shell.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because many command-store actions ultimately resolve into Tauri command invocations.
- [WAI-ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/) are relevant because command palette and shortcut-driven actions should stay consistent and understandable across multiple entry points.
- [JSON Schema: What is JSON Schema?](https://json-schema.org/overview/what-is-jsonschema)
  - relevant because artifacts, reports, manifests, and replay payloads behave like versioned serialized contracts that should stay explicit and reviewable

## Proposed Improvement Approach

1. Treat `commandActionStore` as the canonical action router for shared shell commands.
2. Document each action in terms of:
   - upstream service/store target
   - allowed entry surfaces
   - gating conditions
   - report-path or runtime-path dependencies
3. Freeze the latest-report precedence order in tests:
   - local store state
   - workflow artifacts fallback
   - root runtime dataset
4. Keep components thin so all meaningful action logic stays in the store.
5. Audit whether any current command belongs elsewhere because it is too domain-specific for the shared router.
6. Treat artifact-surface code quality as part of the implementation scope: keep persisted shapes version-conscious, replayable, diagnosable, and obvious to future contributors.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document every action exported by `commandActionStore`.
- [ ] Document and test latest-report and latest-meta-report precedence rules.
- [ ] Add or tighten tests for runtime-running guards on open-report actions.
- [ ] Audit each consuming component to ensure it delegates instead of duplicating logic.
- [ ] Decide whether any current action should be split into a more domain-local store instead of expanding this one indefinitely.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `commandActionStore.test.ts`
- run `GlobalShortcutBridge.test.ts`
- run `DesktopTitlebar.test.ts`
- run `RunFooterPanel.test.ts`
- manually sanity-check command palette and report-opening flows if action routing changes materially
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant quality and parity gates for touched artifact/report surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the applicable output-parity or artifact-shape checks

## Rollout / Fallback / Compatibility Notes

- preserve current action names and entry surfaces unless a compatibility reason forces change
- keep legacy snapshot support until the repo fully removes that dependency across the shell
- if action centralization becomes too broad, split carefully by domain instead of reintroducing ad hoc duplication
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/25_command_palette_store_2026-04-08.md`
- `REPO_Improvement/01_shell/33_shell_task_lifecycle_summary_2026-04-08.md`
- `REPO_Improvement/04_results/16_report_path_reopen_flow_2026-04-08.md`
- `REPO_Improvement/06_artifacts/29_artifact_identity_workflow_model_2026-04-08.md`
