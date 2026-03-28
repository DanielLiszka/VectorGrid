# Frontend Depth / UX Implementation Brief

Date: 2026-03-28

Scope: Vue 3 frontend under `web/`

Status: implementation brief for the next UX pass

## Objective

Improve perceived depth, hierarchy, and information density without:

- forcing a stylized faux-3D theme
- increasing modal/overlay usage
- replacing `lightweight-charts` for its current price/timeline role

This brief treats "more 3D" as a hierarchy problem, not a decoration problem.

## Decision

Do not push the product toward a visibly skeuomorphic or heavily elevated UI.

Instead:

- keep the shell visually restrained
- introduce a small depth model only where it improves comprehension
- use surface contrast, selected-state emphasis, sticky context, and progressive disclosure before introducing new chrome

In practice, the app should feel more layered and easier to scan, not more ornamental.

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

### Motion Tokens

- `--motion-fast`
- `--motion-ui`
- `--motion-slow`
- `--ease-standard`

Rules:

- use shadows sparingly; tonal depth should do most of the work
- focus surfaces should feel intentional, not floating
- selection rails should read more strongly than generic borders

## Shared Utility Additions For `app.css`

Add utilities that make layered information cheap to apply.

### Recommended Utilities

- `.surfaceSunken`
- `.surfaceWorking`
- `.surfaceFocus`
- `.surfaceSticky`
- `.selectionRail`
- `.overflowShadowX`
- `.overflowShadowY`
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
- `stickySelectionSummary`: keep local context visible without opening another panel
- `inlineDisclosure*`: reveal depth inside the current pane rather than outside it

## Component-Level Changes

### `web/src/styles/app.css`

Primary job:

- define the depth model and make it reusable

Specific changes:

- add depth tokens listed above
- introduce restrained overflow shadows for scrollable wrappers
- strengthen selected-row and selected-chip visual language
- add sticky local-context utility styles
- add short transition tokens for underline, row-state, disclosure, and focus-surface emphasis

### `web/src/components/VueWorkspaceShell.vue`

Primary job:

- make the shell feel layered without becoming heavy

Specific changes:

- keep workspace tabs on the base plane
- let the active workspace and active workflow step use a stronger underline plus subtle focus field, not pill chrome
- convert the shell context strip into a slightly recessed support band
- ensure the controls/results tab row reads as a working-plane switch, not a second header
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

Do not use it to replace:

- current price-oriented chart surfaces already served well by `lightweight-charts`

## Suggested Delivery Order

### Phase A: Low-Risk Depth Improvements

- add depth tokens and shared utilities in `app.css`
- improve selected-state treatment
- add overflow shadows to dense scrollers
- add sticky local-context summaries where the workflow benefits

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

### Phase C: Tooltip / Help System

- central setting in `settingsStore`
- `reka-ui` tooltip/toggletip-style primitives
- phased rollout on the densest screens first

### Phase D: Table And Chart Technology Upgrades

- pilot `@tanstack/vue-table` on one major results surface
- pilot `vue-echarts` only on one non-price analytical surface
- keep `lightweight-charts` unchanged elsewhere

## Success Criteria

This work is successful if:

- the user can identify the active decision region immediately
- selected state feels more obvious without extra labels
- dense pages reveal more detail only when needed
- scrollable tables/charts imply continuation without adding clutter
- help content becomes more available but less noisy
- the UI feels clearer and deeper without becoming visually busier

## Source Notes

This brief is informed by current official guidance from:

- Fluent elevation and depth
- Atlassian elevation
- Carbon disclosures/forms/data visualization guidance
- Windows/Carbon tooltip guidance
- Apache ECharts accessibility guidance

Use those as functional references, not as a mandate to copy their visual language directly.
