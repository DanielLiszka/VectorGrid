# Meta Panel Ia Plan

Last updated: 2026-04-09

## Purpose

Define the improvement plan for the Meta workspace information architecture so Meta Selection, Batch Generation, Saved Meta, and Compare remain understandable as one coherent workspace instead of a dense pile of loosely related controls.

## Scope Boundaries

In scope:

- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/services/metaService.ts`
- `web/src/stores/analysisStore.ts`
- meta workspace tab structure, section grouping, action labeling, and state presentation

Out of scope:

- redesigning the underlying meta algorithms
- options-related workspaces
- rewriting every downstream artifact family in the same pass

## Current User-Facing Behavior

The Meta workspace already covers several substantial surfaces inside one panel:

- Meta Selection
- Batch Generation
- Saved Meta / Saved Stage 5 / Saved Policy Eval
- Compare
- ML confidence and Stage 4/5 controls

This is powerful, but it also creates density. The current UI works best when the user already understands the internal distinctions between meta, batch, stage5, policy eval, and compare. The IA needs to help more, not just expose more.

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/services/metaService.ts`
- `web/src/stores/analysisStore.ts`
- related iconography/help surfaces in `web/src/utils/appSvgAssets.ts` and `web/src/utils/iconography.ts`

## Current Verification Surface

- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/services/metaService.test.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`

## Strengths Already Present

- the workspace already groups related meta surfaces into one intentional place
- aggregate tabs and saved-artifact family selectors already exist
- contextual menus and saved-artifact secondary actions are already modeled explicitly
- the component already carries mode-aware summaries and preview panels rather than only raw form fields
- tests already cover important panel behavior and saved-artifact interactions

## Weaknesses / Failure Modes

- the panel is dense enough that small additions can make it harder to learn
- Stage 4/5 and policy-eval controls sit close to Meta Selection and Batch Generation, which can blur conceptual boundaries
- saved-artifact families are real subdomains, but they are easy to mistake for one generic saved-results list
- action density can outpace explanatory labeling

## Regressions To Avoid

- making the panel denser without clarifying the hierarchy
- hiding high-value actions behind ambiguous menus
- changing grouping or labels in ways that break existing user muscle memory without a clear win
- letting saved-artifact family behavior drift apart visually and conceptually
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the meta workspace is a shared, state-dense domain rather than a simple local form.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because IA changes must preserve stateful behavior across many controls.
- [WAI-ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/) are relevant because tab groups, menus, and dense form layouts need strong navigation semantics.
- [Tauri event namespace](https://v2.tauri.app/es/reference/javascript/api/namespaceevent/) is relevant because long-running meta and campaign flows surface progress and result changes through runtime events.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because much of the workspace exists to review or launch structured artifact workflows.
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Treat the Meta workspace as a multi-subdomain console, not a single flat panel.
2. Document the intended mental model for each top-level tab and saved-artifact family.
3. Audit headings, summaries, and helper text so each section answers:
   - what this section is for
   - when to use it
   - what artifact family it produces or consumes
4. Preserve action visibility while reducing conceptual clutter through better grouping and copy.
5. Add tests for IA-critical affordances such as tab switching, saved-family switching, and destructive-action confirmation.
6. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document the intended top-level information architecture of `MetaWorkspacePanel`.
- [x] Audit headings, helper text, and action labels for conceptual clarity.
- [x] Add or tighten tests for tab/family switching and context-menu behavior.
- [x] Verify that Stage 4/5 and policy-eval controls remain distinguishable from Meta Selection and Batch Generation.
- [x] Decide whether some sections need stronger subheadings or summary chips instead of more controls.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [x] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `MetaWorkspacePanel.test.ts`
- run `metaService.test.ts`
- run `metaServiceCampaignResume.test.ts`
- run `metaServiceCampaignStrategy.test.ts`
- perform a bounded manual scan of the Meta workspace after IA-only copy/grouping changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- preserve existing capabilities and action reachability
- prefer clearer grouping/copy over structural upheaval in the first wave
- if an IA change hides too much, restore the more explicit layout and improve explanation incrementally
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/07_meta_selection_compare_2026-04-09.md`
- `REPO_Improvement/08_ml/08_meta_batch_generation_2026-04-09.md`
- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`
- `REPO_Improvement/01_shell/21_shell_context_menus_help_2026-04-09.md`

## Execution Notes

- Closed by clarifying the Meta workspace hierarchy instead of restructuring the whole panel.
- `MetaWorkspacePanel.vue` now makes the Compare surface self-describing: it explicitly states that Compare uses a Saved Meta baseline, shows the current working-side source, and keeps later-stage Saved Stage 5 and Saved Policy Evaluation artifacts conceptually separated in copy.
- The Saved Aggregate family summary remains visible and explicit, so the workspace now better distinguishes baseline meta artifacts from Stage 5 confidence reranks and policy-evaluation outputs without removing any actions.
- `MetaWorkspacePanel.test.ts` now covers the compare-provenance presentation and preserves the previously verified saved-family action/menu behavior.
- `REPO_MAP.md` was synced so contributors can find the Meta workspace provenance behavior quickly.
- Stable fixture / replay payload:
  - selected compare baseline `C:/runs/vectorgrid_meta_baseline.json` with compare option label `2026-04-09T10:00:00Z | CompositeMo | 1m | groups=4`
- Acceptance evidence:
  - `npm.cmd --prefix web run test -- src/components/MetaWorkspacePanel.test.ts src/stores/analysisStore.actions.test.ts src/services/metaService.test.ts`
  - `npm.cmd --prefix web run lint`
  - `npm.cmd --prefix web run typecheck`
