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

## External Research Summary

- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the meta workspace is a shared, state-dense domain rather than a simple local form.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because IA changes must preserve stateful behavior across many controls.
- [WAI-ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/) are relevant because tab groups, menus, and dense form layouts need strong navigation semantics.
- [Tauri event namespace](https://v2.tauri.app/es/reference/javascript/api/namespaceevent/) is relevant because long-running meta and campaign flows surface progress and result changes through runtime events.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because much of the workspace exists to review or launch structured artifact workflows.

## Proposed Improvement Approach

1. Treat the Meta workspace as a multi-subdomain console, not a single flat panel.
2. Document the intended mental model for each top-level tab and saved-artifact family.
3. Audit headings, summaries, and helper text so each section answers:
   - what this section is for
   - when to use it
   - what artifact family it produces or consumes
4. Preserve action visibility while reducing conceptual clutter through better grouping and copy.
5. Add tests for IA-critical affordances such as tab switching, saved-family switching, and destructive-action confirmation.

## Checklist

- [ ] Document the intended top-level information architecture of `MetaWorkspacePanel`.
- [ ] Audit headings, helper text, and action labels for conceptual clarity.
- [ ] Add or tighten tests for tab/family switching and context-menu behavior.
- [ ] Verify that Stage 4/5 and policy-eval controls remain distinguishable from Meta Selection and Batch Generation.
- [ ] Decide whether some sections need stronger subheadings or summary chips instead of more controls.

## Verification Plan

- run `MetaWorkspacePanel.test.ts`
- run `metaService.test.ts`
- run `metaServiceCampaignResume.test.ts`
- run `metaServiceCampaignStrategy.test.ts`
- perform a bounded manual scan of the Meta workspace after IA-only copy/grouping changes

## Rollout / Fallback / Compatibility Notes

- preserve existing capabilities and action reachability
- prefer clearer grouping/copy over structural upheaval in the first wave
- if an IA change hides too much, restore the more explicit layout and improve explanation incrementally

## Related Docs

- `REPO_Improvement/08_ml/07_meta_selection_compare_2026-04-09.md`
- `REPO_Improvement/08_ml/08_meta_batch_generation_2026-04-09.md`
- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`
- `REPO_Improvement/01_shell/21_shell_context_menus_help_2026-04-08.md`
