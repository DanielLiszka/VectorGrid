# Consolidated Review — Live Execution Session Audit Trail

This consolidation keeps only claims that are supported by the attached bundle files. Suggestions from the earlier first-pass and second-pass critiques were retained only when the packet itself validates them.  
**Method anchor:** `PROMPT_03_consolidation.md:23-41`.

## 1. Surface understanding

The bundle targets a real repo gap inside a live surface that already exists but is intentionally narrow. The umbrella live productization doc keeps this scope limited to the current non-options lane: Alpaca over REST, Polygon stocks live feed, one active live stream slot, one active live runner slot, and current live strategy coverage limited to `DoubleMa` and `CompositeCpu`. The current execution UI already exposes runtime status, broker-order review, position/chart meta, raw output preview, runtime-event handling, and optimistic `submitted*` rows, but none of that becomes a durable, first-class session artifact after the session ends. The session-audit bundle is therefore repo-fit, but its contract is still too incomplete in the places that matter most: artifact ownership, redaction, provenance, reopen behavior, and execution-review truthfulness.  
**Sources:** `05_live_execution_productization_2026-04-10.md:13-24, 36-42, 97-113`; `07_live_execution_session_audit_trail_2026-04-11.md:13-16, 25-34, 55-67, 84-98`; `REPO_MAP.md:1569-1607, 901-910`; `ExecutionWorkspacePanel.vue:651-737`; `executionService.ts:439-492, 811-843`; `executionStore.ts:749-780`.

## 2. Consolidated must-fix findings

### 2.1 Freeze the artifact owner and hydration boundary before implementation

- **Severity:** critical
- **Type:** migration / correctness
- **Location:** `07_live_execution_session_audit_trail_2026-04-11.md` (`Proposed Improvement Approach`, `Checklist`); `run_artifacts.rs` (`WorkflowArtifactsHydrationPayload`, `build_workflow_artifacts_hydration_payload`, `load_run_artifact_hydration_blocking`); `workflowArtifactsStore.ts` (`latestWorkflowRunSnapshot`, `applyNormalizedArtifacts`); `executionService.ts` (`buildExecutionInvokePayload`)
- **Why it matters:**  
  The biggest hidden risk is not just storage drift. It is behavior drift. In the current repo, artifact persistence is centralized in `run_artifacts.rs`, and the current reopen/hydration contract is specifically a **run-artifact** contract: Rust hydrates `current_run` plus optional `current_validation`, and `workflowArtifactsStore.ts` applies that payload into `currentRun/currentValidation` while also mutating the module-level `latestWorkflowRunSnapshot`. `executionService.ts` then reads `latestWorkflowRunSnapshot` to build live execution requests for selected/best/manual flows. A live-session artifact that reuses this hydration path would not just “live beside” run artifacts. It could become an execution input source and create a second artifact truth system.
- **Recommended change:**  
  The doc should choose the owner model now. Reusing `run_artifacts.rs` / `commands_artifacts.rs` for storage, listing, detail, tags, and local open/reveal can still be reasonable, but a live-session artifact must **not** hydrate through `currentRun/currentValidation` and must **not** mutate `latestWorkflowRunSnapshot`. If reopened state is needed, use a separate read-only detail/review payload and a separate review store.
- **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:97-116`; `REPO_MAP.md:1600-1607, 1625-1638`; `commands_artifacts.rs:120-142`; `run_artifacts.rs:22-27, 256-260, 1460-1514, 1983-1997`; `workflowArtifactsStore.ts:96-98, 116-143, 297-359, 413-421, 433-520`; `executionService.ts:609-671`.

### 2.2 Replace “redaction is required” with a field-level redaction contract

- **Severity:** critical
- **Type:** safety
- **Location:** `07_live_execution_session_audit_trail_2026-04-11.md` (`Regressions To Avoid`, `Proposed Improvement Approach`, `Checklist`); `executionService.ts`; `executionStore.ts`; `commands_execution.rs`
- **Why it matters:**  
  The packet already shows multiple sensitive paths that a naive session-artifact implementation could accidentally persist. Execution request payloads include `alpaca_api_key` and `alpaca_api_secret`. The frontend also persists remembered keys in storage and mirrors both current and persisted keys into Pinia state. `testConnection()` and `refreshOrders()` serialize broker responses into `outputPreview`, and `executionStore.ts` stores that raw preview text. On the backend side, `test_alpaca_connection` returns `account_id`, `account_number`, `buying_power`, `cash`, and `portfolio_value`. That means the plan cannot stop at “redaction rules should exist.” It needs a concrete denylist/allowlist.
- **Recommended change:**  
  Add an explicit redaction section that states, at minimum:
  - never persist `alpaca_api_key`, `alpaca_api_secret`, remembered-key JSON, `apiKey`, `apiSecret`, `persistedApiKey`, `persistedApiSecret`, or whole `ExecutionActionOptions` objects;
  - never persist raw `outputPreview`;
  - decide explicitly whether `account_id`, `account_number`, balances, and portfolio-value fields are excluded entirely, masked, or reduced to compact summaries;
  - prefer field-level event summaries over raw request/response payload copies.
- **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:69-75, 86-98, 108-112`; `executionService.ts:506-553, 609-670, 685-739, 811-836`; `executionStore.ts:206-225, 348-351, 390-410, 439-462, 645-660, 700-705`; `commands_execution.rs:40-80, 1403-1424`.

### 2.3 Define “reopen” as read-only review, not restore / resume / re-arm

- **Severity:** high
- **Type:** correctness / operator-truth
- **Location:** `07_live_execution_session_audit_trail_2026-04-11.md` (`Scope Boundaries`, `Checklist`); `run_artifacts.rs`; `state.rs`; `commands_execution.rs`
- **Why it matters:**  
  The plan currently says session artifacts or logs can be “reopened locally,” but it does not define what that means. In current code, reopen already has a concrete meaning for run artifacts: `load_run_artifact_hydration_blocking()` only accepts run-artifact kinds and rebuilds a `WorkflowArtifactsHydrationPayload` containing `current_run` and optional `current_validation`. That is not a generic live-session restore contract. Separately, live feed state is held in `AppState.live_candle_stream`, while runner state is held in a separate static `LIVE_EXECUTION_RUNNER` slot in `commands_execution.rs`. A reopened session artifact therefore cannot truthfully imply feed restore, runner restore, or auto-rearm of broker-facing behavior.
- **Recommended change:**  
  Add a dedicated **Reopen Contract** section that states: reopened live-session artifacts are for read-only review only. They do not restore live feed state, do not restore runner state, do not repopulate `currentRun/currentValidation`, and do not silently re-arm broker-facing behavior.
- **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:13-16, 112-115, 128-131`; `run_artifacts.rs:256-260, 1460-1514, 1983-1997`; `state.rs:120-127, 573-630`; `commands_execution.rs:181-239, 281-282, 1120-1149`.

### 2.4 Expand the provenance model so it matches the actual runtime vocabulary

- **Severity:** high
- **Type:** operator-truth
- **Location:** `07_live_execution_session_audit_trail_2026-04-11.md` (`Weaknesses / Failure Modes`, `Regressions To Avoid`, `Proposed Improvement Approach`, `Checklist`); `REPO_MAP.md` (`Live execution`, `Polygon historical and live`); `commands_execution.rs`; `executionService.ts`; `executionStore.ts`
- **Why it matters:**  
  The plan’s current three-label vocabulary — local runtime event, optimistic submission event, broker-confirmed snapshot data — is directionally correct but too coarse for this repo. The actual runner event vocabulary is `started`, `stopped`, `triggered`, `queued_latest`, `signal_processed`, `order_submitted`, and `error`. The UI also generates an explicitly optimistic `submitted*` order row, while `runOnce()` tells the operator to use Refresh orders for broker-confirmed state. There is another important edge case: the repo map explicitly says runner cancellation is intentionally silent on the event bus. A truthful session artifact therefore needs a more exact mapping from actual event/status sources to durable truth classes, rather than a high-level summary that could blur submission-attempt evidence into broker-confirmed evidence.
- **Recommended change:**  
  Add a **Persisted Provenance Taxonomy** table that maps:
  - local control events (`start live`, `stop live`, `start runner`, `stop runner`);
  - runner lifecycle events (`started`, `triggered`, `queued_latest`, `stopped`, `error`);
  - signal outcomes (`signal_processed`);
  - optimistic submission evidence (`submitted*`);
  - backend submission-attempt evidence (`order_submitted`);
  - broker snapshot observations (`refresh orders`, `test connection`, `snapshot fetched`);
  - cancellation / silent-stop handling.  
  The table should state explicitly that `submitted*` and `order_submitted` are **not** broker-confirmed fills or positions.
- **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:62-75, 86-98, 108-112`; `REPO_MAP.md:1578-1585, 1599-1607`; `commands_execution.rs:181-239, 486-502, 1105-1116, 1132-1145, 1201-1208, 1322-1392`; `executionService.ts:439-492, 811-843`; `executionStore.ts:749-780`.

### 2.5 Decide whether reconnect/feed evidence is really in scope, then align the owner files

- **Severity:** high
- **Type:** scope / migration
- **Location:** `07_live_execution_session_audit_trail_2026-04-11.md` (`Current Code Surface`, `Proposed Improvement Approach`); `REPO_MAP.md` (`Polygon historical and live`, `Live execution`); `state.rs`
- **Why it matters:**  
  The plan says the persisted session artifact should include “error and reconnect evidence,” but the bundle’s current code surface does not include the feed-owner modules that actually own live-stream lifecycle state. The packet shows that Polygon live-stream lifecycle is exposed through `polygon_live_status` / `polygon_live_bar`, and that only one live stream slot exists at a time. `AppState` owns that stream slot. If reconnect evidence stays in scope, the doc needs to acknowledge that the feed side is part of the contract. If it does not, the current wording overpromises a category of evidence that the listed owners do not fully own.
- **Recommended change:**  
  Pick one of two directions now:
  1. narrow the plan to **execution-session evidence only** and remove reconnect/feed wording; or  
  2. keep reconnect/feed evidence in scope and add `commands_data.rs` / `polygon.rs` (or explicitly cite them via the repo map) to the owner/verification surface, plus a short runtime-ownership note explaining where feed evidence comes from.
- **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:36-45, 87-92`; `REPO_MAP.md:1569-1585, 1599-1607`; `state.rs:120-127, 573-630`.

### 2.6 Capture what was actually armed at session start

- **Severity:** high
- **Type:** correctness
- **Location:** `07_live_execution_session_audit_trail_2026-04-11.md` (`Current User-Facing Behavior`, `Checklist`); `executionService.ts`; `workflowArtifactsStore.ts`; `commands_execution.rs`
- **Why it matters:**  
  A session artifact is not very useful if it only records downstream lifecycle events and order/snapshot evidence. In the current code, execution requests can be built from manual UI inputs or from the latest workflow-run snapshot plus selected/best row context. `workflowArtifactsStore.ts` tracks `artifactPath`, `configFingerprint`, `routeProvenance`, `req`, and `res`; `executionService.ts` reads that snapshot and then builds the live execution payload. `commands_execution.rs` also serializes runner config fields such as symbol, qty, mode, dry-run flag, `data_id`, and `strategy_kind`. Without session-start provenance, reopened evidence cannot answer the most basic operator-review question: *what exact strategy/config/source was armed?*
- **Recommended change:**  
  Require a compact session-start payload that captures, at minimum:
  - symbol, qty, mode, dry-run flag;
  - `strategyKind` and `dataId`;
  - parameter source (`manual_ui`, `selected`, `best`);
  - source run `artifactPath`, `configFingerprint`, and `routeProvenance` when the session was armed from a saved/current run context.
- **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:27-34, 108-113`; `workflowArtifactsStore.ts:413-421, 445-457`; `executionService.ts:555-670`; `commands_execution.rs:187-220`.

## 3. Consolidated should-fix findings

### 3.1 Choose the write model, filename prefix, and retention behavior now

- **Severity:** medium-high
- **Type:** maintainability / migration
- **Location:** `07_live_execution_session_audit_trail_2026-04-11.md` (`Checklist`, `Rollout / Fallback / Compatibility Notes`); `run_artifacts.rs`; `REPO_MAP.md` (`Artifacts`)
- **Why it matters:**  
  The doc asks when session artifacts are created, updated, finalized, and reopened, but it does not choose a write model. The current artifact owner is built around timestamped JSON files in the shared runs directory, prefix-based suggestion helpers, path-constrained load/delete/tag flows, and prefix-based prune helpers. That is not enough detail for an append-heavy session log by itself.
- **Recommended change:**  
  Decide now whether the live-session artifact is:
  - a single summary file written on finalize;
  - a summary file plus event array updated in place;
  - or a summary file plus separate event-log file.  
  Also define the filename prefix, prune behavior, and crash/partial-finalization posture if the shared owner is reused.
- **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:108-116, 126-131`; `REPO_MAP.md:1625-1638`; `run_artifacts.rs:336-422, 886-989`.

### 3.2 Make the mode-capture policy explicit for live, paper, and dry-run

- **Severity:** medium
- **Type:** scope / operator-truth
- **Location:** `07_live_execution_session_audit_trail_2026-04-11.md` (`Checklist`); `05_live_execution_productization_2026-04-10.md`; `executionStore.ts`
- **Why it matters:**  
  The current live surface already distinguishes dry-run, paper, and live, and both the umbrella live doc and the execution store are careful about preserving those distinctions. The session-audit plan still leaves capture policy unresolved (`opt-in`, `default-on`, or mode-dependent). That choice affects noise, operator expectations, and how “audit trail” language will be interpreted.
- **Recommended change:**  
  State the mode matrix directly in the plan. A conservative repo-fit default would be: broker-facing live default-on; paper optional or clearly labeled; dry-run optional/off by default and always labeled simulation-only.
- **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:112-113`; `05_live_execution_productization_2026-04-10.md:28-42, 82-85, 123-131`; `executionStore.ts:304-345, 372-387`.

### 3.3 Strengthen verification for a shared artifact-owner change

- **Severity:** medium
- **Type:** verification / testing
- **Location:** `07_live_execution_session_audit_trail_2026-04-11.md` (`Verification Plan`); `commands_artifacts.rs`; `run_artifacts.rs`; `REPO_MAP.md` (`Artifacts`, `Verification Map`)
- **Why it matters:**  
  The current verification plan is directionally good, but it is too light if the new session evidence reuses `run_artifacts.rs`. That module is a shared persistence owner with list/load/detail/delete/tag behavior, path confinement, and retention logic. The repo map also gives a canonical output-parity command name, while the plan currently calls out the raw node script path instead.
- **Recommended change:**  
  Expand verification to include:
  - no-regression checks for existing artifact families and command wrappers;
  - path-outside-runs-dir rejection tests;
  - redaction tests;
  - reopen/read-only contract tests;
  - and the repo’s canonical `npm --prefix web run parity:outputs` command if artifact/report shape changes.
- **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:118-124`; `commands_artifacts.rs:9-194`; `run_artifacts.rs:425-438, 1983-2025, 2266-2305`; `REPO_MAP.md:1625-1638`.

### 3.4 Name the review-surface owner instead of leaving it implicit

- **Severity:** medium
- **Type:** maintainability / scope
- **Location:** `07_live_execution_session_audit_trail_2026-04-11.md` (`Checklist`); `REPO_MAP.md` (`settings workspace note`, `Saved run rehydration and history bridge`)
- **Why it matters:**  
  The plan says to add a lightweight operator-facing review path, but it does not say where that review lives. The packet shows two plausible homes: the execution surface itself, and the existing saved-artifact/history bridge described in the repo map. Without naming the owner surface, the implementation could fragment between multiple places.
- **Recommended change:**  
  State whether saved live-session review belongs:
  - in the settings-mounted execution surface,
  - in the history/library surface,
  - or in a new read-only surface.  
  The plan does not need to redesign the shell, but it should stop leaving UI ownership undefined.
- **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:115`; `REPO_MAP.md:901-910`.

### 3.5 Add a runtime-ownership note for feed state versus runner state

- **Severity:** medium
- **Type:** maintainability
- **Location:** `state.rs`; `commands_execution.rs`; `07_live_execution_session_audit_trail_2026-04-11.md` (`Current Code Surface`)
- **Why it matters:**  
  The packet shows that feed and runner state are not owned in one place. `AppState` owns the live-candle-stream slot, but runner state is stored in a separate static slot inside `commands_execution.rs`. A session recorder that spans both domains needs an explicit note about which layer records which class of evidence.
- **Recommended change:**  
  Add a short runtime-ownership section that names:
  - feed-state evidence source,
  - runner-state evidence source,
  - and the single module or API boundary responsible for writing the durable session artifact.
- **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:36-45`; `state.rs:120-127, 573-630`; `commands_execution.rs:281-282, 411-418`.

## 4. Optional improvements worth considering

### 4.1 Use a compact summary-plus-events shape instead of a giant monolithic blob

A single large JSON dump would work technically but would fight the plan’s “compact” goal. A small session summary plus a bounded event list is easier to review, easier to redact, and easier to tag or display in a read-only review surface.  
**Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:87-98, 126-131`; `run_artifacts.rs:336-422, 886-989`.

### 4.2 Reuse current artifact conventions where they help, but not the run-hydration contract

If the owner stays shared, reuse what is already working well: app-local `runs` placement, filename-prefix discipline, path confinement, tagging, and prune conventions. Do **not** reuse `currentRun/currentValidation` hydration semantics unless the doc explicitly proves that fit.  
**Sources:** `REPO_MAP.md:1625-1638`; `run_artifacts.rs:336-422, 425-438, 886-989, 2266-2305`; `workflowArtifactsStore.ts:433-520`.

### 4.3 Prefer a read-only detail/replay API over raw artifact load for this family

The current artifact surface already distinguishes raw load, hydration, detail, and replay patterns for different artifact families. A live-session artifact is more likely to fit a detail/replay pattern than a workflow-hydration pattern.  
**Sources:** `commands_artifacts.rs:111-178`; `run_artifacts.rs:1460-1514, 1517-1603, 1983-2025`.

### 4.4 Add compact “observed” vs “confirmed” cues in the review UI

The current execution surface already separates optimistic `submitted*` rows, runner-event handling, broker-order review, and raw preview text. A future review path could keep that truthful by showing a small “observed locally” vs “observed from broker snapshot” cue instead of dumping raw status text back at the user.  
**Sources:** `ExecutionWorkspacePanel.vue:651-737`; `executionService.ts:439-458, 811-843`; `executionStore.ts:749-780`.

## 5. Suggestions to reject as out of scope or unsupported

### 5.1 Reject scope broadening beyond the current narrow live lane

Do not let this plan widen into compliance-grade archival, cloud sync, multi-user trails, multi-broker work, options live trading, or a broad session-history platform redesign. Both the session-audit bundle and the umbrella live productization plan explicitly keep scope narrower than that.  
**Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:18-23, 126-131`; `05_live_execution_productization_2026-04-10.md:19-24, 145-148`.

### 5.2 Reject any proposal that makes a live-session artifact an execution-input artifact

Suggestions that make a session artifact rehydrate `currentRun/currentValidation`, update `latestWorkflowRunSnapshot`, or behave like a resumable run artifact should be rejected. That would violate the current artifact contract and introduce exactly the second truth system the plan says it wants to avoid.  
**Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:71-74`; `run_artifacts.rs:1460-1514, 1983-1997`; `workflowArtifactsStore.ts:413-421, 433-520`; `executionService.ts:609-671`.

### 5.3 Do not carry forward claims that require external docs not included in the packet

The earlier critiques surfaced some attractive ideas that depend on detailed Alpaca endpoint behavior. Those claims are not validated by the attached packet and should not drive the consolidated plan until the relevant official docs are explicitly added to the review basis.  
**Sources:** `PROMPT_03_consolidation.md:23-41`; `07_live_execution_session_audit_trail_2026-04-11.md:77-82`.

### 5.4 Do not mark the related-doc references as stale based only on this packet

The bundle references several related docs that were not part of the attached review packet. They should be treated as **unvalidated here**, not as confirmed stale or nonexistent.  
**Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:133-138`.

## 6. Recommended doc edits

1. **Add a “Chosen owner and hydration boundary” section.**  
   State whether the artifact uses the shared artifact owner or a separate module, and explicitly forbid `currentRun/currentValidation` hydration for this family.  
   **Source anchors:** `07_live_execution_session_audit_trail_2026-04-11.md:97-116`; `workflowArtifactsStore.ts:413-421, 433-520`; `run_artifacts.rs:1460-1514, 1983-1997`.

2. **Add a “Session Artifact Schema v1” section.**  
   Include required fields only: session identity, start/stop timestamps, armed symbol/qty/mode, strategy/config provenance, compact event list, compact broker-snapshot summaries, and tags if the shared owner is reused.  
   **Source anchors:** `07_live_execution_session_audit_trail_2026-04-11.md:87-98, 108-116`; `commands_execution.rs:187-220`; `executionService.ts:555-670`.

3. **Add a “Redaction Contract” section.**  
   Include explicit forbidden fields and explicit summary-only replacements.  
   **Source anchors:** `07_live_execution_session_audit_trail_2026-04-11.md:71-75, 108-112`; `executionService.ts:506-553, 685-739`; `executionStore.ts:206-225, 390-410, 439-462`; `commands_execution.rs:40-80`.

4. **Add an “Observed Evidence and Provenance Taxonomy” table.**  
   Map the actual runtime vocabulary (`started`, `triggered`, `queued_latest`, `signal_processed`, `order_submitted`, `error`, optimistic `submitted*`, broker snapshot refreshes) into durable truth classes.  
   **Source anchors:** `07_live_execution_session_audit_trail_2026-04-11.md:93-97`; `REPO_MAP.md:1599-1607`; `commands_execution.rs:1105-1145, 1201-1208, 1322-1392`; `executionService.ts:439-458, 811-843`.

5. **Add a “Reopen Contract” section.**  
   State that reopen is read-only review only and does not restore feed/runner state or repopulate workflow hydration state.  
   **Source anchors:** `07_live_execution_session_audit_trail_2026-04-11.md:13-16, 112-115`; `run_artifacts.rs:1460-1514, 1983-1997`; `state.rs:573-630`; `commands_execution.rs:281-282`.

6. **Add a “Write Lifecycle / Prefix / Retention” section.**  
   Choose create/update/finalize semantics, filename prefix, partial/crash handling, and prune behavior now.  
   **Source anchors:** `07_live_execution_session_audit_trail_2026-04-11.md:108-116`; `run_artifacts.rs:336-422, 886-989`.

7. **Add a short “Mode Capture Policy” subsection.**  
   Decide live vs paper vs dry-run capture defaults.  
   **Source anchors:** `07_live_execution_session_audit_trail_2026-04-11.md:112-113`; `05_live_execution_productization_2026-04-10.md:28-42`; `executionStore.ts:304-345, 372-387`.

8. **If reconnect evidence stays in scope, expand the owner/verification surface accordingly.**  
   Add feed-owner references or narrow the wording to execution-only evidence.  
   **Source anchors:** `07_live_execution_session_audit_trail_2026-04-11.md:87-92`; `REPO_MAP.md:1569-1585`.

9. **Clarify the review-surface owner.**  
   Name where the saved session review appears.  
   **Source anchors:** `07_live_execution_session_audit_trail_2026-04-11.md:115`; `REPO_MAP.md:901-910`.

## 7. Recommended implementation order

1. **Freeze the contract first.**  
   Decide owner, hydration boundary, reconnect scope, reopen semantics, and live/paper/dry-run capture policy before writing code.  
   **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:97-116`; `05_live_execution_productization_2026-04-10.md:97-113`.

2. **Write the schema, redaction rules, and provenance table second.**  
   This is the main trust boundary for the bundle.  
   **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:69-75, 86-98, 108-112`; `executionService.ts:506-553, 685-739`; `commands_execution.rs:40-80, 1105-1145, 1322-1392`.

3. **Implement backend persistence/detail/replay primitives third.**  
   Prefer backend contract work before any UI review surface. Reuse shared artifact plumbing only where the fit is explicit.  
   **Sources:** `commands_artifacts.rs:9-194`; `run_artifacts.rs:336-438, 1460-1514, 1983-2025`.

4. **Add no-regression and redaction verification fourth.**  
   Cover shared-owner regressions, path confinement, no-`latestWorkflowRunSnapshot` mutation, redaction, and the canonical output-parity lane if shape changes.  
   **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:118-124`; `REPO_MAP.md:1625-1638`; `run_artifacts.rs:425-438, 2266-2305`; `workflowArtifactsStore.ts:433-520`.

5. **Add a read-only review surface fifth.**  
   Keep it lightweight and explicit about provenance rather than coupling it immediately to a broader live-shell redesign.  
   **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:115, 126-131`; `REPO_MAP.md:901-910`; `ExecutionWorkspacePanel.vue:651-737`.

6. **Defer broader history-browser or artifact-platform expansion.**  
   The plan should prove a compact truthful contract first.  
   **Sources:** `07_live_execution_session_audit_trail_2026-04-11.md:128-131`; `05_live_execution_productization_2026-04-10.md:145-148`.
