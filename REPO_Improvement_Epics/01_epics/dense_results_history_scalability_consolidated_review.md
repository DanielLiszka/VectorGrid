# Dense Results And History Scalability — Consolidated Review

This document packages the consolidated conclusion for the **Dense Results And History Scalability** epic bundle as a portable Markdown artifact.

## Surface understanding

The epic is aimed at the right problem: keep dense result and history surfaces responsive under heavier real-world loads without weakening readability, keyboard access, or table-truth semantics. Its stated risks are also the right ones: wrong scroll host, drift in virtualization assumptions, keyboard/focus regressions, and fixtures that are too small to prove anything meaningful.

The main consolidation result is that the attached modules do **not** behave like one uniform “dense table” family. From the attached panel modules themselves:

- `TopWorkspacePanel.vue` is a keyboardable/selectable virtualized review table.
- `ValidationWorkspacePanel.vue` is a simpler virtualized results table.
- `HistoryWorkspacePanel.vue` is a saved-run/library list workflow with its own selection/search behavior.
- `OverviewWorkspacePanel.vue` is winner/baseline review rather than a dense table.
- `ExecutionWorkspacePanel.vue` is best treated as a mechanical virtualization reference, not a semantic interaction template.

`REPO_MAP.md` reinforces that split by locating history state in `historyStore.ts`, shell placement in `App.vue` and `VueWorkspaceShell.vue`, mount behavior in `useShellMount.ts`, artifact handoff in `workflowArtifactsStore.ts`, and shared dense-table infrastructure in `useHeadlessMatrixTable.ts`, `usePersistedMatrixColumnState.ts`, and `useVirtualTableRows.ts`.

I do **not** see a confirmed stale or nonexistent file reference in the provided bundle. The stronger issue is omitted owner files and over-unified scope language, not broken filenames.

**Primary sources:** `02_dense_results_history_scalability_2026-04-10.md` (Purpose, Scope Boundaries, Current User-Facing Behavior, Current Code Surface, Weaknesses / Failure Modes); `REPO_MAP.md` (Shell and panel ownership, Important stores, Composable infrastructure, Verification Map); `TopWorkspacePanel.vue`; `ValidationWorkspacePanel.vue`; `HistoryWorkspacePanel.vue`; `OverviewWorkspacePanel.vue`; `ExecutionWorkspacePanel.vue`.

## Consolidated must-fix findings

### 1. The scope taxonomy is misleading and needs to be split before implementation

- **Severity:** High
- **Type:** Scope / Correctness
- **Location:** `02_dense_results_history_scalability_2026-04-10.md` — Scope Boundaries, Current User-Facing Behavior, Proposed Improvement Approach
- **Why it matters:** The doc still frames the work as “large Top, Validation, Overview, and History table behavior,” but the attached surfaces are not one table class. That makes the acceptance criteria too vague and invites the wrong abstraction, especially for History and Overview.
- **Recommended change:** Split the epic into explicit surface families inside the same doc:
  - dense matrix surfaces: Top Results, Validation Results
  - dense library/list surfaces: History / Saved Runs
  - winner/baseline review under large-run context: Overview
  - execution-order table: reference and regression surface only, unless scope is explicitly widened

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Scope Boundaries, Current User-Facing Behavior, Proposed Improvement Approach); `REPO_MAP.md` (Shell and panel ownership, Important stores); `TopWorkspacePanel.vue`; `ValidationWorkspacePanel.vue`; `HistoryWorkspacePanel.vue`; `OverviewWorkspacePanel.vue`; `ExecutionWorkspacePanel.vue`.

### 2. The owner map is incomplete for the exact failure modes the doc names

- **Severity:** High
- **Type:** Migration
- **Location:** `02_dense_results_history_scalability_2026-04-10.md` — Current Code Surface, Rollout / Fallback / Compatibility Notes; `REPO_MAP.md` — Shell and panel ownership, Important stores, Saved run rehydration and history bridge, Composable infrastructure
- **Why it matters:** The doc is explicitly worried about wrong scroll-host binding and shell-layout assumptions, but its owner list stops at panels plus two shared composables. `REPO_MAP.md` says shell placement is owned by `App.vue` and `VueWorkspaceShell.vue`, mount behavior by `useShellMount.ts`, history behavior by `historyStore.ts` and `historyService.ts`, and artifact handoff by `workflowArtifactsStore.ts`. Those omissions are large enough to let the implementation chase panel-local symptoms while the real owner sits elsewhere.
- **Recommended change:** Expand “Current Code Surface” to include at least `App.vue`, `VueWorkspaceShell.vue`, `useShellMount.ts`, `historyStore.ts`, `historyService.ts`, `workflowArtifactsStore.ts`, and `usePersistedMatrixColumnState.ts`, or explicitly state why each is ruled out.

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Current Code Surface, Rollout / Fallback / Compatibility Notes); `REPO_MAP.md` (Shell and panel ownership, Important stores, Saved run rehydration and history bridge, Composable infrastructure).

### 3. The virtualization contract is underspecified and the current “threshold” wording is inaccurate

- **Severity:** High
- **Type:** Correctness / Operator-truth
- **Location:** `02_dense_results_history_scalability_2026-04-10.md` — Scope Boundaries, Proposed Improvement Approach, Checklist; `useVirtualTableRows.ts`; `useHeadlessMatrixTable.ts`
- **Why it matters:** The doc keeps talking about “virtualization thresholds” and “row-height contracts,” but `useVirtualTableRows.ts` does not expose a threshold switch at all: it always virtualizes the rows passed in, it binds through an explicit `scrollElementRef`, it estimates size from `rowHeightPx`, and it keys virtual items by index. `useHeadlessMatrixTable.ts` also falls back to index-based row ids when no explicit `rowId` exists. That means the current doc language sounds more precise than the underlying contract actually is.
- **Recommended change:** Replace generic “threshold” wording with a per-surface contract covering:
  - when virtualization is enabled, or whether it is always-on
  - expected `rowHeightPx`
  - expected `overscan`
  - which element owns scrolling
  - whether stable row identity is required
  - how visible-row truth and focus restoration will be proven

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Scope Boundaries, Proposed Improvement Approach, Checklist); `useVirtualTableRows.ts`; `useHeadlessMatrixTable.ts`.

### 4. The doc’s keyboard/focus/a11y requirement is too generic for the actual surface differences

- **Severity:** High
- **Type:** Correctness / Verification
- **Location:** `02_dense_results_history_scalability_2026-04-10.md` — Scope Boundaries, Checklist, Verification Plan
- **Why it matters:** The bundle-specific focus requires clear specs for keyboard/a11y behavior, and the adversarial challenge requires strong protection for keyboard navigation, focus restoration, and visible-row truth. But the current doc only says “verify keyboard navigation, focus restoration, and screen-reader-visible structure.” That is not enough when Top, Validation, and History already have materially different interaction models.
- **Recommended change:** Add a per-surface interaction contract table that defines, for each touched surface:
  - role model (`grid`, `table`, `list`, or summary/review)
  - what is selectable
  - supported key bindings
  - what should retain focus after rerender/filter/sort/search
  - what must remain screen-reader-visible

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Scope Boundaries, Checklist, Verification Plan); `TopWorkspacePanel.vue`; `ValidationWorkspacePanel.vue`; `HistoryWorkspacePanel.vue`; `OverviewWorkspacePanel.vue`.

### 5. Verification and rollback are weaker than the named risks

- **Severity:** High
- **Type:** Verification / Migration
- **Location:** `02_dense_results_history_scalability_2026-04-10.md` — Weaknesses / Failure Modes, Checklist, Verification Plan, Rollout / Fallback / Compatibility Notes; `REPO_MAP.md` — Verification Map; `AGENTS.md` — Frontend work policy
- **Why it matters:** The doc names wrong scroll hosts, tiny fixtures, visible-row truth, and keyboard/focus regressions as the hard part, but the verification plan still boils down to targeted Vitest, one browser dense-surface journey, smoke accessibility, and screenshots. `REPO_MAP.md` already points to a browser-first manual accessibility review path for high-risk UI changes, and `AGENTS.md` explicitly prefers Playwright and screenshot for frontend verification. The rollback section also asks to “record rollback conditions” without defining any.
- **Recommended change:** Strengthen the doc so closure requires:
  - an explicit browser proof of the actual bound scroll element
  - an explicit browser proof that visible rows update correctly after scroll
  - one heavy-load browser proof per changed interaction family, not just one generic journey
  - one manual browser accessibility slice per changed dense surface
  - concrete rollback triggers such as wrong scroll host, wrong visible-row truth, lost focus restoration, History navigation regression, or helper-level regression on non-target surfaces

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Weaknesses / Failure Modes, Checklist, Verification Plan, Rollout / Fallback / Compatibility Notes); `REPO_MAP.md` (Verification Map, Build / Dev / Release Workflow); `AGENTS.md` (Frontend work policy).

### 6. Shared-helper blast radius is missing from the rollout story

- **Severity:** High
- **Type:** Migration
- **Location:** `02_dense_results_history_scalability_2026-04-10.md` — Current Code Surface, Rollout / Fallback / Compatibility Notes; `REPO_MAP.md` — Composable infrastructure
- **Why it matters:** `REPO_MAP.md` says `useHeadlessMatrixTable.ts` is shared across Top, Validation, Meta, and execution-order review, and `useVirtualTableRows.ts` is shared across Top, Validation, Meta, and execution-order review. A helper-level change can therefore break Meta or Execution even if the epic’s primary target is Top/Validation/History/Overview.
- **Recommended change:** Add an explicit “shared-helper blast radius” note: if either helper changes, Meta and Execution must be regression-checked even if they are not implementation targets.

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Current Code Surface, Rollout / Fallback / Compatibility Notes); `REPO_MAP.md` (Composable infrastructure).

## Consolidated should-fix findings

### 1. The fixture plan needs concrete sizes and sources, not just “representative dense fixtures”

- **Severity:** Medium
- **Type:** Testing
- **Location:** `02_dense_results_history_scalability_2026-04-10.md` — Proposed Improvement Approach, Checklist, Verification Plan
- **Why it matters:** The bundle focus explicitly asked whether the plan uses realistic dense fixtures rather than synthetic micro-tests. The current doc says to define fixtures, but never states row counts, width pressure, or whether the fixtures are artifact-derived, store-derived, or synthetic. That leaves too much room for underpowered proofs.
- **Recommended change:** Add a fixture matrix with explicit target row counts, column counts, wide-table pressure, and source type for Top, Validation, and History.

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Proposed Improvement Approach, Checklist, Verification Plan).

### 2. “Selection persistence” should be rewritten as per-surface state preservation

- **Severity:** Medium
- **Type:** Correctness
- **Location:** `02_dense_results_history_scalability_2026-04-10.md` — Regressions To Avoid, Checklist; `REPO_MAP.md` — history ownership, persisted column posture, saved-run rehydration
- **Why it matters:** The epic currently talks about “selection persistence” as though that were one shared behavior. In the attached bundle, the preserved state differs by surface: Top has candidate-focused review state, Validation has persisted column posture, History has saved-run/search state, and Overview has baseline review state. `REPO_MAP.md` also makes clear that history/library state and artifact handoff are owned outside a single table surface.
- **Recommended change:** Replace the generic phrase with an explicit list of preserved state per surface.

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Regressions To Avoid, Checklist); `REPO_MAP.md` (Important stores, Saved run rehydration and history bridge, Composable infrastructure).

### 3. Execution should be described as a mechanical reference, not a semantic interaction template

- **Severity:** Medium
- **Type:** Scope
- **Location:** `02_dense_results_history_scalability_2026-04-10.md` — Current User-Facing Behavior; `useVirtualTableRows.ts`; `REPO_MAP.md` — Composable infrastructure
- **Why it matters:** The current epic says Execution provides “at least one clear virtualization pattern,” which is fair mechanically, but it should not imply that Execution proves the right keyboard/focus/selection semantics for Top or History. It proves shared-helper usage and an alternate `rowHeightPx` / `overscan` choice, not a shared interaction contract.
- **Recommended change:** Reword Execution as a mechanical virtualization reference and regression surface only.

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Current User-Facing Behavior); `useVirtualTableRows.ts`; `REPO_MAP.md` (Composable infrastructure).

### 4. The verification command hierarchy should be clarified

- **Severity:** Medium
- **Type:** Verification
- **Location:** `02_dense_results_history_scalability_2026-04-10.md` — Current Verification Surface, Verification Plan; `REPO_MAP.md` — Verification Map
- **Why it matters:** The doc names `test:browser:ux` and `baseline:ui`, but `REPO_MAP.md` elevates the broader frontend gates (`test`, `typecheck`, `lint`, `parity`, `a11y:smoke`) and the repo quality gate. That does not prove the named commands are wrong; it does mean the doc should say whether they are canonical, supplemental, or local-only evidence.
- **Recommended change:** Label each named verification command by role: required gate, supplemental evidence, or only-needed-when-visual-layout-changes.

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Current Verification Surface, Verification Plan); `REPO_MAP.md` (Verification Map, Build / Dev / Release Workflow).

### 5. Stable row identity should be required before the doc claims focus restoration

- **Severity:** Medium
- **Type:** Correctness
- **Location:** `useHeadlessMatrixTable.ts`; `useVirtualTableRows.ts`; `02_dense_results_history_scalability_2026-04-10.md` — Checklist, Verification Plan
- **Why it matters:** The shared table helper falls back to index-based row ids, and the virtualizer keys items by index. That is workable for rendering, but it is weak as a proof basis for focus restoration after reorder/filter changes unless the touched surface can guarantee stable row identity.
- **Recommended change:** Add a doc note that any surface claiming focus-restoration guarantees under virtualization must first confirm stable row identity.

**Sources:** `useHeadlessMatrixTable.ts`; `useVirtualTableRows.ts`; `02_dense_results_history_scalability_2026-04-10.md` (Checklist, Verification Plan).

## Optional improvements worth considering

1. Add a short **surface contract matrix** section near the top of the doc. One compact table covering owner files, scroll owner, role model, state to preserve, virtualizer settings, and proof requirements would remove most of the current ambiguity without widening scope.

2. Add an **acceptance evidence template** so closure requires the same evidence every time: fixture size, scroll-owner proof, visible-row proof, keyboard/focus proof, and accessibility review note. That would keep the epic aligned with the bundle’s emphasis on realistic fixtures and browser-side accessibility evidence.

3. Keep the MDN `content-visibility` note explicitly secondary. The doc’s external research section mentions it, but the current owner files do not make it a first-class contract. It is fine as an additive experiment later, but it should not substitute for explicit viewport/virtualization proof.

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (External Research Summary, Verification Plan, Checklist); `REPO_MAP.md` (Verification Map, Composable infrastructure).

## Suggestions to reject as out of scope or unsupported

1. Reject any suggestion that tries to force History into the same abstraction as Top/Validation just because all three can become “large.” `REPO_MAP.md` explicitly treats History as a saved-run/library surface with store-owned search, ranking, selected-run hydration, and library actions, so collapsing it into a generic dense-table lane would hide real ownership and behavior differences.

2. Reject broad shell redesign proposals. The epic already marks broad shell redesign as out of scope, and the prompt packet asked for additive, low-regression changes rather than architecture churn.

3. Reject widening the epic into backend artifact indexing or unrelated feature expansion. The epic explicitly keeps backend indexing out of scope except where it directly shapes visible dense-surface load.

4. Reject treating Meta as a primary implementation target unless scope is formally expanded. `REPO_MAP.md` makes Meta part of the shared-helper blast radius, but the bundle does not make it a primary target surface for this epic.

5. Reject any claim that the provided bundle proves a stale or nonexistent file reference. That is not supported by the files here.

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Scope Boundaries, Rollout / Fallback / Compatibility Notes); `REPO_MAP.md` (Shell and panel ownership, Important stores, Composable infrastructure).

## Recommended doc edits

1. Replace the scope bullet “large Top, Validation, Overview, and History table behavior” with three separate bullets: dense matrix surfaces, dense library/list surfaces, and Overview winner/baseline review under large-run context. Keep Execution reference-only.

2. Expand “Current Code Surface” to include `App.vue`, `VueWorkspaceShell.vue`, `useShellMount.ts`, `historyStore.ts`, `historyService.ts`, `workflowArtifactsStore.ts`, and `usePersistedMatrixColumnState.ts`.

3. Add a “Current surface contract matrix” section with columns for: owner files, scroll owner, interaction model, preserved state, virtualizer settings, stable-row-id requirement, and required proof.

4. Replace “virtualization thresholds” wording with the actual contract language the helpers support: activation policy, `rowHeightPx`, `overscan`, scroll owner, and identity guarantees.

5. Replace “Add one browser dense-surface journey” with explicit verification bullets for scroll-owner binding, visible-row truth, keyboard/focus restoration, and accessibility review by surface family.

6. Add a named rollback subsection with hard triggers instead of a generic reminder.

7. Add a shared-helper blast-radius note that requires Meta and Execution regression proof whenever `useHeadlessMatrixTable.ts` or `useVirtualTableRows.ts` changes.

8. Add a fixture matrix with concrete sizes and source types for Top, Validation, and History.

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Scope Boundaries, Current Code Surface, Proposed Improvement Approach, Verification Plan, Rollout / Fallback / Compatibility Notes, Checklist); `REPO_MAP.md` (Shell and panel ownership, Important stores, Composable infrastructure, Verification Map); `useHeadlessMatrixTable.ts`; `useVirtualTableRows.ts`.

## Recommended implementation order

1. **Fix scope language and owner map first.**  
   Until the doc stops treating all five surfaces as one table family, the rest of the plan stays ambiguous.

2. **Add the per-surface contract and fixture matrix next.**  
   That locks scroll ownership, interaction model, state preservation, and proof requirements before any code moves.

3. **Resolve shell/mount ownership questions before helper edits.**  
   If scroll ownership is ambiguous, settle `App.vue` / `VueWorkspaceShell.vue` / `useShellMount.ts` implications before changing shared virtualization behavior.

4. **Harden Top and Validation first.**  
   They are the clearest primary consumers of the shared dense-table path, so they should define the table-side contract.

5. **Handle History separately after the table contract is stable.**  
   Treat it as dense library/list scalability, not as table standardization by default.

6. **Only then touch shared helpers, with Meta and Execution regression proof.**  
   That contains blast radius and keeps non-target surfaces honest.

7. **Close only after browser, accessibility, and rollback evidence is recorded.**  
   Do not close on component tests plus screenshots alone.

**Sources:** `02_dense_results_history_scalability_2026-04-10.md` (Scope Boundaries, Proposed Improvement Approach, Verification Plan, Rollout / Fallback / Compatibility Notes, Checklist); `REPO_MAP.md` (Shell and panel ownership, Composable infrastructure, Verification Map); `AGENTS.md` (Frontend work policy).

## Source packet used

- `PROMPT_01_first_pass.md`
- `PROMPT_02_second_pass.md`
- `PROMPT_03_consolidation.md`
- `CONTEXT_FILE_LIST.txt`
- `02_dense_results_history_scalability_2026-04-10.md`
- `AGENTS.md`
- `REPO_MAP.md`
- `useHeadlessMatrixTable.ts`
- `useVirtualTableRows.ts`
- panel modules: `TopWorkspacePanel.vue`, `ValidationWorkspacePanel.vue`, `HistoryWorkspacePanel.vue`, `OverviewWorkspacePanel.vue`, `ExecutionWorkspacePanel.vue`
- program-level context: `REPO_Improvement_Epics/README.md`

## Note on citations

This Markdown version uses portable source annotations (`file name` + `section/module`) rather than chat-only citation tokens so it remains readable outside the chat UI.
