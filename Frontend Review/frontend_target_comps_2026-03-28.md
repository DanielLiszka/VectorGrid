# Frontend Target Comps

Date: 2026-03-28

Purpose: define the target composition and hierarchy for the core desktop surfaces so implementation can align to a stable design target before Figma is available.

## 1. Workflow Shell

Intent:

- make workspace and workflow state obvious at a glance
- keep navigation stable while the working panel changes

Required composition:

- app title and workflow subtitle
- workspace tab row
- workflow-step tab row
- shell context strip
- controls/results tab rows
- task center launcher
- workflow-only footer

Rules:

- active workspace and step must be obvious visually and announced programmatically
- the subtitle should describe the active task, not generic product copy

## 2. Strategy Authoring

Intent:

- make the next authoring decisions obvious even on dense strategy forms

Required composition:

- summary chips for strategy family, exit mode, execution posture, and constraints
- one prominent pane for shared risk/cost/exit controls
- grouped panes for strategy-specific modules
- disclosure for long route-policy guidance

Rules:

- shared controls appear before strategy-specific parameters
- long warnings use advisory/details patterns, not raw paragraph walls

## 3. Results Review

Intent:

- move quickly from winner identification to deeper inspection

Required composition:

- overview summary
- top-results shortlist table
- one chart-heavy drilldown surface
- explicit keyboard-safe path between shortlist and charts

Rules:

- selected candidate state must be visible and announced
- chart-only actions cannot be the only path forward

## 4. Run Library

Intent:

- make saved-run browsing feel like a managed library, not a form dump

Required composition:

- session history list
- saved run list with search/tags
- selected run detail card
- modal confirmation for clear/delete actions

Rules:

- saved artifact lists expose selection state semantically
- destructive actions always use a confirmation dialog

## 5. Execution and Settings

Intent:

- make live-risk posture explicit before broker-facing actions

Required composition:

- execution safety banner
- clearly grouped action sections
- runtime status grid
- order activity table
- disclosure for raw output

Rules:

- live actions are visually separated from paper/manual actions
- destructive key-clearing remains isolated from normal account review

## 6. Data Source

Intent:

- confirm dataset provenance and timeframe before the user optimizes

Required composition:

- data header with dataset summary chips
- CSV import
- Polygon import/cache surface
- storage/cache controls
- dataset context and timeframe panes

Rules:

- cached dataset selection must be visible and announced
- timeframe/storage changes should not look secondary to import actions

## Current Implementation Note

These target comps are now the canonical frontend targets alongside:

- `frontend_design_system_spec_2026-03-28.md`
- `frontend_design_system_seed_2026-03-28.md`

Figma should mirror these structures once connector access is restored, not redefine them from scratch.
