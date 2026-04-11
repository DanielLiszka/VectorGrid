# Bundle 12 Consolidated Review
## Live Execution Chart And Account Review Completion

Basis: this consolidation is grounded in the provided packet only: `12_live_execution_chart_and_account_review_completion_2026-04-11.md`, `05_live_execution_productization_2026-04-10.md`, `AGENTS.md`, `REPO_MAP.md`, `ExecutionWorkspacePanel.vue`, `executionService.ts`, `executionStore.ts`, `commands_execution.rs`, `analysisService.ts`, `lib.rs`, and the two attached screenshots.

UI evidence used in this review:
- [Screenshot 1 — execution review surface](sandbox:/mnt/data/SCREENSHOT_01_execution_review_surface.jpg)
- [Screenshot 2 — execution review section](sandbox:/mnt/data/SCREENSHOT_02_execution_review_section.jpg)

---

## 1. Surface understanding

Bundle 12 is aimed at a real gap, but the current implementation is less review-complete than the plan language suggests.

The umbrella live-execution doc keeps the live surface intentionally narrow: current Alpaca broker path, Polygon live candles, a single active stream, a single active runner, and a settings-mounted execution surface rather than a fully promoted operating mode. That narrow posture is real and should remain the frame for this bundle. Source: `05_live_execution_productization_2026-04-10.md` (`Scope Boundaries`, `Current User-Facing Behavior`, `Current Code Surface`), `REPO_MAP.md` (`If the task is live execution or broker integration`, frontend shell notes).

The current execution review surface already has useful pieces:
- a monitor section
- runtime text
- a position summary string
- an order table
- a raw output preview
- a visible chart host

But it is still mostly a review shell rather than a finished truthful review surface. In the current UI, “Account and review” is still just buttons plus helper text, the chart host is visibly blank, and the chart metadata is still driven by `Execution chart sync pending`. Sources: `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (`Current User-Facing Behavior`), `ExecutionWorkspacePanel.vue` (`Account and review`, `Execution review surface`, `Execution chart context`), `executionService.ts` (`chartMeta`, `refreshOrders`), screenshots.

The frontend data model is also thinner than the plan implies. The execution panel is still driven by string-style review fields such as `statusHint`, `liveStatusHint`, `runnerStatusHint`, `positionMeta`, `chartMeta`, a flattened `orderRows` list, and a single `outputPreview`. There is no first-class typed account-summary model in the current execution service or store. Sources: `executionService.ts` (`ExecutionActionResult`, `ExecutionSnapshotResponse`, `testConnection`, `refreshOrders`, `runOnce`, `refreshStatuses`), `executionStore.ts` (`ExecutionState`, `applyActionResult`), `ExecutionWorkspacePanel.vue` (bound refs in `<script setup>` and review panel rendering).

That means the bundle’s core question is not only “how do we finish the chart?” It is “how do we create a truthful review contract for account, order, position, chart, and freshness without making the surface look more authoritative than it really is?”

---

## 2. Consolidated must-fix findings

### Must-fix 1 — symbol scope is currently ambiguous enough to make chart/position review misleading

**Status:** confirmed doc gap  
**Severity:** critical  
**Type:** correctness / operator-truth  
**Location:** `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (`Purpose`, `Checklist`), `executionStore.ts` (`setAvailability`), `executionService.ts` (`refreshOrders`, `chartMeta`), `commands_execution.rs` (`symbol_matches`, `get_live_execution_snapshot`), `ExecutionWorkspacePanel.vue` (`Execution chart context`, `Broker order activity`)

**Why it matters:**  
The current review surface does not require a symbol before `Refresh orders` can run. In `executionStore.ts`, `canRefreshOrders` is only blocked by busy state; it is not symbol-gated. In `executionService.ts`, `refreshOrders()` sends an empty symbol when the input is blank and explicitly labels the result as “orders for all symbols.” On the backend, `get_live_execution_snapshot` filters orders by symbol when one exists, but when `symbol` is empty it returns all matching orders and then, for positions, iterates all positions and returns the first normalized position it finds because `symbol_matches(..., "")` returns `true` and the loop breaks on the first hit. The same response then feeds `positionMeta` and `chartMeta`, even though the chart/table copy in `ExecutionWorkspacePanel.vue` assumes a single current symbol.

This means the current surface can show:
- all-symbol order rows,
- a single arbitrary position,
- and a symbol-less chart placeholder,
while the table caption still describes broker order activity “for the current execution symbol.”

That is too ambiguous for a chart-and-account completion bundle.

**Recommended change:**  
Add an explicit symbol scope contract to the doc:
1. chart review requires one explicit symbol;
2. position review requires one explicit symbol;
3. all-symbol order review is allowed only in a non-chart, no-position mode;
4. no-symbol refresh must not populate symbol-level chart or position summaries;
5. the execution review surface must visibly label the current scope (`symbol-scoped` vs `all-symbol`).

---

### Must-fix 2 — the bundle does not define a real freshness contract, and the current header timestamp is not broker freshness

**Status:** confirmed doc gap  
**Severity:** high  
**Type:** operator-truth  
**Location:** `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (`Checklist`, `Rollout / Fallback / Compatibility Notes`), `commands_execution.rs` (`GetLiveExecutionSnapshotResponse`, `get_live_execution_snapshot`), `executionService.ts` (`ExecutionSnapshotResponse`, `refreshOrders`), `executionStore.ts` (`markVue`, `updatedAtUnixMs`), `ExecutionWorkspacePanel.vue` (`stamp`, header meta)

**Why it matters:**  
The backend already exposes `fetched_at_unix_ms` on `GetLiveExecutionSnapshotResponse`, but the frontend snapshot type in `executionService.ts` does not model it and `refreshOrders()` does not surface it. Instead, the visible header stamp in `ExecutionWorkspacePanel.vue` is derived from store `updatedAtUnixMs`, and `executionStore.ts` updates that value through `markVue()` on many unrelated state changes, including input edits, pending-state transitions, event handling, and action completion.

So the header can show a fresh-looking timestamp even when:
- no broker snapshot was fetched,
- only local form state changed,
- only live/runner hints changed,
- or only optimistic rows were prepended.

For a review bundle centered on operator trust, that is a direct truthfulness problem.

**Recommended change:**  
The doc should require a freshness contract for each review cluster:
- source command,
- authoritative fetch timestamp field,
- visible “as of” display rule,
- stale threshold,
- and degraded-state copy.

The current generic store mutation timestamp should not be used as broker-review freshness.

---

### Must-fix 3 — the plan does not choose an authoritative account-review source, and the frontend has no first-class account-summary model

**Status:** confirmed doc gap  
**Severity:** high  
**Type:** correctness / migration / operator-truth  
**Location:** `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (`Strengths Already Present`, `Proposed Improvement Approach`, `Checklist`), `commands_execution.rs` (`test_alpaca_connection`, `GetLiveExecutionSnapshotResponse`, `get_live_execution_snapshot`), `executionService.ts` (`ExecutionConnectionResponse`, `ExecutionSnapshotResponse`, `testConnection`, `refreshOrders`), `executionStore.ts` (`ExecutionState`, `applyActionResult`), `ExecutionWorkspacePanel.vue` (`Account and review`)

**Why it matters:**  
The bundle currently overstates the account-review base.

What exists today is split:
- `test_alpaca_connection` can return richer account fields such as `buying_power`, `cash`, and `portfolio_value`.
- `get_live_execution_snapshot` currently returns only `account_status`, `fetched_at_unix_ms`, `position`, and `orders`.
- `refreshOrders()` explicitly calls snapshot with `include_account: false`.
- `ExecutionConnectionResponse` in the frontend only models `account_number`, `account_id`, and `status`.
- `ExecutionSnapshotResponse` does not model `account_status` or `fetched_at_unix_ms`.
- the store has no structured account-summary state.
- the visible “Account and review” block in the current UI is still just action buttons and helper text.

So richer account data exists, but mostly as raw command output rather than as a first-class review model. The bundle needs to decide where account truth comes from before it designs a better chart.

**Recommended change:**  
Add a dedicated “Review Data Contract” subsection that decides:
- whether account truth comes from an extended snapshot response, the existing account-test command, or a composed view from both;
- which account fields are required for this bundle;
- whether account review is account-wide or symbol-scoped;
- what freshness and provenance rules apply;
- and what structured frontend view model will replace today’s thin string-only handling.

---

### Must-fix 4 — optimistic and broker-confirmed states are still mixed too loosely for a chart/account review bundle

**Status:** confirmed doc gap  
**Severity:** high  
**Type:** operator-truth  
**Location:** `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (`Regressions To Avoid`, `Checklist`), `executionService.ts` (`optimisticOrderRowFromExecution`, `runOnce`), `executionStore.ts` (`prependOptimisticOrderRows`, `applyActionResult`, `applyLiveExecutionRunnerEvent`), `ExecutionWorkspacePanel.vue` (header count, order table)

**Why it matters:**  
The current frontend already prepends optimistic `submitted*` rows before broker-confirmed refresh. `runOnce()` can return `prependOrderRows`, runner events can do the same, and `executionStore.ts` merges those rows into `orderRows` and updates `orderCount`. The header then renders a plain `{{ orderCount }} orders`, and the order table has no top-level provenance summary beyond the row status text.

Bundle 12 warns against plotting optimistic or stale data as broker truth, but it does not yet lock down the same distinction for:
- top-line counts,
- table state,
- or future chart markers.

Without that, the bundle can make the surface look cleaner while preserving a hidden truth mix.

**Recommended change:**  
Require a provenance matrix covering:
- broker-confirmed order rows,
- locally optimistic submitted rows,
- partial fills,
- filled/canceled/rejected states,
- position-confirmed states,
- and any chart markers derived from those.

Also require that counters and summaries be labeled as `broker-confirmed`, `optimistic`, or `mixed` whenever appropriate.

---

### Must-fix 5 — the chart contract is still too vague, and current chart copy can already overclaim

**Status:** confirmed doc gap  
**Severity:** high  
**Type:** operator-truth / correctness  
**Location:** `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (`Current User-Facing Behavior`, `Weaknesses / Failure Modes`, `Checklist`), `ExecutionWorkspacePanel.vue` (`Execution chart context`), `executionService.ts` (`chartMeta`), screenshots

**Why it matters:**  
The current component hardcodes chart copy that says `Execution chart (loaded dataset + broker orders)`, but the surrounding surface can simultaneously show no dataset loaded at the app level, and the chart host can still be blank. The service still generates `Execution chart sync pending. symbol=... orders=...`, and there is no actual execution chart wiring in the provided files beyond the host div and metadata text.

This means the current chart shell is already visually ahead of the underlying contract. The bundle cannot safely move forward with “prettier chart” work unless it first defines truthful chart states.

**Recommended change:**  
Add an explicit chart-state taxonomy to the doc:
- no dataset loaded,
- no symbol selected,
- selected symbol has no chart data,
- broker review loaded but chart unavailable,
- chart loaded with no broker markers,
- chart loaded with broker-confirmed markers,
- stale broker data,
- markers suppressed because provenance rules are not satisfied.

The doc should also say that the chart must never be the sole carrier of execution truth.

---

### Must-fix 6 — the current raw preview and review sections do not have explicit provenance boundaries

**Status:** likely issue, but strongly grounded in current owner files  
**Severity:** high  
**Type:** operator-truth / maintainability  
**Location:** `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (`Proposed Improvement Approach`, `Checklist`), `executionService.ts` (`testConnection`, `refreshOrders`, `runOnce`, `refreshStatuses`), `executionStore.ts` (`applyActionResult`, `failAction`), `ExecutionWorkspacePanel.vue` (`Execution runtime status`, `Execution output preview`)

**Why it matters:**  
The current review surface aggregates multiple source domains:
- live-feed status,
- runner status,
- execution status,
- broker orders,
- position text,
- chart metadata,
- and a single raw output preview.

But the raw preview is a single shared pane whose contents depend on the last action that wrote `outputPreview`. That may be:
- account test output,
- broker snapshot output,
- one-shot execution output,
- or an error string.

At the same time, the chart advisory text tells the user to refresh broker orders and review the raw preview below, even though the preview may no longer correspond to the visible chart/order state.

**Recommended change:**  
The doc should require explicit section-level provenance:
- which command populated the section,
- what scope it covers,
- when it was fetched,
- and whether it is broker-confirmed or local/runtime-derived.

For the preview itself, either split the preview by source class or visibly label it with command, scope, and timestamp.

---

## 3. Consolidated should-fix findings

### Should-fix 1 — the bundle’s owner-file list is incomplete for the work it describes

**Status:** confirmed doc gap  
**Severity:** medium  
**Type:** maintainability / migration  
**Location:** `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (`Current Code Surface`), `05_live_execution_productization_2026-04-10.md` (`Current Code Surface`), `REPO_MAP.md` (`If the task is live execution or broker integration`, Tauri support/runtime state sections)

**Why it matters:**  
Bundle 12 omits `src-tauri/src/state.rs`, even though the repo routing documents say live execution tasks should start with `executionService.ts`, `executionStore.ts`, `commands_execution.rs`, and `state.rs`. The umbrella live doc also includes `commands_data.rs`, `state.rs`, and `polygon.rs` in the current live surface. Because bundle 12 talks about chart prerequisites, loaded-dataset posture, and review truth, those runtime/data owner files are relevant even if the first implementation slice stays small.

**Recommended change:**  
Expand `Current Code Surface` to include `src-tauri/src/state.rs` at minimum. If the bundle touches dataset-readiness truth, add the relevant data owner files explicitly rather than leaving them implicit.

**Stale-file note:** no stale or nonexistent file reference was confirmed from the provided packet. The problem is missing owner coverage, not obviously broken filenames.

---

### Should-fix 2 — reuse guidance around `analysisService.ts` is too vague and risks ownership drift

**Status:** confirmed doc gap  
**Severity:** medium  
**Type:** maintainability  
**Location:** `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (`Proposed Improvement Approach` item 2), `analysisService.ts`, `REPO_MAP.md` (`If the task is drilldown, local stability, or follow-up analysis`)

**Why it matters:**  
The plan says to reuse the existing charting approach already present in drilldown/lightweight chart flows. But `analysisService.ts` is not a small chart helper. It is a large owner for drilldown, stability, heatmap, pareto, cache, and chart DOM behavior. Reusing “the charting approach” without tighter wording invites accidental coupling between execution review and analysis follow-up ownership.

**Recommended change:**  
Replace the reuse line with a narrower statement:
- reuse a minimal lightweight-charts wrapper or formatting helpers if helpful,
- do not import drilldown/heatmap/pareto state or analysis caches into execution review,
- and keep execution review ownership separate from analysis follow-up logic.

---

### Should-fix 3 — the doc should explicitly state that this bundle must work while the surface is still settings-mounted

**Status:** confirmed doc gap  
**Severity:** medium  
**Type:** migration  
**Location:** `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (`Purpose`, `Related Docs`), `05_live_execution_productization_2026-04-10.md` (`Current User-Facing Behavior`, `Checklist`), `REPO_MAP.md` (frontend shell notes)

**Why it matters:**  
The bundle purpose is phrased in terms of discrete live mode, but the current repo still mounts `ExecutionWorkspacePanel.vue` under the settings workspace. The umbrella live doc treats discrete surface promotion as a separate decision/bundle. Without an explicit compatibility note, contributors can incorrectly assume surface promotion is a prerequisite for truthful chart/account review.

**Recommended change:**  
Add a compatibility sentence that this bundle must improve the current settings-mounted review surface first, and that discrete-surface promotion is separate work rather than a hidden dependency.

---

### Should-fix 4 — the verification plan is too generic for the current truth risks

**Status:** confirmed doc gap  
**Severity:** medium  
**Type:** verification / testing  
**Location:** `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (`Current Verification Surface`, `Verification Plan`), owner files listed above

**Why it matters:**  
The current verification section is not wrong, but it does not explicitly target the highest-risk review cases visible in the owner files:
- no symbol selected,
- no dataset loaded,
- loaded dataset symbol mismatch,
- stale broker freshness,
- optimistic order counts/rows,
- raw preview provenance,
- configured live mode vs verified account state,
- and all-symbol order scope.

Those are exactly the cases most likely to weaken trust while still leaving the UI “working.”

**Recommended change:**  
Add explicit non-happy-path verification items and require screenshot evidence for them, not only for the final polished state.

---

### Should-fix 5 — configured broker mode and verified account state should be explicitly separated

**Status:** likely issue  
**Severity:** medium  
**Type:** operator-truth  
**Location:** `ExecutionWorkspacePanel.vue` (header badge, posture cards), `executionService.ts` (`testConnection`), `12_live_execution_chart_and_account_review_completion_2026-04-11.md` (`Checklist`)

**Why it matters:**  
The visible mode badge in the current header says `Live broker` based on selected mode, not on a successful account test. The posture cards also summarize configured broker mode, submission mode, feed, runner, and key state, but not verified account state. If bundle 12 adds richer account review without explicitly separating `configured live mode` from `verified account state`, the surface can still look more broker-ready than it really is.

**Recommended change:**  
Make the doc require a separate verified-account indicator so configured live mode never substitutes for successful account verification.

---

## 4. Optional improvements worth considering

### Optional improvement 1 — add a compact visual legend for chart provenance

A small chart legend would help if the bundle later introduces markers. The legend should distinguish:
- dataset candles,
- broker-confirmed markers,
- optimistic markers,
- unavailable markers,
- and suppressed markers.

This would force provenance into the visual design instead of leaving it implicit.

### Optional improvement 2 — keep the chart visually subordinate until truth contracts are satisfied

The screenshots show a very large blank chart host. If chart truth remains incomplete while account/order/position review improves, consider keeping the chart section visually subordinate or suppressed rather than allowing it to dominate the review surface.

### Optional improvement 3 — preserve raw preview as an intentional troubleshooting surface

Even after account review becomes structured, it may still be useful to keep raw command output available as a compact troubleshooting disclosure, provided the preview has explicit source labeling.

### Optional improvement 4 — add screenshot acceptance evidence for non-happy paths

The bundle already asks for screenshots. It would be stronger to require screenshots for:
- no dataset,
- symbol mismatch,
- account-test-only review,
- and broker snapshot loaded with chart suppressed.

---

## 5. Suggestions to reject as out of scope or unsupported

These suggestions should be rejected for this bundle based on the provided packet:

1. **Full portfolio dashboard work.**  
   The bundle explicitly excludes that, and the current live scope remains narrow.

2. **Multi-broker expansion or options live trading.**  
   Both are outside the current live posture described in the umbrella doc.

3. **Turning this into a generic chart redesign.**  
   The bundle is about truthful execution review, not a broad charting rewrite.

4. **Treating discrete-surface promotion as a hard prerequisite.**  
   Bundle 12 should improve the current settings-mounted surface first.

5. **Reusing broad `analysisService.ts` state as the execution chart owner.**  
   That would increase coupling without evidence that the repo wants that ownership model.

6. **Cloud sync, compliance platforms, or enterprise archival systems.**  
   None of that is required by the provided files, and it would widen the scope well beyond the repo’s current live posture.

---

## 6. Recommended doc edits

The most useful edits would be additive and specific.

### A. Add a new subsection: `Review Data Contract`

Suggested fields:
- review element
- scope (`account-wide`, `symbol-wide`, `all-symbol`, `local-runtime`, `optimistic-local`)
- source command
- freshness field
- provenance class
- fallback/hidden-state rule

This subsection is the missing center of gravity for the bundle.

### B. Add a new subsection: `Symbol Scope Rules`

Suggested rules:
- no chart without a symbol,
- no position summary in all-symbol mode,
- all-symbol order refresh is explicitly labeled and non-chartable,
- and the UI must not reuse single-symbol copy when review scope is broader than one symbol.

### C. Correct `Strengths Already Present`

Rewrite the current “snapshot can already return richer account and order/position state” line so it is accurate:
- richer order/position review already exists on the snapshot path,
- richer account fields already exist, but mainly through the separate account-test path.

### D. Expand `Current Code Surface`

At minimum add:
- `src-tauri/src/state.rs`

If chart readiness or loaded-dataset truth becomes part of the implementation, add the relevant data owner files explicitly instead of leaving them implied.

### E. Add a new subsection: `Freshness And Provenance Rules`

Required rules:
- backend fetch timestamps drive review freshness,
- local store mutation timestamps never impersonate broker freshness,
- optimistic/local state must be visually distinct from broker-confirmed state,
- and stale review sections must have explicit copy.

### F. Add a new subsection: `Chart Empty / Suppressed States`

Define the no-dataset, no-symbol, mismatch, no-marker, stale, and suppressed-marker states explicitly.

### G. Add a new subsection: `Raw Preview Provenance`

Require the preview to show which command produced it, what scope it covers, and when it was generated.

### H. Add a compatibility note

State that bundle 12 improves the current settings-mounted review surface and does not depend on bundle 09 surface promotion.

### I. Strengthen `Verification Plan`

Add explicit coverage for:
- no symbol,
- no dataset,
- symbol mismatch,
- stale snapshot freshness,
- optimistic rows/counters,
- configured live mode vs verified account state,
- and raw preview provenance.

### J. Add a rollback condition

State plainly that richer account/order/position review may ship without chart overlays if chart provenance cannot yet be made truthful enough.

---

## 7. Recommended implementation order

A safe implementation order would be:

1. **Define the review data contract first.**  
   Decide account source, symbol scope, freshness fields, provenance classes, and fallback behavior before adding chart logic.

2. **Tighten the frontend review model.**  
   Move from today’s string-heavy `positionMeta` / `chartMeta` / single-preview approach toward structured review state for account, orders, position, freshness, and provenance.

3. **Fix scope and freshness truth before visual chart work.**  
   Specifically resolve the no-symbol / all-symbol ambiguity and replace UI-local freshness with backend-backed freshness.

4. **Ship richer structured account/order/position review first.**  
   This is the lower-regression way to improve the surface while keeping the current chart honesty.

5. **Add section-level provenance and mixed-state counter rules.**  
   Do this before introducing any markers or summary counters that could overstate broker certainty.

6. **Only then add chart wiring and markers.**  
   The chart should be the last step, after provenance, scope, and freshness rules are already explicit.

7. **Treat discrete-surface promotion as separate follow-up work.**  
   Bundle 09 can improve surface placement later, but it should not block the truth contract required here.

---

## Bottom line

The bundle is pointed at the right gap, but it is not yet specific enough about review truth. The highest-priority fixes are not visual polish. They are:
- symbol scope,
- freshness,
- account source ownership,
- optimistic-vs-broker provenance,
- and truthful chart/preview states.

If those contracts are added first, the bundle can remain narrow, additive, and low-regression while still materially improving operator trust.
