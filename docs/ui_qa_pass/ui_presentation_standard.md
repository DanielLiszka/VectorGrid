# UI Presentation Standard

This standard governs the UI QA pass. Use it before making screen-specific visual changes.

## Design Target

The app should read as a serious desktop research tool: compact, confident, and technical. It should look built around real workflows, not assembled from every available component.

Use the language of lab instruments, trading terminals, and research cockpits:

- structured density
- strong alignment
- restrained color
- precise state language
- quiet secondary detail
- diagnostics available when needed

## Screen Contract

Every major screen should answer these questions immediately:

1. What workflow am I in?
2. What is the current state?
3. What is blocking me, if anything?
4. What is the primary next action?
5. What result or artifact is currently selected?
6. Where can I inspect advanced detail?

If a screen cannot answer those questions cleanly, reduce visible competing information before adding more UI.

## Canonical State

Each workflow surface should derive visible status from one canonical state.

Recommended top-level states:

- `missing_data`
- `configure_required`
- `ready`
- `running`
- `completed`
- `failed`
- `blocked_by_dependency`
- `previous_result_available`

Avoid letting independent widgets invent local status phrases that conflict visually.

Example problem state:

- `Completed`
- `Validation not run`
- `no_data`
- `0 rows`
- `Load data first`
- `Latest result`

A better combined state:

> Validation blocked: load a dataset. A previous optimization result is available, but validation needs active candle data.

## Visual Hierarchy

Each screen should have:

- one dominant status/message area
- one primary action
- one selected object, if applicable
- one place for diagnostic or advanced detail

Avoid giving equal visual weight to:

- workflow steps
- summary cards
- inspector boxes
- status ribbons
- nested forms
- empty states
- debug metadata
- footer bars

If everything is boxed, nothing is important.

## Card and Panel Rules

Use cards for:

- major app regions
- repeated comparable objects
- modal/dialog surfaces
- selected-object summaries

Avoid:

- cards inside cards
- boxed rows inside boxed sections
- bordered wrappers around every small control group
- multiple shadows on nested surfaces

Inside a major panel, prefer:

- compact rows
- section dividers
- tables
- definition lists
- grouped controls without extra card chrome

## Copy Rules

Visible copy should be short, public-facing, and action-oriented.

Prefer:

- `Dataset missing`
- `Validation blocked`
- `CUDA available`
- `No candidate passed validation gates`
- `Load a dataset to continue`

Avoid visible raw/internal strings:

- `no_data`
- `no_candidate_survived`
- `cuda_available=true`
- `strategy=DoubleMa`
- `combos=38605`
- `gpu_speedups.csv`
- request/response field names

Internal strings can appear in diagnostics, logs, developer panels, exported evidence, or explicit details views.

## Progressive Disclosure

Keep visible:

- current workflow
- current state
- blocking reason
- primary action
- selected candidate/artifact
- key result metrics
- required thresholds/gates
- destructive or irreversible consequences

Move to tooltip, popover, expandable details, inspector, or diagnostics:

- definitions
- rationale
- backend route policy
- GPU capability details
- build flags
- CSV policy files
- timing metadata
- command names
- raw IDs
- long helper blurbs

Do not hide:

- hard errors
- required next steps
- compliance/release warnings
- disabled-control reasons when no nearby state explains the disabled state

## Tooltips and Help

Tooltips should explain why or define terms. They should not carry information needed to complete the workflow.

Good tooltip candidates:

- PSR, DSR, CSCV, PBO definitions
- why a backend route was selected
- why a control is disabled after the nearby state already says the screen is blocked
- what an advanced option changes

Bad tooltip candidates:

- the only explanation of an error
- the only place a user can discover the next step
- large paragraphs that should be documentation or diagnostics

## Workflow Relevance

Do not expose unrelated workflow controls by default.

Examples:

- Options-specific controls should not take space in a conventional equity optimization flow unless the user selected options mode.
- ML/CatBoost controls should not compete with conventional validation unless ML Signal is active.
- GPU diagnostics should not dominate routine validation unless the user opens diagnostics or the selected route needs attention.
- Previous optimization setup should collapse into a read-only snapshot while validation is active unless the user is editing setup.

## Color Semantics

Use color sparingly and consistently.

- Blue: selected item or primary action
- Green: completed/success
- Amber: blocked/warning/needs attention
- Red: failed/error/destructive
- Neutral: inactive, unavailable, historical, or informational

Avoid soft tinted panels everywhere. Too many subtle color washes make every state feel passive and equal.

## Typography and Data Display

Use monospace only for:

- file paths
- symbols/tickers
- IDs
- raw metrics
- logs
- code-like backend labels

Use normal UI text for:

- labels
- status
- user-facing explanations
- button text
- workflow titles

Keep helper text short. Prefer one decisive sentence over several local captions.

## Inspector Contract

The inspector should not merely repeat the main screen.

It should contain:

- selected object details
- requirements/gates
- actionable warnings
- evidence/artifact links
- diagnostics when explicitly selected

If the inspector only restates status chips from the main panel, collapse it or make its content more specific.

## Empty and Blocked States

Blocked states should suppress unavailable machinery.

A blocked state may show:

- the blocked reason
- required next action
- a compact selected-object summary
- a collapsed preview of relevant settings

Avoid showing full parameter forms, export actions, or unrelated advanced controls when the prerequisite is missing.

## Accessibility and Interaction

Progressive disclosure must remain keyboard and screen-reader accessible.

Requirements:

- visible focus states
- semantic button/summary/disclosure controls
- accessible names for icon-only buttons
- status regions for async state changes
- no tooltip-only critical information
- Playwright coverage for high-risk disclosure and disabled states
