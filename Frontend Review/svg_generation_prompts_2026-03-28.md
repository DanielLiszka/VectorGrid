# SVG Generation Prompts

Date: 2026-03-28

Purpose: provide paste-ready GPT-5.4 Pro prompts for generating the custom SVG assets proposed for this repo.

This document is a companion to:

- [existing_stack_utilization_plan_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\existing_stack_utilization_plan_2026-03-28.md)

It is intentionally separate so each asset can be generated one at a time without extracting prompt text from the broader implementation plan.

The already-supplied VectorAlpha logo assets are not covered here. Those are separate brand assets and are already being integrated directly.

## How To Use This Document

For each SVG you want GPT-5.4 Pro to create:

1. Use the shared `System Prompt` below.
2. Then use the asset-specific `User Prompt` for the exact SVG you want.
3. Ask for raw SVG output only.
4. Generate one asset per conversation/request.
5. Tell the model to use multiple internal refinement passes before returning its final SVG.

Recommended workflow:

- start with Batch A
- review the returned SVG visually
- if needed, run a second refinement prompt against the same asset before moving on
- prefer a model that can spend real reasoning budget on self-critique rather than a one-shot fast draft

The intended generation posture is:

- draft internally
- critique internally
- refine internally
- only then emit one polished SVG

The model should not treat the first obvious composition as the answer.

## Required Iteration Policy

Every SVG generation request in this document should explicitly assume the following quality process:

1. Generate more than one internal concept direction before settling on a final composition.
2. Reject concepts that drift into generic dashboard UI, office-document metaphors, or hype-fintech iconography.
3. Evaluate the candidate against:
   - small-size legibility
   - family consistency
   - distinctness from adjacent icons
   - monochrome/theming resilience
   - restrained quant-workstation tone
4. Refine the geometry at least twice before returning the final SVG.
5. Return only the final polished SVG, not the intermediate drafts.

This is especially important for GPT-5.4 Pro because it can spend multiple internal reasoning turns improving the SVG before it emits the final answer.

## Shared System Prompt

Use this system prompt for every SVG generation request in this document:

```text
You are generating one production-ready SVG asset for a Tauri + Vue 3 desktop trading workstation called VectorGrid.

Return only raw SVG markup. Do not wrap the SVG in Markdown fences. Do not explain your work. Do not output JSON. Do not output multiple variants unless explicitly requested.

Use a multi-pass internal design process before you answer:
- explore more than one composition internally
- critique weak or generic directions internally
- refine the best candidate multiple times
- self-check the final candidate against the requested meaning, family consistency, distinctness, and small-size legibility
- only then output the final raw SVG

Do not output your intermediate concepts, critique, or iteration notes. Output only the final polished SVG.

Global constraints:
- Output exactly one standalone Two SVG files, one with color, one without.
- Transparent background.
- No embedded JavaScript.
- No raster images or base64 image payloads.
- No external font dependencies.
- Prefer paths, rects, circles, lines, and polygons over text nodes.
- If a letterform or symbol would normally use text, convert it into vector shapes instead of using <text>.
- Use a clean viewBox such as 0 0 24 24, 0 0 32 32, 0 0 48 48, 0 0 96 96, or 0 0 128 128 depending on the requested asset.
- Keep the geometry centered and well-balanced inside the viewBox.
- The asset must work in both light and dark UI themes.
- Prefer currentColor-compatible fills/strokes for icons unless the prompt explicitly requests a restrained multi-tone illustration.
- If you use more than one tone, keep it subtle and easy to recolor later.
- Avoid gradients unless the prompt explicitly allows them.
- No glossy effects, glassmorphism, drop shadows, or fake-3D styling.
- No crypto-fintech clichés, trophy imagery, rockets, moons, flames, or hype visuals.
- The style should feel like a restrained quant workstation: technical, trustworthy, calm, minimal, operator-bench rather than marketing-heavy.
- Avoid tiny visual details that disappear at small sizes.
- Use one unified symbol structure per asset rather than multiple unrelated mini-icons grouped together.
- Do not add detached sidebars, rulers, axes, legends, browser chrome, selected-row backgrounds, enclosing cards, or highlight capsules unless the prompt explicitly requires them.
- Do not solve emphasis by placing a background shape behind the main symbol unless the prompt explicitly requires that composition.
- Do not mix incompatible metaphors in the same icon, such as chart + folder + badge, unless the prompt explicitly asks for that combination.
- Rounded joins and linecaps are preferred where appropriate.
- If the asset is an icon, optimize it to read clearly at 14px to 18px rendered size.
- If the asset is an illustration, optimize it to read clearly at 72px to 128px rendered size.

Animation rules if the prompt requests an animated variant:
- No JavaScript animation.
- Use only SVG/CSS-safe animation techniques such as transform, opacity, stroke-dashoffset, or simple keyframes.
- No flashing.
- No hyperactive looping.
- No bounce-heavy motion.
- Animation must still look acceptable as a frozen frame.
- Include a prefers-reduced-motion fallback that effectively pauses or removes the motion.

Accessibility:
- Include role="img".
- Include a <title> and <desc>.
- Keep naming generic enough for app integration.

Output quality bar:
- clean, production-friendly SVG
- visually balanced
- semantically aligned to the requested concept
- easy for a frontend engineer to drop into a Vue/Tauri app
```

## Prompting Notes

- If the model returns `<text>` for an icon asset, reject it and ask for vector-only geometry.
- If the model uses hardcoded colors on an icon that should be theme-friendly, ask it to switch to `currentColor`.
- If the model adds too much visual noise, ask for fewer shapes and clearer silhouette first.
- If the model solves emphasis using a background capsule, selected-row highlight, detached sidebar, or enclosing panel that the prompt did not ask for, reject it and ask for the concept to be expressed through the symbol geometry itself.
- If the model returns something that looks like generic dashboard UI chrome instead of a compact symbolic icon, ask for a more self-contained mark with fewer UI-like framing elements.
- For animated variants, ask for static-first clarity and motion-second polish.

## Family-Level Consistency Rules

Use these additional rules to keep the generated set visually coherent across multiple requests.

### Provenance Icon Family

- All provenance icons should feel like one family:
  - same overall stroke weight
  - same corner softness
  - similar visual density
  - similar silhouette complexity
- Prefer stroke-first construction with only occasional restrained fill accents.
- Avoid circular badges, shield badges, or chip-container outlines as the dominant form.
- Each icon should communicate a different analytical caveat, not a generic status.
- Keep the core concept centered and compact so it fits naturally inside provenance chips.
- Do not let `validation-stale` and `dataset-changed` collapse into the same “time/update” metaphor.
- Do not let `review-attention` drift into a warning icon.

### Compare / Pin Icon Family

- All compare-family icons should share a common visual language built around:
  - compact candidate cards
  - paired tiles
  - shared axes or aligned review frames
- Avoid introducing unrelated metaphors between icons.
- `pin-candidate` should still look like a compare/research action, not an office pin or bookmark.
- `compare-candidates` should read as simultaneous side-by-side review.
- `replace-pinned` should read as controlled substitution inside a compare set, not generic sync.
- `remove-from-set` should read as removal from a research tray, not destructive deletion.
- `candidate-focus` should read as “promote to primary review” rather than “winner” or “favorite.”

### Task Lifecycle Icon Family

- All task-state icons should share one underlying computational grammar:
  - staged path
  - segmented ring
  - compact progression spine
- Then vary the terminal cue or interruption cue by state.
- `queued` should look orderly and waiting.
- `running` should look active and computational.
- `cancelling` should look transitional.
- `completed` should look resolved.
- `failed` should look interrupted or broken.
- `cancelled` should look intentionally halted, not broken.
- Keep these states visually distinguishable even when rendered at 14px.

### Artifact / Utility Icon Family

- All artifact icons should feel like one compact utility family:
  - structured
  - document-adjacent
  - analytical rather than office-themed
- Prefer implying reports, bundles, paths, and snapshots through internal structure rather than folded-corner document clichés.
- Avoid generic filesystem or browser metaphors unless the action truly requires one.
- Keep them crisp and low-noise because they will often sit beside text labels.

### Shell Mode Glyph Family

- Mode glyphs should share one family language:
  - compact workspace tile
  - one focal structure
  - one supporting analytical cue
- `mode-build` should read as configuration or authoring.
- `mode-review` should read as inspection or analytical comparison.
- `mode-operate` should read as monitored runtime or execution posture.
- Avoid literal tool icons, trophy metaphors, or media-playback symbols as the dominant form.

### Empty-State Illustration Family

- Empty-state illustrations must stay restrained and schematic.
- Use no more than three conceptual layers:
  - main workspace frame
  - one focal placeholder or action target
  - one secondary supporting structure
- Avoid decorative scenery, floating particles, or atmospheric backgrounds.
- Avoid characters, hands, mascots, or “friendly helper” illustrations.
- If using an accent color, keep it to one small focal region.
- Preserve lots of negative space.
- The illustration should read as a quiet workstation placeholder, not a poster.

## Acceptance Checklist For Each Generated Asset

Before accepting an SVG from GPT-5.4 Pro, check:

- Does it read clearly at the intended size?
- Does it avoid `<text>` and external font dependencies?
- Does it match the restrained quant-workstation style rather than generic fintech visuals?
- Is it visually distinct from adjacent icons in the same family?
- Would it still look acceptable in monochrome if the UI remaps it later?
- If animated, does it still look good when frozen on a single frame?
- If animated, does it include a reduced-motion-safe fallback?

## Batch A: Highest-Priority SVGs

These are the most immediately useful assets for the current app:

- provenance/confidence glyphs
- compare/pin workflow glyphs
- task lifecycle glyphs

Batch-wide hardening rules:

- Every icon in this batch must be a compact, self-contained symbol with no detached framing chrome.
- Prefer a single dominant concept cluster rather than several unrelated shapes arranged like a dashboard screenshot.
- Do not use pale background highlights, list-row selection metaphors, or card containers unless the prompt explicitly asks for them.
- Keep the silhouette readable first and the internal detail second.

### 1. `provenance-oos-stable.svg`

```text
Create a production-ready SVG icon named `provenance-oos-stable.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- out-of-sample stable
- calm, reliable, balanced performance across in-sample and out-of-sample review

Visual direction:
- technical and restrained
- should feel trustworthy, not celebratory
- should suggest stability and alignment rather than growth hype

Preferred motif:
- two aligned parallel traces, bands, or evaluation lanes
- a subtle stabilizing cue such as a calm check-like resolution or balanced anchor point
- the mark should imply “consistency under a second look”

Avoid:
- trophy/checkmark badge tropes
- big arrows
- generic success starbursts

Styling:
- currentColor-friendly
- mostly stroke-driven or mixed stroke/fill
- clear silhouette at small size

Output:
- Two SVG files, one with color, one without
```

### 2. `provenance-low-sample.svg`

```text
Create a production-ready SVG icon named `provenance-low-sample.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- thin evidence / low sample count
- not total failure, but insufficient depth or confidence

Preferred motif:
- a sparse histogram, few data bars, or very small cluster of points
- the composition should visually imply “not enough observations”
- keep it calm and analytical

Avoid:
- warning triangle clichés
- skull / hazard language
- anything that reads as catastrophic failure

Styling:
- currentColor-friendly
- minimal and legible
- should pair well with other provenance icons in the same family

Output:
- Two SVG files, one with color, one without
```

### 3. `provenance-slippage-sensitive.svg`

```text
Create a production-ready SVG icon named `provenance-slippage-sensitive.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- performance is sensitive to execution friction / slippage

Preferred motif:
- a clean path or channel that is visibly offset or dragged by lateral friction
- a compact market/execution feel without looking like a chart screenshot
- the mark should read as “fragile under trading friction”

Avoid:
- literal dollar symbols
- speed lines
- dramatic crash visuals

Styling:
- currentColor-friendly
- one compact silhouette
- use the minimum number of shapes needed to communicate drag / sensitivity

Output:
- Two SVG files, one with color, one without
```

### 4. `provenance-regime-concentrated.svg`  ******

```text
Create a production-ready SVG icon named `provenance-regime-concentrated.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- result quality is concentrated in a narrow regime rather than broadly robust

Preferred motif:
- a compact regime-strip or segmented analytical lane with three to five regime segments
- one narrow segment or band should read as the dominant contributing regime
- the other segments should remain present but clearly secondary
- should suggest that only one market regime is carrying the result
- keep it analytical and understated
- the icon should read as regime segmentation, not as a generic histogram or list row

Avoid:
- spotlight metaphors
- target/bullseye clichés
- tall vertical axes
- oversized background highlight capsules
- generic dashboard bar-chart or menu-row silhouettes

Styling:
- currentColor-friendly
- slight asymmetry is okay if it clarifies the concept
- should remain readable at very small size

Composition constraints:
- keep the icon compact and self-contained
- do not place a detached left-side axis or ruler next to the regime segments
- do not use one large background pill behind the main segment
- prefer one unified symbolic structure over separate unrelated bars

Distinctness requirement:
- must remain visually distinct from `provenance-low-sample.svg`
- prioritize “multiple regimes with one dominant regime” over “too few observations”

Output:
- Two SVG files, one with color, one without
```

### 5. `provenance-validation-stale.svg`

```text
Create a production-ready SVG icon named `provenance-validation-stale.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- validation exists, but it is no longer current relative to the latest context

Preferred motif:
- a compact checkpoint, badge, or validation marker combined with subtle time drift / lag semantics
- should communicate “outdated” rather than “broken”

Avoid:
- clocks that dominate the whole icon
- alarm visuals
- red-alert language

Styling:
- currentColor-friendly
- quiet, operator-friendly
- should sit naturally in a row of provenance chips

Distinctness requirement:
- must remain visually distinct from `provenance-dataset-changed.svg`
- prioritize stale/lag semantics over source-reload semantics

Output:
- Two SVG files, one with color, one without
```

### 6. `provenance-dataset-changed.svg`

```text
Create a production-ready SVG icon named `provenance-dataset-changed.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- the dataset or source context changed since the reviewed result was produced

Preferred motif:
- stacked records, bars, or layers with one shifted/reloaded segment
- should suggest lineage or source drift
- avoid overly literal folder/database iconography

Avoid:
- big refresh arrows as the dominant form
- generic sync icon feel

Styling:
- currentColor-friendly
- compact and structured
- should look like a data-context icon, not a browser reload button

Distinctness requirement:
- must remain visually distinct from `provenance-validation-stale.svg`
- prioritize lineage/source-shift semantics over time-lag semantics

Output:
- Two SVG files, one with color, one without
```

### 7. `provenance-review-attention.svg`

```text
Create a production-ready SVG icon named `provenance-review-attention.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- this result deserves closer review
- attention-worthy, but not necessarily a warning or failure

Preferred motif:
- a focal frame, bracket, or centered analytical emphasis mark around one signal point or tile
- should read as “worth inspecting”

Avoid:
- warning triangles
- exclamation marks as the main form
- notification bell language

Styling:
- currentColor-friendly
- confident, crisp silhouette
- should feel review-oriented, not alarming

Output:
- Two SVG files, one with color, one without
```

### 8. `compare-pin-candidate.svg`

```text
Create a production-ready SVG icon named `compare-pin-candidate.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px to 18px
- use viewBox 0 0 24 24

Meaning:
- hold this result in the compare set

Preferred motif:
- not a generic office pushpin
- combine the idea of a pin/marker with a compact result tile or metric card
- should feel like “retain this candidate for research review”

Avoid:
- stationery pushpin clichés
- bookmark language
- map pin shapes that imply geographic location

Styling:
- currentColor-friendly
- compact and balanced
- should pair visually with compare and replace icons

Distinctness requirement:
- must remain visually distinct from `compare-candidates.svg`
- the pin/retain action cue must be readable without turning into a stationery pushpin

Output:
- Two SVG files, one with color, one without
```

### 9. `compare-candidates.svg`

```text
Create a production-ready SVG icon named `compare-candidates.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px to 18px
- use viewBox 0 0 24 24

Meaning:
- compare two or more shortlisted candidates side by side

Preferred motif:
- two aligned result cards, metric columns, or mirrored analysis tiles
- a shared axis or relationship line is welcome if subtle
- must clearly read as comparative review, not duplicate documents

Avoid:
- scales/justice metaphors
- generic compare arrows with no card structure

Styling:
- currentColor-friendly
- stable and symmetric
- high clarity at small size

Distinctness requirement:
- must remain visually distinct from `compare-replace-pinned.svg`
- prioritize simultaneous side-by-side review over substitution/exchange semantics

Output:
- Two SVG files, one with color, one without
```

### 10. `compare-replace-pinned.svg`

```text
Create a production-ready SVG icon named `compare-replace-pinned.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px to 18px
- use viewBox 0 0 24 24

Meaning:
- replace a currently pinned compare slot with the actively selected candidate

Preferred motif:
- two compact review tiles with a restrained exchange or swap direction
- the composition should communicate replacement inside a compare workflow, not generic file transfer

Avoid:
- heavy two-way arrows dominating the icon
- sync icon language

Styling:
- currentColor-friendly
- clear but low-noise
- should belong to the same family as compare and remove-from-compare

Distinctness requirement:
- must remain visually distinct from `compare-candidates.svg`
- substitution should be obvious, but the icon must not collapse into a generic sync symbol

Output:
- Two SVG files, one with color, one without
```

### 11. `compare-remove-from-set.svg`

```text
Create a production-ready SVG icon named `compare-remove-from-set.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px to 18px
- use viewBox 0 0 24 24

Meaning:
- remove one candidate from the compare tray

Preferred motif:
- a compare tile or paired-card structure with a quiet subtraction cue
- should feel reversible and controlled, not destructive

Avoid:
- trash can metaphors
- harsh delete icon language

Styling:
- currentColor-friendly
- should look like a compare action, not a global delete action

Output:
- Two SVG files, one with color, one without
```

### 12. `compare-candidate-focus.svg`

```text
Create a production-ready SVG icon named `compare-candidate-focus.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px to 18px
- use viewBox 0 0 24 24

Meaning:
- promote this candidate to the primary review focus

Preferred motif:
- one central highlighted review tile with surrounding muted peer tiles
- should communicate “make primary” inside an analysis set

Avoid:
- trophy / winner podium language
- star-badge language

Styling:
- currentColor-friendly
- centered and calm
- should fit naturally beside pin and compare icons

Output:
- Two SVG files, one with color, one without
```

### 13. `task-queued.svg`

```text
Create a production-ready SVG icon named `task-queued.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- background task is queued and waiting

Preferred motif:
- stacked items, staged tiles, or a queue lane with one pending item
- should feel orderly and technical

Avoid:
- pause/media icon dominance
- hourglass clichés

Styling:
- currentColor-friendly
- should work inside compact status chips and task lists

Distinctness requirement:
- must remain visually distinct from `task-cancelled.svg`
- queued should read as waiting in order, not already stopped

Output:
- Two SVG files, one with color, one without
```

### 14. `task-running.svg`

```text
Create a production-ready SVG icon named `task-running.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- active computation / running task

Preferred motif:
- segmented ring, staged progression, or compact computational pulse
- should feel like processing work, not media playback

Avoid:
- generic play triangle
- cliché spinner icon with no structure

Styling:
- currentColor-friendly
- crisp at small size
- must still read well as a static icon

Distinctness requirement:
- must remain visually distinct from `task-cancelling.svg`
- active computational motion should be implied without relying on animation

Output:
- Two SVG files, one with color, one without
```

### 15. `task-cancelling.svg`

```text
Create a production-ready SVG icon named `task-cancelling.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- a task is in the process of stopping, not fully stopped yet

Preferred motif:
- a running/progression structure interrupted by a controlled barrier or stop cue
- should read as transitional state

Avoid:
- hard failure language
- big prohibition circles

Styling:
- currentColor-friendly
- should look related to the running icon family

Distinctness requirement:
- must remain visually distinct from `task-running.svg` and `task-cancelled.svg`
- prioritise transitional interruption, not neutral stop or active progress

Output:
- Two SVG files, one with color, one without
```

### 16. `task-completed.svg`

```text
Create a production-ready SVG icon named `task-completed.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- background task completed successfully

Preferred motif:
- resolved progression ending in a calm completion mark
- should feel operationally done, not celebratory

Avoid:
- confetti / trophy semantics
- giant check-badge clichés

Styling:
- currentColor-friendly
- should pair naturally with running, failed, and cancelled

Distinctness requirement:
- must remain visually distinct from `task-completed.svg`
- completion should read as resolved work, not a generic success badge

Output:
- Two SVG files, one with color, one without
```

### 17. `task-failed.svg`

```text
Create a production-ready SVG icon named `task-failed.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- background task failed and needs attention

Preferred motif:
- broken or interrupted progression path
- should communicate failure clearly without becoming theatrical

Avoid:
- sirens
- explosions
- dramatic hazard visuals

Styling:
- currentColor-friendly
- compact and serious
- should still belong to the same family as the other task-state icons

Distinctness requirement:
- must remain visually distinct from `task-cancelled.svg`
- failure should read as broken/interrupted, not merely stopped

Output:
- Two SVG files, one with color, one without
```

### 18. `task-cancelled.svg`

```text
Create a production-ready SVG icon named `task-cancelled.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- task was intentionally stopped by the operator

Preferred motif:
- a halted progression with a neutral stop cue
- should feel deliberate, not erroneous

Avoid:
- failure semantics
- danger emphasis

Styling:
- currentColor-friendly
- should visually sit between queued/running and completed/failed

Distinctness requirement:
- must remain visually distinct from `task-failed.svg` and `task-queued.svg`
- cancellation should read as deliberate operator stop, not passive waiting or system failure

Output:
- Two SVG files, one with color, one without
```

### 19. `runtime-ready.svg`

```text
Create a production-ready SVG icon named `runtime-ready.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 14px to 18px
- use viewBox 0 0 24 24

Meaning:
- desktop runtime is ready and healthy

Preferred motif:
- a compact workstation/status panel or runtime surface with a stable indicator
- should feel like system readiness, not just generic success

Avoid:
- power button clichés
- cloud/server clichés

Styling:
- currentColor-friendly
- technical and operator-facing

Output:
- Two SVG files, one with color, one without
```

## Batch B: Artifact And Utility SVGs

Batch-wide hardening rules:

- These icons should read as analytical utility actions, not generic office documents or generic operating-system folder icons.
- Use internal structure to distinguish report, bundle, reveal, export, and reopen.
- Avoid solving the icon by adding a large arrow to an otherwise generic file/folder shape.
- Keep the icons compact and action-oriented rather than scenic.

### 20. `artifact-report-workbook.svg`

```text
Create a production-ready SVG icon named `artifact-report-workbook.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px
- use viewBox 0 0 24 24

Meaning:
- structured report workbook or analytical spreadsheet artifact

Preferred motif:
- a sheet/workbook with visible grid structure and one emphasized summary region
- should read as analytical output, not a generic document

Avoid:
- generic file-folded-corner icon with no table structure

Styling:
- currentColor-friendly
- compact and clean

Distinctness requirement:
- must remain visually distinct from `artifact-bundle.svg`
- prioritize workbook/grid structure over generic document silhouette

Output:
- Two SVG files, one with color, one without
```

### 21. `artifact-bundle.svg`

```text
Create a production-ready SVG icon named `artifact-bundle.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px
- use viewBox 0 0 24 24

Meaning:
- saved run artifact package / grouped output bundle

Preferred motif:
- layered result cards, compact archive block, or structured bundle stack
- should feel like a packaged analysis artifact, not a zip file icon

Avoid:
- cardboard box metaphors
- download package clichés

Styling:
- currentColor-friendly
- clearly distinguishable from report workbook

Distinctness requirement:
- must remain visually distinct from `artifact-report-workbook.svg`
- prioritize grouped artifact layers over single-sheet document structure

Output:
- Two SVG files, one with color, one without
```

### 22. `artifact-reveal-path.svg`

```text
Create a production-ready SVG icon named `artifact-reveal-path.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px
- use viewBox 0 0 24 24

Meaning:
- reveal artifact in the file system / open containing location

Preferred motif:
- a folder/path tray with subtle outward reveal semantics
- should communicate “show me where it lives”

Avoid:
- exaggerated arrow launch visuals
- generic open-folder clip-art feel

Styling:
- currentColor-friendly
- crisp and utility-oriented

Distinctness requirement:
- must remain visually distinct from `artifact-export-snapshot.svg`
- prioritize “show containing location” over “send outward” semantics

Output:
- Two SVG files, one with color, one without
```

### 23. `artifact-export-snapshot.svg`

```text
Create a production-ready SVG icon named `artifact-export-snapshot.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px
- use viewBox 0 0 24 24

Meaning:
- export the currently reviewed state as an artifact or snapshot

Preferred motif:
- result card or analytical panel leaving a bounded frame
- should feel like controlled export, not generic share/upload

Avoid:
- cloud upload metaphors
- social share icon language

Styling:
- currentColor-friendly
- consistent with other artifact icons

Distinctness requirement:
- must remain visually distinct from `artifact-reveal-path.svg`
- prioritize exporting a reviewed state over opening a location

Output:
- Two SVG files, one with color, one without
```

### 24. `artifact-reopen-report.svg`

```text
Create a production-ready SVG icon named `artifact-reopen-report.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px
- use viewBox 0 0 24 24

Meaning:
- reopen the latest generated report

Preferred motif:
- workbook/report structure with a subtle return/open cue
- should imply revisiting a previously generated artifact

Avoid:
- generic browser back arrows
- dramatic document motion

Styling:
- currentColor-friendly
- related to the report workbook icon but clearly distinct in purpose

Distinctness requirement:
- must remain visually distinct from `artifact-report-workbook.svg`
- prioritize revisiting/opening semantics over passive report identity

Output:
- Two SVG files, one with color, one without
```

## Batch C: Empty-State Illustrations

Batch-wide hardening rules:

- Each empty-state illustration should look like a sparse workstation placeholder, not a marketing scene.
- Keep the scene schematic and orthographic-feeling rather than cinematic or atmospheric.
- Do not add decorative particles, ambient gradients, floating mini-panels, or background scenery.
- Preserve negative space and let one focal placeholder explain the missing action.

### 25. `empty-results-workspace.svg`

```text
Create a production-ready SVG illustration named `empty-results-workspace.svg`.

Asset type:
- larger empty-state illustration
- static only
- intended rendered size: 72px to 128px
- use viewBox 0 0 96 96

Meaning:
- analysis/results area is empty because nothing has been run yet

Preferred motif:
- a sparse operator bench with a muted table region, one empty chart frame, and a small focal placeholder
- should imply “run something, then review it here”

Style:
- restrained
- low-detail
- monochrome or mostly monochrome with one soft accent
- no characters, mascots, or scenery

Composition constraints:
- one main workspace frame
- one empty chart or grid surface
- one focal placeholder only
- preserve substantial negative space
- do not add decorative background elements

Avoid:
- trophy podiums
- “start now” marketing illustration tropes
- decorative backgrounds

Output:
- Two SVG files, one with color, one without
```

### 26. `empty-compare-workspace.svg`

```text
Create a production-ready SVG illustration named `empty-compare-workspace.svg`.

Asset type:
- larger empty-state illustration
- static only
- intended rendered size: 72px to 128px
- use viewBox 0 0 96 96

Meaning:
- compare tray/workspace is waiting for pinned candidates

Preferred motif:
- two empty comparison cards or panels with a shared axis or connector
- should communicate “pin two or more candidates to compare them here”

Style:
- restrained
- analytical
- low-detail
- one soft accent allowed

Composition constraints:
- exactly two primary compare panels or cards
- one shared axis, connector, or comparison anchor
- one focal empty drop zone or target area
- no extra floating decorative cards beyond the main comparison structure

Avoid:
- versus-battle visuals
- aggressive arrows
- glowing comparison tropes

Output:
- Two SVG files, one with color, one without
```

### 27. `empty-artifact-library.svg`

```text
Create a production-ready SVG illustration named `empty-artifact-library.svg`.

Asset type:
- larger empty-state illustration
- static only
- intended rendered size: 72px to 128px
- use viewBox 0 0 96 96

Meaning:
- saved run library or artifact shelf is currently empty

Preferred motif:
- organized archive trays, shelves, or structured slots with one clearly empty focal slot
- should feel orderly and operational

Avoid:
- cardboard storage metaphors
- cluttered office visuals

Style:
- restrained
- minimal
- no characters
- no decorative scenery

Composition constraints:
- one archive frame or shelf structure
- two to four storage slots maximum
- one clearly empty focal slot
- keep the scene schematic rather than illustrative

Output:
- Two SVG files, one with color, one without
```

## Batch D: Optional Shell Mode Glyphs

Batch-wide hardening rules:

- These are mode glyphs, not app logos.
- Each mode should be represented by a compact workspace symbol with one clear posture difference.
- Avoid literal tools, trophies, rockets, browser tabs, or media controls as the main metaphor.
- The three mode glyphs should look like siblings when placed side by side.

These are lower-priority, but useful if we later reinforce build/review/operate shell posture.

### 28. `mode-build.svg`

```text
Create a production-ready SVG icon named `mode-build.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px to 18px
- use viewBox 0 0 24 24

Meaning:
- build mode / authoring posture
- strategy, data, and setup work

Preferred motif:
- construction grid or parameter tile system with one active element
- should feel like configuring a model or system, not using a hammer/wrench cliché

Avoid:
- literal tool belt / wrench / screwdriver tropes

Styling:
- currentColor-friendly
- should feel like a mode glyph, not a standalone app icon

Distinctness requirement:
- must remain visually distinct from `mode-review.svg` and `mode-operate.svg`
- prioritize configuration-grid posture over inspection or live-runtime posture

Output:
- Two SVG files, one with color, one without
```

### 29. `mode-review.svg`

```text
Create a production-ready SVG icon named `mode-review.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px to 18px
- use viewBox 0 0 24 24

Meaning:
- review mode / analytical interpretation posture

Preferred motif:
- reviewed result card paired with a compact chart or metric frame
- should communicate inspection and comparison

Avoid:
- magnifying glass clichés as the entire icon

Styling:
- currentColor-friendly
- should pair naturally with build and operate mode glyphs

Distinctness requirement:
- must remain visually distinct from `mode-build.svg` and `mode-operate.svg`
- prioritize inspection/review posture over construction or live operation

Output:
- Two SVG files, one with color, one without
```

### 30. `mode-operate.svg`

```text
Create a production-ready SVG icon named `mode-operate.svg`.

Asset type:
- small UI icon
- static only
- intended rendered size: 16px to 18px
- use viewBox 0 0 24 24

Meaning:
- operate mode / execution posture

Preferred motif:
- compact runtime console, execution panel, or live status trace
- should feel like monitored operation, not entertainment playback

Avoid:
- generic play button as the dominant form
- cockpit gimmicks

Styling:
- currentColor-friendly
- should complete the same family as build and review mode glyphs

Distinctness requirement:
- must remain visually distinct from `mode-build.svg` and `mode-review.svg`
- prioritize monitored runtime posture over setup or analysis posture

Output:
- Two SVG files, one with color, one without
```

## Optional Animated Variant Prompts

Batch-wide hardening rules:

- Animation must reinforce an already-clear static icon, never rescue a weak silhouette.
- Keep movement local to the symbol rather than moving the whole icon around the viewBox.
- Avoid easing curves that feel playful, bouncy, or decorative.
- The first frame and frozen-frame read should still pass the static acceptance checklist.

Only generate these after the static versions are good.

### A. `task-running.animated.svg`

```text
Create an animated SVG variant named `task-running.animated.svg` based on the static `task-running.svg` concept.

Requirements:
- keep the same core silhouette and meaning as the static running-task icon
- use the same viewBox: 0 0 24 24
- the icon must still read clearly if frozen on any frame
- animation should feel computational and calm, not flashy
- use a subtle segmented sweep, quiet pulse, or measured progression motion
- no fast spinning
- no flashing
- no bounce
- include prefers-reduced-motion handling that pauses or effectively removes the motion
- no JavaScript

Output:
- Two SVG files, one with color, one without
```

### B. `compare-candidates.animated.svg`

```text
Create an animated SVG variant named `compare-candidates.animated.svg` based on the static `compare-candidates.svg` concept.

Requirements:
- keep the same viewBox and same core geometry family as the static icon
- animation should be extremely subtle
- suggested motion: alternating emphasis between two aligned candidate tiles or a light comparative pulse across a shared axis
- no movement that suggests instability or competition drama
- no bouncing cards
- no flashing
- include prefers-reduced-motion handling
- no JavaScript

Output:
- Two SVG files, one with color, one without
```

### C. `empty-results-workspace.animated.svg`

```text
Create an animated SVG variant named `empty-results-workspace.animated.svg` based on the static `empty-results-workspace.svg` illustration.

Requirements:
- keep the same scene and visual hierarchy as the static illustration
- use the same viewBox: 0 0 96 96
- animation should be ambient and minimal
- suggested motion: a slow axis sweep, a faint opacity pulse on one focal placeholder, or a gentle reveal on one analytical segment
- no constant busy motion
- no flashing
- no large translation or scaling
- include prefers-reduced-motion handling
- no JavaScript

Output:
- Two SVG files, one with color, one without
```

## Recommended Generation Order

1. provenance icons
2. compare/pin icons
3. task lifecycle icons
4. artifact/report/reveal icons
5. empty-state illustrations
6. optional mode glyphs
7. animated variants only after the static assets are approved

## Optional Refinement Prompt Template

If the first SVG result is close but not good enough, use this follow-up prompt:

```text
Refine the SVG you just generated.

Keep the same core concept and file name, but make these corrections:
- reduce visual noise
- improve small-size legibility
- preserve the same restrained quant-workstation tone
- keep it clearly distinct from related icons in the same family
- keep the geometry centered and balanced
- do not add text
- do not add decorative background elements
- keep the output as Two SVG files, one with color, one without

Specific corrections:
- [paste the concrete issues here]
```

## Optional Quality-First Wrapper Line

If you want to make the iteration requirement even more explicit when prompting GPT-5.4 Pro, prepend this line before any asset-specific user prompt:

```text
Use a quality-first multi-iteration process internally: explore multiple candidate compositions, reject generic or weak ones, self-critique against the acceptance checklist, refine the geometry at least twice, and only then return the final raw SVG.
```
