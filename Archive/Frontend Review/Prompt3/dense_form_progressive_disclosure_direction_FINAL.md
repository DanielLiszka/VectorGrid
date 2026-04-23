# Dense Form Progressive Disclosure Direction — Final

## 1. Executive summary

VectorGrid should keep dense authoring and configuration surfaces as **single-page expert workstations**. The near-term move is not a schema/form-library rewrite, not a wizardized flow, and not a panel-by-panel visual redesign. The repo already decided that schema-driven migration is **later, not now**, with **Strategy** still the largest and riskiest form surface, **Validation** the next-heaviest, **Advanced** materially smaller, and **Data** the bounded pilot only if a later migration is ever reopened. The current frontend still relies on store-driven fields plus queued writes, so the right next step is structural: clearer operational spines, shallower disclosure, stronger non-default summaries, and better sequencing between setup and review. fileciteturn26file1 fileciteturn26file2 fileciteturn26file3 fileciteturn26file4

This pass must stay inside the already-implemented Prompt 2 baseline. Prompt 2 already locked in a shared visual language, a single focus plane on dense screens, one disclosure/seam style, shared density/type tokens, and a requirement that the form pass not broaden into a shell or visual-system relitigation. The winning move now is to reuse those shared primitives more consistently across Data, Strategy, Advanced, and Validation. fileciteturn26file5 fileciteturn26file6 fileciteturn26file0

## 2. Final recommendation for dense-form architecture

Adopt **single-page expert forms with selective, shallow disclosure**.

That means:

- keep each panel on one page rather than splitting work into steps or wizards
- organize each surface around one visible **operational spine**
- keep prerequisites, high-frequency controls, primary actions, and active non-default state visible
- move explanation, long-tail tuning, and low-frequency support controls behind the shared Prompt 2 disclosure seam
- auto-open disclosure only when a control is required, non-default, invalid, or already populated
- preserve the current store-owned + queued-input-write model rather than introducing a second form-state architecture during this pass

This direction matches the repo’s target comps and design-system rules: dense screens should expose summary chips, one prominent next-action pane, grouped working sections, and details/disclosure for longer guidance instead of raw paragraph walls or local card systems. fileciteturn26file10 fileciteturn26file11

## 3. Surface-by-surface guidance

### Data

**Data should stay the least clever and most visible of the four surfaces.** Keep source selection, dataset provenance, cached-dataset visibility, and timeframe immediately readable. The page should still read as: load data, inspect what is loaded, then choose the effective timeframe. That matches the repo’s canonical Data composition and preserves the bounded, operator-facing posture that made Data the only credible later pilot for any schema-owned migration. fileciteturn26file10 fileciteturn26file1

The cleanup here is emphasis, not restructuring. Keep CSV import, sample datasets, Polygon import/cache, dataset context, and timeframe on the page. Quiet down storage/cache explanation into a subordinate support treatment and keep deeper market-data-root management canonical in **Settings**, with Data retaining the current-folder view and direct open-folder utility. Do not hide schema/columns or cached-dataset selection just to make the page look cleaner; those are provenance, not clutter. fileciteturn26file9 fileciteturn26file10

### Strategy

**Strategy gets the largest structural change.** It is still the densest and riskiest authoring surface, and the repo already identified it as the biggest form-density problem. The target composition is also already clear: summary chips, one prominent pane for shared risk/cost/exit work, grouped strategy modules, and disclosure for long guidance. fileciteturn26file1 fileciteturn26file10 fileciteturn27file19

The near-term Strategy model should be:

- visible context at the top via summary chips and any current execution-risk notice
- one always-visible authoring spine for **execution cost model**, **risk/sizing essentials**, and **core exit behavior**
- the active strategy module as the only module that feels primary on the page
- one shallow **More constraints** disclosure for long-tail or low-frequency guardrails

The key correction is granularity. The repo should **not** hide all of “Constraints and exits.” Instead, keep the essentials visible: exit mode, stop-loss, take-profit, signal timing, and the smallest set of constraints used frequently enough to belong in the main scan path. Move long-tail utility/exposure/Omega/ulcer/downside caps and similar low-frequency gates into one shared disclosure that auto-opens whenever any of those settings is non-default. This follows the depth brief’s instruction to keep shared risk/cost/exit in the focus surface while pushing long route-policy explanation behind inline disclosure. fileciteturn27file18 fileciteturn26file5

### Advanced

**Advanced should be consolidation, not redesign.** The repo already classifies it as materially smaller than Strategy, and the depth brief already says its job is to stop diagnostics and guidance from flattening the main control surface. fileciteturn26file1 fileciteturn27file18

Keep three visible groups:

- **ranking setup**
- **output shaping**
- **sampler state**

Keep objective, mode, score mode when relevant, top K, heatmap bins, include-all, export path, and sampler-reset visibility immediately reachable. What should move down in emphasis is the repeated explanation and long exact-kernel nuance. Consolidate that support material into one standardized subordinate disclosure pattern, and only place lower-frequency shortlist diagnostics behind a shallow subordinate section that auto-opens when any of those values is non-default or incompatible-but-populated. Non-default advanced settings should also echo back as summary chips or a short support line so users do not need to reopen details to remember what is active. This preserves reachability while cutting scan noise. fileciteturn27file18 fileciteturn26file0

### Validation

**Validation gets the biggest sequencing fix.** The repo already concluded that Validation should gain clearer tiering but should **not** be forced into Top Results’ sticky-selection model. That stays fixed. fileciteturn26file6 fileciteturn26file5

The new Validation structure should be:

- always-visible **run essentials** before results exist
- one subordinate **modules and automation** setup section available before first run
- one primary **results review plane** after validation completes
- export actions split by timing: setup-time actions with setup, result-time exports with results

Keep candidate count, seed, core thresholds, expected-route summary, and **Run validation** visible before the first run. Move walk-forward / CSCV / SPA / Monte Carlo / re-opt / cross-dataset policy tuning into a collapsed subordinate section that is reachable **before** execution, not only after results appear. When any of those modules is enabled or non-default, surface that state in summary chips. Cross-dataset selection stays on the same page and should expand fully when enabled because it is real operational setup, not background help text. After results exist, the summary/table block becomes the primary review plane and result-specific exports should visually live with it rather than competing inside the first-run setup band. This preserves Validation as the canonical run/export/report surface while improving scanability. fileciteturn26file9 fileciteturn26file1 fileciteturn26file6

## 4. Always-visible versus progressive-disclosure rules

**Always visible** means:

- current context and prerequisites
- primary CTA for the current task
- the small set of controls most users touch on most runs
- any active non-default state that materially changes run behavior or outputs
- any unsupported-but-populated state that the user still needs to understand and clear

**Progressively disclosed** means:

- route-policy explanation
- formulas and deeper interpretation text
- low-frequency diagnostic tuning
- long-tail constraints and advanced guards
- raw previews, debug payloads, and support copy
- bulk sublists that matter only after a feature is enabled

Disclosure rules for this pass:

- use one shared disclosure seam style across panels
- default to closed when the content is optional and inactive
- auto-open when the content is required, non-default, invalid, or already populated
- do not go deeper than two disclosure levels total

This is consistent with the shared design-system rules that dense guidance belongs behind details/disclosure when it competes with working controls, and with Prompt 2’s rule that the app should use one disclosure/seam language rather than panel-local variants. fileciteturn26file11 fileciteturn26file5 fileciteturn26file0

## 5. Shared Prompt 2 visual-system reuse assumptions

These assumptions stay fixed for Prompt 3 work:

1. **One focus plane per dense screen.** Promote exactly one current decision surface; do not add multiple competing prominent panes on the same form. fileciteturn26file5
2. **One disclosure/seam language.** Reuse the shared details/disclosure treatment already established in `app.css` rather than inventing per-panel expanders or card grammars. fileciteturn26file0 fileciteturn26file5
3. **Default panes are working surfaces, not dashboard cards.** Panels should keep reading like page sections with summary chips, section leads, and compact support regions. fileciteturn26file11 fileciteturn26file5
4. **Prompt 2 visual decisions are not reopened here.** No new tab system, no shell chrome changes, no motion-library expansion, and no panel-local aesthetic experiments. fileciteturn26file6 fileciteturn26file1
5. **Validation is not Top Results.** Reuse tiering and dense-table treatment, but do not force sticky-selection UX where the workflow is not row-driven. fileciteturn26file6

## 6. Capability-preservation rules

This pass is structural, not subtractive. Every backend-backed capability must keep at least one clear frontend path. That means:

- **Data** keeps direct dataset loading, cached-dataset review, timeframe control, and open-folder utility in-context, while deeper market-data-root management remains canonical in **Settings**. fileciteturn26file9
- **Validation** remains the canonical surface for validation runs, shortlist/report export, reports-folder access, and cross-dataset selection; dock, footer, and palette entries stay shortcuts rather than replacements. fileciteturn26file9
- **Strategy** and **Advanced** may regroup controls, but cannot hide real backend-backed tuning behind unreachable secondary UI.
- Disclosure cannot be used to make incompatible-but-populated state disappear; users must be able to see what is set and why it no longer applies.
- Any removed duplicate entry point must still leave one obvious primary path and should be documented as intentional consolidation, not accidental loss. fileciteturn26file9 fileciteturn26file6

## 7. Interaction-speed guardrails for expert users

1. **No wizardization.** Data, Strategy, Advanced, and Validation remain single-page workstations.
2. **No extra click for common work.** Controls used on most runs stay visible.
3. **At most one disclosure click for secondary work.** Long-tail controls may be one level down, not buried in nested stacks.
4. **Auto-open active state.** If a hidden region contains non-default, invalid, or required state, it opens automatically.
5. **Keep the queued-write model.** Do not replace the current `useQueuedInputWriter` pattern with a new ownership model or blur-only commit behavior as part of this pass. fileciteturn26file3 fileciteturn26file1
6. **Respect density modes.** Comfortable, Standard, and Dense remain valid operating postures; new layouts cannot assume only the default density. fileciteturn26file8 fileciteturn26file0
7. **Keep keyboard reachability obvious.** Details/disclosure, quick actions, and regrouped controls must remain keyboard-safe and visually clear in focus states. fileciteturn26file11 fileciteturn26file6

## 8. Implementation order by file/component/composable

### Phase 1 — `web/src/styles/app.css`

Use `app.css` as the shared foundation for any new dense-form utilities. Add only small, reusable primitives that the panels can all share:

- subordinate support-region spacing and borders
- a compact non-default summary chip style if the existing chip treatment needs one clearer “active advanced state” variant
- minor disclosure-body spacing or summary-state refinements
- reduced-motion-safe transitions for disclosure open/close only if the current shared timing band is insufficient

Do not add a second token system or panel-specific card language. fileciteturn26file0 fileciteturn26file6

### Phase 2 — `web/src/components/AdvancedWorkspacePanel.vue`

Use Advanced as the first pilot for the pattern because it is smaller and already close to the desired structure. Land:

- clearer visible ranking/output/sampler grouping
- consolidated support disclosure for route/exact-kernel explanation
- summary chips or short state lines for active non-default advanced settings

### Phase 3 — `web/src/components/ValidationWorkspacePanel.vue`

Apply the largest sequencing fix next:

- separate run essentials from modules/automation
- make pre-run advanced modules reachable before first run
- keep the results summary/table as the dominant post-run plane
- relocate result-time export emphasis to the results region

This phase should explicitly preserve Prompt 2’s dense-table tiering work. fileciteturn26file6

### Phase 4 — `web/src/components/StrategyWorkspacePanel.vue`

Once the disclosure/support pattern is proven, apply it to Strategy:

- visible cost/risk/exit essentials
- one `More constraints` disclosure for long-tail gates
- cleaner shared-control spine before strategy-module controls
- inline advisories/details for long route-policy explanation

### Phase 5 — `web/src/components/DataWorkspacePanel.vue`

Finish with the bounded cleanup:

- preserve the visible load / inspect / timeframe flow
- quiet storage/cache explanation
- keep the Settings handoff explicit for directory management
- keep schema/provenance readable rather than treating them as secondary debris

### Narrow store/composable follow-through

If templates need better non-default summaries, add narrow derived helpers to the current panel stores rather than introducing new form ownership. `optimizeStore.ts`, `validationStore.ts`, and `useQueuedInputWriter.ts` remain the architectural baseline; they should only receive targeted support changes, not a rewrite. fileciteturn26file2 fileciteturn26file4 fileciteturn26file3

## 9. Regression and UX validation plan

### Required repo checks

For each non-trivial phase, run the repo’s existing frontend gates:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run a11y:smoke`
- `npm --prefix web run baseline:ui`

If a phase adds meaningful markup or support-state logic across several panels, also run `npm run check:bundle-budget` to keep the workstation behavior and lazy-surface budget intact. fileciteturn26file6 fileciteturn26file7 fileciteturn26file8

### Required manual review

Manual review for this prompt should cover:

- **Data**: CSV load, Polygon open/close, cached dataset selection, timeframe apply
- **Strategy**: one simple strategy and one large strategy family, plus non-default constraints
- **Advanced**: default route, exact route, and a case with active non-default shortlist diagnostics
- **Validation**: pre-run setup, enabled cross-dataset mode, post-run results table, result exports
- light and dark themes
- Standard and Dense density modes
- reduced-motion behavior
- keyboard traversal through visible and disclosed regions

This extends the same evidence posture already used in Prompt 2’s verification plan. fileciteturn26file6 fileciteturn26file8

### Guardrail review

After each phase, re-check the functional-coverage matrix to confirm that at least one clear frontend path remains for every relevant backend-backed capability, especially:

- market-data-root visibility/open-folder handoff in Data + Settings
- validation run/export/report actions
- cross-dataset selection
- any regrouped advanced or strategy controls that materially affect request construction

This pass is allowed to declutter and regroup. It is not allowed to silently reduce capability reachability. fileciteturn26file9 fileciteturn26file6

### Targeted test additions

Only add focused tests where behavior actually changes, for example:

- disclosure auto-opens when non-default Strategy constraints exist
- Validation advanced modules are reachable before first run
- active non-default Advanced settings surface correctly in summary chips or support lines
- keyboard focus remains deterministic when moving through newly disclosed regions

Avoid broad new harness work for this prompt; the value here is targeted regression coverage, not a second testing program. fileciteturn26file15

## 10. Explicit defers and rejects

### Defer

- `vee-validate` / `@vee-validate/zod` or any schema-owned form migration
- `DataWorkspacePanel.vue` as a later migration pilot only if a dedicated form-state phase is reopened
- new motion/disclosure helper libraries while the shared CSS path is still sufficient
- any broader help-system or overlay expansion that goes beyond the current inline disclosure / help-tip architecture
- any shell, execution/settings, or mode-separation work that belongs to later prompts rather than Prompt 3

These are valid later topics, but they are not the right vehicle for this pass. fileciteturn26file1 fileciteturn26file6

### Reject

- wizardizing Data, Strategy, Advanced, or Validation
- deep accordion stacks that force repeated reopening during expert workflows
- hiding primary run/export actions in menus
- forcing Validation into Top Results’ sticky-selection interaction model
- panel-local tab/disclosure/card systems that bypass Prompt 2’s shared language
- broad store-ownership or form-library rewrites disguised as “form cleanup”
- moving canonical market-data-root management out of Settings or moving canonical validation exports out of Validation
- disappearing unsupported-but-populated state instead of showing it with a clear explanation

The correct near-term form direction for this repo is **clearer structure, shallower disclosure, and stronger active-state summaries inside the architecture the repo already chose**. fileciteturn26file9 fileciteturn26file5 fileciteturn26file1
