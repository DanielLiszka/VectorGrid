# Results Triage And Analysis Direction — Final

## 1. Executive summary

VectorGrid should keep its analysis workflow as a **Top-centered workstation review loop**, not a chart-led explorer and not a multi-surface tie between table and charts. Prompt 3 remains the upstream boundary: validation setup, modules/automation policy, and validation export/report ownership stay canonical in `ValidationWorkspacePanel.vue`; the analysis surfaces should sharpen review and handoff, not absorb setup ownership. The repo’s target composition for results review is already explicit: overview summary, Top shortlist table, and one chart-heavy drilldown surface with a keyboard-safe path between shortlist and charts. fileciteturn16file12 fileciteturn16file2 fileciteturn16file3 fileciteturn16file7

The adopted direction is:

- **Overview** becomes the decision header and baseline-review plane.
- **Top** becomes the undisputed primary triage workbench.
- **Drill** remains the deep inspection surface.
- **Pareto** and **Heatmap** remain contextual lenses that sharpen judgment and click through into Drill.
- **Compare, provenance, and stale cues** stay richest in Top, summarized in Overview, lightly echoed elsewhere.

This direction fits the repo’s current workstation behavior, preserves dense information access, keeps one obvious keyboard-safe path through review, and builds on the store/service architecture that is already implemented instead of reopening analysis-shell architecture. fileciteturn16file5 fileciteturn16file8 fileciteturn16file0 fileciteturn16file1

## 2. Final triage/workflow direction

### Adopted direction

Use **Top as the command center for candidate choice**.

The repo already has the right primitives for this: selected-candidate title/summary/decision note/provenance chips, pinned compare candidates, stale cues, baseline comparison, Top CSV/JSON export, drill orchestration, and Heatmap PNG export all live in `analysisStore.ts` and `analysisService.ts`. `selectTopRow()` updates row selection and immediately runs the drill pipeline; pin/replace/remove compare state is already implemented; baseline comparison is already computed from the active run plus a selected saved baseline. fileciteturn16file1 fileciteturn16file0

That means the next UX step is coordination, not expansion:

- make **Top** the main review plane
- make **Overview** the concise decision framing surface
- keep **Drill** subordinate to Top-owned candidate selection
- keep **Pareto** and **Heatmap** useful, but explicitly secondary to shortlist review

This also matches the repo’s shared dense-surface rule: one focus plane per dense screen, with charts supported by text summaries and keyboard-safe alternate paths instead of becoming the only route through the workflow. fileciteturn16file2 fileciteturn16file8

### Deferred now

Defer heavier analysis-shell expansion:

- no always-on split-bench analysis shell
- no second permanent compare pane outside Top
- no chart-owned candidate-navigation model
- no richer chart-control systems beyond selected/pinned contextual emphasis
- no new panel-local visual grammar for analysis

Those ideas are plausible later, but they are lower ROI than clarifying control ownership and would risk cognitive sprawl. The current repo direction favors incremental workstation behavior improvements built on the existing shell/store model. fileciteturn16file5 fileciteturn16file4

## 3. Surface-by-surface guidance

### Overview

Overview should stay narrow and decisive.

Its job is to answer:

- what candidate is currently leading
- what baseline comparison says about that candidate
- what execution-risk or review caution should frame the decision

The existing Overview already has the right building blocks: best-candidate title/meta/stats, copy action, baseline selection, baseline comparison details, and Top CSV/JSON export access. Keep those, but do not turn Overview into a second shortlist or a richer compare workspace than Top. It should remain the **decision header** for the current run. fileciteturn16file1 fileciteturn16file0

Design implication: Overview may keep one details block for best-summary text and one comparison-details disclosure, but it should not gain pinned-candidate trays, richer candidate switching, or chart-heavy review responsibilities. That would violate the repo’s one-focus-plane rule and blur the setup/review boundary Prompt 3 already stabilized. fileciteturn16file2 fileciteturn16file8

### Top

Top should become the clearest and richest review surface in the analysis workflow.

The repo already supports the key behaviors here: shortlist filtering, sorting, row selection, selected-candidate summary/provenance, pinned compare candidates, stale cues, and compare-slot mutation. Top is also already part of the shared dense-table infrastructure that has persisted column visibility in the shipped stack plan. fileciteturn16file1 fileciteturn16file0 fileciteturn16file4

The next-step direction is:

- keep shortlist filters immediately visible
- keep the selected-candidate rail prominent
- keep compare compact and subordinate to shortlist scanning
- keep stale cues inline and local to the current candidate / compare state
- keep column posture controls available, but secondary

Top should remain the fastest place to answer “which candidate deserves deeper trust work next?” It should also remain the richest home for provenance, compare, and shortlist-specific guardrails, because those cues are most valuable at the point of candidate choice. The rest of analysis should inherit that choice rather than compete with it. fileciteturn16file5 fileciteturn16file4

### Drill

Drill should remain the deep inspection surface and not become a second candidate browser.

The current analysis service already treats Drill as a follow-on surface driven from Top selection, with rerender support, cached drill/stability state, trade navigation, and chart render recovery. Keep that model. The right next refinement is stronger current-candidate context at the top of Drill and clearer continuity back to Top, not a second selection system inside charts. fileciteturn16file0

Drill’s practical role is:

- inspect price/trade context
- inspect equity and drawdown behavior
- inspect local parameter stability
- inspect trade list details

It should continue to say, both visually and behaviorally, that candidate selection belongs to Top. That separation is a strength, not a deficiency. It keeps Drill focused on proof and disproof rather than shortlist management. fileciteturn16file7 fileciteturn16file8

### Pareto

Pareto should remain the frontier lens.

The repo already treats Pareto as a chart with axis selection, Top-oriented keyboard guidance, and point click-through into drilldown. Keep that role. The near-term improvement should be contextual emphasis, not more controls: highlight the current Top selection and pinned compare candidates on the frontier while leaving the rest of the field visible. fileciteturn16file0 fileciteturn16file1

Pareto should therefore:

- keep axis selection
- keep click-to-drill
- keep explicit Top handoff
- avoid compare trays, secondary candidate summaries, or chart-owned navigation state

This preserves Pareto as a useful judgment lens without letting it compete with Top for control ownership. fileciteturn16file7 fileciteturn16file8

### Heatmap

Heatmap should remain the parameter-landscape lens.

The current analysis service already constrains it to a zero-centered diverging scale, marker support, crosshair support, click-to-drill behavior, and PNG export. That is already the right posture. The next step is the same as Pareto: emphasize the current selection and pinned candidates in context, then keep click-through immediate. fileciteturn16file0

Heatmap should therefore:

- keep objective-focused landscape reading
- keep hover and click-to-drill behavior
- keep PNG export
- keep Top as the keyboard-safe candidate-navigation path
- avoid accumulating more chart furniture or independent compare state

This keeps Heatmap scanable, useful, and subordinate to the shortlist workflow instead of trying to become a second workbench. fileciteturn16file6 fileciteturn16file8

## 4. Compare / provenance / stale-state guidance

### Compare

Compare should be weighted by surface, not repeated equally everywhere.

- **Top** gets the richest compare model: pin, clear, replace, remove, and compact compare cards.
- **Overview** keeps baseline comparison.
- **Drill** gets only a compact echo of the current candidate and, later, optional light compare context.
- **Pareto** and **Heatmap** get contextual highlighting only.

This matches the current store/service split: pinned compare candidates are Top-oriented, while baseline comparison is Overview-oriented through `compareSummary`, `setBaseline()`, and `refreshCompareSummary()`. fileciteturn16file1 fileciteturn16file0

### Provenance

Provenance should also be strongest at the point of candidate choice.

Use the current selected-candidate chips and candidate badges most richly in Top. In Overview, reduce provenance to the minimum decision-framing cues that help interpret the winning candidate. In Drill, Pareto, and Heatmap, provenance should become a short reminder or marker-level context, not a repeated chip wall. The workstation plan already made provenance a core workstation value, but it also framed the goal as better explanation and trust, not more chrome. fileciteturn16file5 fileciteturn16file4

### Stale-state cues

Stale cues should remain advisory and inline.

The current repo direction is already right: stale/dirty cues live as compact advisory chips or banners and distinguish informational drift from blocking conditions. For analysis, that means shortlist selection drift, pinned candidate filter mismatch, unavailable compare candidates, and validation-age cues should stay inline near the affected review context rather than escalate into modals. fileciteturn16file5 fileciteturn16file0

Practically:

- keep shortlist/filter drift in Top
- keep pinned-candidate stale reasons in compare cards
- keep validation-age or context-drift reminders lightweight in Overview / Drill
- do not let stale-state messaging dominate empty states or chart panes

## 5. Keyboard-safe path definition

The canonical keyboard-safe review path is:

1. **Overview** for winner framing and baseline context
2. **Top** for filtering, sorting, row movement, selection, pinning, and compare decisions
3. **Drill** for deeper inspection of the currently selected candidate
4. **Pareto** and **Heatmap** only as optional contextual lenses, with clear return-to-Top guidance

This aligns with the repo’s target composition and design-system rules: selected candidate state must stay visible, chart-only actions cannot be the only path forward, and chart-heavy pages need text summaries plus a keyboard-safe alternate route. The earlier frontend audit also explicitly required keyboard-accessible alternate action paths for chart-heavy pages and stronger textual fallbacks. fileciteturn16file7 fileciteturn16file8

Implications:

- candidate navigation remains table-first
- chart click-through remains additive, not required
- any future chart enhancement must preserve Top as the non-pointer navigation spine
- Drill, Pareto, and Heatmap should continue to expose explicit “Review Top results” handoffs

## 6. Export and capability-preservation note

This pass is coordination work, not capability reduction.

The functional-coverage matrix is explicit that analysis must continue to expose overview review, Top shortlist filtering/sorting/selection, Top export, Pareto and Heatmap refresh/export, drilldown, local stability analysis, and workflow-run refresh into analysis views. Validation remains canonical for validation run/report/export behavior and should not be hollowed out by analysis cleanup. fileciteturn16file6 fileciteturn16file2

So the preservation rules are:

- keep **Top CSV / JSON export** reachable from analysis
- keep **Heatmap PNG export** reachable from analysis
- keep **drilldown and local stability** reachable through Top selection and chart click-through
- keep **baseline comparison** reachable from Overview
- do not move validation shortlist/report export ownership out of `ValidationWorkspacePanel.vue`
- treat docks, footer, and palette routes as shortcuts, not replacements, if analysis navigation is cleaned up later

The next-step design may quiet duplicate export affordances, but it cannot silently remove any backend-backed analysis path. fileciteturn16file6 fileciteturn16file0

## 7. Implementation order by file / store / service

### Phase 1 — strengthen hierarchy in `TopWorkspacePanel.vue` and `OverviewWorkspacePanel.vue`

Primary goal: make control ownership obvious.

- tighten the selected-candidate rail in `TopWorkspacePanel.vue`
- compress the compare tray so shortlist scanning remains dominant
- keep stale cues close to the selected/compare context
- simplify `OverviewWorkspacePanel.vue` around winner framing, baseline comparison, and execution-review caution

Supporting state already exists in:

- `web/src/stores/analysisStore.ts`
- `web/src/services/analysisService.ts`

This phase should mostly be template/hierarchy work plus narrow derived-state shaping, not architecture change. fileciteturn16file1 fileciteturn16file0

### Phase 2 — contextual chart emphasis in `ParetoWorkspacePanel.vue` and `HeatmapWorkspacePanel.vue`

Primary goal: make charts better lenses, not broader workbenches.

- add selected/pinned candidate emphasis in Pareto rendering
- add selected/pinned candidate emphasis in Heatmap marker logic
- preserve click-to-drill
- preserve explicit Top handoff copy

Primary logic touchpoint:

- `web/src/services/analysisService.ts`

Template touchpoints:

- `web/src/components/ParetoWorkspacePanel.vue`
- `web/src/components/HeatmapWorkspacePanel.vue`

Keep changes narrow: renderer behavior, marker emphasis, summary/meta wording. fileciteturn16file0

### Phase 3 — contextual polish in `DrillWorkspacePanel.vue`

Primary goal: strengthen candidate context without creating a second selection model.

- add a compact current-candidate context header sourced from existing analysis-store state
- keep rerender / trade navigation behavior intact
- improve return-to-Top clarity
- preserve the current chart + stability + trade structure

Primary files:

- `web/src/components/DrillWorkspacePanel.vue`
- `web/src/services/analysisService.ts`
- `web/src/stores/analysisStore.ts`

Reuse current cached drill/stability state rather than adding more drill-local ownership. fileciteturn16file0 fileciteturn16file1

### Phase 4 — follow-through on shared dense-table / icon / summary polish

Only after Phases 1–3 settle:

- tune Top column posture only if it materially improves triage speed
- keep iconography restrained and consistent with the workstation plan
- add only narrow tests where interaction behavior actually changed

Relevant shared infrastructure:

- `web/src/composables/useHeadlessMatrixTable.ts`
- `web/src/composables/usePersistedMatrixColumnState.ts`
- `web/src/utils/iconography.ts`

These should remain support layers, not the center of the redesign. fileciteturn16file4 fileciteturn16file5

## 8. Verification plan

### Required repo checks

For any non-trivial phase in this prompt, run the repo’s real frontend gates:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run a11y:smoke`
- `npm --prefix web run baseline:ui`

If a phase changes shared dense-surface behavior or adds weight to analysis surfaces, also run `npm run check:bundle-budget`. These are already the repo’s normal frontend quality and bundle-discipline gates. fileciteturn16file9 fileciteturn16file10 fileciteturn16file2 fileciteturn16file4 fileciteturn16file5

### Manual review checklist

Manual/browser review for Prompt 4 should cover:

- **Top**: filter edits, sort toggles, row selection, Arrow/Home/End navigation, pin, replace, remove, clear compare
- **Overview**: best-candidate framing, baseline selection, comparison details, CSV/JSON export access
- **Drill**: rerender after tab reveal, Top handoff, trade navigation, stability visibility
- **Pareto**: axis changes, visible frontier rendering, click-through to Drill, Top handoff clarity
- **Heatmap**: ready/empty states, click-through to Drill, marker/crosshair behavior, PNG export
- theme and posture checks: light, dark, Standard, Dense, reduced motion
- guardrail check: validation still owns validation run/report/export responsibilities

This follows the same evidence posture already used in the repo’s recent UI/UX implementation tracking. fileciteturn16file3 fileciteturn16file2

### Targeted test additions

Only add focused tests where behavior changes, for example:

- Top compare tray remains compact and selection-driven after hierarchy changes
- selected/pinned contextual emphasis is reflected correctly in Pareto / Heatmap render logic
- Drill context header reflects current selection without breaking rerender/cache restore
- chart panes continue to expose explicit Top handoffs and accessible status text

Avoid broad new harness work for this prompt; the right move is narrow regression coverage around changed review behavior. That stays consistent with the repo’s current UI/UX validation discipline. fileciteturn16file2 fileciteturn16file10

## 9. Explicit rejects

Reject these directions for the near-term Prompt 4 pass:

- turning analysis into a **chart-only exploration workflow**
- making **Overview and Top** feel equally primary
- adding a **persistent second compare workbench** outside Top
- moving validation setup, modules/automation policy, or validation export/report ownership into analysis
- repeating compare/provenance/stale-state UI with equal weight on every analysis tab
- adding chart-local control systems rich enough to replace Top as the primary navigation plane
- introducing panel-local analysis styling systems that bypass the shared Prompt 2 language
- pursuing spreadsheet-grade table complexity in Top without clear review-speed benefit

Those paths would either blur ownership, add noise, or reduce the clarity of the repo’s current workstation model. The right near-term result is simpler and stronger: **Top chooses, Overview frames, Drill proves, Pareto and Heatmap contextualize**. fileciteturn16file12 fileciteturn16file2 fileciteturn16file5
