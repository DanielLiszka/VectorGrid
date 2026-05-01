# Frontend QA Gate Wiring Matrix

Date: 2026-04-30

## Goal

Map frontend feature expansion to exact existing scripts and fixture owners so new UI work does not rely on vague "run frontend tests" guidance.

## Current Script Surface

From `web/package.json`:

| Script | Role |
|---|---|
| `npm --prefix web run typecheck` | Vue/TypeScript app typecheck. |
| `npm --prefix web run typecheck:desktop` | WDIO/desktop harness TypeScript check. |
| `npm --prefix web run lint` | ESLint plus command/binding/known-command checks. |
| `npm --prefix web run parity` | Frontend/backend Tauri command parity. |
| `npm --prefix web run test` | Vitest unit/component tests. |
| `npm --prefix web run build` | Clean dist, Vue type build, Vite build, release dist check. |
| `npm --prefix web run check:bundle-budget` | JS/CSS bundle budget. |
| `npm --prefix web run perf:ui:budget` | UI performance budget check. |
| `npm --prefix web run check:qa-static` | Action coverage, selector inventory, artifact fixtures, Tauri capabilities. |
| `npm --prefix web run test:e2e:qa:built` | Built workbench smoke, action coverage, workflow core. |
| `npm --prefix web run test:e2e:workflow:full:built` | Full built Playwright browser workflow suite. |
| `npm --prefix web run test:e2e:responsive:built` | Responsive workflow and layout regression specs. |
| `npm --prefix web run test:e2e:regressions:built` | Browser regression repro specs. |
| `npm --prefix web run test:e2e:visual:built` | Visual baseline specs. |
| `npm --prefix web run a11y:smoke:built` | Built browser axe smoke. |
| `npm --prefix web run a11y:smoke:wcag:built` | Stronger WCAG-oriented axe smoke. |
| `npm --prefix web run qa:dashboard` | Local frontend QA dashboard. |
| `npm --prefix web run test:desktop:smoke` | WDIO Tauri desktop smoke. |
| `npm --prefix web run test:desktop:geometry` | WDIO desktop geometry suite. |

Repo wrappers:

- `powershell -File scripts/run_frontend_e2e_gate.ps1`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeBrowserWorkflowQa`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeBrowserResponsiveE2E`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeBrowserRegressionE2E`
- `powershell -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopSmoke`

## Feature-To-Gate Matrix

| Feature type | Minimum targeted checks | Broader gate before handoff |
|---|---|---|
| Simple Mode first-run | `GuidedWorkspacePanel` tests, `simpleSummaryService` tests, `test:e2e:qa:built`, `a11y:smoke:built` | `scripts/run_frontend_e2e_gate.ps1`; public release gate if resources changed. |
| Workbench navigation/actions | command/action store tests, `check:qa-static`, `test:e2e:action-coverage:built` | `test:e2e:qa:built`. |
| Virtualized table behavior | component tests, virtualization tests, keyboard/focus tests | `test:e2e:workflow:full:built`; `a11y:smoke:wcag:built` for semantics changes. |
| Chart/visualization changes | visualization model tests, component tests, `check:bundle-budget` | `test:e2e:visual:built`; `perf:ui:budget`. |
| Responsive layout | component tests for changed surface | `test:e2e:responsive:built`. |
| Regression fix | focused test reproducing issue | `test:e2e:regressions:built`. |
| Settings/privacy/security UI | settings store/service tests, redaction tests, `security:desktop-boundary` | `a11y:smoke:wcag:built`; release doctor if release-facing. |
| Options panel | options service/store/component tests | `test:e2e:workflow:full:built`; output parity if export/report shape changes. |
| Execution workspace | execution service/store/machine tests | `powershell -File scripts/run_execution_safety_gate.ps1`. |
| Desktop shell/titlebar/window geometry | desktop harness typecheck, component tests | `test:desktop:geometry`; `test:desktop:smoke` where available. |

## Fixture Ownership

Preferred fixture classes:

- empty app;
- sample dataset loaded;
- optimization result populated;
- validation result populated;
- artifact history with available report;
- artifact history with missing report;
- options artifact/result;
- ML Signal artifact/result;
- portfolio feasibility result;
- execution dry-run snapshot;
- feed stale/interrupted state.

Fixture files should live with existing E2E fixture infrastructure under `web/e2e/browser/qa/` or the matching `web/src/test/` helper area. Do not create ad hoc fixture JSON inside component tests when a shared fixture would improve workflow coverage.

## New Spec Candidates

### Simple Mode Happy Path

Potential new file:

- `web/e2e/browser/qa/simpleModeHappyPath.spec.ts`

Coverage:

- start empty;
- load packaged sample via mocked command;
- run CPU-safe optimization;
- copy summary;
- optional validation recommendation;
- switch to Advanced without losing context.

### Populated Visualization Smoke

Potential new file:

- `web/e2e/browser/qa/populatedVisualization.spec.ts`

Coverage:

- top table populated;
- validation table populated;
- price chart markers rendered;
- metric distribution chart rendered;
- no text overlap at desktop and mobile-ish widths.

### Settings Privacy Smoke

Potential new file:

- `web/e2e/browser/qa/settingsPrivacy.spec.ts`

Coverage:

- local data inventory panel;
- redacted support bundle dry run;
- cache inventory dry run;
- keyring credential presence display without revealing value.

## Browser Mode Unit Test Candidate

Vitest Browser Mode is a candidate only for layout/focus behavior that jsdom cannot represent well.

Good targets:

- command palette focus trap;
- confirmation dialogs;
- virtual table keyboard behavior;
- anchored overlays;
- dense toolbar overflow;
- chart host resize behavior.

Rules:

- keep Browser Mode opt-in and small;
- do not migrate ordinary pure model tests;
- require a flake/quarantine policy before adding many browser-mode tests.

## Visual Snapshot Policy

Use Playwright visual snapshots sparingly:

- Simple Mode first-run;
- Advanced shell default;
- populated validation table;
- settings/privacy panel;
- one chart/visualization surface.

Risk controls:

- pin viewport and theme;
- hide clock/timestamps where possible;
- keep snapshots deterministic;
- avoid GPU/font-sensitive broad pages.

## Accessibility QA Policy

Automated:

- `a11y:smoke:built`;
- `a11y:smoke:wcag:built` for high-risk surfaces;
- `a11y:smoke:contrast:built` after palette/contrast changes.

Manual matrix:

- Simple Mode first-run;
- validation table;
- history actions;
- execution emergency controls;
- settings/privacy;
- command palette;
- options finalist dashboard.

Manual evidence should record:

- keyboard-only path;
- focus visibility;
- accessible names for icon buttons;
- live-region behavior;
- screen-reader summary availability for charts/tables.

## Acceptance Criteria

- Every frontend expansion names a minimum targeted script set.
- New E2E fixtures have owners and fixture validation.
- Visual tests remain small and deterministic.
- Desktop smoke is used only when the local machine has the required Tauri/WebDriver setup.
- Browser-only coverage is not used as backend correctness evidence.

