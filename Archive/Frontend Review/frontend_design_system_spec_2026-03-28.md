# Frontend Design System Specification

Date: 2026-03-28

Scope: Vue 3 frontend under `web/`

Status: canonical repo-local source of truth until a dedicated Figma library is re-established.

## Purpose

This document defines the target design system for the current desktop UI so implementation work is not blocked on Figma auth or a separate design file.

## Product Thesis

VectorGrid should read as an operational desktop tool:

- workflow first
- data-dense but readable
- strong state visibility
- progressive disclosure over clutter
- one obvious keyboard-safe path through every primary task

## Semantic Color Roles

Use semantic roles, not component-local hex values.

- App background: `--bg`
- Primary panel surface: `--panel`, `--panel-footer`, `--surface`
- Secondary surface: `--surface2`
- Raised surface: `--surface-raised`
- Accent surface: `--surface-accent`
- Warning surface: `--surface-warning`
- Danger surface: `--surface-danger`
- Success surface: `--surface-success`
- Standard border: `--border`
- Strong border: `--border-strong`
- Primary text: `--text`
- Support text: `--muted`
- Strong support text: `--muted-strong`
- Subtle metadata: `--muted-soft`
- Primary action: `--accent`
- Destructive action: `--danger`
- Caution state: `--warning`
- Success state: `--ok`
- Legible state text on tinted surfaces: `--danger-ink`, `--warning-ink`, `--ok-ink`

Rules:

- warning/danger/success panes should use semantic surface plus ink tokens
- raw hex values inside component styles are migration exceptions only
- selection state should not reuse warning or danger colors

## Typography Scale

The desktop shell uses one compact operational scale.

- App title: `--text-xl`
- Workspace title: `--text-lg`
- Section lead and primary body: `--text-md`
- Labels, metadata, supporting copy: `--text-sm`
- Chips, eyebrows, compact summaries: `--text-xs`

Line-height rules:

- titles: `--leading-tight`
- advisories and labels: `--leading-compact`
- body copy and subtitles: `--leading-body`

Usage rules:

- default to the shared `panelTitle`, `panelPaneTitle`, `panelSubtitle`, `panelSectionLead`, `panelMuted`, and `panelStatus` utilities
- keep mono text for ids, timestamps, sizes, and aligned metrics only
- routine product copy should not drop below the shared small-text scale

## Spacing Scale

The shell should converge on one spacing cadence.

- `--space-1`: tight inline separation
- `--space-2`: small group separation
- `--space-3`: default section and control gap
- `--space-4`: standard internal pane padding
- `--space-5`: page-level separation

Rules:

- prefer repeating 8/12/16/20-style rhythm over panel-specific spacing values
- panel headers, chip rows, details blocks, and list rows should align to the same scale

## Component State Model

Every reusable interactive component should support:

- rest
- hover
- focus-visible
- active / selected
- disabled
- loading / pending
- warning
- error

Required reusable patterns:

- shell tabs
- workflow-step tabs
- summary chips
- primary buttons
- destructive buttons
- selectable list rows
- table-row selection
- details/disclosure summaries
- dialogs and command overlays

## Layout Rules

- The shell owns workspace selection, workflow selection, and the controls/results split.
- Panels should behave like page sections, not nested dashboard cards.
- Docks and overlays are contextual aids only. They must yield on narrower widths instead of obscuring primary controls.
- Dense policy/debug guidance belongs behind details/disclosure when it competes with working controls.
- Every chart or canvas-heavy page needs a text summary plus a keyboard-safe alternate path.

## Canonical Shared Patterns

- Panel header: eyebrow, title, subtitle, trailing timestamp/meta
- Summary chips: compact state recap directly below the header
- Prominent pane: reserved for the next action or highest-value state
- Advisory block: short operational guidance, never dump-sized
- Details block: long guidance, raw output, or troubleshooting payloads
- Modal confirmation: alert dialog with focus management and explicit cancel/confirm

## Implementation Mapping

Current code anchors:

- shared tokens and type utilities: `web/src/styles/app.css`
- shared confirmation dialog: `web/src/components/ConfirmationDialog.vue`
- shell navigation semantics: `web/src/components/VueWorkspaceShell.vue`
- task center dialog pattern: `web/src/components/TaskCenterOverlay.vue`

## Figma Follow-up

When Figma auth is working again, this spec should be mirrored into a library file with:

- color variables
- type styles
- spacing tokens
- shell/tab/button/list/dialog components
- the canonical screen comps listed in `frontend_target_comps_2026-03-28.md`
