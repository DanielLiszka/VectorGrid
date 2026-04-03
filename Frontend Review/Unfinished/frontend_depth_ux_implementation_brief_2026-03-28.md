# Frontend Depth / UX Implementation Brief

Date: 2026-03-28

Scope: Vue 3 frontend under `web/`

Status: implementation brief for the next UX pass

## Objective

Improve perceived depth, hierarchy, and information density without:

- forcing a stylized faux-3D theme
- increasing modal/overlay usage
- removing existing backend-backed user capability from the frontend surface
- replacing `lightweight-charts` for its current price/timeline role

This brief treats "more 3D" as a hierarchy problem, not a decoration problem.

## Decision

Do not push the product toward a visibly skeuomorphic or heavily elevated UI.

Instead:

- keep the shell visually restrained
- introduce a small depth model only where it improves comprehension
- use surface contrast, selected-state emphasis, sticky context, and progressive disclosure before introducing new chrome

In practice, the app should feel more layered and easier to scan, not more ornamental.

## Functional-Coverage Guardrail

Frontend UX cleanup may:

- regroup controls
- collapse redundant actions
- move secondary actions behind clearer disclosure or menus
- promote safer/default paths

Frontend UX cleanup may not:

- silently remove backend-backed functionality from user reach
- require console/manual work for functionality that was previously exposed in the UI
- hide high-stakes capability so deeply that it is no longer realistically discoverable

Rule:

- if multiple buttons/options currently trigger the same backend capability, consolidation is allowed
- if a backend-backed capability is distinct, at least one clear supported frontend path must remain unless the product owner explicitly approves removal

## Product-Level UX Thesis

The app is a workflow workstation:

- import data
- configure strategy
- optimize
- validate
- review results
- inspect execution posture

The UI should therefore emphasize:

- one current working surface at a time
- stable task context
- obvious selection state
- layered information release
- minimal pointer-only dependencies

## Depth Model

Use four depth levels only.

### 1. Base

Use for:

- shell background
- inactive page regions
- low-priority support surfaces

Behavior:

- visually recedes
- low contrast
- no heavy border or shadow emphasis

### 2. Working Surface

Use for:

- most primary panes
- tables
- standard form regions
- chart containers

Behavior:

- default reading plane
- strongest use of layout, spacing, and heading hierarchy
- not visibly "raised" by default

### 3. Focus Surface

Use for:

- the one pane the user is actively deciding in
- selected result summary
- current execution action region
- active data-source subsection

Behavior:

- slightly brighter or denser surface
- stronger top edge or accent rail
- subtle local shadow or inner highlight
- never more than one dominant focus surface per screen unless the workflow truly splits attention

### 4. Overlay

Use only for:

- command palette
- real dialogs
- tooltip/toggletip/help overlays

Behavior:

- explicit separation from the main workspace
- not a substitute for structure inside dense pages

## Token Additions For `app.css`

Add or refine a small set of semantic tokens instead of introducing component-local styling.

Implementation rule:

- shared shells, overlays, dialogs, list states, and table states should consume semantic tokens only
- avoid new raw hex or raw `rgba(...)` values in shared UI primitives
- existing hardcoded overlay colors are migration targets, not acceptable endpoints

### Surface / Depth Tokens

- `--surface-sunken`
- `--surface-working`
- `--surface-focus`
- `--surface-inline`
- `--surface-sticky`
- `--panel-focus-edge`

### Depth / Shadow Tokens

- `--shadow-focus`
- `--shadow-overflow-top`
- `--shadow-overflow-bottom`
- `--shadow-overflow-left`
- `--shadow-overflow-right`
- `--shadow-inline`

### Selection Tokens

- `--selection-rail`
- `--selection-fill`
- `--selection-fill-strong`
- `--selection-text`

### State Refinements

- `--warning-hover`
- `--danger-hover`
- `--ok-hover`

### Motion Tokens

- `--motion-fast`
- `--motion-ui`
- `--motion-slow`
- `--ease-standard`

Rules:

- use shadows sparingly; tonal depth should do most of the work
- focus surfaces should feel intentional, not floating
- selection rails should read more strongly than generic borders
- dark mode depth should rely primarily on surface separation and contrast, not stronger shadows
- overlay surfaces, selected rows, and status colors require explicit contrast validation in both light and dark themes

## Shared Utility Additions For `app.css`

Add utilities that make layered information cheap to apply.

### Recommended Utilities

- `.surfaceSunken`
- `.surfaceWorking`
- `.surfaceFocus`
- `.surfaceSticky`
- `.elevSunken`
- `.elevDefault`
- `.elevRaised`
- `.elevOverlay`
- `.selectionRail`
- `.overflowShadowX`
- `.overflowShadowY`
- `.overflowShadowTop`
- `.overflowShadowBottom`
- `.overflowShadowLeft`
- `.overflowShadowRight`
- `.inlineDisclosure`
- `.inlineDisclosureSummary`
- `.inlineDisclosureBody`
- `.contextStripSticky`
- `.stickySelectionSummary`
- `.focusShift`

### Utility Intent

- `surfaceSunken`: de-emphasize support regions without hiding them
- `surfaceFocus`: make the current decision area feel forward
- `overflowShadowX/Y`: imply hidden continuation in dense scrollers
- `elev*`: make the four-layer depth model first-class and measurable in component usage
- `stickySelectionSummary`: keep local context visible without opening another panel
- `inlineDisclosure*`: reveal depth inside the current pane rather than outside it

## Component-Level Changes

### `web/src/styles/app.css`

Primary job:

- define the depth model and make it reusable

Specific changes:

- add depth tokens listed above
- add first-class elevation utilities mapped to the four-layer model
- introduce restrained overflow shadows for scrollable wrappers
- strengthen selected-row and selected-chip visual language
- add sticky local-context utility styles
- add short transition tokens for underline, row-state, disclosure, and focus-surface emphasis
- migrate shared overlays and dialogs away from raw colors toward token-only styling

### `web/src/components/VueWorkspaceShell.vue`

Primary job:

- make the shell feel layered without becoming heavy

Specific changes:

- keep workspace tabs on the base plane
- let the active workspace and active workflow step use a stronger underline plus subtle focus field, not pill chrome
- convert the shell context strip into a slightly recessed support band
- ensure the controls/results tab row reads as a working-plane switch, not a second header
- make local tab rows sticky within scroll columns where long controls/results panes would otherwise lose context
- add subtle active-state transition so the shell feels physically coherent when context changes

Do not:

- add large shadows
- add glossy tab treatments
- turn the shell into a dashboard header

### `web/src/components/TopWorkspacePanel.vue`

Primary job:

- make the shortlist table and active candidate read as one analytical unit

Specific changes:

- give the selected row a left-edge rail plus stronger fill than generic selection tint
- add a sticky selected-candidate summary above the chart/drill area
- add overflow shadows to the table wrapper so lateral/vertical continuation is legible
- ensure the active drill surface becomes the focus plane when a candidate is selected
- keep non-selected charts and passive text on the working plane
- treat the shortlist table as a candidate pilot surface for a future `@tanstack/vue-table` migration if manual table logic continues to grow

### `web/src/components/StrategyWorkspacePanel.vue`

Primary job:

- reduce cognitive flatness on the densest authoring screen

Specific changes:

- elevate exactly one "current decision" pane at a time
- keep shared risk/cost/exit controls in the focus surface when relevant
- move long route-policy text into inline disclosures with single-open behavior
- use summary chips and short advisories as the visible layer; keep heavy explanation behind disclosure

### `web/src/components/AdvancedWorkspacePanel.vue`

Primary job:

- stop diagnostics and guidance from flattening the main control surface

Specific changes:

- use sunken support sections for lower-priority ranking diagnostics
- keep primary advanced inputs on the working plane
- make any currently recommended or active advanced subgroup the focus surface
- collapse explanatory blocks into inline disclosures by default

### `web/src/components\ValidationWorkspacePanel.vue`

Primary job:

- keep setup, summary, and table review visually tiered

Specific changes:

- use a sticky validation summary strip when results are present
- keep setup controls on the working plane
- bring the results summary or table container forward only after validation completes
- use overflow shadows and stronger section dividers around the results table

### `web/src/components/HistoryWorkspacePanel.vue`

Primary job:

- make browsing feel like moving through a managed archive, not a flat form

Specific changes:

- recess filter/search support regions
- use a stronger selected-state rail for the chosen run
- add a sticky selected-run summary for long details views
- keep list metadata and filters visible while long run lists scroll
- keep delete/clear affordances isolated inside their own action plane

### `web/src/components\ExecutionWorkspacePanel.vue`

Primary job:

- express live-risk posture through hierarchy, not just color

Specific changes:

- make the live or broker-facing action group the focus plane only when relevant
- keep account status and recent orders on the working plane
- isolate destructive and live actions inside compact but visually distinct subsections
- use inline disclosure for raw broker/debug output

## Information-Density Strategy

The goal is to convey more information without spending more 2D space.

Use these methods, in order:

1. stronger selection state
2. sticky local context
3. inline disclosure
4. sectional hierarchy
5. short tooltip/help text
6. overlays only when truly necessary

This is more effective than adding:

- more labels
- more cards
- more iconography
- more permanent side panels

## Tooltip / Help Architecture

Tooltips are useful here, but only if they are classified correctly.

### 1. Short Tooltip

Use for:

- terse control clarification
- abbreviation expansion
- one-sentence behavioral explanation

Trigger:

- normal hover/focus delay

### 2. Delayed Help Callout

Use for:

- "how this works" guidance
- advanced metric explanation
- warnings about side effects or interpretation

Trigger:

- long hover or deliberate help affordance
- target threshold: `3` to `5` seconds if hover-driven

### 3. Inline Disclosure

Use for:

- persistent tutorial content
- route-policy explanation
- formulas
- advanced behavioral nuance

Rule:

- if the user needs it to complete the task safely, it should not live only in a tooltip

### Global User Control

Implement a single global setting, not ad hoc local opt-outs.

Recommended model:

- `full`: short tooltips + delayed help callouts
- `concise`: short tooltips only
- `off`: no non-essential tooltip/help overlays

Suggested implementation anchors:

- setting persisted in `settingsStore`
- central provider/composable that all tooltip/help primitives read from
- components do not decide globally on their own whether tutorials are enabled
- tooltip/help behavior must stay keyboard-accessible and reduced-motion compatible

## Backend Functionality Exposure Audit

This should run alongside major frontend restructuring work.

Current companion document:

- `frontend_functional_coverage_matrix_2026-03-28.md`

### Required deliverables

1. capability inventory
2. frontend entry-point map
3. redundancy/consolidation recommendations
4. post-refactor exposure verification

### Capability inventory

Track user-meaningful backend-backed capabilities, not only raw commands.

Examples:

- data import/load/repair/cache actions
- optimization start/cancel/save/export flows
- validation run/export/review flows
- result drilldown and artifact loading
- execution connection/start/stop/review actions
- report/reveal/open artifact actions

### Frontend entry-point map

For each backend-backed capability, record:

- current frontend location
- primary entry point
- any secondary or redundant entry points
- whether the capability is high-stakes, advanced, or routine

### Consolidation rule

When simplifying the UI:

- remove redundant buttons only after confirming they map to the same underlying capability
- prefer one primary path plus optional advanced/disclosure placement over many parallel buttons
- preserve discoverability for high-stakes and advanced capabilities

### Verification rule

After each major shell/page refactor:

- verify every inventoried backend-backed capability still has at least one supported frontend path
- verify the remaining path is visible, understandable, and not blocked by unrelated prerequisites
- document intentional consolidations so they are not mistaken for accidental removals

## Package Guidance

### Use More Of What Already Exists

Prioritize broader use of:

- `reka-ui`
- `@floating-ui/vue`
- `@tanstack/vue-virtual`

Reason:

- these already align with the custom shell and avoid introducing a heavy UI opinion layer

### Strong Additions

#### `@tanstack/vue-table`

Use for:

- Top Results
- Validation tables
- Meta analysis tables
- Run Library list/table views

Value:

- richer table state
- column control
- filtering
- selection logic
- cleaner separation between table model and visual shell

Adoption note:

- keep `@tanstack/vue-virtual`
- integrate the two together rather than replacing virtualization with a heavier table implementation

#### `vee-validate` + `@vee-validate/zod`

Use for:

- Strategy
- Advanced
- Data import/config
- Validation setup

Value:

- typed validation
- clearer field error/touched/dirty state
- easier progressive disclosure of advanced validation

#### `notivue`

Use for:

- save/export success and failure
- validation completion
- run lifecycle updates
- non-blocking operational alerts

Value:

- central async feedback system instead of one-off status placements

### Optional UI Primitive Upgrade

#### Tree-shakable icon system

Use for:

- action rows
- status badges
- report/reveal/open affordances
- compact tool surfaces where glyph consistency improves scan speed

Constraint:

- choose one lightweight tree-shakable icon set only if the current text-glyph approach becomes a real clarity limit
- do not adopt multiple icon packs

#### `splitpanes`

Use only if:

- users benefit from actively trading control-pane width against tables or charts

Value:

- real desktop-workstation flexibility without new overlays

### Complementary Charting, Not Replacement

Keep `lightweight-charts` for its current use.

Use another charting stack only where richer categorical or analytical visuals matter more than maximum chart throughput.

Recommended candidate:

- `echarts` + `vue-echarts`

Good additive use cases:

- heatmaps
- Pareto/frontier charts
- correlation or interaction matrices
- validation distribution views
- categorical regime comparisons
- richer annotated metric dashboards

Implementation note:

- if adopted, load it dynamically on the specific analytical surfaces that need it
- keep `aria` support enabled and documented for those charts

Do not use it to replace:

- current price-oriented chart surfaces already served well by `lightweight-charts`

## Suggested Delivery Order

### Phase A: Low-Risk Depth Improvements

- add depth tokens and shared utilities in `app.css`
- add first-class elevation utility classes mapped to the four-layer model
- improve selected-state treatment
- add overflow shadows to dense scrollers
- add sticky local-context summaries where the workflow benefits
- migrate shared overlays away from hardcoded colors where practical

### Phase B: Dense Page Restructuring

- Strategy
- Advanced
- Validation
- History
- Execution

Focus:

- one focus plane per screen
- single-open inline disclosures
- support regions recede instead of competing
- capability grouping improves clarity without reducing functional coverage

### Phase C: Tooltip / Help System

- central setting in `settingsStore`
- `reka-ui` tooltip/toggletip-style primitives
- phased rollout on the densest screens first

Current implementation status:

- `settingsStore` now owns a persistent `helpMode` preference with `full`, `concise`, and `off`
- the first shared help primitive ships as `ContextHelpTip.vue`
- first-rollout help affordances are active in Data, Validation, Meta, and Execution

### Phase D: Table And Chart Technology Upgrades

- pilot `@tanstack/vue-table` on one major results surface
- pilot `vue-echarts` only on one non-price analytical surface
- keep `lightweight-charts` unchanged elsewhere

### Separate Desktop Productization Track

These items are valid product-quality work, but they should remain separate from the frontend-only depth pass:

- Tauri capability scoping review
- CSP hardening review
- updater flow
- OS notifications for long-run lifecycle
- tray/menu quick actions
- window-state persistence

Treat these as a desktop platform track, not as blockers for the current web/UI hierarchy work.

### Cross-Cutting Functional Coverage Track

- build a backend-functionality exposure matrix before large control-surface refactors
- use that matrix to drive redundancy cleanup and consolidation
- re-run the exposure audit after each major workflow/page rewrite

## Success Criteria

This work is successful if:

- the user can identify the active decision region immediately
- selected state feels more obvious without extra labels
- dense pages reveal more detail only when needed
- scrollable tables/charts imply continuation without adding clutter
- help content becomes more available but less noisy
- the UI feels clearer and deeper without becoming visually busier

## Acceptance / Definition Of Done

### Depth-System Acceptance

- shared depth/elevation utilities exist and are token-driven
- no new shared overlay or dialog styling lands with raw colors
- at least the shell, Top Results, and Run Library visibly use the depth model

### Keyboard-First Acceptance

- shell workspace tabs remain keyboard complete
- shell workflow-step tabs remain keyboard complete
- Top Results remains keyboard complete
- Validation table path remains keyboard complete
- Run Library list browsing remains keyboard complete

### Visual/Contrast Acceptance

- selected rows remain obvious in light and dark themes
- warning/danger/success surfaces remain visually distinct and readable
- overlay and sticky surfaces pass a contrast sanity check in both themes

### Regression Acceptance

- `npm run typecheck` passes
- `npm run test` passes
- `npm run build` passes
- `npm run baseline:ui` passes

### Functional Coverage Acceptance

- no previously exposed backend-backed user capability is removed without explicit approval
- any removed duplicate control is documented as a consolidation, not an accidental loss
- every high-stakes backend-backed capability still has at least one clear frontend path
- the frontend entry-point map is updated after major UX restructuring

## Source Notes

This brief is informed by current official guidance from:

- Fluent elevation and depth
- Atlassian elevation
- Carbon disclosures/forms/data visualization guidance
- Windows/Carbon tooltip guidance
- Apache ECharts accessibility guidance

Use those as functional references, not as a mandate to copy their visual language directly.
