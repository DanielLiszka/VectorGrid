# Manual Accessibility Review Matrix

Representative manual-accessibility slices for the non-options app. This matrix augments `npm --prefix web run a11y:smoke`; it does not replace the automated smoke gate.

## Fast Gate First

Run these before manual review:

```powershell
npm --prefix web run a11y:smoke
npm --prefix web run test:browser:ux
```

Add desktop smoke when the touched change includes native-shell or OS-integrated behavior:

```powershell
npm --prefix web run test:desktop:smoke
```

## Representative Review Slices

| Slice | Representative surfaces | Keyboard-only task | Manual review points | Browser enough when... | Desktop also required when... |
| --- | --- | --- | --- | --- | --- |
| Shell orientation | `WorkbenchShell`, `DesktopTitlebar`, `RunFooterPanel`, Settings entry points | Move across workspace tabs and workflow-step tabs, open Settings, return to Workflow without losing orientation | visible focus, tab order, selected-state semantics, posture/status copy, icon-button labels | change is limited to shell layout, copy, tab semantics, or mocked-Tauri shell flows | titlebar, native window controls, geometry, desktop-presence diagnostics, or window-state copy changes |
| Palette and overlays | command palette, task center, confirmation dialogs, context help | Open from keyboard, navigate choices, close with `Escape`, confirm focus return to the launcher/trigger | focus trap, focus return, combobox/listbox semantics, close labels, help/disclosure copy | change is limited to overlay semantics and mocked browser flows | overlay trigger lives in native titlebar or the flow depends on desktop notifications / close guards |
| Dense result surfaces | Overview, Top, Validation, Meta, history table views with representative saved or seeded data | Reach the dense table, move through primary controls, toggle columns if present, confirm the primary status/help text remains understandable without a mouse | table label, column-toggle naming, sticky-header readability, spacer rows hidden from AT, summary/status clarity, drilldown affordance naming | change is limited to browser-rendered table semantics, virtualization, summaries, or dense-panel copy | the change depends on native file dialogs, reveal/open path actions, or desktop-only window behavior |
| Validation and history reopen | validation summary/report entry, history library, saved-artifact reopen/reveal affordances | Reopen a representative saved run or report entry, confirm focus lands in a useful place, and complete the primary follow-up action | action naming, status text, reopen orientation, local-vs-global search clarity, report/open/reveal wording | reopen stays inside the browser shell with mocked file/path behavior | open/reveal path, OS dialogs, saved-file handoff, or native desktop diagnostics change |
| ML and live status | ML Signal workspace, execution/live status surfaces, runtime banner | Trigger the primary keyboard path, review status copy, and confirm the next action remains clear after status changes | live-region clarity, keyboard reachability of primary actions, error/success wording, dense status readability | review is limited to browser-rendered status text and control labels | desktop notifications, runtime log bridge, broker presence, or other native-desktop status surfaces change |

## How Much Review To Run

- small shell-copy or semantic tweak:
  - `a11y:smoke`
  - `test:browser:ux`
  - the one matching manual slice above
- dense results / validation / history UI change:
  - `a11y:smoke`
  - `test:browser:ux`
  - `Dense result surfaces`
  - `Validation and history reopen`
- native-shell or release-facing desktop UI change:
  - `a11y:smoke`
  - `test:browser:ux`
  - `test:desktop:smoke`
  - the matching desktop-required slice above

## Review Log Format

Save completed review notes under `output/manual_a11y_review/<YYYY-MM-DD>_<slice>.md`.

Use this template:

```md
# Manual Accessibility Review

- Date:
- Reviewer:
- Change / branch:
- Slice(s):
- Environment:
  - Browser or desktop:
  - Node:
  - OS:
- Automated gates:
  - `npm --prefix web run a11y:smoke`
  - `npm --prefix web run test:browser:ux`
  - `npm --prefix web run test:desktop:smoke` (if used)
- Representative screen / dataset / artifact:
- Keyboard-only task path:
- Accessible-name / live-region checks:
- Dense-surface readability checks:
- Findings:
- Follow-up required:
```

## Notes

- Use representative saved artifacts or seeded sample data for dense tables; empty states alone are not enough when the change touches result-heavy screens.
- Browser review is the default for non-native UI changes because it is faster and more deterministic.
- Desktop review is required when the user-facing behavior crosses into native titlebar, window geometry, OS notification, file-open/reveal, or other desktop-only flows.
