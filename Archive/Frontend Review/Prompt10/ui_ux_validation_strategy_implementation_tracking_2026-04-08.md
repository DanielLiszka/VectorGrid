# UI/UX Validation Strategy Implementation Tracking

Created: 2026-04-08

Prompt source: [ui_ux_validation_strategy_FINAL.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Prompt10/ui_ux_validation_strategy_FINAL.md)

Scope: grounded Prompt 10 implementation only. This tracker converts the repo-backed Prompt 10 findings into a practical verification implementation plan for frontend UI/UX modernization work.

Status: Implemented on 2026-04-08. Objective checkpoint complete. Optional interpretive follow-up remains deferred.

Owner posture: objective-first, layered, low-maintenance, browser-vs-desktop separated, anti-snapshot-sprawl.

## Purpose

Turn the grounded Prompt 10 findings into a detailed implementation plan that:

- keeps the current frontend verification base intact
- preserves the distinction between browser-side UX verification and real desktop-shell verification
- adds the missing deterministic browser interaction layer without creating a second giant QA framework
- keeps the desktop smoke harness focused on native-shell integrity
- keeps broad baseline capture useful as evidence and coarse performance review
- makes canonical UX evidence explicit for future shell/forms/results modernization passes
- avoids turning routine frontend work into screenshot upkeep or flaky native orchestration

This is not a full QA platform rewrite, not a live-service integration test expansion, and not a reason to duplicate the same workflow path across component, browser, and desktop depth.

## Verification Posture

This implementation must be additive and maintenance-aware.

- Do not remove the current baseline frontend commands from [package.json](C:/Rust%20Projects/ta_desktop_demo/web/package.json).
- Do not replace the current browser accessibility smoke in [check-a11y-smoke.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/check-a11y-smoke.mjs).
- Do not replace the current broad screenshot/perf capture in [capture-baseline-ui.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/capture-baseline-ui.mjs).
- Do not expand [wdio.desktop.conf.ts](C:/Rust%20Projects/ta_desktop_demo/web/wdio.desktop.conf.ts) into a general-purpose browser interaction runner.
- Do not make desktop smoke depend on live market data, broker flows, or unstable local profile state.
- Do not introduce broad pixel-golden gating for every workspace, tab, and shell state.
- Do not duplicate the same UX behavior at component, browser, and desktop depth unless each layer is proving something different.
- Do not turn changed-surface manual review into a new blocking repo policy during the objective pass.

## Progress Ledger

- [x] Phase 0 complete: current verification baseline and boundary audit finished
- [x] Phase 1 complete: browser interaction smoke foundation landed
- [x] Phase 2 complete: first canonical browser UX smoke specs landed
- [x] Phase 3 complete: docs, support helpers, and maintenance boundaries landed
- [x] Phase 4 complete: objective verification and evidence refresh landed
- [x] Objective checkpoint complete and ready for clean commit
- [x] Optional interpretive follow-up reviewed
- [x] Targeted verification complete
- [x] Final evidence and documentation attached

## Verification Notes

- New browser UX layer landed in:
  - [playwright.config.ts](C:/Rust%20Projects/ta_desktop_demo/web/playwright.config.ts)
  - [README.md](C:/Rust%20Projects/ta_desktop_demo/web/e2e/browser/README.md)
  - [mockTauri.ts](C:/Rust%20Projects/ta_desktop_demo/web/e2e/browser/support/mockTauri.ts)
  - [appHarness.ts](C:/Rust%20Projects/ta_desktop_demo/web/e2e/browser/support/appHarness.ts)
  - [shell-ux.spec.ts](C:/Rust%20Projects/ta_desktop_demo/web/e2e/browser/shell-ux.spec.ts)
  - [command-palette-overlays.spec.ts](C:/Rust%20Projects/ta_desktop_demo/web/e2e/browser/command-palette-overlays.spec.ts)
- Existing browser scripts remained in place and unchanged in role:
  - [check-a11y-smoke.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/check-a11y-smoke.mjs)
  - [capture-baseline-ui.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/capture-baseline-ui.mjs)
- Desktop boundary documentation was updated in [README.md](C:/Rust%20Projects/ta_desktop_demo/web/e2e/desktop/README.md).
- Manual browser QA artifacts were captured at:
  - `scripts/baselines/ui_capture/prompt10_browser_shell_manual.png`
  - `scripts/baselines/ui_capture/prompt10_browser_task_center_manual.png`
- Automated verification that passed on the final implementation state:
  - `npm --prefix web run lint`
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test`
  - `npm --prefix web run build`
  - `npm --prefix web run check:bundle-budget`
  - `npm --prefix web run parity`
  - `npm --prefix web run a11y:smoke`
  - `npm --prefix web run baseline:ui`
  - `npm --prefix web run test:browser:ux`
  - `npm --prefix web run test:desktop:smoke`
- Desktop smoke note:
  - the suite passed cleanly, but the run still reported the familiar post-suite preview-port collision on `1420`; this did not invalidate the passing desktop results.

## Implementation Outcome Target

At the end of the objective checkpoint:

- the repo still has the current baseline frontend verification commands
- the repo still has the current broad accessibility smoke script
- the repo still has the current broad screenshot/perf capture script
- the repo still has the current real-shell desktop smoke harness
- the repo also has one small deterministic browser interaction suite in mocked-Tauri mode
- browser and desktop responsibilities are documented clearly enough that future UI work does not drift between them
- canonical UX evidence paths are explicit for shell navigation, command palette, overlays, and one representative dense-surface interaction
- the verification stack stays small enough to run routinely during UI-heavy work

## Checkpoint Strategy

### Checkpoint A — Objective Improvements

Land only the grounded, high-confidence changes first:

- audit the current verification layers and their distinct responsibilities
- add one small browser interaction suite in mocked-Tauri mode
- add a new npm command for that suite
- add the first canonical browser interaction specs
- keep desktop smoke thin and native-specific
- keep the current browser a11y and baseline scripts in their current broad roles
- document the browser-vs-desktop boundary clearly
- verify the whole stack together

Checkpoint A should be suitable for a clean commit on its own.

### Checkpoint B — Optional Interpretive Follow-Up

Only after Checkpoint A is stable:

- decide the exact long-term spec-count budget for browser UX smoke
- decide whether any ARIA snapshots are worth keeping
- decide whether any screenshot assertions are justified
- decide whether a PR template or checklist should enforce changed-surface manual review
- decide whether `baseline:ui` should remain purely artifact-driven or gain any policy-level gating

Checkpoint B should stay easy to split, stash, or revert.

## Objective-First Execution Rule

Execute this work in the following order:

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. Objective verification
7. Clean objective commit checkpoint
8. Optional interpretive follow-up

Do not widen the objective pass into generalized native QA, live-service UX testing, or screenshot-golden expansion.

## Source Basis

- [ui_ux_validation_strategy_FINAL.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Prompt10/ui_ux_validation_strategy_FINAL.md)
- [package.json](C:/Rust%20Projects/ta_desktop_demo/web/package.json)
- [check-a11y-smoke.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/check-a11y-smoke.mjs)
- [capture-baseline-ui.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/capture-baseline-ui.mjs)
- [wdio.desktop.conf.ts](C:/Rust%20Projects/ta_desktop_demo/web/wdio.desktop.conf.ts)
- [README.md](C:/Rust%20Projects/ta_desktop_demo/web/e2e/desktop/README.md)
- [tauri_desktop_e2e_plan_2026-03-29.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Unfinished/tauri_desktop_e2e_plan_2026-03-29.md)
- [VueWorkspaceShell.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/VueWorkspaceShell.vue)
- [CommandPaletteOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/CommandPaletteOverlay.vue)
- [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue)
- [TopWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TopWorkspacePanel.vue)
- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue)
- [ContextActionMenu.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ContextActionMenu.vue)

## Execution Guardrails

- Keep each verification layer proving something unique.
- Keep browser interaction smoke deterministic and mocked-Tauri.
- Keep desktop smoke focused on native-shell integrity only.
- Keep broad accessibility scan separate from interaction semantics.
- Keep broad screenshot/perf capture separate from pass/fail interaction proof.
- Keep the default green path small enough for routine local use.
- Prefer targeted coverage on changed surfaces over blanket verification growth.
- Avoid policy decisions about brittle screenshot gating during the objective pass.

## Explicit Working Decisions

### 1. Baseline frontend gates stay in place

- Keep `lint`, `typecheck`, `test`, `build`, `check:bundle-budget`, and `parity`.
- Treat them as required prerequisites for UI-heavy work, not as sufficient UX proof on their own.

### 2. Broad browser accessibility smoke stays broad

- Keep [check-a11y-smoke.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/check-a11y-smoke.mjs) as the broad shell-level accessibility scan.
- Do not overload it with complex user-interaction flows.

### 3. Broad screenshot/perf capture stays artifact-oriented

- Keep [capture-baseline-ui.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/capture-baseline-ui.mjs) as the broad screenshot and coarse performance artifact generator.
- Do not expand it into a universal screenshot-golden matrix in the objective pass.

### 4. Desktop smoke stays native-specific

- Keep [wdio.desktop.conf.ts](C:/Rust%20Projects/ta_desktop_demo/web/wdio.desktop.conf.ts) as the real-shell smoke owner.
- Do not shift general browser interaction coverage into WDIO.

### 5. Add one small deterministic browser interaction layer

- Add one small browser interaction suite in mocked-Tauri mode.
- Use the existing Playwright ecosystem already present in the repo rather than introducing another browser framework.
- Keep the first slice intentionally small and shell/overlay focused.

### 6. Changed-surface component tests continue to cover exact local interaction contracts

- Keep detailed keyboard, focus-return, and selection-state assertions near the changed components, stores, and composables when those surfaces already have precise local coverage.
- Use browser smoke for app-level journey proof that component tests do not express as cleanly.

## Expected Edit Inventory

Primary expected edit targets:

- [package.json](C:/Rust%20Projects/ta_desktop_demo/web/package.json)
- `web/playwright.config.ts`
- `web/e2e/browser/`
- `web/e2e/browser/support/mockTauri.ts`
- [README.md](C:/Rust%20Projects/ta_desktop_demo/web/e2e/desktop/README.md)

Likely review-only or narrow-touch targets:

- [check-a11y-smoke.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/check-a11y-smoke.mjs)
- [capture-baseline-ui.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/capture-baseline-ui.mjs)
- [wdio.desktop.conf.ts](C:/Rust%20Projects/ta_desktop_demo/web/wdio.desktop.conf.ts)
- [VueWorkspaceShell.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/VueWorkspaceShell.vue)
- [CommandPaletteOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/CommandPaletteOverlay.vue)
- [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue)
- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue)
- [TopWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TopWorkspacePanel.vue)

## Implementation Phases

## Phase 0 — Baseline Audit And Boundary Lock

Goal: record the current verification stack and freeze the responsibility boundaries before adding anything new.

Checklist:

- [x] Record the current baseline frontend commands in [package.json](C:/Rust%20Projects/ta_desktop_demo/web/package.json)
- [x] Record the current browser accessibility smoke scope in [check-a11y-smoke.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/check-a11y-smoke.mjs)
- [x] Record the current screenshot/perf artifact scope in [capture-baseline-ui.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/capture-baseline-ui.mjs)
- [x] Record the current real-shell desktop smoke suite in [wdio.desktop.conf.ts](C:/Rust%20Projects/ta_desktop_demo/web/wdio.desktop.conf.ts)
- [x] Confirm that `web/e2e` currently contains desktop-only coverage and no browser interaction suite
- [x] Record the highest-value changed-surface component test targets:
  - shell/navigation
  - command palette
  - task center
  - settings diagnostics
  - dense results or form surfaces
- [x] Record the current duplication of mocked-Tauri bootstrap logic between browser-side scripts
- [x] Capture the current boundary explicitly:
  - component tests prove exact interaction contracts
  - browser smoke proves deterministic UX journeys
  - desktop smoke proves real native shell integrity
  - baseline capture produces artifact/perf evidence

Acceptance criteria:

- [x] The repo’s current verification layers are explicitly documented
- [x] The browser-vs-desktop boundary is clear before implementation starts
- [x] The missing browser interaction layer is defined narrowly enough to implement without scope drift

## Phase 1 — Browser Interaction Smoke Foundation

Goal: add the minimum browser-side runner/config/support needed for deterministic UX smoke in mocked-Tauri mode.

Primary files:

- [package.json](C:/Rust%20Projects/ta_desktop_demo/web/package.json)
- `web/playwright.config.ts`
- `web/e2e/browser/support/mockTauri.ts`

Checklist:

- [x] Add one browser UX command to [package.json](C:/Rust%20Projects/ta_desktop_demo/web/package.json), for example `test:browser:ux`
- [x] Add a Playwright Test config under `web/`
- [x] Keep the browser interaction suite using mocked-Tauri mode instead of real desktop runtime
- [x] Extract reusable mocked-Tauri install/bootstrap helpers into `web/e2e/browser/support/mockTauri.ts`
- [x] Reuse the current mocked-Tauri assumptions from:
  - [check-a11y-smoke.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/check-a11y-smoke.mjs)
  - [capture-baseline-ui.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/capture-baseline-ui.mjs)
- [x] Keep the foundation deterministic:
  - no live services
  - no broker dependencies
  - no native-shell driver requirements
- [x] Keep artifacts/logging simple and local to the browser UX suite

Acceptance criteria:

- [x] One repo-local command runs the browser UX smoke suite
- [x] The suite can boot the built frontend in mocked-Tauri mode deterministically
- [x] Mocked-Tauri support logic is not duplicated across every new spec

## Phase 2 — Canonical Browser UX Smoke

Goal: add the first small set of canonical browser interaction journeys.

Primary files:

- `web/e2e/browser/shell-ux.spec.ts`
- `web/e2e/browser/command-palette-overlays.spec.ts`
- optional `web/e2e/browser/dense-surface-ux.spec.ts`

Checklist:

- [x] Add one shell navigation browser spec proving:
  - app reaches an interactive shell
  - keyboard user can move across main shell tabs
  - orientation is preserved
  - no keyboard trap appears
- [x] Add one command-palette and overlay browser spec proving:
  - command palette open
  - filtering
  - keyboard movement through results
  - safe command execution
  - close and focus restoration
  - task-center open/close and focus restoration
- [x] Dense-surface browser spec intentionally deferred for this slice because Prompt 10 adds verification infrastructure rather than changing forms or results
- [x] Keep browser assertions focused on:
  - behavior
  - visibility
  - focus
  - reachability
  - stable ARIA state
- [x] Do not add broad screenshot-golden assertions in this phase
- [x] Keep browser UX smoke intentionally small and shell/overlay centered

Acceptance criteria:

- [x] The browser suite covers at least the shell navigation path and command-palette/overlay focus path
- [x] Browser UX smoke proves behavior that current component/unit tests and desktop smoke do not already prove at the same depth
- [x] The suite remains deterministic and fast enough for routine use

## Phase 3 — Docs, Boundaries, And Maintenance Rules

Goal: make the layered strategy discoverable and prevent future test sprawl.

Primary files:

- [README.md](C:/Rust%20Projects/ta_desktop_demo/web/e2e/desktop/README.md)
- optional `web/e2e/browser/README.md`
- [package.json](C:/Rust%20Projects/ta_desktop_demo/web/package.json)

Checklist:

- [x] Update [README.md](C:/Rust%20Projects/ta_desktop_demo/web/e2e/desktop/README.md) to document the browser-vs-desktop verification boundary explicitly
- [x] Document what desktop smoke should continue to own:
  - launch and readiness
  - native shell presence
  - workspace navigation in real shell
  - command palette reachability in real shell
  - task center reachability in real shell
  - settings diagnostics in real shell
- [x] Document what browser UX smoke should own:
  - deterministic interaction journeys
  - keyboard and focus verification at app level
  - mocked-Tauri shell behavior
- [x] Document what remains component/unit-test responsibility:
  - exact interaction contracts
  - state-driven rendering rules
  - overlay logic
  - selection logic
  - menu behavior
- [x] Document the “do not duplicate layers” rule
- [x] Document the “changed-surface evidence over blanket expansion” rule

Acceptance criteria:

- [x] Future contributors can tell where a new UX regression test belongs
- [x] Browser and desktop suites have clearly separated responsibilities
- [x] The strategy discourages redundant verification growth

## Phase 4 — Verification And Evidence Refresh

Goal: verify the new layered stack as a coherent repo verification surface.

Checklist:

- [x] Run frontend verification:
  - `npm --prefix web run lint`
  - `npm --prefix web run typecheck`
  - `npm --prefix web run test`
  - `npm --prefix web run build`
  - `npm --prefix web run check:bundle-budget`
  - `npm --prefix web run parity`
- [x] Run existing browser verification:
  - `npm --prefix web run a11y:smoke`
  - `npm --prefix web run baseline:ui`
- [x] Run the new browser interaction smoke command
- [x] Run desktop verification:
  - `npm --prefix web run test:desktop:smoke`
- [x] Capture final verification notes for:
  - shell navigation
  - command palette
  - overlay focus return
  - dense-surface path if applicable
  - desktop shell readiness
- [x] Confirm no existing verification layer had to absorb responsibilities that belong elsewhere

Acceptance criteria:

- [x] All required verification layers pass or any unrelated failures are documented explicitly
- [x] The repo now has deterministic browser interaction proof in addition to component and desktop layers
- [x] Evidence remains credible without excessive maintenance overhead

## Objective Verification Checklist

- [x] Baseline frontend commands remain intact
- [x] Broad accessibility smoke remains intact
- [x] Broad screenshot/perf capture remains intact
- [x] Desktop smoke remains intact and native-specific
- [x] One deterministic browser interaction suite now exists
- [x] Browser and desktop verification boundaries are documented clearly
- [x] No broad screenshot-golden matrix was introduced
- [x] No live-service or broker dependency was added to default UX verification

## Initial Browser UX Evidence Checklist

This section tracks the first implementation slice for the missing browser interaction layer. It is an implementation target, not a permanent repo-law workflow taxonomy.

- [x] Launch to interactive shell has an automated evidence path
- [x] Keyboard shell navigation has an automated evidence path
- [x] Command palette open/filter/execute/close/focus-return has an automated evidence path
- [x] Task-center open/close/focus-return has an automated evidence path
- [x] Dense workflow interaction remains intentionally out of scope for this slice because no forms/results UI changed
- [x] Desktop-native operator path has an automated evidence path in the real shell

## Targeted Test Additions Checklist

- [x] Add browser interaction coverage for shell navigation
- [x] Add browser interaction coverage for command palette and focus return
- [x] Add browser interaction coverage for task-center overlay focus return
- [x] Dense-surface browser interaction coverage intentionally deferred until a forms/results modernization slice warrants it
- [x] No new changed-surface component tests were required because Prompt 10 added verification infrastructure rather than changing UI behavior in those components
  - shell navigation and ARIA state
  - command palette filtering and execution
  - task-center visibility and focus return
  - settings diagnostics interaction
  - dense form/results logic where changed
- [x] Keep desktop smoke additions limited to native-only gaps

## Recommended Manual Review Checklist

Reference checklist only. This is useful operating guidance, but it is not an objective acceptance gate for Prompt 10 implementation.

- [x] Review changed surfaces for visible focus state
- [x] Review focus return after dialogs, overlays, and palettes close
- [x] Review changed surfaces for keyboard traps
- [x] Review changed surfaces for contrast regressions
- [x] Review changed surfaces for primary/secondary action discoverability
- [x] Review that baseline artifacts remain useful for broad UI inspection

## Guardrail Review Checklist

- [x] No duplicated full workflow replay was added across all verification layers
- [x] No broad screenshot-golden gate was added
- [x] No desktop smoke expansion into general browser UX logic landed
- [x] No live-service-dependent default UX verification landed
- [x] No unstable native-driver complexity was added beyond the current desktop smoke harness
- [x] No verification layer lost its distinct job

## Defers And Rejects

- [x] Defer broad screenshot assertion coverage for every workspace/tab/state
- [x] Defer broad ARIA snapshot expansion
- [x] Defer heavy deterministic desktop seeding unless a real native-only case requires it
- [x] Defer desktop smoke expansion into general form/results interaction verification
- [x] Reject turning [capture-baseline-ui.mjs](C:/Rust%20Projects/ta_desktop_demo/web/scripts/capture-baseline-ui.mjs) into a universal blocking gate in the objective pass
- [x] Reject adding another browser framework when the repo already ships Playwright tooling
- [x] Reject using live broker/data flows as default UX verification proof

## Optional Interpretive Follow-Up Checklist

- [ ] Decide the exact long-term browser UX suite size budget
- [ ] Decide whether any stable ARIA snapshots are worth keeping
- [ ] Decide whether any stable screenshot assertions are justified
- [ ] Decide whether changed-surface manual review should be enforced through PR templates or repo checklists
- [ ] Decide whether `baseline:ui` should remain purely artifact-oriented or gain any formal policy weighting
- [ ] Decide whether any geometry or richer desktop-native cases should move into default smoke later
- [ ] Decide whether to formalize component/unit coverage as the primary proof layer in repo policy instead of treating it as current preferred practice
- [ ] Decide whether to standardize a larger canonical UX workflow set beyond the initial shell, palette, overlay, and representative dense-surface paths

## Final Pass/Fail Criteria

Pass if:

- the repo has a layered verification model with clearly distinct responsibilities
- the missing deterministic browser interaction layer exists
- desktop smoke stays thin and native-specific
- broad accessibility and broad screenshot/perf evidence remain useful
- the strategy is practical enough to sustain during ongoing UI modernization

Fail if:

- the new verification layer duplicates existing component or desktop proof without adding unique evidence
- desktop smoke starts absorbing browser interaction logic
- screenshot maintenance grows faster than UX confidence
- contributors need too many heavy commands to validate ordinary UI changes
