# Execution, Settings, And Operator-Safety Direction — FINAL

## Executive summary

VectorGrid has already fixed the obvious execution-safety debt. The repo now has confirmation coverage for `Clear keys`, `Start live`, and `Start runner`; clearer separation between risky and routine actions; a more structured execution review pane; a shipped Task Center with keyboard-safe focus behavior; and a first-pass Settings diagnostics surface. The canonical target comp for this area is still an execution safety banner, grouped action sections, a runtime-status grid, an order-activity table, and raw output behind disclosure. fileciteturn12file3 fileciteturn12file10

What still feels weak is not capability exposure. It is operator legibility. The functional-coverage matrix already says execution is relatively concentrated in `ExecutionWorkspacePanel.vue` and should stay focused on safer grouping and clearer affordances rather than control removal. But the current surface still makes the operator mentally combine broker mode, dry-run state, feed state, runner state, symbol match, and key persistence before acting. The store and service already know most of those facts; the UI just does not present them as one explicit preflight posture. fileciteturn12file9 fileciteturn12file1 fileciteturn12file2

The final direction is to keep the current four-surface model and sharpen ownership instead of redesigning it:

- **Execution panel** = the only broker-facing control plane
- **Settings panel** = canonical utilities and deeper diagnostics
- **Task center** = active/recent operational posture with jump-back actions
- **Runtime banner** = boot/error sentinel only

Do not create a larger “control center,” and do not duplicate broker controls into Settings. This matches the repo’s design-system rules, target comp, and current shell ownership, including the fact that the Settings workspace already mounts both `SettingsWorkspacePanel.vue` and `ExecutionWorkspacePanel.vue`. fileciteturn12file11 fileciteturn12file10 fileciteturn12file13

## Final execution/settings UX direction

Adopt a **posture-first operator console** in the execution panel.

The execution header should stop leading with generic summary chips and instead answer five operator questions immediately:

1. **Broker mode**: paper or live
2. **Submission mode**: simulation or broker-armed
3. **Feed state**: off, delayed, realtime, and symbol
4. **Runner state**: off or active
5. **Key state**: session-only, persisted, or missing

That change is the highest-ROI improvement because it turns the existing guardrails into a readable preflight model. The repo already tracks the raw facts for this: execution services know whether `dry_run` is broker-safe, whether credentials are required, whether the live stream is active, and whether the live symbol matches the requested symbol; the execution store already derives button availability from those same facts. fileciteturn12file1 fileciteturn12file2

The interaction model should then become:

- **Review**: test account, refresh orders, inspect runtime and order state
- **Manual execution**: simulate once, submit paper order once, or submit live order once
- **Automation**: start/stop feed, start/stop runner
- **Credentials**: enter keys, show key persistence state, clear stored keys

This keeps every backend-backed capability reachable while making the risk model explicit instead of implicit. That aligns with the functional-coverage guardrail and the repo’s target comp for execution/settings. fileciteturn12file9 fileciteturn12file10

Settings should remain the **deep diagnostics and utilities home**, not a second execution console. The current architecture already treats Settings as the canonical home for environment/runtime utilities unless an in-context control is essential, and the existing diagnostics additions already cover runtime, desktop state, recent task posture, timeline, and recent alerts. Keep that ownership, but improve triage and hierarchy inside the diagnostics area. fileciteturn12file9 fileciteturn12file3

The Task Center should remain the **what is active / what just happened / where do I go next** surface. The task model already supports queued, running, cancelling, success, failed, and cancelled states, plus events, timeline, toasts, and overall progress. That is enough to support execution-related jump-back actions without turning the Task Center into a diagnostics console. fileciteturn12file6 fileciteturn12file5 fileciteturn12file8

The runtime banner should remain **strictly boot/error scoped**. It should gain a better escape hatch into diagnostics, not broaden into a persistent app-wide status rail. The current component already shows only for `booting` or `error`, which is the right boundary. fileciteturn12file5

## High-stakes safety posture

The repo already solved the old baseline problem by adding modal confirmation to `Clear keys`, `Start live`, and `Start runner`. The final direction now is to **move confirmation friction to actual broker-affecting transitions**, rather than leaving it attached to every stateful action equally. fileciteturn12file3

Adopt these rules:

- **Always confirm** `Clear keys`
- **Confirm any non-dry-run `Execute once` action**
- **Confirm `Start runner` only when the runner is broker-armed (`dry_run = false`)**
- **Do not confirm** `Test account`, `Refresh orders`, `Stop feed`, or `Stop runner`
- **Rename** `Start live` / `Stop live` to `Start feed` / `Stop feed`
- **Do not require a modal for `Start feed`** once the terminology collision with “live mode” is removed

This is the most important change in the whole pass. `runOnce()` can submit a broker order whenever `dry_run` is off, and it even prepends an optimistic order row before broker-confirmed refresh. `startRunner()` can repeatedly execute decisions on incoming bars and already hard-checks for broker credentials, active live stream, and symbol match. Those are the actual high-stakes paths that should carry modal friction. fileciteturn12file1 fileciteturn12file13

The one-shot execution button should become **mode-aware**:

- `Simulate once` when `dryRun` is on
- `Submit paper order once` when `dryRun` is off and broker mode is paper
- `Submit live order once` when `dryRun` is off and broker mode is live

That small change closes the biggest remaining mismatch between visual weight and actual consequence.

`Start runner` should also become **mode-aware** in its warning copy. Because the repo has only one active live stream slot and one active live execution runner slot, the UI should present a singular preflight state rather than a multi-job control model. The confirmation body for broker-armed runner start should include symbol, broker mode, submission mode, parameter source, and any risk overrides. fileciteturn12file13 fileciteturn12file1

Finally, the execution surface should stop treating disabled buttons as the primary explanation model. Surface explicit preflight reasons before click:

- credentials required
- feed inactive
- feed symbol mismatch
- runner already active
- live runner unavailable in current posture

The store already has the raw gating behavior; it needs to expose those reasons as readable UI state rather than only implicit `canStartRunner` / `canRunOnce` booleans. fileciteturn12file2

## Runtime-status and diagnostics presentation rules

Use one severity/tone model across execution, Task Center, and Settings diagnostics:

- **Info**: paper mode, dry-run on, feed off, routine refreshes
- **Warn**: live account selected, delayed feed, persisted keys present, prerequisite missing, stale diagnostic attention state
- **Error/Danger**: runtime error, broker-armed live submission, broker-armed runner, destructive key-clearing path
- **Success**: connection test ok, runner stopped cleanly, order refresh success, completed background task

The shared tone vocabulary should come from the existing task/runtime presentation helpers. Do not fork a second color-language just for execution. But also do not try to force broker/feed posture into task-lifecycle labels such as `queued` or `running`; broker posture needs a small execution-specific presenter layered on top of the existing info/warn/error/success tone model. fileciteturn12file5

Diagnostics should remain **summary first, history second**.

In Settings, the first visible diagnostics cards should be:

1. runtime state
2. native desktop state
3. task posture
4. open task center / copy snapshot actions

Then come the evidence trails:

- desktop logs
- recent timeline
- recent alerts

That preserves diagnostic richness without making every visit feel like a log viewer.

Desktop notifications should remain **opt-in, high-signal, and background-only**. The current policy already suppresses notifications while the window is visible, ignores non-terminal task states, and only promotes failures plus a narrow set of important successes. Keep that centralized policy; refine copy only if needed. fileciteturn12file0 fileciteturn12file7

Raw output and traces should stay behind disclosure. The target comp and design-system rules already prefer summary chips, advisory blocks, and details/disclosure for heavy payloads. Do not pull raw JSON or stack traces back into the primary visible layer. fileciteturn12file10 fileciteturn12file11

## Surface-by-surface guidance

### Execution panel

Adopt these changes in the current [ExecutionWorkspacePanel.vue](sandbox:/mnt/data/ExecutionWorkspacePanel.vue):

- Replace the current generic summary chips with the five-part posture summary
- Rename `Mode` to **Broker mode**
- Rename `Start live` / `Stop live` to **Start feed** / **Stop feed**
- Rename `Paper and manual actions` to **Manual execution**
- Make the primary one-shot action dynamic: simulate / submit paper / submit live
- Split credentials into a clearer subsection so `Remember keys` is not the only signal for persistence state
- Keep the order table and runtime review surface visible
- Keep raw preview behind a disclosure block instead of treating it as equal weight with order review
- Add a compact preflight checklist near automation controls

The chart area should remain secondary evidence, not the main safety surface. The audit already moved the execution review pane in the right direction by separating runtime status, chart context, and broker order review; the next step is better posture, not more chart chrome. fileciteturn12file3

### Settings panel

Adopt these changes in the current [SettingsWorkspacePanel.vue](sandbox:/mnt/data/SettingsWorkspacePanel.vue):

- Keep environment/runtime utilities canonical here
- Keep notification settings centralized here
- Keep diagnostics rich, but reorder them summary-first
- Preserve `Open task center` and `Copy diagnostics snapshot` as first-class actions
- Do **not** move broker controls or runner controls into Settings
- Optionally add a small “Open execution” shortcut only when execution activity is currently live or attention-worthy

The settings workspace already coexists with the execution panel. The goal is not more surface area. It is a clearer division between operating and diagnosing. fileciteturn12file13 fileciteturn12file9

### Runtime banner

Adopt these changes in the current [AppRuntimeBanner.vue](sandbox:/mnt/data/AppRuntimeBanner.vue):

- Keep booting and error as the only visible states
- Add `Open diagnostics` on error
- Add `Copy error` on error when a trace exists
- Keep stack detail behind disclosure
- Do not show general execution warnings, task alerts, or background lifecycle messages here

This should remain a narrow sentinel, not a general-purpose alert layer.

### Task center

Adopt these changes in the current [TaskCenterOverlay.vue](sandbox:/mnt/data/TaskCenterOverlay.vue):

- Keep grouped posture as the primary model: queued, running, cancelling, recent attention, completed
- Extend `relatedActionFor()` so execution-related tasks can reopen the execution surface directly
- Use existing task and status-presentation vocabulary; do not create a second execution-specific task taxonomy
- Keep focus restoration, Escape close, and keyboard loop behavior unchanged
- Do not move deep diagnostics or raw logs into the Task Center

The task store already tracks execution commands and runtime event channels, so this change is about better related actions and message polish, not new orchestration infrastructure. fileciteturn12file6 fileciteturn12file5

## Immediate versus deferred changes

### Immediate

1. **Execution copy and grouping pass**
   - dynamic one-shot label
   - `Broker mode`
   - `Start/Stop feed`
   - clearer credentials section

2. **Derived posture and preflight reasons**
   - expose readable blocked reasons from store state
   - show key persistence state explicitly
   - show feed/symbol match explicitly

3. **Confirmation-scope correction**
   - keep `Clear keys`
   - add non-dry-run one-shot confirmation
   - keep runner confirmation only for broker-armed runner start
   - remove feed-start modal after label correction

4. **Task Center execution jump-back actions**
   - execution tasks should reopen the execution surface directly

5. **Runtime banner error escape hatch**
   - open diagnostics
   - copy error

### Deferred

- richer confirmation-body evidence blocks if the current modal template becomes cramped
- an `executionPresentation.ts` helper if posture copy starts duplicating across component and store
- optional “Open execution” shortcut card inside Settings diagnostics
- later notification copy refinement, but not a routing-policy change
- any larger execution chart polish beyond the current textual/runtime improvements

## File-level implementation order

1. **[ExecutionWorkspacePanel.vue](sandbox:/mnt/data/ExecutionWorkspacePanel.vue)**
   - rename labels and action groups
   - add posture summary strip
   - add preflight checklist and key-state signal
   - update one-shot button copy
   - move raw preview behind stronger disclosure if needed

2. **`web/src/stores/executionStore.ts`**
   - expose derived posture fields and blocked-reason arrays
   - keep existing hard gates
   - add explicit persisted/session/missing key-state helpers
   - add feed-symbol-match helper and broker-armed booleans fileciteturn12file2

3. **`web/src/services/executionService.ts`**
   - add reusable posture/confirmation summary builders
   - preserve current hard validations for credentials, live feed, and symbol match
   - keep optimistic order-row behavior, but pair it with clearer preflight and post-submit language fileciteturn12file1

4. **[TaskCenterOverlay.vue](sandbox:/mnt/data/TaskCenterOverlay.vue)**
   - extend `relatedActionFor()` for execution commands
   - optionally refine task secondary copy for execution outcomes

5. **[SettingsWorkspacePanel.vue](sandbox:/mnt/data/SettingsWorkspacePanel.vue)**
   - reorder diagnostics cards and evidence trails
   - keep notification controls and diagnostics copy centralized

6. **[AppRuntimeBanner.vue](sandbox:/mnt/data/AppRuntimeBanner.vue)**
   - add error-path actions only

7. **`web/src/utils/statusPresentation.ts`**
   - keep severity/tone vocabulary shared
   - add only the minimal helper needed for execution posture tone if duplication appears fileciteturn12file5

8. **`web/src/utils/desktopNotificationPolicy.ts`**
   - keep routing logic unchanged unless copy refinement requires a test update fileciteturn12file0

## Verification plan

Run the repo’s standard frontend confidence stack after each meaningful slice:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run parity`
- `npm --prefix web run a11y:smoke`
- `npm --prefix web run baseline:ui` fileciteturn12file12 fileciteturn12file13

Add or update focused tests for:

- `ExecutionWorkspacePanel`
  - dynamic one-shot labels
  - confirmation coverage by risk mode
  - visible preflight reasons
  - focus return after confirmation dialogs
- `executionStore`
  - derived posture fields
  - runner/feed mismatch reasons
  - broker-armed vs simulated action availability
- `executionService`
  - confirmation-summary builder output
  - runner start preflight failures
  - one-shot status text for dry-run vs broker submission
- `TaskCenterOverlay`
  - execution-related jump-back actions
- `SettingsWorkspacePanel`
  - diagnostics ordering and copy snapshot flow
- `desktopNotificationPolicy`
  - background-only routing and important-success filtering remain intact fileciteturn12file0

Add a browser-side regression sweep for these concrete operator states:

1. paper mode + dry-run on
2. paper mode + broker-armed one-shot
3. live mode + broker-armed one-shot
4. feed active for wrong symbol
5. runner active
6. settings diagnostics with recent logs and alerts
7. runtime error banner with diagnostics handoff

If the diagnostics, runtime, or titlebar/runtime integration changes materially, rerun the desktop smoke harness as an extra check because these surfaces live inside the real Tauri shell, not just the browser preview. fileciteturn12file12

## Explicit rejects

Reject these directions for this repo:

- a new all-in-one “operator control center” workspace
- duplicating broker controls into Settings
- promoting the runtime banner into a second task center or general alert rail
- blanket modal confirmation for stop/review actions
- hiding live or destructive actions behind kebab menus to make the panel “cleaner”
- notification expansion that breaks the current high-signal, background-only policy
- a separate execution-only color/status language that diverges from the existing shared tone vocabulary
- chart-first execution review that makes the textual/runtime posture harder to scan

The deciding principle is simple: keep every execution/runtime capability reachable, but make the broker-armed state obvious before action and keep deeper diagnostics subordinate to the operator’s immediate posture.
