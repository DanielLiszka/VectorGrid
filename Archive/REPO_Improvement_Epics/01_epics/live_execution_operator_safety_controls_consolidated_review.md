# Live Execution Operator Safety Controls — Third-Pass Consolidated Review

## Scope of this review

This document consolidates the repo-grounded third-pass review for **Bundle 06: Live Execution Operator Safety Controls**.

It is based on the attached live-mode plan docs, current frontend and backend owner files, the repo routing documents, and the current UI evidence in `ExecutionWorkspacePanel.vue` and `SCREENSHOT_01_runner_confirmation_state.jpg`.

## Reviewed materials

- `05_live_execution_productization_2026-04-10.md`
- `06_live_execution_operator_safety_controls_2026-04-11.md`
- `AGENTS.md`
- `REPO_MAP.md`
- `commands_execution.rs`
- `executionService.ts`
- `executionStore.ts`
- `executionRisk.ts`
- `ExecutionWorkspacePanel.vue`
- `lib.rs`
- `SCREENSHOT_01_runner_confirmation_state.jpg`

## 1. Surface understanding

Bundle 06 is correctly aimed at a narrow live surface, not a broader trading platform. In the provided repo posture, live execution is real but still constrained to the current non-options path, current Alpaca broker path, current Polygon stocks live-feed path, one active stream slot, one active runner slot, and a settings-mounted execution panel rather than a fully promoted live shell. The current live strategy surface is also still limited to `DoubleMa` and `CompositeCpu`.

The bundle is directionally right that the missing work is not “make live execution exist,” but “make operator control semantics truthful and explicit.” Today the shipped surface already has one-shot execution, live-feed start/stop, runner start/stop, broker snapshot refresh, and Alpaca connectivity tests, while flatten, cancel-open-orders, and emergency halt are not first-class controls yet.

The consolidation outcome is that the plan is useful, but it is not implementation-ready until it becomes much more explicit about control semantics, backend authority, scope, and confirmation truth.

## 2. Consolidated must-fix findings

### 1. The doc still does not cleanly separate all five controls

**Severity:** High  
**Type:** Safety / Correctness  
**Location:** `06_live_execution_operator_safety_controls_2026-04-11.md` — Scope Boundaries, Proposed Improvement Approach, Checklist

The bundle-specific focus is to separate stop runner, stop feed, cancel open orders, flatten, and emergency halt. But the plan’s own in-scope list names stop runner, flatten, cancel-open-orders, and emergency-halt semantics, while `stop live feed` appears later in the proposed approach and checklist rather than in the formal scope list. The checklist also records exact semantics for stop runner vs stop feed vs flatten vs cancel-open-orders, but emergency halt is only framed as a persistence question rather than a full operator-control contract.

That is still not a clean five-way separation.

**Recommended change:** Add one explicit control matrix with a row for each action and columns for:
- what it does
- what it does not do
- persistence
- runner interaction
- feed interaction
- symbol scope
- confirmation requirements
- blocked-state rules

### 2. The current owner surface is incomplete for the scope the doc claims

**Severity:** High  
**Type:** Maintainability / Migration  
**Location:** `06_live_execution_operator_safety_controls_2026-04-11.md` — Current Code Surface, Current Verification Surface; compare `05_live_execution_productization_2026-04-10.md` — Current Code Surface; `REPO_MAP.md` — live execution / live stream ownership; `lib.rs` — command registry

Bundle 06 omits `src-tauri/src/commands_data.rs` and `src-tauri/src/state.rs`, even though it explicitly wants to define stop-feed semantics and backend-authoritative control behavior. The umbrella live doc includes those owners, `REPO_MAP.md` says live stream state and live runner state live in `AppState`, and `lib.rs` shows live-feed start/stop/status as public commands that sit outside `commands_execution.rs`.

If the doc keeps stop-feed in scope, those files are not optional context; they are owner files.

**Recommended change:** Add them to both the current code surface and the verification surface, and likely name `src-tauri/src/polygon.rs` as an adjacent owner when feed-stop or halt behavior touches live-stream internals.

### 3. Flatten, cancel-open-orders, and emergency halt need explicit new backend contracts and scope rules

**Severity:** Critical  
**Type:** Correctness / Safety  
**Location:** `06_live_execution_operator_safety_controls_2026-04-11.md` — Proposed Improvement Approach, Checklist; `commands_execution.rs` — `execute_strategy_signal_once()`, `execute_strategy_signal_blocking()`, `get_live_execution_snapshot()`; `executionService.ts` — `refreshOrders()`

The current backend contract is not a generic safety-action layer. `commands_execution.rs` currently exposes one-shot signal execution, runner start/stop/status, broker snapshot, and account test. The actual execution path is delta-to-target-per-symbol logic driven by strategy output; it is not a reusable flatten/cancel/halt primitive.

Separately, the current review path is not scope-safe enough to be reused implicitly for safety actions:
- `get_live_execution_snapshot()` fetches `status=all` orders
- when `symbol` is blank the matching logic broadens to all symbols
- `executionService.ts` also labels blank-symbol order refresh as “all symbols”

That means the doc must define dedicated backend request/response contracts for flatten, cancel-open-orders, and any shipped halt/disarm behavior, and it must freeze symbol scope explicitly.

The safest repo-grounded default is **current-symbol-only with a required non-blank canonical symbol**, unless another narrower follow-up doc deliberately widens scope.

**Recommended change:** Define dedicated backend request/response contracts for flatten, cancel-open-orders, and emergency halt/disarm, including:
- symbol scope
- request shape
- response shape
- partial-failure handling
- result-state wording

### 4. Runner-active interaction and backend-owned disarm semantics are still unresolved

**Severity:** Critical  
**Type:** Safety / Operator-truth  
**Location:** `06_live_execution_operator_safety_controls_2026-04-11.md` — Checklist, Rollout / Fallback / Compatibility Notes; `commands_execution.rs` — `start_live_execution_runner()`, `on_live_execution_bar()`; `lib.rs` — public command registry; `REPO_MAP.md` — runner event semantics

The current runner keeps reacting to new live bars and can keep submitting orders while active. It also depends on an active matching live stream, and backend runner replacement is possible because `start_live_execution_runner()` cancels the prior runtime and installs a new one. `REPO_MAP.md` also notes that runner cancellation is intentionally silent on the event bus outside the shipped event vocabulary.

That makes three safety interactions mandatory to specify before implementation:
- what happens if an operator flattens while the runner is still active
- what happens if they cancel orders while the runner is still active
- what happens if they stop the live feed while the runner is still armed

The doc does not need to pick a specific policy yet, but it does need to force a choice:
- blocked
- allowed with explicit warning
- auto-stop-runner first
- backend disarm

The same issue makes emergency halt/disarm a backend concern, not a frontend-only UX concern: if halt is meant to block future arming, it must reject at least `execute_strategy_signal_once` and `start_live_execution_runner`, or it should stay out of the shipped control set.

**Recommended change:** Add an explicit runner/feed interaction section and a backend disarm section that names the commands that must reject while disarmed.

### 5. The confirmation posture is not yet concrete enough for the current broker-facing UI evidence

**Severity:** High  
**Type:** Operator-truth  
**Location:** `06_live_execution_operator_safety_controls_2026-04-11.md` — Checklist; current UI owner `ExecutionWorkspacePanel.vue`; current screenshot `SCREENSHOT_01_runner_confirmation_state.jpg`

The current UI evidence shows a focused modal, but the copy is still generic. In the current UI owner file, `startLive` and `startRunner` route through a shared `ConfirmationDialog`, and the runner-start body says only that the auto-runner can issue repeated execution decisions as new bars arrive and that the operator should confirm mode, parameter source, and risk settings first.

The screenshot confirms the practical problem: the background is blurred and the modal does not restate the actual symbol, dry-run vs broker-armed posture, feed state, parameter source, or risk overrides inside the confirmation itself.

The same component also passes a single `danger` posture into the shared dialog, which flattens confirmation severity unless the doc defines a real tiering rule.

**Recommended change:** Bundle 06 needs a concrete confirmation contract:
- echoed live values
- explicit “does not do” statements
- severity tiers driven by actual broker risk rather than by generic action names

### 6. Cancel-open-orders, bracket-order behavior, and broker-truth wording are still underspecified

**Severity:** High  
**Type:** Correctness / Operator-truth  
**Location:** `06_live_execution_operator_safety_controls_2026-04-11.md` — Current User-Facing Behavior, Checklist; `commands_execution.rs` — `fetch_alpaca_orders()`, `get_live_execution_snapshot()`, `alpaca_order_payload()`; `executionService.ts` — `runOnce()`

The current broker review surface is historical order review, not open-order truth: `fetch_alpaca_orders()` requests `status=all`, and the snapshot normalizes those rows into review data.

Separately, the current live execution path only attaches bracket orders when opening a new position and only when both stop-loss and take-profit are present. So the doc cannot leave flatten/cancel semantics vague around attached working orders.

It needs to state:
- whether flatten auto-cancels outstanding working legs first
- whether cancel-open-orders leaves the current position untouched
- how partial failure is surfaced

It should also reuse the truth discipline that already exists in `executionService.ts`’s one-shot flow, where submitted broker actions are not presented as broker-confirmed state and the user is told to refresh orders for confirmation.

**Recommended change:** Add explicit bracket-order interaction rules plus a standard result-state vocabulary such as:
- request submitted
- broker accepted
- broker confirmed
- partially failed
- review refresh required

## 3. Consolidated should-fix findings

### 1. The blocked-reason and disabled-state story is still uneven across the existing surface

**Severity:** High  
**Type:** Operator-truth / Testing  
**Location:** `executionStore.ts` — blocked-reason getters and availability logic; `executionService.ts` — `refreshOrders()`; `06_live_execution_operator_safety_controls_2026-04-11.md` — Strengths Already Present, Checklist

The plan leans on “blocked reasons already surface why actions are unavailable,” but current coverage is partial.

`executionStore.ts` has explicit blocked-reason getters for:
- run once
- start live
- start runner

But:
- stop-live and stop-runner are only boolean-gated
- `canRefreshOrders` stays enabled whenever the surface is not busy
- `executionService.ts` later throws if credentials are missing

The doc should broaden its disabled-state rule from “new safety controls” to “all execution-surface operator actions,” so the safety model does not become inconsistent around the edges.

### 2. Bundle 06 should not quietly absorb shell-promotion work

**Severity:** High  
**Type:** Scope / Migration  
**Location:** `06_live_execution_operator_safety_controls_2026-04-11.md` — Purpose, Proposed Improvement Approach; `05_live_execution_productization_2026-04-10.md` — Proposed Improvement Approach, Related Docs; `REPO_MAP.md` — shell and settings-workspace ownership

The bundle can absolutely harden operator semantics inside the current surface, but the actual settings-vs-dedicated-live-shell decision belongs to different owners. `REPO_MAP.md` says the settings workspace currently mounts both `SettingsWorkspacePanel.vue` and `ExecutionWorkspacePanel.vue`, and the umbrella doc already treats live-surface promotion as a separate follow-up decision with its own related doc.

**Recommended change:** Keep bundle 06 about safety semantics and control truth, and do not let it implicitly take ownership of full shell migration unless its owner list is explicitly expanded.

### 3. The verification plan should be widened when command surface or modal behavior changes

**Severity:** Medium  
**Type:** Verification  
**Location:** `06_live_execution_operator_safety_controls_2026-04-11.md` — Verification Plan; `REPO_MAP.md` — Verification Map

The current verification list is directionally good, but it underspecifies two repo-native checks that matter for this bundle:
- `npm --prefix web run parity` when Tauri command surface changes
- `npm --prefix web run a11y:smoke` when confirmation modals, danger controls, or state/status presentation change

It should also stop assuming all backend verification will live only in `commands_execution.rs` once stop-feed semantics or disarm ownership pull in `commands_data.rs` or `state.rs`.

### 4. The Tauri Dialog research note is misaligned with the provided current owner surface

**Severity:** Low  
**Type:** Maintainability  
**Location:** `06_live_execution_operator_safety_controls_2026-04-11.md` — External Research Summary; `lib.rs`; current UI owner `ExecutionWorkspacePanel.vue`

The doc cites Tauri Dialog as external research, but the provided current implementation is Vue-modal-driven and `lib.rs` does not show dialog-plugin initialization. That does not make the research invalid, but it does make it look more current-owner-like than the provided files support.

**Recommended change:** Either remove that implication or label it as optional research rather than as a likely implementation owner.

### 5. `executionRisk.ts` looks like adjacent context, not a primary safety-control owner

**Severity:** Low  
**Type:** Maintainability  
**Location:** `06_live_execution_operator_safety_controls_2026-04-11.md` — Current Code Surface / Verification Surface; `executionRisk.ts`

`executionRisk.ts` is a strategy-risk helper about same-bar execution and short exposure. That is useful context, but it is a different concern from operator controls like stop feed, flatten, cancel open orders, and emergency halt.

**Recommended change:** Unless the bundle is intentionally reusing those notices inside safety confirmations, it should probably move that file out of the primary owner list and keep it as optional context instead.

## 4. Optional improvements worth considering

A small but valuable addition would be a standard result-state vocabulary that all broker-facing controls share:
- request submitted
- broker accepted
- broker confirmed
- partially failed
- review refresh required

That would extend a truth boundary the code already uses in one-shot execution.

It would also help to require a fixed acceptance screenshot set for closure:
- start-live confirmation
- broker-armed runner-start confirmation
- flatten confirmation
- cancel-open-orders confirmation
- any halt/disarm posture

This bundle is unusually visual because the confirmation posture is part of the safety story.

Finally, the doc could include canned “does not do” sentences for each control so future copy work does not drift. That is an easy way to keep the operator-truth contract stable across UI revisions.

## 5. Suggestions to reject as out of scope or unsupported

Reject any move toward account-wide flatten or account-wide cancel semantics in this bundle unless a narrower follow-up doc explicitly widens scope. The umbrella live doc is still single-symbol/single-runner hardening, and the current command/service behavior is not safe enough to treat blank-symbol scope as harmless.

Reject turning bundle 06 into a full live-shell redesign. The current repo places execution in the settings workspace, and the umbrella doc already treats surface promotion as a separate related plan. Bundle 06 should harden semantics first.

Reject any assumption that the current historical broker-order table is already an open-order-control surface. The provided backend file does not support that claim.

Reject “Tauri Dialog adoption” as a required outcome from the provided evidence. The concrete mismatch here is owner-surface drift, not a proven need to replace the current Vue confirmation path.

Reject broader product drift into:
- multi-broker live trading
- multi-symbol orchestration
- options live trading
- portfolio dashboards
- compliance-grade archival inside this bundle

Those are explicitly outside the current scope posture.

There is also not enough evidence in the provided packet to call any related-doc path in the “Related Docs” block nonexistent. The concrete, supportable issue is missing owner coverage and unsupported implementation assumptions inside bundle 06 itself.

## 6. Recommended doc edits

Add `stop live feed` explicitly to the in-scope list, then add one five-row safety-control matrix under the proposed approach or checklist. Each row should cover:
- stop runner
- stop feed
- flatten current position
- cancel open orders
- emergency halt/disarm

Each row should state:
- what the control does
- what it does not do
- current-symbol-only vs broader scope
- runner-active behavior
- persistence
- confirmation requirements

Expand the current code and verification surfaces to include:
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/polygon.rs` when feed-stop or halt semantics touch stream internals

Add a dedicated backend-contract subsection for flatten, cancel-open-orders, and emergency halt/disarm. It should define:
- request/response shape
- symbol scope
- partial failure handling
- the minimum commands that must reject while disarmed

Add a runner/feed interaction subsection that forces a choice for these cases:
- stop feed while runner active
- flatten while runner active
- cancel open orders while runner active
- halt while a runner is present

The doc does not need to choose a preferred policy here; it just needs to stop leaving those cases implicit.

Add a confirmation contract subsection. Require the modal to restate:
- the actual symbol
- dry-run/paper/live arm state
- feed state
- parameter source
- risk overrides
- a negative statement such as “This does not flatten positions” or “This does not stop the runner”

Also add a severity-tier rule so the current shared danger posture can become more truthful.

Update the verification plan to add:
- parity when commands change
- accessibility smoke when relevant modal/status behavior changes
- screenshot evidence for the final confirmation states
- targeted Rust tests in all touched owner files, not just `commands_execution.rs`

## 7. Recommended implementation order

### 1. Do a doc-only pass first

Fix:
- scope
- owner surface
- the five-control matrix

This is the lowest-regression step and will flush out unresolved semantics before code starts moving.

### 2. Decide the backend-owned contracts next

Freeze:
- flatten
- cancel-open-orders
- whether emergency halt/disarm is backend-enforced now or deferred

Do not ship a frontend-only halt.

### 3. Freeze runner-active and feed-stop interaction rules

Also freeze:
- bracket-order interaction
- submitted-vs-confirmed result wording

These are the main operator-truth traps.

### 4. Update the current settings-mounted UI after the semantics are fixed

Implement:
- the confirmation contract
- full blocked-reason coverage
- full disabled-state coverage

Keep shell promotion out of this change unless the scope is intentionally expanded.

### 5. Land verification and evidence changes in the same slice

Include:
- targeted frontend tests
- targeted Rust tests in all touched owners
- parity when commands change
- accessibility smoke when modal behavior changes
- acceptance screenshots for the final operator states

## Final consolidated conclusion

Bundle 06 has the right target, but it still needs to move from “we should add safety controls” to a precise operator-truth contract that says exactly:
- what each control affects
- what it never affects
- who owns it
- what scope it operates in
- what the operator must see before confirming it

Until that contract is explicit, the plan is still not specific enough to implement safely.
