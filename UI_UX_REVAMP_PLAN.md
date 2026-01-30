# UI / UX Revamp Plan (Tauri “dist/” UI)

## Context

The current Tauri UI is functional but looks “demo-like” (low visual hierarchy, limited interaction feedback, and a dark palette that feels heavy/flat). The UI is currently shipped as static files under `dist/` (no frontend build pipeline found), so the highest-ROI path is an incremental redesign using **CSS variables + small JS enhancements**, keeping existing IDs/events stable to avoid regressions.

## Goals (what “better” means)

1) **Modern, presentable look**: clearer hierarchy, better spacing, typography, and consistent component styling.
2) **Better interaction feedback**: hover/focus states, meaningful status/toasts, empty states, and progressive disclosure.
3) **Theme that feels good**: ship a **light default** (or at least a less-saturated dark) + optional dark; persist preference; optionally follow system.
4) **Accessibility & usability**: keyboard navigable tabs, visible focus rings, readable contrast, larger default text.
5) **No regressions**: keep existing workflows and “muscle memory” (button locations, IDs) working.

## Constraints / current architecture

- UI is in `dist/index.html`, `dist/app.css`, `dist/app.js`.
- Tauri backend is Rust; UI talks via `invoke(...)`.
- Keep changes “surgical”: prefer updating CSS + adding small DOM blocks rather than large rewrites.
- Cross-platform: avoid Windows-only features unless guarded/fallback exists.

## Research (design direction + implementation levers)

### Desktop design systems / visual language

- **Microsoft Fluent 2** design guidance (layout, typography, color, component patterns):  
  https://fluent2.microsoft.design/
- **Windows materials** (Mica, Acrylic) + when to use them:  
  https://learn.microsoft.com/en-us/windows/apps/design/style/mica  
  https://learn.microsoft.com/en-us/windows/apps/design/style/

### Practical “make it feel native” for Tauri (optional)

- **Mica / Acrylic / Blur via Rust**: `window-vibrancy` crate (Windows/macOS).  
  https://crates.io/crates/window-vibrancy  
  (If we adopt this: only enhance the window background; still keep a good CSS theme for other OSes.)

### Color tokens / palettes

- **Radix Colors** (high-quality color scales that work well for UI tokens):  
  https://www.radix-ui.com/colors
- **Radix Themes** (design system built on tokens; even if we don’t adopt the React layer, the token approach is useful):  
  https://www.radix-ui.com/themes

### Interaction patterns & accessibility

- **WAI-ARIA Tabs pattern** (correct keyboard + ARIA behavior for tabs):  
  https://www.w3.org/WAI/ARIA/apg/patterns/tabs/

### Dark mode reality check (why light default may be better)

- Readability often improves with dark text on light backgrounds (esp. for small fonts):  
  https://www.wired.com/story/do-you-need-dark-mode

## Recommended approach (highest ROI / lowest risk)

**Keep the current vanilla UI**, but add a small “design system” layer:

1) Introduce a **tokenized theme** (CSS variables) with:
   - `data-theme="light"` and `data-theme="dark"` on `<html>` or `<body>`
   - `color-scheme: light|dark` per theme
   - semantic variables (`--bg`, `--surface`, `--surface2`, `--text`, `--muted`, `--border`, `--accent`, `--danger`, `--ok`, etc.)
2) Improve core components:
   - Buttons (primary/secondary/ghost/danger), disabled styling, focus rings, hover/active.
   - Inputs/selects: higher contrast, better padding, clearer focus.
   - Tabs: modern segmented style + active indicator; keyboard support.
   - Cards: less “samey” backgrounds; clearer headers; consistent spacing scale.
3) Add “micro-interactions”:
   - Toast notifications (non-blocking) and inline status chip.
   - Loading states: subtle progress + skeletons for big tables.
   - Empty states: short guidance + link to action (“Load CSV”, “Cache in Polygon”, etc.).
4) Optional: enable Windows 11 “Mica” background (guarded) once the CSS theme is solid.

## Theme direction (proposal)

### Default: Light theme

Rationale: better readability for dense controls + tables, better “professional desktop tool” feel, reduces the “washed-out” look from low-contrast dark surfaces.

Suggested palette (example targets, not final):

- Background: near-neutral (e.g. slate/gray 50–100)
- Surfaces: white + subtle elevation
- Text: near-black (not pure)
- Accent: one saturated hue (blue/cyan) used sparingly for primary actions + selected states

### Dark theme (optional)

If we keep dark mode:

- Avoid ultra-saturated backgrounds; use neutral dark grays.
- Avoid pure white text; use off-white.
- Use elevation (overlays/borders) to preserve hierarchy.

## Interactivity improvements (concrete ideas)

### Navigation + structure

- Make left “Controls” column resizable (drag handle) and/or collapsible.
- Add “Quick actions” panel (Load, Run, Validate, Export) with keyboard shortcuts shown.
- Reduce “wall of empty space” on the right by adding curated empty states and summaries.

### Feedback

- Replace `Status: Ready.` with a more obvious status bar:
  - last action, elapsed time, backend mode (CPU/GPU), dataset id + timeframe.
- Toasts for: saved artifact, exported report, validation done, batch started/stopped.

### Data viz polish

- Improve tables: sticky headers, zebra rows, compact density toggle, column alignment.
- Better charts/heatmaps styling: consistent color ramps and legend components.

### Accessibility / keyboard UX

- Implement proper tab semantics (ARIA + keyboard arrows/home/end).
- Ensure all interactive elements have visible focus (not just hover).
- Support `Ctrl+K` command palette (optional phase 3).

## Implementation plan (phased)

### Phase 0 — Baseline + guardrails (0.5–1 day)

- Take “before” screenshots for each tab.
- Add a UI regression checklist (manual) for critical flows:
  - Load CSV, run optimization, view top table, validation, export report, Polygon download/load, batch run, meta selection.

### Phase 1 — Theme + tokens (1–2 days)

- Refactor `dist/app.css` into token layers:
  - base tokens, light theme, dark theme.
- Add Theme toggle in header:
  - `Light / Dark / System`
  - persist in `localStorage`
  - apply at startup before render (avoid flash)

### Phase 2 — Component polish (2–4 days)

- Buttons, inputs, selects, badges, cards.
- Tab bars (Controls + Results): better active state + spacing + hover + focus.
- Improve typography: base font size, line heights, headings, mono usage.

### Phase 3 — Interaction upgrades (2–5 days)

- Toast system + status bar redesign.
- Empty states + skeleton loading.
- Resizable left panel (drag handle).

### Phase 4 — Optional “native” desktop feel (1–3 days)

- Add Windows/macOS vibrancy behind the webview where supported (`window-vibrancy`).
- Keep CSS themes as primary; vibrancy is a bonus.

## Success criteria / acceptance checks

- App looks good in both light + dark; light is default (unless system says otherwise).
- No broken IDs/events; main workflows still work.
- Contrast + focus rings are clearly visible.
- Tabs are keyboard navigable.
- New UI elements (theme toggle, toasts) do not block workflows.

## Next action (if approved)

Implement Phase 1 first (theme system + light default) with minimal markup changes, then iterate on component polish.

