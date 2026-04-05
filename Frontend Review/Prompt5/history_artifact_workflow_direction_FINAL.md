# history_artifact_workflow_direction_FINAL.md

## Executive summary

VectorGrid should make **History the canonical Run Library for run artifacts**, keep **Session History as a secondary transient recall surface inside that library**, and keep **Meta as the contextual review surface for aggregate artifacts rather than a second canonical library**. The repo already defines Run Library as a library-shaped surface with a session list, saved-run list with search/tags, a selected-run detail card, and modal confirmation for clear/delete actions. The design-system spec also already treats modal confirmation as a shared canonical pattern, not an optional flourish. fileciteturn15file0 fileciteturn15file1

The right consolidation move is therefore **behavioral consolidation, not IA consolidation**: reuse one shared artifact-browsing pattern across History and Meta, but do **not** create a new top-level Artifact Center and do **not** merge Meta into History. The functional-coverage matrix already says History is the primary frontend path for run-artifact management, Meta is an overlapping secondary surface, a shared artifact-browser abstraction is the strongest safe consolidation candidate, and delete remains the high-stakes artifact action. fileciteturn16file11

The strongest current implementation base is already in `historyStore.ts` and `historyService.ts`: local fuzzy search for Saved Runs, selected-run detail hydration and caching, explicit `Load` / `Apply` / `Rerun` semantics, tag editing, reveal/open actions, and rich detail summaries for validation, metric surface, and ML-specific artifact fields. That product value should be preserved and then reused as the standard for aggregate artifact browsing. fileciteturn16file10 fileciteturn15file16 fileciteturn15file5

## Final artifact/history UX direction

### Adopted direction

**1. History becomes the unmistakable Run Library.**

History should carry the strongest “managed library” posture in copy, weighting, and interaction model. Saved Runs is the primary artifact-management surface. Session History remains present, but clearly secondary and explicitly framed as transient current-session recall. This matches the repo’s target comp for Run Library and the prior audit work that already rebuilt the area around richer list/detail browsing rather than a select-only dump. fileciteturn15file0 fileciteturn16file8

**2. Meta remains contextual, not canonical, for aggregate artifacts.**

Meta should continue to own aggregate artifact review for Meta, Stage 5, and policy-eval families because that work remains analysis-contextual. But Meta should stop behaving like three repeated mini-management forms. It should reuse the same browse/select/detail/action contract as History, with family-specific adapters. This preserves context without fragmenting artifact behavior. The repo map already places saved-run inspection in `HistoryWorkspacePanel.vue` and aggregate subworkspace ownership in `MetaWorkspacePanel.vue`. fileciteturn16file15

**3. Consolidate the pattern, not the workspace IA.**

Introduce a shared artifact-browser pattern that covers:

- a searchable/selectable artifact list
- a selected-artifact detail card
- a shared action taxonomy
- shared destructive-confirmation handling
- shared tags editing affordances where the artifact family supports tags

Do not introduce a new top-level Artifact Center now. The current repo evidence supports shared abstraction, but not a shell-level merge. fileciteturn16file11

**4. Keep run-artifact actions semantically distinct.**

For run artifacts, `Load`, `Apply`, and `Rerun` remain separate visible actions. `historyService.ts` makes clear they are materially different operations: `loadSavedRun` rehydrates active workflow artifacts, `applySavedRun` writes saved request data back into working stores, and `rerunSavedRun` rebuilds execution context and launches either optimization or ML Signal flows. Flattening these into one generic “Open” would remove real product meaning. fileciteturn15file16 fileciteturn15file5

**5. Make destructive safety consistent across History and Meta.**

Clear Session and delete actions should always route through the shared confirmation-dialog pattern. History already aligns with the target comp and design-system rule here. Meta should gain the same delete-confirmation posture instead of allowing direct menu-to-delete flows. fileciteturn15file0 fileciteturn15file1

### What remains intentionally unchanged

- History stays the primary run-artifact home. fileciteturn16file11
- Meta keeps contextual ownership of aggregate artifacts. fileciteturn16file15
- Local fuzzy search stays local to the library surface and remains subordinate to `Ctrl+K`. fileciteturn16file13
- `workflowArtifactsStore.ts` remains the canonical artifact handoff layer for active run/validation state. fileciteturn16file3
- The selected-detail surface remains the place to understand an artifact before acting on it. `historyStore.ts` already holds the right detail vocabulary for that model. fileciteturn16file16 fileciteturn15file17

## Recommended relationship between Run Library, History, and Meta artifact review

### Run Library

Run Library is the product term and mental model for persisted run artifacts. It should be surfaced through History, but the page should read library-first rather than activity-log-first. Its job is to let the operator:

- retrieve saved runs quickly
- inspect them deeply enough to trust them
- load/apply/rerun them without ambiguity
- manage tags and cleanup safely

The target comp already defines this surface as session history list + saved-run list with search/tags + selected detail card + modal confirmation. fileciteturn15file0

### History

History is the workspace route and should remain the canonical **run-artifact management** location. It may still contain a Session History section, but that section is secondary. In practice:

- **Saved Runs** = primary pane
- **Session History** = secondary recall pane
- **Selected run detail** = decision pane

This also aligns with the repo’s earlier audit direction to rebuild Run Library browsing around richer list/detail surfaces and add clearer compare/detail affordances and delete-confirmation flows. fileciteturn16file8

### Meta artifact review

Meta should remain the contextual management surface for:

- saved Meta artifacts
- saved Stage 5 artifacts
- saved policy-eval artifacts

But it should not compete with Run Library for ownership of run-artifact management. The right model is:

- History owns canonical run-artifact management.
- Meta owns contextual aggregate-artifact browsing and follow-up actions.
- Shared artifact-browser behavior keeps the two surfaces consistent where they overlap.

## Artifact-browser abstraction recommendation

Yes: introduce a **shared artifact-browser abstraction**, but keep it mostly visible in code rather than in shell navigation.

### Recommended abstraction level

Prefer a **pattern contract** first:

- list selection model
- selected-detail contract
- tags draft + save pattern
- action weighting model
- shared secondary-action menu contract
- shared confirmation-flow contract

Only extract shared components once the Meta Saved refactor proves the overlap is real and stable.

### Likely implementation shape

Near-term likely shared pieces:

- `web/src/components/artifacts/ArtifactListPane.vue`
- `web/src/components/artifacts/ArtifactDetailCard.vue`
- `web/src/components/artifacts/ArtifactFamilySwitch.vue` or a simpler family selector
- optionally a small artifact view-model helper/composable

What should **not** be extracted yet:

- a giant generic artifact store
- a new shell-level artifact workspace
- a universal cross-family schema that hides run-vs-aggregate differences

This is a “shared behavior, separate ownership” decision.

## Row actions, detail cards, tags, rerun, reveal/open, and delete-flow guidance

### Row actions

Rows should stay selection-first and dense. Do **not** add large inline button clusters to every row.

Use this weighting model:

- **Row interaction:** select the artifact
- **Primary selected-artifact actions:** visible in the detail/action plane
- **Secondary selected-artifact actions:** in `ContextActionMenu.vue`
- **Page utilities:** outside the selected-artifact action cluster

The repo already adopted a shared contextual action menu for secondary saved-artifact actions and already documented the discoverability and keyboard guardrails for that pattern. fileciteturn16file13

### Recommended action weighting in History

Keep these **visible** for the selected run:

- `Load`
- `Apply`
- `Rerun`
- `Open report` when available

Move these to **secondary** selected-item actions or a smaller utility plane:

- `Reveal`
- `Delete` (still explicit, still confirmed)

Keep these as **page-level utilities**, not selected-item peers:

- `Refresh library`
- `Open runs folder`

That prevents one flat action row from mixing core run decisions with generic utilities.

### Detail cards

History’s current state model is already the correct baseline for selected-run details. `historyStore.ts` carries the exact detail shape the product should keep: strategy, dataset, timeframe, validation status, metric-surface summary, admission summary, report status, size, tags, ML bundle/cache/schema/policy summaries, and a request snapshot. fileciteturn15file17 fileciteturn15file12

That means the detail card should remain the operator’s decision surface. It should answer:

- what this artifact is
- whether it is validated/exported
- what dataset/timeframe it used
- whether it carries ML-specific context
- whether it is trustworthy enough to load/apply/rerun

Meta’s saved artifacts should gain the same pattern depth rather than staying as repeated search/select/tags blocks with terse metadata.

### Tags

Keep tag editing selected-item-scoped, not page-scoped. `historyService.ts` already supports direct tag updates through `update_artifact_tags`, and `historyStore.ts` already keeps `savedRunsTagsDraft` coupled to the selected artifact. That is the right model to preserve. fileciteturn15file2 fileciteturn15file10

### Rerun

For run artifacts, `Rerun` stays a first-class visible action. It is not equivalent to `Load` or `Apply`, and it already contains the ML Signal branch logic needed to reactivate dataset export, score-cache generation, validation, or policy-sweep paths when the saved artifact is ML-specific. fileciteturn15file16 fileciteturn15file5

For aggregate artifacts in Meta, there is no need to force run-style rerun semantics where they do not map cleanly.

### Reveal / open report

`Open report` and `Reveal` remain selected-artifact follow-up actions. `historyService.ts` already treats them as separate capabilities, and both must remain reachable. `Open runs folder` remains a workspace utility rather than a selected-item decision. fileciteturn15file5 fileciteturn15file2

### Delete flows

Delete remains the explicit high-stakes artifact action. Use these rules:

- no direct delete from a dropdown item without confirmation
- show artifact identity in the dialog
- keep Cancel easy and obvious
- return focus deterministically to the invoking control after close
- disable backdrop-click dismissal for destructive dialogs
- keep Escape as a deliberate cancel path unless testing shows it creates accidental dismissals in this context

The design-system spec already defines modal confirmation with focus management and explicit cancel/confirm, and the Run Library target comp already requires modal confirmation for clear/delete actions. fileciteturn15file1 fileciteturn15file0

## Capability-preservation note

This direction preserves the current backend-backed artifact capability set and only changes ownership and weighting.

### Must remain clearly reachable

For run artifacts:

- list saved run artifacts
- search/filter them locally
- inspect selected-run details
- load saved run artifacts
- apply saved run settings
- rerun saved runs
- update tags
- reveal artifact paths
- open reports
- delete saved runs
- open runs directory
- clear session history

Those capabilities are already represented in `historyService.ts` and `historyStore.ts`, and History remains the canonical frontend path for them. fileciteturn16file11 fileciteturn15file16 fileciteturn15file8

For aggregate artifacts:

- keep at least one clear contextual path in Meta for load/review/reveal/report/delete actions that still belong with aggregate analysis work

What changes is not reachability, but **how duplicated management patterns are standardized**.

## Implementation order by file/store/service

### Stage 1 — safety and weighting cleanup in the current surfaces

**Primary files**

- [HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue)
- [MetaWorkspacePanel.vue](sandbox:/mnt/data/MetaWorkspacePanel.vue)
- [ConfirmationDialog.vue](sandbox:/mnt/data/ConfirmationDialog.vue)
- [ContextActionMenu.vue](sandbox:/mnt/data/ContextActionMenu.vue)

**Tasks**

- Change History copy so the page reads as Run Library first.
- Demote Session History visually relative to Saved Runs.
- Split page utilities out of the selected-run action cluster in History.
- Route Meta delete actions through a confirmation dialog instead of direct menu execution.
- Add a destructive-dialog option in `ConfirmationDialog.vue` that disables backdrop dismissal.

### Stage 2 — shared artifact-browser behavior contract

**Primary files**

- `web/src/components/artifacts/*` (new)
- [ContextActionMenu.vue](sandbox:/mnt/data/ContextActionMenu.vue)
- `historyStore.ts`
- `historyService.ts`

**Tasks**

- Define shared list/detail/action props or a composable contract.
- Reuse History’s stronger detail model as the baseline.
- Keep domain stores separate.

### Stage 3 — Meta Saved refactor

**Primary files**

- [MetaWorkspacePanel.vue](sandbox:/mnt/data/MetaWorkspacePanel.vue)
- likely `analysisStore.ts` / aggregate artifact actions

**Tasks**

- Replace the current three repeated Saved blocks with a family switch:
  - Meta
  - Stage 5
  - Policy Eval
- Reuse the shared browser block beneath that switch.
- Keep aggregate-specific follow-up actions local to Meta.

### Stage 4 — History refinement and polish

**Primary files**

- [HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue)
- `historyStore.ts`

**Tasks**

- Tighten selected-detail layout and action planes.
- Consider a sticky selected-run summary for long details views.
- Keep request snapshot behind disclosure.

### Stage 5 — optional trust cues

**Primary files**

- `historyStore.ts`
- [HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue)
- [MetaWorkspacePanel.vue](sandbox:/mnt/data/MetaWorkspacePanel.vue)

**Tasks**

- Add compact trust/status chips only where they are driven deterministically by artifact metadata.
- Defer broader stale-context drift markers until they are proven and stable.

## Regression and UX validation plan

Use the repo’s existing non-trivial frontend verification stack for each meaningful slice:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run a11y:smoke`
- `npm --prefix web run baseline:ui`
- `npm --prefix web run check:bundle-budget`

That is already the documented minimum guardrail for this class of implementation work. fileciteturn10file7

### Focused regression coverage

**Store/service tests**

- `historyStore.search.test.ts`
  - short-query determinism
  - typo tolerance
  - ranking stability
- `historyService` tests
  - `Load` / `Apply` / `Rerun` semantic separation
  - report/reveal/delete error handling

**Component tests**

- `HistoryWorkspacePanel.test.ts`
  - keyboard selection path
  - selected-detail rendering
  - action enablement and disabled states
  - clear/delete confirmation flow
- `MetaWorkspacePanel.test.ts`
  - family-switch behavior
  - contextual menu discoverability
  - confirmation parity for delete
- `ContextActionMenu.test.ts`
  - keyboard open/select
  - focus return
- `ConfirmationDialog.test.ts`
  - focus trap
  - focus return
  - explicit cancel/confirm behavior
  - backdrop-disabled destructive behavior

### Browser and UX validation

Capture/update browser baselines for:

- Run Library default state
- Run Library selected-run detail state
- Meta Saved family-browser state
- delete-confirmation state in History and Meta

Manual keyboard-first acceptance pass:

- select saved run
- inspect detail
- `Load`
- `Apply`
- `Rerun`
- open `More actions`
- cancel delete
- confirm delete
- verify focus return at each overlay close

### Success criteria

- Saved Runs feels like the primary artifact surface.
- Session History is still available but not visually dominant.
- Meta no longer feels like three repeated artifact forms.
- Delete feels explicit and trustworthy everywhere.
- No artifact capability loses a clear frontend path.

## Explicit defers and rejects

### Defer

- a universal top-level Artifact Center
- richer stale-context or provenance diffing beyond compact artifact trust/status chips
- advanced filter grammar beyond current local search + tags
- broad generic artifact-store refactors before the Meta Saved simplification proves the overlap

### Reject

- merging Meta fully into History
- hiding `Load`, `Apply`, and `Rerun` behind overflow menus
- flattening run-artifact actions into one generic “Open”
- direct delete from dropdown menus without confirmation
- replacing local library search with another pseudo-global search surface
- overloading artifact rows with large inline button clusters
- turning the detail card into a raw JSON/debug dump

## Bottom line

The repo should **keep History as the canonical Run Library, keep Meta as the contextual aggregate-artifact surface, and standardize both around one shared artifact-browser pattern with explicit delete confirmations and better action weighting**. That is the narrowest change set that improves manageability and trust without sacrificing capability reachability or reopening the app’s shell IA.
