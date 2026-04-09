## Purpose

Define the improvement plan for the saved Meta artifact families so Saved Meta, Saved Stage 5, and Saved Policy Eval behave like clearly distinct, first-class families instead of one overloaded saved-results area.

## Scope Boundaries

In scope:

- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/services/metaService.ts`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_meta.rs`
- saved-family search/select/load/reveal/delete/tag flows for meta, stage5, and policy-eval artifacts

Out of scope:

- underlying meta/stage5/policy-eval algorithms
- options-related saved artifact surfaces
- generic history-library behavior outside the meta workspace

## Current User-Facing Behavior

The meta workspace already exposes three saved artifact families:

- Saved Meta
- Saved Stage 5
- Saved Policy Eval

Each family already has its own search, selected item, tags draft, load/reveal/delete affordances, and status messaging. That is a strong start. The risk is conceptual drift: these families live near one another, share UI patterns, and can easily be treated as “the same but with different labels.”

## Current Code Surface

- `web/src/components/MetaWorkspacePanel.vue`
- `web/src/components/MetaWorkspacePanel.test.ts`
- `web/src/services/metaService.ts`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_meta.rs`

## Current Verification Surface

- `MetaWorkspacePanel.test.ts`
- `metaService.test.ts`
- `metaServiceCampaignResume.test.ts`
- `metaServiceCampaignStrategy.test.ts`
- backend typed-artifact command handling in `commands_meta.rs`

## Strengths Already Present

- the families are already explicitly named and separated in the UI
- each family already has its own search/select/meta/status state in `metaService.ts`
- saved family actions are not purely generic; they already use typed load/reveal/delete flows
- tests already cover parts of saved meta, saved stage5, and saved policy evaluation behavior

## Weaknesses / Failure Modes

- UI similarity can make the families feel interchangeable when they are not
- family-specific action semantics can drift if copy and state summaries are too generic
- saved artifact selection state is spread across several parallel variables, which makes maintenance noisier
- typed backend support exists, but the frontend still needs clearer provenance messaging

## Regressions To Avoid

- collapsing saved-family behavior into vague generic “saved results” language
- breaking one family while another still appears to work
- losing family-specific reveal/delete/load semantics during UI cleanup
- making it less obvious which family a compare or replay action is targeting

## External Research Summary

- [Vue state management](https://vuejs.org/guide/scaling-up/state-management.html) is relevant because the saved-family surface is a shared, parallel-state domain rather than a local widget.
- [Pinia testing cookbook](https://pinia.vuejs.org/cookbook/testing.html) is relevant because saved-family behavior involves parallel search/select/action state that needs targeted tests.
- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because these families are backed by distinct typed artifact contracts.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because saved-family load/reveal/delete flows are typed command boundaries, not just local UI state changes.
- [WAI-ARIA Authoring Practices](https://www.w3.org/WAI/ARIA/apg/) are relevant because the saved-family switcher, menus, and search/select interactions are stateful UI patterns that must remain understandable.

## Proposed Improvement Approach

1. Treat each saved family as a separate artifact domain with shared UI primitives.
2. Document for each family:
   - what artifact it represents
   - what actions are valid
   - what comparison/replay role it plays
3. Keep UI layout shared where useful, but make provenance and family purpose more explicit.
4. Tighten tests so family-specific actions cannot regress independently.
5. Consider helper extraction only if it reduces duplication without erasing family identity.

## Checklist

- [ ] Document the purpose of Saved Meta, Saved Stage 5, and Saved Policy Eval as separate artifact families.
- [ ] Audit labels, empty states, and action menus for clear family-specific wording.
- [ ] Add or tighten tests for family switching, selection, and action dispatch.
- [ ] Add or tighten tests for load/reveal/delete behavior per family.
- [ ] Decide whether any duplicated `metaService.ts` helper logic should be shared while preserving explicit family ownership.

## Verification Plan

- run `MetaWorkspacePanel.test.ts`
- run `metaService.test.ts`
- run `metaServiceCampaignResume.test.ts`
- run `metaServiceCampaignStrategy.test.ts`
- run typed artifact command tests if backend saved-family loading behavior changes

## Rollout / Fallback / Compatibility Notes

- preserve explicit family separation even if helper code becomes more shared
- prefer clearer wording and provenance over collapsing UI distinctions
- if a cleanup makes family identity less obvious, restore the more explicit current grouping

## Related Docs

- `REPO_Improvement/08_ml/06_meta_panel_ia_2026-04-09.md`
- `REPO_Improvement/08_ml/07_meta_selection_compare_2026-04-09.md`
- `REPO_Improvement/06_artifacts/31_batch_summary_artifacts_2026-04-09.md`
- `REPO_Improvement/06_artifacts/17_report_meta_workbook_2026-04-09.md`
