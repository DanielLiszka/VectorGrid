# Frontend Design System Seed

Date: 2026-03-28

Scope: Vue 3 frontend under `web/`

Purpose: provide a code-aligned starting point for a canonical design system and eventual Figma source of truth.

Superseded by:

- `frontend_design_system_spec_2026-03-28.md`
- `frontend_target_comps_2026-03-28.md`

## Current Direction

Visual thesis:
Operational desktop UI with restrained chrome, strong hierarchy, fast scanning, and explicit workflow state.

Primary product posture:
- workflow first
- data-dense but readable
- utility copy over marketing copy
- progressive disclosure for dense route-policy and debug detail

## Semantic Color Roles

Keep the token system semantic, not panel-specific.

- `--bg-app`: main shell background
- `--bg-surface`: default panel background
- `--bg-surface-strong`: prominent pane background
- `--bg-surface-muted`: secondary/docked background
- `--border`: standard panel and control border
- `--border-strong`: section and emphasized border
- `--text`: primary text
- `--text-muted`: secondary/supporting text
- `--text-subtle`: low-priority metadata
- `--accent`: primary action and selection state
- `--accent-strong`: active tab / active action emphasis
- `--focus-border`: focus-visible inner edge
- `--focus-ring`: full focus ring shadow
- `--success`: successful run or healthy state
- `--warning`: caution / route warning / live review advisory
- `--danger`: destructive action or failed state
- `--info`: neutral status / async progress

Usage rules:
- use semantic tokens for meaning, not component ownership
- never introduce raw hex values inside component styles unless tied to a temporary migration
- keep warning and danger visually distinct from accent

## Typography Scale

The frontend should keep one compact operational scale across the shell.

- App brand: `h1`, 1.125rem to 1.25rem
- Workspace panel title: `h2`, 1rem to 1.1rem
- Section title: `h3`, 0.92rem to 0.98rem
- Eyebrow / overline: 0.68rem to 0.74rem, uppercase, tracked
- Body: 0.84rem to 0.9rem
- Support / hint / status: 0.78rem to 0.82rem
- Avoid routine UI text below 0.78rem unless it is purely decorative metadata

Font roles:
- primary UI: current sans stack already in repo
- dense metrics and ids: mono only where scanning benefits from alignment

## Spacing Scale

Use one spacing rhythm across all panels.

- `4px`: tight control clusters only
- `8px`: default inline gap
- `12px`: small group separation
- `16px`: standard section padding
- `24px`: major section separation
- `32px`: page-level separation

Rules:
- dense forms should prefer repeated 8/12/16 spacing instead of ad hoc values
- summary chips, status bars, and small advisories should align to the same vertical rhythm

## Component States

Every reusable interactive surface should define these states:

- rest
- hover
- focus-visible
- active / selected
- disabled
- warning
- error
- loading / pending

Required examples:
- shell tabs
- workflow step tabs
- primary buttons
- destructive buttons
- details summaries
- table rows
- selectable list rows
- dock toggles

## Desktop Layout Rules

The desktop app should continue using explicit workspace ownership rather than dashboard-card mosaics.

- shell owns navigation, workspace selection, and result/controls split
- major panels should render as page sections, not nested card stacks
- docks are secondary context only and must not hide primary controls on constrained widths
- route-policy and debug detail belong behind disclosure when they compete with working controls
- results pages should always expose one non-pointer path to the next action

## Canonical Surface Targets

These are the screens that should become the first canonical target comps.

1. Workflow shell
   Includes workspace bar, workflow-step bar, controls/results tabs, task center, footer behavior

2. Data authoring
   Includes CSV load, Polygon load, timeframe change, dataset context

3. Strategy authoring
   Includes shared risk/cost controls plus one representative strategy-specific module

4. Advanced diagnostics
   Includes route mode, exact-kernel diagnostics, output shaping, sampler state

5. Results review
   Includes Overview, Top Results, Drill or Heatmap

6. Run Library
   Includes session history, saved run review, destructive-action confirmation

7. Execution / Settings
   Includes appearance/storage settings plus paper/live execution guardrails

## Immediate Figma Backlog

When Figma auth is available again, the first artifact should be a small system file containing:

- color tokens
- type styles
- spacing tokens
- tab states
- button states
- panel header pattern
- summary chip pattern
- details/disclosure pattern
- table/list selection pattern

The first screen comps to create:

1. workflow shell default state
2. strategy setup with one dense strategy expanded
3. advanced diagnostics with disclosures
4. top results review
5. library detail view
6. execution safety view

## Connector Note

Attempted to use the Figma design-system-rules connector on 2026-03-28, but the connector returned:

`401 Unauthorized. Manual reauthentication required.`

Until that is fixed, this document is the repo-local fallback seed for Phase 10.
