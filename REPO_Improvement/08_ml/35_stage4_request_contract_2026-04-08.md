# Stage 4 Request Contract Plan

Last updated: 2026-04-08

## Purpose

Define the improvement plan for the Stage 4 request contract so frontend defaults, numeric normalization, backend request semantics, and Stage 4/Stage 5 compatibility aliases remain explicit and stable.

## Scope Boundaries

In scope:

- Stage 4 request building and normalization
- frontend default values and field clamping
- backend request semantics for Stage 4 execution
- Stage 4 / Stage 5 naming and artifact compatibility boundaries

Out of scope:

- Stage 3 meta selection request details
- Stage 5 result interpretation beyond what the Stage 4 request directly shapes
- options workflows

## Current User-Facing Behavior

Users can configure Stage 4 from the Meta workspace using strategy kind, timeframe, source artifact path, top-k, ML/meta weighting, confidence thresholds, and model-health flags. The frontend normalizes these values before invoking the backend. The backend then canonicalizes timeframe, clamps or percentage-normalizes thresholds, re-normalizes weights to sum to `1`, coordinates auto-train and health-gating flags, and writes one Stage-5-kind artifact whose path is returned through both Stage-4 and Stage-5 compatibility fields.

## Current Code Surface

- `web/src/services/metaStage4RequestBuilder.ts`
- `web/src/services/metaStage4RequestBuilder.test.ts`
- `web/src/services/metaService.ts`
- `web/src/components/MetaWorkspacePanel.vue`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_artifacts.rs`

## Current Verification Surface

- `web/src/services/metaStage4RequestBuilder.test.ts`
- `web/src/services/metaService.test.ts`
- Meta workspace component tests
- Stage 4 related tests in `src-tauri/src/commands_meta.rs`

## Strengths Already Present

- Stage 4 request building is already centralized in its own frontend builder
- defaults and numeric clamping are already covered by dedicated tests
- backend normalization and artifact handling already exist
- Stage 4 compatibility aliases are already acknowledged instead of ignored

## Weaknesses / Failure Modes

- Stage 4 and Stage 5 naming is partially overlapping and easy to confuse
- defaults and clamping rules are not yet owned by one explicit contract doc
- product-choice defaults can drift from backend expectations if not documented
- model-health and fallback flags are subtle enough to be misinterpreted by users or later contributors
- source artifact selection is restricted to saved meta-selection artifacts inside the runs directory, which is easy to miss if the contract is only described at the UI layer

## Regressions To Avoid

- changing Stage 4 defaults without documenting the product impact
- breaking current builder/back-end compatibility
- hiding Stage 4 vs Stage 5 alias behavior
- weakening numeric clamping and fallback behavior that currently protects bad input
- letting Stage 4 request cleanup silently change downstream meta behavior

## External Research Summary

- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant for documenting a stable Stage 4 request contract with explicit defaults.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because Stage 4 request semantics cross the command boundary into backend normalization.
- [scikit-learn probability calibration](https://scikit-learn.org/stable/modules/calibration.html) is relevant because Stage 4 ranking and threshold semantics assume calibrated confidence-style scores.
- [RFC 8259](https://datatracker.ietf.org/doc/html/rfc8259) is relevant because Stage 4 request/result payloads and saved artifact fields are JSON contracts.

## Proposed Improvement Approach

1. Document the Stage 4 request field set, defaults, and normalization rules in one place.
2. Separate fields into:
   - identity/source fields
   - ranking/weighting fields
   - confidence/uncertainty thresholds
   - model-health and fallback flags
3. Keep frontend default/clamp rules and backend normalization explicitly synchronized, including weight renormalization and auto-train / auto-retrain behavior.
4. Document source-artifact resolution and the restriction that Stage 4 sources come from saved meta artifacts under the runs directory.
5. Document Stage 4 / Stage 5 naming and artifact compatibility aliases so the current hybrid naming is not surprising.
6. Expand tests if any request semantics or alias behavior changes.

## Checklist

- [ ] Document all Stage 4 request fields and default values.
- [ ] Document numeric clamping and normalization rules.
- [ ] Document weight renormalization and auto-train / auto-retrain interactions.
- [ ] Distinguish product defaults from backend-required normalization.
- [ ] Document model-health and fallback flag semantics.
- [ ] Document source-artifact resolution and path restrictions.
- [ ] Document current Stage 4 / Stage 5 naming and artifact alias behavior.
- [ ] Document key response fields such as decision bucket, fallback reason, and Stage 4 vs Stage 5 rank aliases.
- [ ] Add or maintain request-builder tests for defaults and explicit overrides.
- [ ] Keep Meta workspace UI copy aligned with the documented contract.
- [ ] Avoid silent semantic drift when renaming compatibility aliases later.

## Verification Plan

- run `npm test -- src/services/metaStage4RequestBuilder.test.ts`
- run `npm test -- src/services/metaService.test.ts`
- run Meta workspace component tests
- run targeted Stage 4 tests in `src-tauri/src/commands_meta.rs`

## Rollout / Fallback / Compatibility Notes

- preserve current field names, defaults, and alias behavior unless a documented migration is ready
- improve clarity before changing semantics
- if Stage 4 and Stage 5 naming is unified later, keep compatibility aliases until saved workflows are migrated

## Related Docs

- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`
- `REPO_Improvement/08_ml/40_meta_ml_status_surface_DATE.md`
- `REPO_Improvement/06_artifacts/32_campaign_summary_checkpoint_DATE.md`
- `REPO_Improvement/11_docs_ops/10_research_source_policy_2026-04-08.md`
