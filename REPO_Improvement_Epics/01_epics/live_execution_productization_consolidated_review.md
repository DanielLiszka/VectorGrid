# Live Execution Productization — Consolidated Review

Generated from the attached review packet for the **Live Execution Productization** umbrella plan.

## 1. Surface understanding

The umbrella plan is pointed in the right direction: it keeps live execution narrow, explicitly scoped to the current non-options path, current Alpaca and Polygon posture, and single-symbol / single-runner hardening rather than pretending the repo already has a broad trading platform. It correctly frames the live surface as real but still narrow, and it points toward safety, recovery, evidence, and operator clarity.

But the owner files show that several of the highest-risk boundaries are still implicit rather than treated as umbrella-level invariants: reconnect behavior, secret ownership, provisional versus broker-confirmed truth, current shell/runtime ownership, and chart/account review incompleteness. Those are not polish items. They are the core constraints that should shape the rest of the child docs.

**Source anchors:** `05_live_execution_productization_2026-04-10.md` (Purpose, Scope Boundaries, Current User-Facing Behavior, Weaknesses / Failure Modes, Proposed Improvement Approach, Related Docs); `REPO_MAP.md` (live-execution scope and shell ownership sections).

The current repo posture is also very clearly narrow in the owner files: live execution coverage is limited to `DoubleMa` and `CompositeCpu`, broker integration is Alpaca over REST, live streaming is Polygon stocks only, the runner depends on an active live stream, delayed streaming is the default, and only one active live stream slot and one active runner slot exist at a time. The settings workspace still mounts the execution panel, and the frontend shell is a Vue workspace shell rather than a page-routed SPA.

**Source anchors:** `REPO_MAP.md` (frontend shell map, settings workspace note, live-execution scope and constraints); `executionService.ts`; `executionStore.ts`; `commands_data.rs`; `commands_execution.rs`; `state.rs`.

## 2. Consolidated must-fix findings

### A. The umbrella doc needs a hard “no silent re-arm” invariant for reconnect / restart behavior

This is the strongest must-fix item. The plan says recovery and reconnect need hardening, but it does not state the key invariant: an unexpected feed close, error, or restart must not silently leave broker-facing automation effectively armed.

In the current code, the Polygon stream loop emits `closed` or `error` on unexpected interruptions and then exits, but it does not stop the live execution runner on those paths. The runner is only forcibly stopped on explicit feed stop, or when a replaced prior stream is for a different symbol. That means a same-symbol feed restart can resume bars into an already-active runner without a new explicit runner start. The review map explicitly calls this out as the challenge focus for the recovery doc, which is a strong sign this should be promoted into the umbrella epic itself rather than left as a downstream interpretation.

**Source anchors:** `commands_data.rs` (`run_polygon_live_candle_stream`, `polygon_start_live_candle_stream`, `polygon_stop_live_candle_stream`); `commands_execution.rs` (`start_live_execution_runner`, `on_live_execution_bar`); `05_live_execution_productization_2026-04-10.md` (Proposed Improvement Approach); `gpt54_live_mode_review_map.md` (Bundle 08 focus / challenge).

### B. Secure credential storage is not just a related child doc; it is a prerequisite

The umbrella doc currently lists secure credential storage as a related doc, but the repo shows a split secret-ownership model today. Alpaca credentials are still loaded from frontend storage, mirrored in frontend state, persisted back to frontend storage, and cleared from frontend storage only. By contrast, Polygon keys already prefer backend keyring storage with JSON fallback and migration logic. The review map also explicitly says remembered live broker credentials are an identified hardening gap.

This should be elevated from “related doc” to an explicit prerequisite before surface promotion or stronger live-mode claims.

**Source anchors:** `executionService.ts` (persisted execution key loading / syncing); `executionStore.ts` (persisted key mirror / initial state); `polygon.rs` (`set_api_key`, `clear_api_key`, `read_api_key`, keyring migration); `REPO_MAP.md` (current repo live posture, app-local storage layout); `gpt54_live_mode_review_map.md` (current repo live posture, Bundle 11 focus).

### C. The umbrella doc must define operator-truth invariants for provisional versus broker-confirmed state

The current code already has a two-layer truth model. The frontend can synthesize optimistic `submitted*` rows from runner events, and `runOnce` explicitly tells the operator to use refresh for broker-confirmed state. If the umbrella doc does not elevate that distinction, later docs on reconciliation, session evidence, and chart review can blur provisional local state into fake certainty.

This is one of the clearest places where the repo already knows the truth model, but the umbrella doc still understates it.

**Source anchors:** `executionService.ts` (`optimisticOrderRowFromExecution`, `runOnce`); `executionStore.ts` (`applyLiveExecutionRunnerEvent`, optimistic row prepend); `gpt54_live_mode_review_map.md` (Bundle 10 focus / challenge); `05_live_execution_productization_2026-04-10.md` (Proposed Improvement Approach).

### D. Chart and account review completion should be treated as a prerequisite, not a later nice-to-have

The current review surface is explicitly incomplete. The service still reports `Execution chart sync pending`, and the execution store initializes the chart state with `Load a dataset and refresh orders to overlay broker events.` The review map for Bundle 12 explicitly says to determine whether the chart/account review doc separates truthful account review from the still-incomplete execution chart.

That means the umbrella epic should call out this incompleteness directly and avoid implying the current execution review surface is already a finished operator-truth surface.

**Source anchors:** `executionService.ts` (`chartMeta`); `executionStore.ts` (initial `chartMeta` / `positionMeta` / review state); `gpt54_live_mode_review_map.md` (current repo live posture, Bundle 12 focus / challenge); `05_live_execution_productization_2026-04-10.md` (Related Docs).

### E. The umbrella doc needs a corrected ownership map because the current packet contains a stale runtime-owner statement

This is a confirmed packet contradiction. `REPO_MAP.md` says `AppState` owns live runner state, but the provided code shows something different: `AppState` owns the live candle stream slot, while the live execution runner lives in a static `OnceLock<Mutex<Option<...>>>` inside `commands_execution.rs`.

If the umbrella plan is going to talk about shared-state ownership, reconnect, and recovery, it should explicitly describe the current split rather than inheriting the stale summary from `REPO_MAP.md`.

**Source anchors:** `REPO_MAP.md` (Tauri runtime state section); `state.rs` (`AppState`, `live_candle_stream`, stream slot ownership); `commands_execution.rs` (`LIVE_EXECUTION_RUNNER`, runner runtime ownership); `05_live_execution_productization_2026-04-10.md` (External Research Summary, Current Code Surface).

### F. The verification and rollback section is too weak for the actual risk surface

The current verification plan is decent as a local touched-file checklist, but too generic for this umbrella epic. The repo’s routing docs and agent instructions already define broader high-signal gates such as `cargo test --workspace`, frontend `test`, `parity`, `a11y:smoke`, and the workflow gate. The current plan also does not require scenario testing around reconnect / no-silent-rearm behavior, secret migration precedence, provisional-versus-confirmed state, or dataset-mutation side effects.

Those should be part of the umbrella verification / rollback contract, not left to chance in later child docs.

**Source anchors:** `05_live_execution_productization_2026-04-10.md` (Current Verification Surface, Verification Plan, Rollout / Fallback / Compatibility Notes); `REPO_MAP.md` (verification order and quality-gate sections); `AGENTS.md` (common commands, repo quality gate, workspace tests).

## 3. Consolidated should-fix findings

### A. The plan should explicitly state that current safety controls are not yet implemented backend controls

The doc says flatten / kill-switch / cancel-order style controls are not first-class today, but it does not spell out that these are net-new backend semantics rather than UI refinements. The current live command surface in `lib.rs` only exposes one-shot execution, runner start/stop, snapshots, account testing, and Polygon live stream start/stop on the live path. The execution surface today does not expose flatten, cancel-open-orders, or emergency-halt commands as first-class backend controls.

That should be explicit in the umbrella doc so later child docs do not accidentally assume the backend semantics already exist.

**Source anchors:** `05_live_execution_productization_2026-04-10.md` (Weaknesses / Failure Modes, Proposed Improvement Approach); `lib.rs` (Tauri command registry); `ExecutionWorkspacePanel.vue` (current action groups); `commands_execution.rs` (current live commands).

### B. The current “preflight gating already exists” claim is true but overstated

The plan says the frontend already has preflight gating, and that is partly true, but incomplete. Frontend start-live gating does not model Polygon-key readiness even though the backend start-live command fails if the Polygon key is missing. Frontend refresh-orders availability is also broader than credential readiness, and the runtime fails later if Alpaca keys are absent.

This does not make the current doc wrong, but it does make it too flattering to the current readiness story.

**Source anchors:** `05_live_execution_productization_2026-04-10.md` (Current User-Facing Behavior, Strengths Already Present); `executionStore.ts` (`setAvailability`, blocked-reason helpers); `executionService.ts` (runtime credential checks); `commands_data.rs` (`polygon_start_live_candle_stream`).

### C. The plan should surface current runtime side effects: live bars mutate the loaded dataset by `data_id`

This is important for chart truth, recovery semantics, and auditability. Live bars are not just transient feed events; they are written into the in-memory loaded dataset by `data_id`, and derived descendants are invalidated when the dataset changes. The umbrella doc talks about safety, recovery, and session evidence, but it does not mention this shared-state side effect.

That omission can cause downstream docs to miss how charting, resampling, and operator review surfaces interact.

**Source anchors:** `state.rs` (`upsert_live_bar`, derived invalidation, dataset versioning); `REPO_MAP.md` (live-bar / dataset mutation note); `commands_execution.rs` (`on_live_execution_bar`).

### D. The shell-placement decision should be explicitly bounded to the current shell model

The current app is not a route-based SPA, and `REPO_MAP.md` explicitly says the settings workspace currently mounts the execution panel. The review map also warns that Bundle 09 should challenge assumptions about `AppShell.vue` or `router` ownership.

The umbrella doc already says to decide whether the execution surface stays under settings or moves, but it should go one step further and state that this epic does not imply a route / AppShell rewrite.

**Source anchors:** `05_live_execution_productization_2026-04-10.md` (Proposed Improvement Approach, Checklist); `REPO_MAP.md` (frontend shell model, settings workspace note); `gpt54_live_mode_review_map.md` (Bundle 09 focus / challenge).

### E. The umbrella doc should summarize the dependency role of child docs 06–12 instead of relying on them by name alone

The current packet includes only the umbrella doc, not the child docs. The related-doc list is useful, but the umbrella plan would be stronger if it said which lanes are prerequisites and which are later-facing improvements. Given the repo-grounded review map, the likely prerequisites are credentials, chart/account review, reconciliation truth, and reconnect semantics.

The umbrella doc should summarize that dependency order itself so the plan remains legible without requiring the entire child-doc tree in the reader’s head.

**Source anchors:** `05_live_execution_productization_2026-04-10.md` (Related Docs, Checklist, Rollout / Fallback / Compatibility Notes); `gpt54_live_mode_review_map.md` (Bundle 05 focus / challenge and recommended execution shape).

### F. The current event vocabulary and silent-cancellation posture should be called out explicitly

`REPO_MAP.md` says runner cancellation remains intentionally silent on the event bus, and the current event vocabulary is limited to `started`, `stopped`, `triggered`, `queued_latest`, `signal_processed`, `order_submitted`, and `error`. The umbrella plan should either preserve that contract explicitly or state that a child doc will revisit it. Right now it is implicit.

**Source anchors:** `REPO_MAP.md` (live-execution event vocabulary / runner-cancellation note); `commands_execution.rs` (runner event emission paths).

## 4. Optional improvements worth considering

Make the delayed-feed default explicit in the operator journey. Both the repo map and the store/backend start-live path show that live stock streaming defaults to delayed mode unless disabled. That is meaningful operator posture, not just implementation trivia.

**Source anchors:** `REPO_MAP.md` (current live posture); `executionStore.ts` (initial `liveUseDelayed` state); `commands_data.rs` (`polygon_start_live_candle_stream` delayed default).

Add a short **Current runtime reality** subsection to the umbrella doc with four bullets: current command surface, current owner map, current shared-state side effects, and current known incompleteness. That would make the rest of the child docs easier to interpret without changing scope.

**Source anchors:** `05_live_execution_productization_2026-04-10.md` (Current User-Facing Behavior, Weaknesses / Failure Modes, Proposed Improvement Approach).

Call out that live execution currently reuses drilldown compute paths and depends on a loaded dataset context rather than being a standalone live-strategy authoring mode. That keeps the epic truthful and helps prevent later docs from drifting into broader authoring-surface ideas.

**Source anchors:** `REPO_MAP.md` (live-execution scope and drilldown routing); `executionService.ts` (strategy resolution from run context / manual execution path).

## 5. Suggestions to reject as out of scope or unsupported

Reject any suggestion that turns this umbrella epic into multi-broker expansion, options live trading, cloud sync, portfolio dashboards, OMS redesign, or compliance-grade archival. The scope boundaries and review-map rules are explicit that this bundle should stay inside the current narrow live posture.

**Source anchors:** `05_live_execution_productization_2026-04-10.md` (Scope Boundaries, Rollout / Fallback / Compatibility Notes); `PROMPT_03_consolidation.md` (constraints block); `gpt54_live_mode_review_map.md` (Hard scope guard, common prompt rules).

Reject any suggestion that assumes a route-based shell rewrite or owners like `AppShell.vue` / `router`. The review map explicitly says those owners do not exist in this repo context, and `REPO_MAP.md` says the frontend is a workspace shell, not a page-routed SPA.

**Source anchors:** `REPO_MAP.md` (frontend runtime map, shell ownership); `gpt54_live_mode_review_map.md` (Bundle 09 challenge focus).

Do not overstate a proven dataset/symbol identity bug from the current packet. There is a plausible hidden assumption there because the runner validates live stream symbol against requested symbol and separately uses strategy `data_id`, but the attached files do not prove a concrete mismatch path in normal product use. That is a reasonable open question for a child doc, not a confirmed umbrella must-fix based on this packet alone.

**Source anchors:** `commands_execution.rs` (`strategy_data_id`, runner preflight checks, `on_live_execution_bar`); `state.rs` (`upsert_live_bar`).

Likewise, do not assume the child-doc filenames in Related Docs are stale or nonexistent just because they were not attached here. The confirmed stale element in the packet is the ownership statement in `REPO_MAP.md`, not the existence of those child docs.

**Source anchors:** `05_live_execution_productization_2026-04-10.md` (Related Docs); `REPO_MAP.md` (Tauri runtime state section); `state.rs`; `commands_execution.rs`.

## 6. Recommended doc edits

Add a new **Current runtime reality** section after **Current User-Facing Behavior** that states:

- live execution remains narrow: `DoubleMa` and `CompositeCpu`, Alpaca REST, Polygon stocks, single live stream, single runner;
- the execution panel is still settings-mounted in the current shell;
- Alpaca credentials are still frontend-persisted today while Polygon keys already prefer backend keyring storage;
- chart/account review is still incomplete and chart sync is still pending in the current service/store surfaces.

**Source anchors:** `05_live_execution_productization_2026-04-10.md` (Current User-Facing Behavior, Weaknesses / Failure Modes); `executionService.ts` (`chartMeta`, persisted keys); `executionStore.ts` (initial chart/review state, persisted keys); `REPO_MAP.md` (settings workspace note, live-execution scope).

Add an **Operator truth invariants** section that says:

- runner events and optimistic rows are provisional;
- broker snapshots are authoritative for order/position confirmation;
- chart overlays and review surfaces must preserve provenance;
- no reconnect or restart path may silently resume broker-armed automation after interruption.

**Source anchors:** `executionService.ts` (`optimisticOrderRowFromExecution`, `runOnce` guidance); `commands_data.rs` (live stream event paths and stop behavior); `commands_execution.rs` (runner events, bar-triggered execution).

Add a **Current ownership map** section that corrects the stale routing summary:

- frontend shell placement is owned by the workspace shell model, not a router;
- `AppState` owns loaded datasets and the live stream slot;
- `commands_execution.rs` owns the runner runtime today;
- `commands_data.rs` and `polygon.rs` own feed lifecycle and Polygon-key behavior.

**Source anchors:** `REPO_MAP.md` (frontend runtime map, Tauri runtime state, shell ownership); `state.rs` (`AppState`, live stream slot); `commands_execution.rs` (`LIVE_EXECUTION_RUNNER`); `commands_data.rs`; `polygon.rs`.

Promote secure credential storage and chart/account review completion from passive related-doc entries into explicit prerequisites in either the checklist or rollout section.

**Source anchors:** `05_live_execution_productization_2026-04-10.md` (Checklist, Rollout / Fallback / Compatibility Notes, Related Docs).

Replace the current generic verification bullets with a small lane-based matrix that includes:

- reconnect interruption / restart scenario tests,
- secret migration and precedence tests,
- provisional-versus-confirmed truth tests,
- dataset mutation / invalidation awareness where relevant,
- broader repo gates for higher-risk cross-layer changes.

**Source anchors:** `05_live_execution_productization_2026-04-10.md` (Verification Plan); `REPO_MAP.md` (verification order, quality gate, workflow gate); `AGENTS.md` (workspace tests, repo quality gate).

## 7. Recommended implementation order

1. **Update the umbrella doc first** so it accurately states current runtime reality, ownership, operator-truth invariants, and rollback / verification expectations. That gives the later child docs a truthful top-level contract.

   **Source anchors:** `05_live_execution_productization_2026-04-10.md` (Proposed Improvement Approach, Checklist, Verification Plan).

2. **Make secure credential storage an explicit prerequisite lane.** The current split between Alpaca frontend persistence and Polygon backend keyring is too fundamental to leave as a sibling convenience item.

   **Source anchors:** `executionService.ts`; `executionStore.ts`; `polygon.rs`; `gpt54_live_mode_review_map.md` (Bundle 11 focus).

3. **Do broker position/order reconciliation truth work next,** because it defines how optimistic local state, refresh flows, and broker-confirmed state relate.

   **Source anchors:** `executionService.ts`; `executionStore.ts`; `gpt54_live_mode_review_map.md` (Bundle 10 focus).

4. **Do chart/account review completion after reconciliation truth,** so chart and account surfaces can be grounded in explicit provenance instead of `chart sync pending` placeholders.

   **Source anchors:** `executionService.ts`; `executionStore.ts`; `gpt54_live_mode_review_map.md` (Bundle 12 focus).

5. **Then do operator safety controls,** once the backend-authoritative truth model is explicit and the doc clearly says which controls are net-new.

   **Source anchors:** `05_live_execution_productization_2026-04-10.md` (Weaknesses / Failure Modes, Proposed Improvement Approach); `gpt54_live_mode_review_map.md` (Bundle 06 focus).

6. **Then do recovery/reconnect,** with the umbrella-level no-silent-rearm invariant already in place.

   **Source anchors:** `commands_data.rs`; `commands_execution.rs`; `gpt54_live_mode_review_map.md` (Bundle 08 challenge focus).

7. **Then do session audit trail,** once provenance rules and reconnect semantics are stable enough to capture truthfully.

   **Source anchors:** `05_live_execution_productization_2026-04-10.md` (Proposed Improvement Approach); `gpt54_live_mode_review_map.md` (Bundle 07 focus).

8. **Leave discrete surface promotion for last.** The current shell-placement discussion is real, but it should follow safety, truth, credentials, and review-surface completion rather than lead them.

   **Source anchors:** `05_live_execution_productization_2026-04-10.md` (Proposed Improvement Approach, Checklist); `REPO_MAP.md` (frontend runtime map, settings workspace note); `gpt54_live_mode_review_map.md` (Bundle 09 focus / challenge).
