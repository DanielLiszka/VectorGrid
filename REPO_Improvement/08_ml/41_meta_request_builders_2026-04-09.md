# Meta Request Builders Plan

Last updated: 2026-04-09

## Purpose

Define the current meta request-builder contract so the split between Stage 3, Stage 5, batch, and ML-scope builders remains explicit and future refactors do not blur builder responsibilities.

## Scope Boundaries

In scope:

- `metaRequestBuilder.ts`
- `metaStage4RequestBuilder.ts`
- `metaBatchRequestBuilder.ts`
- `metaMlScopeBuilder.ts`
- service wiring that composes these builders into the current meta workflow

Out of scope:

- ML Signal frontend service/builders
- backend normalization details except where builder/backend drift is a core risk
- options workflows

## Current User-Facing Behavior

Users experience the builders indirectly through the Meta workspace. The shipped frontend already uses a split builder model:

- Stage 3 meta selection builder
- Stage 5 compatibility builder
- batch request builder
- ML-confidence scope builder

This means different parts of the meta workflow already have distinct frontend request-shaping owners, even though the UI presents them inside one larger Meta workspace.

## Current Code Surface

- `web/src/services/metaRequestBuilder.ts`
  - Stage 3 request shaping
- `web/src/services/metaStage4RequestBuilder.ts`
  - current Stage 5 request shaping through Stage 4 compatibility naming
- `web/src/services/metaBatchRequestBuilder.ts`
  - campaign request shaping
- `web/src/services/metaMlScopeBuilder.ts`
  - ML-confidence scope shaping
- `web/src/services/metaService.ts`
  - actual wiring from store state into each builder and onward to Tauri commands
- `src-tauri/src/commands_meta.rs`
  - backend command contracts these builders feed
- `src-tauri/src/commands_campaign.rs`
  - backend batch contract fed by `metaBatchRequestBuilder`

## Current Verification Surface

- `web/src/services/metaRequestBuilder.test.ts`
- `web/src/services/metaStage4RequestBuilder.test.ts`
- `web/src/services/metaBatchRequestBuilder.test.ts`
- `web/src/services/metaMlScopeBuilder.test.ts`
- `web/src/services/metaService.test.ts`

## Strengths Already Present

- builder ownership is already split by workflow purpose rather than one monolithic builder file
- each builder has dedicated tests
- service wiring already makes the builder split concrete in the shipped flow
- backend contracts are already specific enough that builders can remain purpose-built

## Weaknesses / Failure Modes

- the current Stage 5 builder is still named Stage 4, which is the biggest drift risk in this layer
- defaults can still drift between builders and backend serde defaults
- one frontend service (`metaService`) composes all builders, so boundary mistakes can still spread quickly
- scope-building is small enough that it can be overlooked even though it drives ML action correctness

## Regressions To Avoid

- collapsing these builders into one generic meta builder without a clear contract reason
- renaming the Stage 4 compatibility builder without updating its service/store call sites together
- weakening dedicated builder tests
- treating builder output as authoritative when the backend still normalizes important fields
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema: Getting Started](https://json-schema.org/learn/getting-started-step-by-step)
  - relevant because each builder targets a distinct structured request contract
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because builder outputs cross a command boundary and must remain serializable and shape-stable
- [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259)
  - relevant because null-versus-omitted fields and JSON payload stability matter across these builder families
- [Vue State Management](https://vuejs.org/guide/scaling-up/state-management.html)
  - relevant because builders are fed from store-backed state and should keep their responsibilities explicit
- [scikit-learn `TimeSeriesSplit`](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html)
  - relevant because chronological ML and meta-evaluation assumptions should stay explicit and must not drift toward shuffled-validation behavior

## Proposed Improvement Approach

1. Document the builder split by workflow purpose instead of by file name alone.
2. Record which backend contract each builder targets.
3. Make the Stage 4 compatibility naming drift explicit in this doc rather than hiding it.
4. Distinguish builder defaults from backend-required normalization.
5. Keep service wiring documented as the composition layer that binds these builders to the UI and Tauri calls.
6. Preserve separate tests for each builder and for service-level integration.
7. Treat ML/meta code quality as part of the implementation scope: keep evaluation assumptions, defaults, warnings, fixture payloads, and persisted shapes explicit enough for external reviewers to audit.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document each current meta-related builder and the contract it targets.
- [ ] Document the Stage 4 compatibility naming in the current Stage 5 builder.
- [ ] Document builder defaults versus backend normalization responsibilities.
- [ ] Document how `metaService` composes the current builders.
- [ ] Preserve or strengthen dedicated tests for each builder.
- [ ] Preserve integration tests in `metaService.test.ts` that exercise builder wiring.
- [ ] Treat builder consolidation or renaming as compatibility-sensitive work, not a cosmetic refactor.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Name one stable fixture, saved artifact, or replay payload plus the exact frontend and Rust commands that validate this ML/meta contract.

## Verification Plan

- run `npm --prefix web run test -- src/services/metaRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/metaStage4RequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/metaBatchRequestBuilder.test.ts`
- run `npm --prefix web run test -- src/services/metaMlScopeBuilder.test.ts`
- run `npm --prefix web run test -- src/services/metaService.test.ts`
- manually inspect one built Stage 3 request, one built Stage 5 request, one batch request, and one ML scope
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched ML/meta surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and the exact replay or saved-artifact checks named by the plan

## Rollout / Fallback / Compatibility Notes

- keep builder names and targeted contracts aligned even if the UI language evolves
- preserve backend compatibility aliases while the current Stage 4 naming still exists in service/store code
- prefer additive builder wrappers over collapsing distinct contracts into one file
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`
- `REPO_Improvement/08_ml/35_stage4_request_contract_2026-04-08.md`
- `REPO_Improvement/08_ml/42_meta_scope_builders_2026-04-09.md`
- `REPO_Improvement/08_ml/45_meta_frontend_service_2026-04-09.md`
- `REPO_Improvement/08_ml/46_ml_meta_store_status_2026-04-09.md`
