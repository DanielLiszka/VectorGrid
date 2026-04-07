# Execution Settings And Operator-Safety Implementation Tracking

Created: 2026-04-06

Prompt source: [execution_settings_operator_safety_direction_FINAL.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Prompt6/execution_settings_operator_safety_direction_FINAL.md)

Scope: Prompt 6 objective implementation only. This tracker converts the grounded parts of the Prompt 6 guide into an execution plan without reopening broader shell, product-structure, or backend-contract questions.

Status: Objective checkpoint implemented and verified on 2026-04-07.

Owner posture: Objective-first, functionality-preserving, operator-safety-biased.

## Purpose

Turn the grounded Prompt 6 execution/settings/operator-safety guidance into a detailed implementation plan that:

- preserves all backend-backed execution and runtime capabilities
- closes the remaining high-stakes broker-armed one-shot safety gap
- makes execution posture more readable before action
- keeps Settings as the canonical diagnostics/utilities home
- keeps the Task Center and runtime banner within their existing ownership boundaries

This is not a control-surface redesign. It is a tightening pass on execution safety, posture legibility, diagnostics hierarchy, and operator confidence.

## Functionality Posture

This implementation must be additive and capability-preserving.

- Do not remove `Test account`, `Execute once`, `Refresh orders`, live feed controls, runner controls, or key-management controls.
- Do not move broker-facing controls out of [ExecutionWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.vue).
- Do not duplicate broker-facing controls into [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue).
- Do not broaden [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) into a general alert rail.
- Do not turn [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue) into a diagnostics console.
- Preserve existing keyboard-safe and focus-safe behavior for confirmations, Settings diagnostics actions, and the Task Center.
- Preserve the current high-signal, background-only routing posture in [desktopNotificationPolicy.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/desktopNotificationPolicy.ts).

## Progress Ledger

- [x] Phase 0 complete: baseline audit and evidence capture finished
- [x] Phase 1 complete: capability, ownership, and derived-state audit landed
- [x] Phase 2 complete: execution posture and action-weighting improvements landed
- [x] Phase 3 complete: high-stakes confirmation and blocked-reason improvements landed
- [x] Phase 4 complete: settings diagnostics and runtime-sentinel improvements landed
- [x] Objective checkpoint complete and ready for clean commit
- [x] Optional interpretive follow-up reviewed
- [x] Targeted verification complete
- [x] Final evidence and screenshots attached

## Implementation Notes (2026-04-06)

- [x] Objective Prompt 6 implementation landed in [ExecutionWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.vue) and [executionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.ts), with no backend or Tauri command-surface changes required.
- [x] Targeted test coverage landed in [ExecutionWorkspacePanel.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.test.ts), [executionStore.actions.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.actions.test.ts), and [domainStores.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/domainStores.test.ts) to align legacy-bridge expectations with the new derived execution availability model.
- [x] Verification-required generated frontend registry output was refreshed in [commands.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/tauri/commands.ts) because `lint` was blocked by a stale command map from recent repo changes unrelated to Prompt 6.
- [x] Settings diagnostics, runtime-banner scope, Task Center focus behavior, and desktop-notification routing were reviewed against live code and existing tests; no objective source edits were required in [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue), [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue), [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue), or [desktopNotificationPolicy.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/desktopNotificationPolicy.ts).
- [x] Evidence artifacts were added under `scripts/baselines/ui_capture`, including `prompt6_execution_paper_dryrun_manual.png`, `prompt6_execution_paper_broker_manual.png`, `prompt6_execution_live_broker_manual.png`, and `prompt6_settings_diagnostics_manual.png`.
- [x] Runtime-banner error escape-hatch actions and execution-specific Task Center jump-back actions were left deferred in the optional section, matching this tracker’s narrowed objective scope.

## Implementation Outcome Target

At the end of the objective pass:

- the execution surface should make the current operator posture readable before any broker-facing action
- broker-armed one-shot execution should no longer be a zero-friction path
- routine review actions should remain fast and unburied
- live/run gating should be explained in readable UI state, not only in disabled-button behavior
- Settings diagnostics should remain summary-first, with logs/timeline/alerts clearly subordinate
- the runtime banner should remain boot/error scoped during the objective checkpoint, with any clearer error escape hatch treated as optional follow-up
- shared tone vocabulary should remain aligned with existing status/task presentation helpers
- the existing desktop-notification routing model should remain unchanged

## Checkpoint Strategy

### Checkpoint A — Objective Improvements

Land only the grounded, high-confidence changes first:

- execution posture clarity
- broker-armed one-shot safety
- readable blocked reasons
- settings diagnostics hierarchy tightening
- verification and evidence capture

This checkpoint should be suitable for a clean commit on its own.

### Checkpoint B — Optional Interpretive Follow-Up

Only after Checkpoint A is complete should the repo consider:

- wording/model shifts such as `Start live` versus `Start feed`
- confirmation-scope refinements beyond the proven safety gaps
- execution-specific task-center shortcuts
- exact long-term execution severity/presenter abstractions
- any additional operator-console polish that is design-judgment-heavy rather than clearly grounded

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

Do not mix optional wording/model experimentation into the objective checkpoint.

## Source Basis

- [execution_settings_operator_safety_direction_FINAL.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Prompt6/execution_settings_operator_safety_direction_FINAL.md)
- [frontend_audit_2026-03-27.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/frontend_audit_2026-03-27.md)
- [frontend_target_comps_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/frontend_target_comps_2026-03-28.md)
- [frontend_design_system_spec_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/frontend_design_system_spec_2026-03-28.md)
- [frontend_functional_coverage_matrix_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/frontend_functional_coverage_matrix_2026-03-28.md)
- [REPO_MAP.md](C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md)
- [App.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/App.vue)
- [ExecutionWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.vue)
- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue)
- [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)
- [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue)
- [executionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.ts)
- [taskStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/taskStore.ts)
- [executionService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/executionService.ts)
- [settingsService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/settingsService.ts)
- [statusPresentation.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/statusPresentation.ts)
- [desktopNotificationPolicy.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/desktopNotificationPolicy.ts)

## Execution Guardrails

- Preserve all backend-backed execution and runtime capability exposure.
- Treat broker-armed one-shot execution, runner start, and key clearing as safety-sensitive surfaces.
- Keep routine actions fast: account test, order refresh, and stop actions should not become modal-heavy by default.
- Keep execution as the only broker-facing control plane.
- Keep Settings as the canonical home for runtime utilities, diagnostics review, and notification preferences.
- Keep runtime-banner scope limited to boot and runtime-error conditions.
- Keep the Task Center centered on active/recent task posture, not raw diagnostics.
- Preserve existing confirmation-dialog focus behavior and explicit cancel/confirm paths.
- Preserve existing Task Center Escape close, focus loop, and focus restoration behavior.
- Preserve the current desktop notification routing policy unless a change is explicitly justified and tested.
- Avoid backend or Tauri command-surface changes during the objective pass unless they are absolutely required by the frontend implementation.

## Explicit Working Decisions

### 1. Ownership boundary

- [ExecutionWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.vue) remains the only broker-facing control plane.
- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue) remains the canonical diagnostics and runtime-utility surface.
- [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue) remains the active/recent operational-posture surface.
- [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) remains a narrow boot/error sentinel.

### 2. High-stakes safety model

- Preserve explicit confirmation for `Clear keys`.
- Preserve explicit confirmation for existing live feed and runner start actions in the objective pass unless later optional follow-up narrows that scope.
- Add explicit confirmation for broker-armed one-shot execution.
- Do not add new confirmation friction to routine review actions such as `Test account` or `Refresh orders`.
- Do not hide high-stakes actions behind secondary menus just to reduce visible chrome.

### 3. Execution posture model

- Surface the most important current posture facts directly in the execution header region rather than relying on generic summary chips alone.
- The posture model must at minimum make broker mode, submission posture, feed state, runner state, and key-persistence state readable before action.
- Preserve the existing runtime review surface, order-activity table, and raw-output disclosure.
- Keep the chart section secondary to textual/runtime posture scanning.

### 4. Blocked-reason model

- Keep existing hard gates in [executionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.ts) and [executionService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/executionService.ts).
- Add readable preflight and blocked-reason presentation above or beside the relevant controls.
- Do not rely only on disabled booleans such as `canRunOnce` and `canStartRunner` to communicate why something is unavailable.

### 5. Settings diagnostics model

- Keep diagnostics summary-first and evidence-trail-second.
- Preserve the current strong first-class actions for opening the Task Center and copying a diagnostics snapshot.
- Keep runtime, native desktop, and task posture as the first visible diagnostic summaries.
- Keep logs, timeline, and alerts subordinate and scannable.

### 6. Runtime banner model

- Keep `booting` and `error` as the only visible banner states.
- Keep stack or raw error detail behind disclosure.
- Improve error-state escape hatches without broadening the banner into general execution/status chrome.

### 7. Shared presentation model

- Reuse [statusPresentation.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/statusPresentation.ts) where possible for tone/label vocabulary.
- Do not fork a second unrelated execution color/status language.
- Add a narrow execution-specific presenter helper only if duplication becomes concrete during implementation.

## Implementation Phases

## Phase 0 — Baseline Audit And Evidence Capture

Goal: record the current operator flow and safety surface before any implementation changes land.

Primary files:

- [ExecutionWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.vue)
- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue)
- [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)
- [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue)
- [executionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.ts)
- [desktopNotificationPolicy.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/desktopNotificationPolicy.ts)

Checklist:

- [x] Capture current screenshots for execution in paper + dry-run posture
- [x] Capture current screenshots for execution in paper + broker-armed posture
- [x] Capture current screenshots for execution in live + broker-armed posture
- [ ] Capture current screenshots for live feed active with symbol mismatch posture if reproducible
- [ ] Capture current screenshots for runner-active posture if reproducible
- [x] Capture current screenshots for Settings diagnostics default state
- [ ] Capture current screenshots for runtime-banner error state if reproducible
- [x] Record the current visible action groups and button labels in Execution
- [x] Record which execution actions currently require confirmation and which do not
- [x] Record the current blocked-action model for `Execute once`, `Start live`, and `Start runner`
- [x] Record the current diagnostics-card order and action order in Settings
- [x] Record the current Task Center related-action map and confirm that execution is not currently covered
- [x] Record the current desktop-notification routing rules from [desktopNotificationPolicy.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/desktopNotificationPolicy.ts)

Acceptance criteria:

- [x] Baseline screenshots exist for the key operator states that this pass will change
- [x] Current confirmation coverage is documented
- [x] Current diagnostics ordering and runtime-banner scope are documented
- [x] Current notification-routing behavior is documented

## Phase 1 — Capability, Ownership, And Derived-State Audit

Goal: identify what execution/runtime facts already exist in the store/service layer so the implementation adds presentation clarity rather than backend churn.

Primary files:

- [ExecutionWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.vue)
- [executionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.ts)
- [executionService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/executionService.ts)
- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue)
- [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)
- [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue)
- [statusPresentation.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/statusPresentation.ts)

Checklist:

- [x] Audit which posture facts already exist in state or service output: broker mode, dry-run state, live feed active state, live symbol, delayed/realtime state, runner active state, and persisted-key state
- [x] Audit the exact gating logic for `canRunOnce`, `canStartLive`, and `canStartRunner`
- [x] Audit whether current service/store code already exposes enough information to derive readable blocked reasons without backend contract changes
- [x] Audit where optimistic one-shot order rows are introduced so confirmation work does not disturb existing feedback behavior
- [x] Audit the current diagnostics summary/evidence split in Settings and confirm what already matches the desired hierarchy
- [x] Audit the runtime-banner error detail path and identify the narrowest handoff needed for diagnostics open/copy behavior
- [x] Audit the current shared tone helpers in [statusPresentation.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/statusPresentation.ts)
- [x] Confirm no Prompt 5 or earlier shell/history changes need to be reopened to complete Prompt 6
- [x] Avoid backend or Tauri command changes during this phase

Acceptance criteria:

- [x] The implementation knows which posture facts can be derived without new backend data
- [x] The one-shot confirmation gap is traced to concrete current code paths
- [x] The diagnostics-card ordering baseline is confirmed
- [x] The runtime-banner handoff path is scoped narrowly enough to avoid banner expansion

## Phase 2 — Execution Posture And Action-Weighting Improvements

Goal: make the execution surface read like an operator console before action, without removing any current capabilities.

Primary files:

- [ExecutionWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.vue)
- [executionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.ts)
- [executionService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/executionService.ts)

Checklist:

- [x] Replace or reweight the current generic execution-summary chips so the top-of-panel posture communicates safety-relevant state first
- [x] Make broker mode explicitly readable as broker posture, not just generic `Mode`
- [x] Surface a readable submission posture so dry-run versus broker-armed state is obvious before action
- [x] Surface readable feed posture including active/off and symbol or match state where available
- [x] Surface readable runner posture including active/off state
- [x] Surface readable key-persistence state rather than leaving `Remember keys` as the only signal
- [x] Keep routine review actions clearly grouped and visible
- [x] Keep manual one-shot execution clearly separated from automation controls
- [x] Keep destructive key clearing visually separated from routine controls
- [x] Preserve the runtime status grid, order-activity table, and raw-output disclosure
- [x] Keep the chart review area subordinate to textual/runtime posture scanning
- [x] Avoid growing the execution header into decorative chrome or a second diagnostics wall

Acceptance criteria:

- [x] An operator can identify broker mode, broker-armed versus dry-run posture, feed state, runner state, and key state without scanning the full form body
- [x] The execution surface still exposes every current backend-backed action
- [x] Routine review actions remain fast to reach
- [x] The execution review pane still reads as runtime/table/troubleshooting, not chart-first

## Phase 3 — High-Stakes Confirmation And Blocked-Reason Improvements

Goal: close the largest remaining safety gap and make execution gating understandable before click.

Primary files:

- [ExecutionWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.vue)
- [executionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.ts)
- [executionService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/executionService.ts)
- [ConfirmationDialog.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ConfirmationDialog.vue)

Checklist:

- [x] Add explicit confirmation for broker-armed one-shot execution
- [x] Keep explicit confirmation for `Clear keys`
- [x] Preserve existing confirmation behavior for live feed start and runner start during the objective pass unless a narrower scope is unambiguously required
- [x] Keep `Test account` and `Refresh orders` as non-modal routine actions
- [x] Keep `Stop live` and `Stop runner` direct unless a concrete safety issue is discovered
- [x] Make the one-shot action label reflect actual consequence in current posture
- [x] Add readable preflight reasons for blocked or unsafe execution/automation states
- [x] Surface reasons such as missing credentials, inactive feed, feed-symbol mismatch, or runner already active where applicable
- [x] Preserve current store/service hard gates underneath the new presentation layer
- [x] Preserve optimistic broker-order feedback behavior after broker-armed one-shot submission
- [x] Preserve existing confirmation-dialog keyboard and focus-return behavior

Acceptance criteria:

- [x] Broker-armed one-shot execution cannot occur without explicit confirmation
- [x] Dry-run one-shot execution remains fast and obvious
- [x] Blocked automation or execution states have readable reasons in the UI
- [x] No routine review action gained unnecessary modal friction
- [x] Existing hard validation behavior in store/service remains intact

## Phase 4 — Settings Diagnostics And Runtime-Sentinel Improvements

Goal: tighten diagnostics hierarchy while preserving the runtime banner's existing narrow sentinel role.

Primary files:

- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue)
- [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)
- [settingsService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/settingsService.ts)
- [statusPresentation.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/statusPresentation.ts)
- [desktopNotificationPolicy.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/desktopNotificationPolicy.ts)

Checklist:

- [x] Preserve runtime, native desktop, and task posture as the first visible diagnostics summaries
- [x] Preserve `Open task center` and `Copy diagnostics snapshot` as first-class diagnostics actions
- [x] Keep logs, timeline, and alerts clearly subordinate to the primary summary layer
- [x] Tighten diagnostics wording and hierarchy only where current ordering or emphasis is still noisy
- [x] Keep desktop-notification preferences centralized in Settings
- [x] Keep desktop-notification routing behavior unchanged unless copy-only updates require tests
- [x] Keep stack detail behind disclosure in the runtime banner
- [x] Keep the runtime banner limited to `booting` and `error`
- [x] Do not move broker-facing controls or runner controls into Settings
- [x] Do not broaden the runtime banner into a persistent execution/status rail

Acceptance criteria:

- [x] Settings diagnostics still read summary-first and evidence-second
- [x] The runtime banner remains narrow and boot/error scoped
- [x] Desktop-notification routing remains high-signal and background-only
- [x] No broker-facing control has been duplicated into Settings

## Objective Verification Checklist

Automated checks:

- [x] `npm --prefix web run lint`
- [x] `npm --prefix web run typecheck`
- [x] `npm --prefix web run test`
- [x] `npm --prefix web run build`
- [x] `npm --prefix web run parity`
- [x] `npm --prefix web run a11y:smoke`
- [x] `npm --prefix web run baseline:ui`
- [x] `npm --prefix web run check:bundle-budget` if shared helpers or startup-facing code paths change materially

Manual surface review:

- [x] Execution: paper mode + dry-run on
- [x] Execution: paper mode + broker-armed one-shot
- [x] Execution: live mode + broker-armed one-shot
- [ ] Execution: live feed active on matching symbol
- [ ] Execution: live feed active on mismatched symbol if reproducible
- [ ] Execution: runner active
- [x] Settings: diagnostics default state
- [ ] Settings: diagnostics with logs/timeline/alerts present
- [ ] Runtime banner: booting state
- [ ] Runtime banner: error state with detail disclosure
- [x] Task Center: no regression in open, Escape close, focus loop, and focus return
- [x] Light theme review
- [ ] Dark theme review
- [x] Standard density review
- [ ] Dense mode review
- [ ] Reduced-motion review

Manual review notes:

- Live-feed-active, runner-active, and runtime-error states were not reproduced in the static preview harness used for Prompt 6 evidence capture, so those rows remain intentionally unchecked.
- Task Center focus/close behavior was verified through the existing automated test surface rather than a separate manual browser pass.

Guardrail review:

- [x] Confirm `Execute once`, `Test account`, `Refresh orders`, feed controls, runner controls, and key controls all remain reachable
- [x] Confirm broker-armed one-shot now has confirmation coverage
- [x] Confirm existing clear-keys and existing live/runner start confirmation paths still behave correctly in the objective pass
- [x] Confirm blocked reasons appear without weakening existing store/service hard gates
- [x] Confirm Settings remains the canonical diagnostics/utilities surface
- [x] Confirm the runtime banner still appears only for `booting` and `error`
- [x] Confirm the Task Center did not become a diagnostics console
- [x] Confirm desktop-notification routing policy is unchanged unless explicitly intended and tested

## Targeted Test Additions

- [x] [ExecutionWorkspacePanel.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.test.ts) coverage for dynamic one-shot label behavior
- [x] [ExecutionWorkspacePanel.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.test.ts) coverage for broker-armed one-shot confirmation
- [x] [ExecutionWorkspacePanel.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.test.ts) coverage for visible preflight or blocked-reason messaging
- [ ] [ExecutionWorkspacePanel.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.test.ts) coverage for confirmation-dialog focus return if that flow changes materially
- [x] [executionStore.actions.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.actions.test.ts) coverage for derived posture fields or blocked-reason helpers if added
- [x] [executionStore.actions.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.actions.test.ts) coverage for live-symbol mismatch and broker-armed availability behavior if touched
- [ ] [executionService.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/executionService.test.ts) coverage for any new execution-presentation helper or confirmation-summary helper if introduced
- [ ] [SettingsWorkspacePanel.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.test.ts) coverage for diagnostics ordering and diagnostics snapshot path if touched
- [ ] Add [AppRuntimeBanner.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.test.ts) if error-path actions are introduced
- [x] Keep [TaskCenterOverlay.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.test.ts) unchanged unless objective work unexpectedly touches Task Center behavior

## UX Pass/Fail Criteria

Pass:

- [x] The execution surface communicates “what state am I in?” before “what can I click?”
- [x] Broker-armed one-shot execution is unmistakably higher consequence than dry-run one-shot execution
- [x] Routine review actions are still fast and unobstructed
- [x] Settings diagnostics still feel like summary-led operator review, not a log dump
- [x] The runtime banner remains rare and serious rather than ambient
- [x] Operators can tell why execution/automation is unavailable without guessing from disabled buttons

Fail:

- [ ] A current execution capability path becomes hidden, removed, or harder to discover
- [ ] Routine review/stop actions become slower because modal friction was over-applied
- [ ] Execution posture gets noisier instead of clearer
- [ ] Settings starts reading like a second execution console
- [ ] The runtime banner starts carrying general task or execution status noise
- [ ] Notification behavior becomes spammy or diverges from current high-signal routing

## Explicit Defers And Rejects

- [x] Reject a new all-in-one operator-control-center workspace
- [x] Reject duplicating broker-facing controls into Settings
- [x] Reject broadening the runtime banner into a second task center or general alert rail
- [x] Reject removing current backend-backed execution controls in the name of decluttering
- [x] Reject replacing readable operator posture with chart-first or log-first emphasis
- [x] Reject notification-policy expansion that breaks the current background-only, high-signal model
- [x] Reject divergent execution-only color/status language that abandons shared tone vocabulary
- [x] Defer any larger execution chart redesign beyond posture, grouping, and safety clarity
- [x] Defer any broader shell/nav changes; Prompt 6 should not reopen shell hierarchy work

## Optional Interpretive Follow-Up Checklist

- [ ] Decide whether `Start live` / `Stop live` should become `Start feed` / `Stop feed`
- [ ] Decide whether live-feed start should keep its modal confirmation after any terminology cleanup
- [ ] Decide whether runner-start confirmation should become explicitly mode-aware rather than always preserved
- [ ] Decide the final exact presentation model for the execution posture summary: chips, strip, compact checklist, or equivalent
- [ ] Decide whether the runtime-banner error state should gain `Open diagnostics` and/or `Copy error` actions
- [ ] Decide whether [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue) should gain execution-specific jump-back actions
- [ ] Decide whether a narrow execution-specific presenter helper such as `executionPresentation.ts` is worth adding versus keeping derivation local
- [ ] Decide the final execution-specific severity mapping layered on top of existing shared tone helpers
- [ ] Decide whether Settings diagnostics should gain a conditional `Open execution` shortcut when execution is active or attention-worthy

## Expected Edit Inventory

Primary objective touch points:

- [ExecutionWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.vue)
- [executionStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.ts)
- [executionService.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/executionService.ts)
- [SettingsWorkspacePanel.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.vue)
- [statusPresentation.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/statusPresentation.ts)

Conditional or likely-adjacent touch points:

- [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)
- [ConfirmationDialog.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ConfirmationDialog.vue)
- [desktopNotificationPolicy.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/utils/desktopNotificationPolicy.ts)
- [App.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/App.vue)
- [TaskCenterOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.vue)
- [ExecutionWorkspacePanel.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/ExecutionWorkspacePanel.test.ts)
- [SettingsWorkspacePanel.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/SettingsWorkspacePanel.test.ts)
- [TaskCenterOverlay.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/components/TaskCenterOverlay.test.ts)
- [executionStore.actions.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/executionStore.actions.test.ts)
- [executionService.test.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/services/executionService.test.ts)
