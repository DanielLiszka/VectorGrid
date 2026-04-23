# UI/UX Validation Strategy — Final

Date: 2026-04-08

Scope: final repo direction for proving that UI/UX modernization improves the VectorGrid frontend without regressing dense-workflow speed, keyboard access, accessibility, or desktop-shell reliability.

Status: final consolidated direction for Prompt 10.

## 1. Executive summary

The repo already has a credible verification base:

- frontend baseline gates through `lint`, `typecheck`, `test`, `build`, and bundle-budget/parity commands
- a browser accessibility smoke script in `web/scripts/check-a11y-smoke.mjs`
- a browser baseline capture and coarse perf artifact script in `web/scripts/capture-baseline-ui.mjs`
- a Windows-first real-shell desktop smoke harness through `wdio.desktop.conf.ts` and `web/e2e/desktop/`

The missing piece is not another broad layer. It is one narrow layer of explicit browser interaction evidence between component tests and the current desktop smoke harness.

Final direction:

- keep component/unit tests as the main proof of keyboard, focus, and interaction contracts
- keep `a11y:smoke` as the broad shell-level accessibility scan
- add one very small Playwright Test browser interaction suite on mocked Tauri
- keep desktop smoke thin and native-specific
- keep baseline UI capture as review/perf evidence, not a universal red/green screenshot gate
- require a short changed-surface manual pass for focus visibility, focus return, and contrast on UI-heavy work

This is the smallest sustained stack that can catch real UX regressions without turning repo verification into snapshot maintenance.

## 2. Final UX validation strategy

The validation strategy for UI-heavy work should follow these rules:

1. Each layer must prove something unique.
   If two layers are asserting the same behavior at the same depth, one of them should be trimmed.

2. Browser UX verification and real desktop-shell verification stay separate.
   Browser tests should prove interaction semantics in deterministic mocked-Tauri mode. Desktop tests should prove only what requires the real Tauri shell.

3. User-visible behavior matters more than implementation detail.
   The most important regressions to catch are broken keyboard paths, lost focus return, hidden capability reachability, shell-readiness failures, and native-shell breakage.

4. Visual evidence should be broad but low-friction.
   The baseline capture should remain an artifact and review tool. It should not become a brittle pixel-golden matrix.

5. Accessibility evidence must be layered.
   Automated a11y scanning stays valuable, but it is not enough by itself for dense workstation behavior.

6. The default green path must stay small.
   UI verification only stays useful if contributors keep trusting and running it.

Repo baseline commands such as `lint`, `typecheck`, `test`, `build`, and `check:bundle-budget` remain required for frontend changes, but they are prerequisites, not sufficient UX evidence by themselves.

## 3. Test-layer responsibilities

### 3.1 Component / unit

Component and unit tests remain the primary proof layer for exact interaction contracts and state-driven rendering.

They should cover:

- keyboard behavior inside custom widgets or composite UI
- focus movement and focus return after overlays, dialogs, disclosures, and menus close
- command-palette filtering and selection logic
- dense-row selection semantics and selected-summary coupling
- stale/provenance rendering rules
- diagnostics filtering, copy, and clear behavior
- any store or composable logic that directly changes visible UI state

This is the cheapest layer for validating dense-workstation behavior because it avoids native-shell noise while still proving authored interaction logic.

Representative targets include:

- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/CommandPaletteOverlay.vue`
- `web/src/components/TaskCenterOverlay.vue`
- `web/src/components/TopWorkspacePanel.vue`
- `web/src/components/SettingsWorkspacePanel.vue`
- `web/src/components/ContextActionMenu.vue`
- related stores and composables that own selection, focus, and overlay state

### 3.2 Browser smoke

Browser smoke should stay split into two responsibilities.

#### A. Existing broad accessibility smoke

Keep `web/scripts/check-a11y-smoke.mjs` focused on a shell-level accessibility pass over the built app in mocked-Tauri mode.

Its current role is already correct:

- start the built `dist`
- install mocked Tauri bindings
- scan startup overlay
- scan the workflow shell
- scan settings workspace
- scan command palette
- scan task center overlay

Do not turn this script into a monolithic interaction runner.

#### B. New small browser interaction smoke suite

Add one small Playwright Test suite dedicated to explicit keyboard and focus journeys in mocked-Tauri mode.

This suite should stay intentionally tiny:

- default target: 3 to 5 specs
- deterministic fixtures only
- no external services
- no native-shell concerns
- no broad pixel-golden coverage

This layer should prove:

- keyboard-safe workspace/workflow navigation
- command-palette open, filter, move, execute, close, and focus restoration
- task-center open/close and focus restoration
- one representative dense-surface interaction path when the changed work touches forms or results

Optional browser-level assertions are allowed only in narrow form:

- up to 2 very stable ARIA snapshots for shell scaffolding or overlay structure
- at most 1 very stable screenshot assertion for a shell skeleton or startup-ready state

### 3.3 Desktop smoke

Desktop smoke should remain a thin native-shell proof layer.

The current WDIO + `tauri-driver` harness is already shaped correctly for that job. It should continue to prove:

- real launch into the Tauri shell
- shell readiness after launch
- bootstrap recovery when the desktop context initially lands on `about:blank`
- titlebar/native window-control presence
- workspace navigation in the real shell
- command-palette reachability in the real shell
- task-center availability in the real shell
- settings diagnostics visibility in the real shell

Desktop smoke should not absorb general browser UX coverage.

Keep out of the default desktop green path:

- broad form/results interaction logic
- visual snapshot gating
- live data flows
- broker-dependent flows
- deep tray/updater/native-surface expansion
- heavy deterministic seeding until a specific real-shell case truly needs it

### 3.4 Visual baseline capture

`web/scripts/capture-baseline-ui.mjs` should remain the broad review/perf artifact layer.

Its current role is valuable because it already:

- captures stable screenshots across workspaces and tabs
- records coarse metrics such as cold start, time to interactive, idle heap, and synthetic live-feed responsiveness
- gives designers and implementers a quick evidence bundle after UI changes

Keep it non-blocking by default.

Use it to:

- regenerate evidence for UI-heavy changes
- review broad surface changes quickly
- watch for obvious startup or shell-performance drift

Do not turn it into a universal screenshot-golden gate.

### Changed-surface manual review

For UI-heavy work, keep one short human review pass on the changed surfaces.

Minimum manual checks:

- focus is visible
- focus returns to the expected control after overlays/dialogs close
- there are no keyboard traps
- contrast remains acceptable on changed surfaces
- primary and secondary actions remain discoverable

This is especially important because the current automated a11y smoke intentionally does not carry the whole accessibility burden.

## 4. Canonical user workflows to protect

Treat the following as the canonical UX evidence set.

### 1. Launch to ready shell

Prove that startup reaches an interactive shell cleanly.

Evidence sources:

- existing browser accessibility smoke
- browser interaction smoke
- desktop smoke

### 2. Keyboard shell navigation

Prove that a keyboard user can move across the main shell structure without losing orientation or entering traps.

Evidence sources:

- component tests for tab/focus logic
- browser interaction smoke

### 3. Command palette path

Prove the full command-palette flow:

- open
- filter
- move through results
- execute a safe navigation command
- close
- restore focus cleanly

Evidence sources:

- component tests for palette logic
- browser interaction smoke
- a narrow desktop smoke assertion for reachability only

### 4. Representative dense workflow interaction

Protect one representative dense interaction path for any UI-heavy modernization slice.

Possible variants:

- authoring variant: disclosure, grouped controls, and keyboard-safe secondary actions
- results variant: shortlist selection, summary/provenance visibility, and secondary-action reachability

Rule:

- if the change touches forms, protect one authoring path
- if the change touches analysis/results, protect one results path
- if the change touches both, protect one of each in component/browser layers

### 5. Desktop operator path

Protect the most valuable native-shell operator path:

- open Task Center
- move into Settings diagnostics
- confirm real-shell availability of titlebar/native controls and desktop-facing diagnostics surfaces

Evidence sources:

- desktop smoke
- targeted component tests for non-native UI logic behind those surfaces

## 5. Required scripts and file touch points

### Existing required surfaces

- `web/package.json`
  - keep existing frontend verification commands
  - add one browser interaction command, e.g. `test:browser:ux`
- `web/scripts/check-a11y-smoke.mjs`
  - keep as broad shell-level a11y smoke only
- `web/scripts/capture-baseline-ui.mjs`
  - keep as artifact/perf capture only
- `web/wdio.desktop.conf.ts`
  - keep as the real-shell smoke owner
- `web/e2e/desktop/README.md`
  - document the browser-vs-desktop boundary explicitly

### New recommended browser UX surfaces

Add a small Playwright Test layer, for example:

- `web/playwright.config.ts`
- `web/e2e/browser/shell-ux.spec.ts`
- `web/e2e/browser/command-palette-overlays.spec.ts`
- optional: `web/e2e/browser/dense-surface-ux.spec.ts`
- optional shared helper:
  - `web/e2e/browser/support/mockTauri.ts`

The mocked-Tauri bootstrap currently duplicated across the existing browser scripts is a good candidate for extraction into shared support code.

### Component-test touch points to expand when needed

Prefer targeted additions near the changed surfaces rather than a new test framework layer.

High-value examples:

- `VueWorkspaceShell` tests for keyboard movement and active-state exposure
- `CommandPaletteOverlay` tests for filtering, active item movement, execution, and focus return
- `TaskCenterOverlay` tests for dialog/overlay focus return
- `TopWorkspacePanel` tests for selection and provenance/stale rendering
- `SettingsWorkspacePanel` tests for diagnostics filtering/copy/clear interaction
- `ContextActionMenu` tests for menu keyboard behavior and focus restoration

### Implementation order

1. Add the Playwright runner/config and one new npm command.
2. Add the first 2 browser UX specs:
   - shell navigation
   - command palette + overlay focus return
3. Add one representative dense-surface browser spec only if the current modernization slice touches forms or results.
4. Extract shared mocked-Tauri boot helpers from the current browser scripts if duplication becomes annoying.
5. Keep the desktop harness unchanged except for boundary documentation unless a real native-only gap appears.
6. Expand deterministic seed mode only when one concrete scenario cannot be covered reliably otherwise.

## 6. Essential versus optional evidence

### Essential

- existing frontend baseline gates:
  - `lint`
  - `typecheck`
  - `test`
  - `build`
  - `check:bundle-budget`
- existing `a11y:smoke`
- targeted component/unit coverage for any new custom keyboard, focus, or overlay behavior
- new small browser interaction smoke suite in mocked-Tauri mode
- existing real-shell desktop smoke suite
- regenerated `baseline:ui` artifacts for UI-heavy changes
- short changed-surface manual review for keyboard/focus/contrast/discoverability

### Optional

- 1 to 2 stable ARIA snapshots for shell or overlay scaffolding
- 1 stable screenshot assertion for a shell skeleton or startup-ready state
- narrow deterministic seed fixtures for richer browser scenarios
- narrow deterministic desktop seeding only when a native-specific scenario requires it
- additional dense-surface browser specs for larger redesign slices

### Not required for every UI change

- adding new desktop smoke coverage
- adding new snapshots
- extending the visual baseline matrix
- adding new performance thresholds beyond the current coarse baseline capture

## 7. Maintenance-cost guidance

This strategy stays healthy only if it remains deliberately small.

### Keep the default green path short

- browser interaction suite should stay around 3 to 5 default specs
- desktop smoke default suite should remain shell-focused and stable
- optional geometry or richer desktop cases should stay opt-in until they are truly stable

### Give every layer a single job

- component tests prove exact interaction contracts
- browser smoke proves app-level UX behavior in deterministic mocked mode
- desktop smoke proves real-shell integrity
- baseline capture produces artifacts and coarse perf evidence

If a new test does not clearly fit one of those jobs, it probably does not belong.

### Keep snapshot budgets strict

- ARIA snapshots: only for very stable structure
- screenshot assertions: only for very stable shell views
- baseline screenshots: broad but non-blocking

Snapshot updates should be unusual. If contributors are updating many snapshots as routine maintenance, the strategy has drifted too far.

### Prefer changed-surface evidence over blanket expansion

When a UI modernization slice lands, update evidence only where the product actually changed:

- shell work updates shell/browser evidence
- dense form work updates one authoring path
- results work updates one triage path
- desktop-native work updates the desktop layer

Do not expand every layer on every change.

### Keep native complexity on a leash

Do not add broad desktop seeding, more native drivers, or bigger real-shell matrices unless a real regression class cannot be caught with the lighter stack.

## 8. Explicit overkill to avoid

Avoid the following.

### 1. Pixel-golden coverage for every workspace, step, and tab

That would turn the repo into screenshot maintenance instead of UX verification.

### 2. Turning `baseline:ui` into a universal blocking gate

The current capture script is strongest as a review and perf artifact generator.

### 3. Replaying the same workflow in full at component, browser, and desktop depth

This increases cost without adding much evidence.

### 4. Expanding desktop smoke into general UI logic

The desktop layer should not become the primary proof for forms, results triage, or most overlay behavior.

### 5. Depending on live services or broker flows for default UX confidence

UI modernization verification should remain deterministic.

### 6. Treating automated accessibility scans as complete accessibility proof

The repo should keep automated scanning, but not pretend that scan output alone proves keyboard quality, focus quality, or discoverability.

### 7. Adding more tooling before the current missing layer exists

The next justified improvement is the tiny browser interaction suite, not another broad framework decision.

## Final direction

The sustained repo strategy is:

- keep the current baseline gates
- keep the current shell-level accessibility smoke
- add one very small mocked-Tauri Playwright interaction suite
- keep the current WDIO real-shell smoke harness thin and native-specific
- keep broad screenshot/perf capture as non-blocking review evidence
- require a short manual pass on changed UI surfaces

That is the smallest credible evidence stack for a larger shell/forms/results modernization pass in this repo.
