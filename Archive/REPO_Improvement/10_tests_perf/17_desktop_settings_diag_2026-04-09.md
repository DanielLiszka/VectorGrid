# Desktop Settings Diagnostics Plan

Last updated: 2026-04-10

## Purpose

Define the desktop-smoke contract for the settings workspace diagnostics that only make sense in the real desktop runtime.

## Scope Boundaries

In scope:

- the `desktop-settings-diagnostics.spec.ts` smoke contract
- desktop notifications, logs, task-center controls, and window-state/status text shown in settings
- the settings workspace diagnostics area as rendered in the real shell
- cross-checks between settings diagnostics and desktop runtime/plugin availability

Out of scope:

- general settings form behavior already covered elsewhere
- options-related settings surfaces
- deep runtime-store correctness beyond the user-visible diagnostics summary
- native notification delivery benchmarking

## Current User-Facing Behavior

The current desktop settings diagnostics smoke spec opens the settings workspace in the real shell and checks for a cluster of desktop-specific diagnostics and controls, including notification posture, logs, task-center affordances, close/minimize messaging, desktop presence copy, and window-state detail. This makes the settings workspace an important real-shell readout of desktop capabilities and runtime posture that mocked browser tests cannot fully validate.

## Current Code Surface

- `web/e2e/desktop/specs/desktop-settings-diagnostics.spec.ts` contains the current diagnostics smoke journey.
- `web/src/components/SettingsWorkspacePanel.vue` renders the desktop diagnostics grid and supporting copy.
- `web/src/services/desktopProductization.ts` provides desktop-notification and native-runtime helpers surfaced in settings.
- `web/src/stores/runtimeStore.ts` and `web/src/stores/settingsStore.ts` feed the diagnostics surface with runtime and persistence state.

## Current Verification Surface

- `npm --prefix web run test:desktop:smoke:spec -- .\e2e\desktop\specs\desktop-settings-diagnostics.spec.ts`
- the default desktop smoke suite
- targeted component/store tests when a diagnostics failure is rooted in frontend state rather than native-shell behavior

## Strengths Already Present

- the spec protects a distinctly desktop-facing surface rather than duplicating browser-only coverage
- the settings diagnostics area aggregates multiple important runtime cues in one visible panel
- the current assertions already look for meaningful desktop concepts rather than only the existence of the settings grid
- the surface is useful for user trust because it explains what the desktop runtime is doing

## Weaknesses / Failure Modes

- the current assertions are still somewhat copy-heavy and could get noisy if phrasing evolves
- desktop diagnostics combine several plugins and runtime signals, which can make failures multi-causal
- browser harnesses cannot fully replicate this surface, so desktop smoke must stay disciplined and stable
- the settings panel is large, so a narrow diagnostics contract is necessary to avoid overcoverage

## Regressions To Avoid

- removing desktop-specific posture cues from the smoke contract and leaving only generic settings-grid checks
- asserting on every line of explanatory copy rather than the stable diagnostics concepts
- treating settings diagnostics as a replacement for lower-layer runtime/store tests
- allowing the diagnostics surface to drift away from actual desktop capabilities or plugin state
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Tauri notification plugin docs](https://v2.tauri.app/plugin/notification/) are relevant because settings diagnostics surface notification posture and constraints in the desktop runtime.
- [Tauri window-state plugin docs](https://v2.tauri.app/plugin/window-state/) are relevant because settings diagnostics expose persisted window-state readiness and file/path messaging.
- [WebdriverIO best practices](https://webdriver.io/docs/bestpractices/) are relevant because diagnostics smoke should assert on stable concepts with resilient selectors and waits.
- [WAI-ARIA dialog (modal) pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/) is relevant indirectly because settings diagnostics also reference the task center and other overlay-driven desktop affordances.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep desktop settings diagnostics smoke focused on stable desktop-runtime concepts rather than full-copy snapshots.
2. Tie each assertion to a concrete desktop capability or runtime status that users depend on.
3. Use lower-layer tests for store/service correctness and let desktop smoke prove that the real shell surfaces those signals coherently.
4. Review wording churn separately from capability regressions so the smoke contract stays useful.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the required desktop diagnostics concepts visible in settings.
- [x] Map those concepts back to the runtime/plugin sources that feed them.
- [x] Reduce reliance on volatile explanatory text where a stronger semantic signal exists.
- [x] Keep settings diagnostics smoke non-options only.
- [x] Use desktop smoke to prove real-shell visibility, not every runtime permutation.
- [x] Review capability wording whenever plugin behavior or runtime policies change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the targeted diagnostics spec after settings or desktop-runtime changes
- run the full desktop smoke suite after broader shell updates
- review whether a failing assertion reflects wording churn or an actual capability regression
- cross-check runtime and settings stores when the desktop shell no longer surfaces expected diagnostics
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current diagnostics smoke intent while gradually tightening assertion quality
- prefer concept-level assertions over removing diagnostics coverage entirely
- if settings diagnostics expand, add contract coverage selectively instead of snapshotting the whole panel
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Closed after hardening the settings diagnostics contract away from whole-grid text scraping.
- The desktop smoke path now uses stable diagnostics semantics exposed by `SettingsWorkspacePanel.vue`:
  - `Desktop settings diagnostics`
  - `Runtime diagnostics card`
  - `Native desktop diagnostics card`
  - `Desktop presence diagnostics card`
  - `Task posture diagnostics card`
  - `Desktop logs diagnostics card`
  - `Desktop notification permission`
  - `Desktop window state summary`
  - `Desktop presence summary`
- `waitForSettingsWorkspace()` now waits on the diagnostics section instead of a literal heading string, and the desktop settings spec asserts the cards and summary fields directly.
- Supporting component tests were updated to query the diagnostics action buttons and semantic diagnostics labels directly, which keeps the desktop smoke meaningful while lowering wording-churn sensitivity.
- The required desktop diagnostics concepts remain mapped to the real sources documented in this plan:
  - runtime phase / window-state readiness from runtime-store-backed diagnostics
  - notification posture from desktop productization/runtime state
  - task-center controls and task posture from the shared shell/task surfaces
  - desktop logs from the runtime desktop log buffer
- No extra `README.md` or `REPO_MAP.md` sync was required because the hardening change tightened selectors without changing the user-facing diagnostics workflow.
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/TaskCenterOverlay.test.ts src/composables/useTaskCenterVisibility.test.ts src/components/SettingsWorkspacePanel.test.ts src/components/VueWorkspaceShell.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
  - `npm.cmd --prefix web run typecheck:desktop`
  - `npm.cmd --prefix web run test:desktop:smoke:spec -- .\\e2e\\desktop\\specs\\desktop-settings-diagnostics.spec.ts`
  - `npm.cmd --prefix web run test:desktop:smoke`
- Raw evidence paths refreshed in this session:
  - targeted settings diagnostics run: `output/wdio/desktop-smoke/2026-04-10T17-39-40-662Z/`
  - stable desktop smoke: `output/wdio/desktop-smoke/2026-04-10T17-40-18-604Z/`

## Related Docs

- `REPO_Improvement/10_tests_perf/13_desktop_harness_contract_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/16_desktop_task_center_nav_2026-04-09.md`
- `REPO_Improvement/09_live/08_system_gpu_window_surface_2026-04-09.md`
- `REPO_Improvement/09_live/11_settings_secret_storage_2026-04-09.md`
