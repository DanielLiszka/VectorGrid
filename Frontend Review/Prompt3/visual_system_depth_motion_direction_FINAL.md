# Visual System, Depth, and Motion Direction — Final

## 1. Executive summary

VectorGrid should **keep Signal Foundry** and treat the next pass as a **tightening pass, not a redesign**. The repo already decided that the product is a workstation-first desktop tool, that hierarchy should come from semantic surfaces and state clarity before ornamental elevation, and that the Prompt 1 shell hierarchy is the implemented baseline. The remaining problem is not lack of direction. It is that the current implementation still looks slightly more like a polished admin shell than a distinct research workstation because routine chrome is too pill-shaped, light mode is cooler than intended, blur/elevation are still too visible in routine surfaces, and the strongest selection/focus grammar has not yet been propagated consistently across the app. fileciteturn8file0 fileciteturn8file1 fileciteturn8file2 fileciteturn8file3 fileciteturn8file4

The adopted direction is therefore: **de-gloss and sharpen hierarchy**. Make the UI flatter at rest, clearer when active, warmer in light mode, stricter about semantic color, and more disciplined about where elevation and blur are allowed. Do not broaden the token system much, do not relitigate shell information architecture, and do not chase a more decorative “premium” style. The visible step up should come from stronger active-state grammar, better surface separation, calmer chrome, and more consistent depth behavior across shell, forms, tables, overlays, charts, and operator surfaces. fileciteturn8file0 fileciteturn8file1 fileciteturn8file9

## 2. Final visual thesis

The final thesis for this pass is:

**Signal Foundry, tightened into a layered instrument language.**

That means:

- the shell reads like a fixed bench, not a floating app frame
- default panes read as sectional working surfaces, not stacked cards
- the one current decision area on a dense screen reads unmistakably forward
- selection, comparison, and validation context do more of the visual work than decorative elevation
- overlays feel like raised sheets and assistant trays, not glass widgets
- motion is short, productive, and mostly noticed only when it goes missing. fileciteturn8file0 fileciteturn8file1 fileciteturn8file8 fileciteturn8file9

This pass should be **bolder** than the current implementation in four places: active tabs, selected rows, focus-surface promotion, and light-theme tone. It should be **quieter** in four places: pill chrome, glow, routine blur, and medium-strength card elevation used as a default. That is the smallest change set that makes the app feel more intentional without softening its workstation posture or reducing dense readability. fileciteturn8file0 fileciteturn8file1 fileciteturn8file4

## 3. Required token updates

### 3.1 Palette retune, not palette expansion

Keep the existing semantic token model and retune it.

Required changes:

- warm the **light theme** toward the paper/mineral direction already proposed
- keep the **dark theme** in graphite/slate territory, but reduce the need for accent glow to create hierarchy
- shift **warning** from orange-brown toward brass/amber
- keep **blue** as the only true action/focus/selection accent
- keep **green** success-only rather than letting it drift into a second brand color. fileciteturn8file0 fileciteturn8file4 fileciteturn8file9

This is a retune of existing tokens, not a license to add many more. The shared CSS already contains the right semantic families (`--surface-*`, `--selection-*`, semantic state colors, depth shadows, compact type/spacing scales). The next step is to make those values feel more like Signal Foundry and less like default modern SaaS blue. fileciteturn8file4 fileciteturn8file9

### 3.2 Complete the depth token set

The depth brief already established a four-level model and the shared CSS already implements most of it through `surfaceSunken`, `surfaceWorking`, `surfaceFocus`, `surfaceSticky`, `selectionRail`, `stickySelectionSummary`, and overflow-shadow utilities. This pass should finish that normalization rather than invent a second hierarchy system. fileciteturn8file1 fileciteturn8file4

Required changes:

- add a missing `--surface-overlay` token for shared overlay-adjacent use
- optionally add `--surface-shell` if the shell/titlebar bands need their own normalized resting tone
- slightly increase separation between `--surface-sunken`, `--surface-working`, and `--surface-focus` in both themes
- reduce reliance on blur as a substitute for surface separation. fileciteturn8file1 fileciteturn8file4

### 3.3 Keep the type scale, add font tokens

The repo’s compact operational typography scale is already correct. Preserve the current size and line-height model. Add explicit `--font-sans` and `--font-mono` tokens so the system stops depending on implicit font choices. For this pass, keep the default sans stack low-risk; do **not** make font packaging a blocker. `IBM Plex Sans` and `IBM Plex Mono` remain the best later refinement if a dedicated type pass is approved, but the visual system can improve materially before that change. fileciteturn8file0 fileciteturn8file4 fileciteturn8file9

### 3.4 Normalize state and transition usage

Keep the existing motion band close to where it is now and normalize usage first. The shared CSS already has short motion tokens and reduced-motion handling. The real problem is inconsistent application, not a broken timing scale. For this pass, standardize component transitions around the shared motion tokens, remove stray raw timings where practical, and reserve stronger effects for overlays and explicit state changes. fileciteturn8file1 fileciteturn8file4

## 4. Required shared component/state patterns

### 4.1 One tab grammar everywhere

This is the most important pattern change.

Tabs should no longer read like rounded pill controls. The theme proposal and depth brief already called for an **underline plus soft field** active state. The current CSS still uses pill radii and pill grouping in global tabs, and the shell layer still visually leans that way. Replace that with one shared tab grammar across workspace tabs, workflow steps, controls tabs, results tabs, and panel-level tabs. fileciteturn8file0 fileciteturn8file1 fileciteturn8file4

Required tab behavior:

- flatter base track
- quieter inactive state
- active state led by underline, text emphasis, and restrained field tint
- no inactive shadowing
- no pill-button silhouette as the primary cue. fileciteturn8file0 fileciteturn8file4

### 4.2 One focus-plane rule per dense screen

The depth model only works if the product is strict about where emphasis lives. Use exactly one dominant focus surface per dense screen unless the workflow truly splits attention. The current utilities already support this. The pass should apply them more consistently instead of building one-off prominent cards. fileciteturn8file1 fileciteturn8file4

### 4.3 One selection grammar across review surfaces

The Top shortlist already points toward the right selection language: stronger selected fill, left-edge rail, and sticky local summary. Validation and any other dense review surface should converge on that same grammar rather than inventing local variants. This is the clearest way to create depth without adding more chrome. fileciteturn8file1 fileciteturn8file4 fileciteturn8file6

Required selection behavior:

- selected row or item = stronger fill than hover/zebra
- selected context = sticky local summary where the downstream review pane depends on it
- comparison and pinning surfaces reuse the same selection family, not a second accent language. fileciteturn8file1 fileciteturn8file5 fileciteturn8file6

### 4.4 One disclosure/seam language

Column-visibility controls, long explanations, diagnostics details, and other “more information in the same plane” interactions should use one disclosure style. The existing details/disclosure utilities in shared CSS are already close; this pass should standardize chevron behavior, open-state borders, padding, and body separation rather than letting each panel improvise. fileciteturn8file1 fileciteturn8file4 fileciteturn8file9

### 4.5 Overlay-only stronger elevation

Overlays and dialogs should own the strongest separation. Routine shell chrome and routine panel chrome should not. That means command palette, confirmation dialogs, and the task center may keep the clearest raised-sheet treatment, while the shell and normal working panes become flatter and more structural. fileciteturn8file0 fileciteturn8file1 fileciteturn8file9

## 5. Motion rules

### Adopt

- keep motion short and productive
- keep tab, disclosure, row-state, and overlay transitions on the existing shared timing band unless a specific interaction proves too slow or too abrupt
- use fade + slight lift for overlays/dialogs rather than scale-heavy entrances
- use subtle active-state motion to make shell and selection changes feel coherent, not animated for their own sake
- preserve reduced-motion fallbacks already present in shared CSS and component surfaces. fileciteturn8file0 fileciteturn8file1 fileciteturn8file4

### Defer

- a dedicated typography-motion polish pass
- any row-emphasis pulse beyond the most restrained keyboard-selection cue
- any richer staged chart-panel choreography
- view-transition experimentation outside narrow isolated surfaces. fileciteturn8file4

### Reject

- spring or bounce motion
- decorative parallax
- continuous glow or shimmer used as “premium” chrome
- large-scale zoom transitions for overlays
- turning charts into animated hero objects
- motion that becomes noticeable during repetitive operator work. fileciteturn8file0 fileciteturn8file1 fileciteturn8file4

## 6. Surface translation guidance

### 6.1 Shell

The shell should read as machined structure over a fixed bench.

Required translation:

- quieter titlebar and shell bands
- reduced blur/glow in routine shell chrome
- flatter tab bases and stronger active underlines
- recessed context strip rather than card-like telemetry shelf
- task-center launcher reading as a tray entry, not a floating gadget. fileciteturn8file0 fileciteturn8file1 fileciteturn8file2 fileciteturn8file3

Do not use shell chrome to create most of the app’s depth. Use it to create structure and mode clarity. Prompt 1 already solved the shell hierarchy; this pass should visually support that hierarchy by making the shell calmer at rest and clearer only where the user is switching mode or reading immediate context. fileciteturn8file2 fileciteturn8file3

### 6.2 Forms

Forms should remain on the working plane, with only one promoted focus pane for the current decision hotspot.

Required translation:

- use sectional hierarchy and summary chips before extra boxes
- keep shared risk/cost/exit or equivalent current-decision controls on the focus surface
- move long explanations behind standardized details/disclosure seams
- use sunken support regions for secondary diagnostics and informational guidance. fileciteturn8file0 fileciteturn8file1 fileciteturn8file8 fileciteturn8file9

Avoid turning every section into a bordered mini-card. The visual step up should come from stronger hierarchy and calmer support surfaces, not from more containers. fileciteturn8file0 fileciteturn8file9

### 6.3 Tables and lists

Tables and lists should read like research sheets.

Required translation:

- quieter frame and headers
- subtle zebra only
- selected row stronger than hover by a clear margin
- left-edge selection rail as the main selection landmark
- sticky local summary when selection drives downstream review
- overflow shadows retained on dense scrollers
- column-control disclosures visually subordinate to the data itself. fileciteturn8file0 fileciteturn8file1 fileciteturn8file4

The strongest pilot is still Top Results, and Validation should move closer to that model rather than growing a separate review language. History and artifact surfaces should later inherit the same principles when Prompt 5 work lands. fileciteturn8file1 fileciteturn8file6

### 6.4 Dialogs and overlays

Dialogs and overlays should feel like raised sheets above the bench.

Required translation:

- strongest border and edge separation live here
- blur is allowed here more than in routine shell chrome
- command palette should stay visually distinct and crisp
- confirmation dialogs should prioritize action hierarchy and negative space
- task center should feel like an assistant tray, not a floating novelty surface. fileciteturn8file0 fileciteturn8file1 fileciteturn8file9

### 6.5 Chart-adjacent panels

Charts should remain quiet working surfaces.

Required translation:

- low-noise frames
- legends and overlays visually lighter than the selected-candidate summary or current analytical note
- selected candidate / provenance / stale cues live outside the chart canvas where possible
- annotations should read like analyst marks, not presentation callouts
- comparison colors stay semantically disciplined: focus blue, secondary slate, caution amber, failure red. fileciteturn8file0 fileciteturn8file4

Do not try to make charts look more “modern” through extra gradients, glow, or motion. The chart’s job is analytical clarity. The surrounding summary and selection grammar should carry the sense of intention. fileciteturn8file0 fileciteturn8file1

### 6.6 Settings and execution surfaces

Settings and execution surfaces should move toward an operator-console evidence-sheet language.

Required translation:

- diagnostics cards and runtime summaries should read as evidence blocks, not dashboard tiles
- semantic state chips and badges should stay strict and compact
- execution should reserve the strongest visual distinction for live/destructive subsections, not tint the entire workspace
- raw output and deeper debugging stay behind disclosure or subordinate panes
- the task center, settings diagnostics, and future execution safety surfaces should feel visually related, not like separate mini-products. fileciteturn8file0 fileciteturn8file1 fileciteturn8file5 fileciteturn8file6 fileciteturn8file9

This pass should improve the visual cohesion between Settings diagnostics and Task Center now, while leaving broader execution/operator-safety IA work to Prompt 6. fileciteturn8file3 fileciteturn8file6

## 7. File-level implementation order

### Phase 1 — `web/src/styles/app.css`

This is the highest-value file for the pass. Make it responsible for:

- light/dark palette retune
- `--surface-overlay` addition and any minimal shell-surface normalization
- explicit font tokens without forcing font packaging now
- global tab grammar rewrite away from pill chrome
- blur/elevation discipline for sticky and overlay surfaces
- disclosure-state normalization
- removal of stray raw transition usage where practical. fileciteturn8file1 fileciteturn8file4 fileciteturn8file9

### Phase 2 — `web/src/components/VueWorkspaceShell.vue` and `web/src/components/DesktopTitlebar.vue`

Apply the shared token/pattern changes to the implemented Prompt 1 shell baseline:

- quiet routine shell chrome
- align workspace/workflow/controls/results tabs to the new grammar
- reduce glow/raised feeling in resting shell surfaces
- keep the task-center launcher clear, but calmer at idle
- keep the titlebar as chrome plus compact operational affordances rather than a visually active dashboard band. fileciteturn8file2 fileciteturn8file3

### Phase 3 — `web/src/components/TopWorkspacePanel.vue` and `web/src/components/ValidationWorkspacePanel.vue`

Unify dense review surfaces around one selection/focus model:

- keep Top as the pilot for sticky summary + row rail + focused drill context
- finish Validation’s tiering so summary and table read as one analytical unit
- ensure both surfaces reuse the same disclosure and overflow-shadow language
- remove any remaining visual drift caused by local card treatments. fileciteturn8file1 fileciteturn8file4 fileciteturn8file6

### Phase 4 — `web/src/components/TaskCenterOverlay.vue` and `web/src/components/SettingsWorkspacePanel.vue`

Align operator/support surfaces to the same visual language:

- reduce floating-gadget feel in the task center
- keep strongest elevation reserved for the overlay itself, not its toggle
- make diagnostics read like evidence sheets with shared semantic badges and calmer card chrome
- align task, runtime, and diagnostics surfaces with the stricter color and depth discipline. fileciteturn8file0 fileciteturn8file3 fileciteturn8file5 fileciteturn8file6

### Phase 5 — deferred follow-on alignment

Only after the phases above are stable:

- consider a dedicated typography asset pass
- align Execution-specific surfaces in the Prompt 6 operator-safety slice
- carry the same visual grammar into Prompt 3, 4, and 5 surfaces as those passes land. fileciteturn8file13

## 8. Validation plan

### Required repo checks

For any non-trivial slice in this pass, run at minimum:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run a11y:smoke`
- `npm --prefix web run baseline:ui`  
  These are the repo’s real frontend verification surfaces and should remain the main quality bar for this visual pass. fileciteturn8file10 fileciteturn8file11

If the pass later adds font assets or broader shell visual weight, also re-run bundle discipline checks before treating the typography change as complete. Keep the implementation in the Vue/web surface unless scope is explicitly expanded. fileciteturn8file11 fileciteturn8file12

### Required manual review set

Collect manual visual evidence for:

- Workflow / Optimize / Top
- Workflow / Validation
- Settings
- Task Center
- light and dark themes
- Standard and Dense density modes
- reduced-motion behavior
- active-tab clarity, selected-row clarity, and overlay hierarchy. fileciteturn8file6 fileciteturn8file8

### Guardrail review

This is a visual/system pass, not a capability-removal pass. Re-check the functional coverage guardrail explicitly after any decluttering around disclosures, row actions, diagnostics, or shell/task surfaces. Every backend-backed capability must still have at least one clear frontend path. fileciteturn8file1 fileciteturn8file7

## 9. Explicit non-goals

This document does **not** recommend:

- a broader redesign or aesthetic reset
- a generic dashboard/sidebar pivot
- more glassmorphism
- heavier shadow-led pseudo-3D
- more accent colors for pseudo-branding
- decorative chart restyling
- motion-led “premiumization”
- a mandatory font-package swap in this pass
- a chart-library replacement
- execution/settings information-architecture rewrites that belong to later prompts
- hiding advanced or backend-backed capability for cleanliness. fileciteturn8file0 fileciteturn8file1 fileciteturn8file2 fileciteturn8file7

The repo does not need a new visual philosophy. It needs a stricter implementation of the one it already chose. The winning move now is a small, disciplined pass that makes the app look flatter at rest, sharper when active, and more like a coherent research workstation across shell, forms, review surfaces, overlays, and operator tooling. fileciteturn8file0 fileciteturn8file1 fileciteturn8file2 fileciteturn8file13
