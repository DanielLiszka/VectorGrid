# Frontend Theme Proposal

Date: 2026-03-28

Scope: Vue 3 desktop frontend under `web/`

Method:

- grounded in the current frontend audit and design-system docs
- shaped with the `build-web-apps:frontend-skill` rubric
- attempted to use the Figma design-system rules tool, but the MCP call timed out, so this proposal is repo-local for now and ready to mirror into Figma later

## Recommendation

Adopt a single primary visual theme for the product:

## Theme Name: Signal Foundry

Signal Foundry should feel like a professional research workstation, not a retail trading dashboard and not a neon terminal.

Visual thesis:

- a softened control-room interface built from graphite surfaces, mineral blue focus states, warm brass warnings, and editorial hierarchy

Product impression:

- precise
- calm under load
- data-dense without looking cramped
- premium but operational
- serious enough for live-risk workflows

## Why This Fits The App

The app is strongest when it behaves like an operator bench:

- data import and provenance
- strategy construction
- optimization and validation
- results triage
- broker-facing execution review

That workflow wants:

- long-session readability
- one stable accent for action and selection
- obvious safety separation for live-risk states
- restrained chrome around dense tables and charts
- typography that makes technical panes feel intentional rather than improvised

Signal Foundry matches that better than:

- generic fintech SaaS blue-on-white
- hacker-terminal green on black
- glossy glassmorphism
- colorful dashboard-card mosaics

## Core Art Direction

### Material Language

- shell surfaces should read as layered machined panels, not floating cards
- contrast comes from tone, border strength, and spacing before shadows
- raised overlays and dialogs should feel like instrument sheets placed above the bench
- charts remain clean working surfaces, not decorative hero elements

### Color Character

- primary tone: graphite and deep slate
- action accent: mineral blue
- warning accent: brass/amber
- danger accent: muted rose-red
- success accent: restrained green, only for confirmed positive state

Rules:

- blue is for action, focus, and selection
- amber is for caution and review
- red is for destructive or unsafe states only
- green is not a second brand color

### Typography

Recommended pair:

- UI sans: `IBM Plex Sans`
- utility mono: `IBM Plex Mono`

Why:

- Plex Sans feels technical and deliberate without becoming cold
- Plex Mono gives ids, timestamps, metrics, and route details a proper instrument-readout voice
- this pair fits the current desktop-tool posture better than `Inter` or system-ui

Typography behavior:

- workspace titles and section headers should feel like briefing labels
- metadata stays compact but readable
- mono is reserved for metrics, paths, timestamps, ids, and code-like payloads

## Palette Proposal

These values are intended as direction, not mandatory final token values.

### Dark Theme

- `--bg`: `#0d1420`
- `--panel`: `rgba(18, 28, 43, 0.98)`
- `--panel-footer`: `rgba(18, 28, 43, 0.965)`
- `--surface`: `#132033`
- `--surface2`: `#1b2a40`
- `--surface-raised`: `#22344d`
- `--text`: `#e7edf7`
- `--muted`: `#97a7bc`
- `--muted-strong`: `#c8d3e3`
- `--accent`: `#3ea6ff`
- `--accent-hover`: `#68bbff`
- `--warning`: `#d29a39`
- `--danger`: `#e07a8b`
- `--ok`: `#3ec28d`
- `--border`: `#334862`
- `--border-strong`: `#4a6383`

Dark background treatment:

- keep the app background atmospheric but restrained
- one diffuse mineral-blue radial bloom near the upper-left is enough
- avoid adding multiple decorative gradients behind routine working panes

### Light Theme

- `--bg`: `#f3f2ee`
- `--panel`: `rgba(252, 251, 248, 0.985)`
- `--panel-footer`: `rgba(250, 249, 246, 0.972)`
- `--surface`: `#ffffff`
- `--surface2`: `#f1f3f5`
- `--surface-raised`: `#e8edf2`
- `--text`: `#162133`
- `--muted`: `#536277`
- `--muted-strong`: `#334255`
- `--accent`: `#0d84d8`
- `--accent-hover`: `#0b6fb7`
- `--warning`: `#a86a16`
- `--danger`: `#c84e66`
- `--ok`: `#238a61`
- `--border`: `#d6dde6`
- `--border-strong`: `#b7c3d1`

Light background treatment:

- shift the light mode away from cold SaaS white
- use a slightly mineral or paper-warm base so the product still feels like an instrument, not a marketing dashboard

## Surface Language

### Shell

- the shell should feel like a fixed workbench
- workspace tabs should read as machine-mode switches
- workflow-step tabs should feel slimmer and more procedural
- the active state should use a strong underline plus a soft accent field, not pill-button chrome

### Panels

- panels should read as sections cut into the bench, not separate cards laid on top
- use dividers, headings, and chip rows before boxed containers
- prominent panes should be rare and reserved for the current decision hotspot

### Tables And Lists

- top-results and run-library lists should feel like research sheets
- selection should be obvious through row tint, left-edge indicator, and live summary
- zebra striping should stay subtle
- column headers should look like tooling labels, not button rows

### Dialogs And Overlays

- command palette: blackened-glass search console with crisp result rows
- confirmation dialogs: raised sheets with strong action hierarchy and more negative space than inline warnings
- task center: assistant tray, not a floating widget

## Motion Direction

Keep motion short, deliberate, and useful.

Recommended motion thesis:

- active shell tab underline glides into place in `140ms` to `180ms`
- overlays and dialogs fade and lift by a few pixels rather than scaling dramatically
- selected table rows get a brief emphasis pulse when keyboard selection changes

Avoid:

- springy consumer-app motion
- decorative parallax
- continuous shimmer or glow effects

## Iconography And Data Viz

### Icons

- prefer simple geometric line icons
- consistent stroke weight
- no oversized multicolor decorative icons in working panes

### Charts

- chart chrome should stay quiet
- series colors should preserve semantic meaning:
  - focus candidate: blue
  - comparison or secondary reference: slate
  - warning threshold or unsafe region: amber
  - confirmed failure or stop condition: red
- chart annotations should look like analyst marks, not presentation callouts

## Per-Surface Theme Translation

### Data

- emphasize provenance, timeframe, and cache state
- make import methods feel like acquisition tools, not giant call-to-action cards

### Strategy And Advanced

- use clear hierarchy and fewer visual containers
- one prominent pane for shared risk and execution posture
- long route-policy guidance stays behind details disclosures

### Results

- this is the most visually important workspace after strategy authoring
- the shortlist table and drill surface should feel like one analytical unit
- the selected winner should feel unmistakable

### History

- should feel like a managed archive
- lists need more of a library/catalog tone than a form tone

### Execution

- this is the one area that should intentionally tighten and harden visually
- live-risk state should be impossible to confuse with paper or review state
- destructive actions should feel isolated, not neighboring defaults

## What This Theme Explicitly Rejects

- purple gradients
- consumer fintech polish
- oversized glossy cards
- all-neon terminal styling
- indiscriminate glass effects
- using multiple accent colors as pseudo-branding

## Implementation Mapping To Current Code

This theme can be introduced without restructuring the whole app.

Primary implementation path:

1. remap the token palette in `web/src/styles/app.css`
2. swap the default sans stack to `IBM Plex Sans` and keep `IBM Plex Mono` for utility text
3. tighten shell/tab styling to use an underline-plus-field active state
4. reduce card-like treatment where panes still look boxed instead of sectional
5. standardize table selection and list selection visuals around the same blue state language
6. harden execution/liveness colors so the live path reads as categorically different

## Figma Follow-up

When the Figma connector is responsive again, this theme should be mirrored into a Figma library with:

- color variables
- type styles
- shell tabs
- workflow tabs
- summary chips
- table/list row states
- confirmation dialog
- task center tray
- core screen comps for Data, Strategy, Results, History, and Execution

## Recommendation Summary

If you want one clear direction, use Signal Foundry.

It is the right tone for this app because it:

- respects dense analysis work
- reinforces operational trust
- gives live-risk actions sharper visual separation
- upgrades the app from "functional internal tool" to "purposeful trading workstation"
