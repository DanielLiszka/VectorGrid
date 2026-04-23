# Mode Separation vs Workflow Branch Direction — FINAL

## 1. Executive summary

VectorGrid should add exactly **one new shell-level concept now: `mode`**. The adopted near-term product model is:

- **Conventional** remains the default mode.
- **ML Signal** becomes a true top-level mode in the same application shell.
- **Options-style workflows** stay out of the top-level shell for now and should enter later as a **contextual downstream branch inside Conventional mode**, most likely from Validate, Analyze, or History detail.
- **Data** and **Run Library** remain shared foundations rather than being duplicated into mode-specific homes.
- The shell should **not** introduce a generic persisted `branch` primitive yet.

That direction matches the repo’s deferred `MlSignal` plan, fixes the current mismatch where `MlSignal` already behaves like a separate workflow but is still framed as a strategy variant, and avoids speculative shell proliferation for options or other future specialties. The current shell model is built around one workflow vocabulary and one persisted state model, while `MlSignal` is already special-cased in optimize dispatch and history/artifact handling. The cleanest fix is to make `mode` explicit and stop overloading `strategyKind` as a hidden shell selector. fileciteturn12file1 fileciteturn12file2 fileciteturn12file3 fileciteturn12file4 fileciteturn12file5 fileciteturn12file0

## 2. Decision framework for strategy vs branch vs mode separation

A capability should remain **a strategy or feature inside Conventional mode** when the user is still doing the same core job with the same setup language, the same main CTA family, the same results surfaces, and the same reopen target. In the current repo, that means it still fits the shell’s existing guided workflow and truthful subtitles: Data, Optimize, Validate, Analyze, Finalize. fileciteturn12file4 fileciteturn12file10

A capability should become **a workflow branch inside Conventional mode** when it is a specialized downstream path that starts from conventional context or artifacts, but does not need its own daily-use home, restore target, or global shell identity. Branches are entered contextually; they should not immediately add a new top-level shell noun. In this repo, that means the branch should reuse shared dataset loading, artifact plumbing, and command surfaces unless real usage proves those are misleading. fileciteturn12file8 fileciteturn12file12

A capability should become **a true top-level mode** when the current workflow labels become inaccurate, the primary object of work changes, saved artifacts reopen more naturally into that surface than into the conventional one, and the flow routinely bypasses conventional optimizer-native review surfaces. If the shell has to keep lying about what the user is doing, it is time for a mode. That is exactly the pressure documented for `MlSignal`. fileciteturn12file1 fileciteturn12file4

Operationally, use these gates:

- **Keep as strategy/feature** when existing workflow labels remain accurate.
- **Promote to branch** when it is a specialized follow-on path within the conventional research world.
- **Promote to mode** when shell vocabulary, restore behavior, and artifact semantics materially diverge.

## 3. Final product decision on `MlSignal`

`MlSignal` should become a **true shell mode** now. The repo’s own deferred plan already defines that as the target user model: Conventional remains the default mode, `MlSignal` becomes a separate mode in the same shell, shared foundations may remain shared, and the conventional post-meta ML-confidence finalizer stays in Conventional rather than moving into `MlSignal`. fileciteturn12file1

This is not just a naming preference. The current implementation already behaves as though `MlSignal` is separate:

- `optimizeStore.ts` includes `MlSignal` in the normal strategy list and special-cases `runOptimization()` and `runValidation()` so they dispatch to ML-specific services instead of conventional optimization/validation. fileciteturn12file2
- `historyService.ts` special-cases `MlSignal` when applying saved runs, loading saved artifacts, and rerunning them; it loads ML-specific paths and thresholds into the optimize store and bypasses optimizer analysis refresh when the artifact is ML-specific. fileciteturn12file0
- The frontend functional coverage matrix already treats `MlSignal` as its own branch with its own canonical panel, while treating the footer and history rerun as shortcuts rather than primary homes. fileciteturn12file8

So the shell is the lagging layer. The product and service behavior are already split; the shell vocabulary is what remains misleading. `MlSignal` should therefore stop being represented primarily as a strategy inside `OptimizeWorkspacePanel.vue` and should instead be entered through a visible mode choice. fileciteturn12file1 fileciteturn12file2

The second-pass refinement applies here: **do not duplicate Run Library as an ML-only workflow step in the first slice**. The deferred plan listed “History / artifact review” as part of the target `MlSignal` workflow, but the stronger repo direction is to keep artifact browsing shared and mode-aware instead of spawning a second history home. History already overlaps with Meta, and the repo’s broader frontend direction is toward shared artifact-browser abstraction, not more artifact-surface fragmentation. fileciteturn12file1 fileciteturn12file8

## 4. Final product decision on options-style workflows

A plausible options backtest optimization flow should stay **a downstream workflow branch inside Conventional mode**, not a top-level mode and not just another strategy selector value.

The prompt premise matters: options may consume a **validated conventional signal artifact** and shared Polygon/data context rather than replacing the conventional workflow outright. That makes options a downstream specialized path, not a new daily-use shell home. It should therefore begin as a contextual branch launched from Validate, Analyze, or History detail. fileciteturn12file14 fileciteturn12file4

That means, for now:

- **Do not** add an `options` mode.
- **Do not** add options to the standard strategy selector by default.
- **Do** let options emerge later as a contextual branch from conventional artifacts or current conventional review state.
- **Do not** add a generic persisted `branch` key until there is more than one real branch that justifies shell-level treatment.

Options should only be promoted to a true mode later if it gains its own primary start point, its own routine reopen target, its own artifact family, and vocabulary that makes the conventional shell labels inaccurate. The repo does not have that evidence yet. fileciteturn12file1 fileciteturn12file12

## 5. Recommended shell vocabulary

Adopt this user-facing shell vocabulary:

- **Mode**
  - `Conventional`
  - `ML Signal`
- **Workspace**
  - `Workflow`
  - `Run Library`
  - `Aggregate Analysis`
  - `Pro Tools`
  - `Settings`
- **Workflow steps inside Workflow workspace**
  - **Conventional:** `Data`, `Optimize`, `Validate`, `Analyze`, `Finalize`
  - **ML Signal:** `Data`, `Dataset Export`, `Score Prep`, `Validation`, `Policy Sweep`

Do **not** add a first-class `branch` shell noun yet. Use “branch” as product language for future contextual entry points inside a mode, not as a persisted shell axis. The current `workflowModel.ts` and `workflowStore.ts` are already carrying a dense vocabulary of workspace/workflow/step/control/result/aggregate state. Adding `mode` is justified; adding `branch` now would overfit speculative future flows. fileciteturn12file4 fileciteturn12file5

Implementation note: the existing internal `workflow` keys (`data`, `optimize`, `validate`, `analyze`, `finalize`, `history`, `settings`) can be retained temporarily for migration safety, but user-facing shell language should converge on **Mode + Workspace + Workflow Step**. Hidden routing values should not define the product model. fileciteturn12file3 fileciteturn12file4

## 6. Navigation, Data, and persistence implications

### Navigation

The shell should gain a **compact visible mode switch**, not a new full-width navigation row. The current shell already has workspace tabs, workflow-step tabs, a context strip, controls/results tab bars, task center access, and the footer. Adding another major tab row would increase shell noise. The right placement is a compact segmented control or chip-level switch in the shell header/context area, mirrored in `WorkflowQuickSwitch.vue` and the command palette. fileciteturn12file9 fileciteturn12file12

### Data

`DataWorkspacePanel.vue` should remain shared. The deferred `MlSignal` plan is explicit that separate data stores are not required in the first slice, and the current Data surface already owns shared dataset load, timeframe, and Polygon flows. What should change is the **mode-aware framing**: Conventional should continue to describe optimization setup, while ML Signal should describe dataset export / score-pipeline preparation without duplicating the whole data stack. fileciteturn12file1 fileciteturn12file8

### Persistence

`workflowBootstrap.ts` and `workflowStore.ts` need a real `mode` key. Today the bootstrap layer restores only `workspace`, `workflow`, `workflowStep`, `controlsTab`, `resultsTab`, `aggregateTab`, and split fraction, while `workflowStore.ts` remembers tab posture by workflow alone. That is safe only in a single-mode shell. Without a persisted `mode` and mode-scoped tab memory, Conventional and ML Signal state will bleed together. fileciteturn12file3 fileciteturn12file5

The mode migration rule should be:

- if restored persisted state explicitly carries `mode`, trust it;
- otherwise, infer `ml-signal` only when a restored artifact or legacy strategy selection makes that unambiguous;
- otherwise, fall back to `conventional`.

That keeps backward compatibility while avoiding accidental mode switches. fileciteturn12file1 fileciteturn12file3

## 7. Impact on strategy selection, quick switch, command palette, and history restoration

### Strategy selection

Once ML Signal mode exists, `MlSignal` should be removed from `STRATEGY_OPTIONS_DEFAULT` and from the conventional optimization selector. `OptimizeWorkspacePanel.vue` should stop treating `MlSignal` as an alternative strategy and return to being a conventional optimization surface. `MlSignalWorkspacePanel.vue` becomes the canonical home for ML-specific actions. fileciteturn12file2 fileciteturn12file8

### Quick switch

`WorkflowQuickSwitch.vue` should expose explicit **Switch to Conventional** and **Switch to ML Signal** actions. It may later expose contextual “Open Options Backtest” entry points, but those should remain branch entries, not global mode choices. The deferred plan already marks quick switch as a core touch point for the mode split. fileciteturn12file1

### Command palette

`CommandPaletteOverlay.vue` should mirror the same explicit mode actions. It should not become a second canonical home for ML workflow ownership; it should remain a fast route into the already-defined shell model. That preserves the repo’s broader capability-preservation rule: one canonical primary home per capability, with palette/footer/docks as shortcuts where appropriate. fileciteturn12file8 fileciteturn12file12

### History restoration

This is the most important behavioral change. The history layer already treats `MlSignal` differently: ML artifact load avoids optimizer-analysis refresh, apply-saved-run hydrates ML-specific fields into the optimize store, and rerun-saved-run dispatches into dataset export, score-cache generation, validation, or sweep depending on ML stage and validation payload shape. After the mode split, those same signals should become shell routing rules: **switch mode first, then hydrate or rerun**. Saved conventional artifacts should reopen into Conventional mode; saved ML artifacts should reopen into ML Signal mode. fileciteturn12file0 fileciteturn12file1

## 8. Implementation order by file/component/store

### Phase 1 — Add shell vocabulary

- **`web/src/utils/core/workflowModel.ts`**
  - add `ShellMode`
  - add mode-aware workflow-step maps and subtitles
  - add mode-aware allowed-tab/default-tab helpers
  - keep current workspace vocabulary stable
- **`web/src/stores/workflowStore.ts`**
  - add `mode`
  - add `selectMode()`
  - namespace remembered tabs by mode + workflow
  - derive subtitles from mode-aware state instead of workflow alone

This phase is the foundation and should land before panel remount work. fileciteturn12file4 fileciteturn12file5

### Phase 2 — Make restore and migration deterministic

- **`web/src/services/workflowBootstrap.ts`**
  - restore and validate `mode`
  - migrate legacy persisted state into the new mode model
  - preserve split fraction and existing tab-fallback behavior
- **`web/src/utils/core/storageKeys` / storage helpers**
  - add the new mode storage key

This is the risk-reduction phase and should precede any visible shell polish. fileciteturn12file3 fileciteturn12file1

### Phase 3 — Expose mode in the shell

- **`web/src/components/VueWorkspaceShell.vue`**
  - render the compact mode switch
  - render mode-specific workflow-step labels
  - keep workspace tabs stable
  - avoid adding another full nav row
- **`web/src/components/WorkflowQuickSwitch.vue`**
  - add explicit mode-switch actions
- **`web/src/components/CommandPaletteOverlay.vue`**
  - add explicit mode commands

This is the first user-visible product change. fileciteturn12file1 fileciteturn12file12

### Phase 4 — Move ML ownership out of the conventional strategy selector

- **`web/src/App.vue`**
  - mount ML mode from shell mode rather than from `strategyKind`
- **`web/src/components/OptimizeWorkspacePanel.vue`**
  - remove the embedded `MlSignalWorkspacePanel.vue` primary path
  - return this panel to conventional optimization ownership
- **`web/src/components/MlSignalWorkspacePanel.vue`**
  - keep as the canonical ML working panel
  - first slice may keep one panel while using mode/step state for navigation and labels rather than immediately decomposing into multiple panels
- **`web/src/stores/optimizeStore.ts`**
  - remove `MlSignal` from `STRATEGY_OPTIONS_DEFAULT`
  - remove ML special-casing from conventional run/validate actions once shell routing owns that decision

This phase is where the user-model drift actually ends. fileciteturn12file2 fileciteturn12file1

### Phase 5 — Make shared surfaces mode-aware

- **`web/src/components/DataWorkspacePanel.vue`**
  - keep shared functionality
  - adjust mode-aware subtitles, chips, and next-step framing
- **`web/src/components/RunFooterPanel.vue`**
  - make CTA labels and readiness copy mode-aware
  - keep footer actions as shortcuts, not canonical homes
- **`web/src/services/historyService.ts`**
  - switch shell mode before ML/conventional hydration and rerun
- **`web/src/stores/workflowArtifactsStore.ts`**
  - verify current artifact handoff remains sufficient for both modes

This phase ensures shared foundations stay truthful after the split. fileciteturn12file0 fileciteturn12file8

### Phase 6 — Options branch only after ML mode is stable

- add a contextual entry point from Validate, Analyze, or History detail
- keep options state local to its own panel/store at first
- do not add a generic persisted branch abstraction yet

This is intentionally downstream of the ML mode work. fileciteturn12file14

## 9. Validation plan

The mode split should be verified against the repo’s real frontend and desktop test surfaces, not just by manual inspection.

Run the standard frontend gates:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run a11y:smoke`
- `npm --prefix web run baseline:ui`

Also run desktop smoke because this change affects the actual Tauri shell, navigation, and restore behavior:

- `npm --prefix web run test:desktop:smoke`

Those are already the repo’s established frontend/runtime verification surfaces. fileciteturn12file11

Add focused test coverage for:

- **`workflowStore.ts`**
  - mode persistence
  - mode-scoped tab memory
  - fallback behavior for invalid mode/workflow combinations
- **`workflowBootstrap.ts`**
  - legacy migration
  - last-session restore into Conventional vs ML Signal
- **`historyService.ts`**
  - conventional artifact reopen routes to Conventional mode
  - ML artifact reopen routes to ML Signal mode
  - ML rerun preserves dataset/timeframe and stage-specific dispatch
- **`OptimizeWorkspacePanel.vue` / `MlSignalWorkspacePanel.vue` / `VueWorkspaceShell.vue`**
  - strategy selector cleanup
  - visible mode switch behavior
  - footer/quick-switch/palette parity

Capture baseline UI evidence for:

- Conventional workflow home
- ML Signal mode home
- shared Data in both modes
- Run Library reopening a conventional artifact
- Run Library reopening an ML artifact

That keeps the change aligned with the repo’s existing evidence discipline. fileciteturn12file1 fileciteturn12file11 fileciteturn12file12

## 10. Explicit defers, risks, and rejection conditions

### Defer

- generic persisted `branch` abstraction
- options as a top-level mode
- separate Tauri window for ML Signal
- separate shared-data stores for ML mode
- backend command renames or artifact-format changes tied only to shell cleanup
- forcing ML outputs into optimizer-native Top / Pareto / heatmap review by default

These are all explicitly outside the first implementation slice or unsupported by current repo evidence. fileciteturn12file1

### Main risks

- state bleed between Conventional and ML Signal when remembered tabs are not mode-scoped
- non-deterministic restore during migration from legacy persisted shell state
- history reopen choosing the right artifact but the wrong shell posture
- duplicated or conflicting labels across shell, quick switch, footer, and palette
- overbuilding shell taxonomy before options or other specialized flows have a real surface

These are the risks the deferred plan already identifies as the real implementation danger. fileciteturn12file1

### Rejection conditions

Reject or pause the mode split if:

- it requires another dense full-width navigation row;
- it duplicates Run Library into both a shared workspace and an ML-only workflow step in the first slice;
- last-session restore or artifact reopen stop being deterministic;
- Conventional mode still depends on `strategyKind === "MlSignal"` hacks after the split.

Reject or defer branch formalization for options if:

- the only way to make it coherent is to invent a shell-level `branch` state before a real options surface exists;
- it cannot be entered from a concrete conventional artifact or conventional review context;
- it quickly demands its own artifact family, start point, and reopen semantics, which would mean it is actually a mode candidate instead.

## Final decision

Adopt now:

- **true `mode` separation for `MlSignal`**
- **shared Data and Run Library foundations**
- **mode-aware history reopen and rerun routing**
- **removal of `MlSignal` from the conventional strategy selector once the mode is stable**

Defer:

- **options as a contextual conventional branch**
- **generic shell-level branch persistence**
- **broader backend or artifact refactors unrelated to shell truthfulness**

Reject for now:

- **keeping `MlSignal` permanently embedded as a strategy**
- **creating a top-level mode for every specialized flow**
- **promoting options to a top-level mode before it proves it needs one**
- **forking the product into separate shells or windows**
