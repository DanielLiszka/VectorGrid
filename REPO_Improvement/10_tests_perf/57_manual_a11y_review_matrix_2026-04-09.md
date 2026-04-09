# Manual Accessibility Review Matrix Plan

Last updated: 2026-04-09

## Purpose

Define the manual accessibility review matrix for the non-options app so automated a11y smoke checks are complemented by deliberate human review of keyboard flow, accessible naming, screen-reader semantics, dense result surfaces, and report-opening workflows.

## Scope Boundaries

In scope:

- manual accessibility review expectations for non-options frontend surfaces
- representative screen coverage for shell, results, validation, ML, live, and history flows
- keyboard and focus behavior
- accessible names/descriptions and landmark/region structure
- dense-table and chart-adjacent review expectations where automation alone is insufficient

Out of scope:

- options-related panels and workflows
- full WCAG certification claims
- backend-only verification
- replacing automated axe smoke with manual review

## Current User-Facing Behavior

The repo already has meaningful accessibility work:

- shell-level accessibility/copy planning
- automated a11y smoke checks
- component-level UX planning in the shell lane

That is a good base, but the current plan set still leaves a gap between:

- automated smoke assertions
- and a human-accessibility review process across the actual app surfaces users navigate

This matters because many accessibility failures in a data-heavy desktop app are only obvious through manual review:

- focus traps
- unexpected tab order
- unreadable dense tables
- missing accessible names on icon-only controls
- ambiguous status messaging
- keyboard-inaccessible drilldowns or context actions

## Current Code Surface

- `web/src/App.vue`
- `web/src/components/`
- `web/src/components/RunFooterPanel.vue`
- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/components/ValidationWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/components/MlSignalWorkspacePanel.vue`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/components/DesktopTitlebar.vue`
- `web/e2e/browser/`
- `web/scripts/check-a11y-smoke.mjs`

## Current Verification Surface

- `npm --prefix web run a11y:smoke`
- component tests that indirectly exercise labels and visible states
- browser UX smoke flows
- desktop smoke flows

## Strengths Already Present

- the repo already treats accessibility as a real quality surface rather than an afterthought
- automated a11y smoke exists instead of relying only on manual checking
- shell and copy planning already mention clarity and keyboard-safe UX concerns
- result-display adequacy is already a separate plan lane, which gives accessibility review a concrete place to anchor

## Weaknesses / Failure Modes

- there is no explicit manual review matrix across screens and workflows
- automated checks cannot fully validate keyboard operability, table comprehension, or assistive-technology usability
- dense analytical surfaces are more likely to regress in semantics, focus behavior, or accessible naming
- icon-only or compact controls can become ambiguous if manual review is not expected
- browser-side accessibility can diverge from desktop/Tauri behavior in subtle ways

## Regressions To Avoid

- treating axe or similar tooling as complete accessibility coverage
- requiring exhaustive manual review of every screen on every tiny change
- ignoring dense table and chart-adjacent surfaces because they are harder to review
- collapsing accessibility into copy polish only
- shipping keyboard regressions because mouse testing still works
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [WCAG-EM Overview](https://www.w3.org/WAI/test-evaluate/conformance/wcag-em/)
  - relevant because W3C recommends a scoped, representative-sample evaluation methodology rather than ad hoc checking
- [WAI-ARIA Overview](https://www.w3.org/WAI/standards-guidelines/aria/)
  - relevant because the app contains dynamic and rich UI surfaces where semantic roles, states, and properties matter
- [ARIA Authoring Practices Guide](https://www.w3.org/WAI/ARIA/apg/)
  - relevant because keyboard support and accessible naming patterns for rich widgets should follow known guidance
- [APG Practices](https://www.w3.org/WAI/ARIA/apg/practices/)
  - relevant because landmark regions, accessible names, and keyboard interfaces are central to a useful review matrix
- [W3C Accessibility Guidelines Evaluation Methodology (WCAG-EM) 2.0 draft note announcement](https://www.w3.org/news/2026/group-note-draft-w3c-accessibility-guidelines-evaluation-methodology-wcag-em-2-0/)
  - relevant because it reinforces structured evaluation, representative sampling, and reporting rather than one-off spot checks
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Define a representative manual-review matrix instead of requiring a full manual sweep for every change.
2. Cover the highest-risk non-options surfaces:
   - shell navigation
   - results and compare tables
   - validation summary/report-entry surfaces
   - history/artifact reopening
   - ML and live-execution status surfaces
3. For each surface, require manual review of:
   - tab order and focus visibility
   - keyboard completion of the primary task
   - accessible names/descriptions on controls
   - status and error announcement clarity
   - readability of dense tables and summaries
4. Use automated smoke as the fast gate and the manual matrix as the public-quality gate for high-risk UI changes.
5. Keep the matrix additive and practical:
   - smaller changes hit a narrower review slice
   - larger UI changes hit the full representative sample
6. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Define the representative non-options screen set for manual accessibility review.
- [ ] Define the keyboard-only task path for each selected surface.
- [ ] Define the accessible-name and status-message review points for each selected surface.
- [ ] Define dense-table and summary-comprehension review points for result-heavy screens.
- [ ] Define when browser review is enough and when desktop review is also required.
- [ ] Add a simple review log format so manual findings are captured consistently.
- [ ] Tie high-risk UI changes to the manual review matrix before release.
- [ ] Keep automated a11y smoke as the fast gate, not the full gate.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run `npm --prefix web run a11y:smoke`
- run the representative browser UX smoke flows
- manually review keyboard navigation and focus order for the chosen representative screens
- manually review at least one result-dense flow and one validation/history reopening flow in desktop context
- record findings in a repeatable checklist rather than informal notes only
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- start with a representative subset of screens instead of trying to manually certify the entire app at once
- keep the matrix stable long enough to be comparable across releases
- do not delay safe bug fixes on a full manual sweep when a narrower matrix slice is sufficient
- preserve existing automated accessibility checks; this plan augments them
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/01_shell/18_ux_a11y_copy_regression_2026-04-09.md`
- `REPO_Improvement/04_results/08_results_display_adequacy_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/05_fe_results_surface_adequacy_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/07_fe_a11y_smoke_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/09_browser_harness_contract_2026-04-09.md`
