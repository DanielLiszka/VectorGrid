# Optimization Dynamic Builder Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the Dynamic Double MA builder and preview flow so builder-state editing, raw `strategy_ir` override, backend preview, and runtime legality stay aligned and explainable.

## Scope Boundaries

In scope:

- dynamic builder UI inside `OptimizeWorkspacePanel.vue`
- preview commands in `src-tauri/src/commands_dynamic.rs`
- strategy IR handling in `crates/app_core/src/strategy_dynamic.rs` and related IR code
- frontend storage of builder state and preview metadata

Out of scope:

- inventing a new strategy IR schema
- redesigning the entire optimize panel
- options-related strategy builders

## Current User-Facing Behavior

`DynamicDoubleMa` already has a constrained builder experience inside the optimize panel. Users can build a supported IR shape through guided controls, inspect the pretty-printed JSON, and optionally paste or edit raw JSON directly.

The repo already supports preview behavior that distinguishes requested backend from resolved backend and explains fallback reasons. Unsupported or non-exact shapes can preview a CPU fallback or clean rejection instead of silently pretending to be GPU-compatible. In browser-only sessions, desktop preview commands are intentionally unavailable.

This is already a real builder pipeline, but the ownership boundary between builder state, canonical `strategy_ir`, preview results, and final request building still deserves a dedicated contract.

## Current Code Surface

- `web/src/components/OptimizeWorkspacePanel.vue`
- `web/src/strategies/dynamicDoubleMaBuilder.ts`
- `web/src/stores/optimizeStore.ts`
- `web/src/services/optimizeService.ts`
- `src-tauri/src/commands_dynamic.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_ir.rs`

## Current Verification Surface

- `web/src/components/OptimizeWorkspacePanel.test.ts`
- `web/src/services/optimizeService.test.ts`
- `src-tauri/src/commands_dynamic.rs`
- `crates/app_core/src/strategy_ir.rs`

## Strengths Already Present

- the builder already exists as a constrained authoring surface instead of raw JSON only
- preview output already carries requested-vs-resolved backend information
- fallback reasons are already surfaced rather than hidden
- raw JSON override remains available for advanced use without replacing the guided builder
- there is already backend-side preview logic rather than frontend-only guessing

## Weaknesses / Failure Modes

- builder-state truth and canonical `strategy_ir` truth are close but not yet explicitly separated in documentation
- preview eligibility can be mistaken for guaranteed runtime behavior if the distinction is not documented
- the current optimize panel mixes builder editing, preview rendering, and run orchestration in one component
- there is no explicit plan freezing which IR shapes are supported, previewable, exact-compatible, or fallback-only
- parity between preview results and request-builder behavior can drift

## Regressions To Avoid

- silently accepting unsupported IR shapes
- losing requested-vs-resolved backend preview information
- conflating preview success with run-time exact-route eligibility
- breaking raw JSON override behavior or builder-state restore behavior
- making browser-only sessions appear to support unavailable preview commands

## External Research Summary

- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the dynamic strategy IR behaves like a schema-governed payload even if support is intentionally restricted.
- [Vue Form Input Bindings](https://vuejs.org/guide/essentials/forms.html) is relevant because the builder mixes guided form inputs with raw JSON editing.
- [Tauri Calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because preview and run paths both cross a command boundary that must stay truthful.
- [Pinia Core Concepts](https://pinia.vuejs.org/core-concepts/) is relevant because builder state, preview state, and run orchestration should remain explicitly owned.

## Proposed Improvement Approach

1. Document builder state and canonical `strategy_ir` as separate but synchronized layers.
2. Freeze the distinction between preview contracts and run contracts.
3. Document fallback-to-CPU behavior and exact-compatible shape requirements clearly.
4. Add parity checks between preview output and request-builder legality where coverage is weak.
5. Keep raw JSON override as an additive advanced path, not a hidden replacement for the guided builder.

## Checklist

- [ ] Document builder-state ownership and canonical IR ownership separately.
- [ ] Document preview output fields, especially requested versus resolved backend.
- [ ] Document supported, fallback-only, and rejected IR-shape categories.
- [ ] Add or tighten preview-to-request parity tests.
- [ ] Document browser-only limitations clearly.

## Verification Plan

- run `npm --prefix web test -- src/components/OptimizeWorkspacePanel.test.ts`
- run `npm --prefix web test -- src/services/optimizeService.test.ts`
- run targeted Rust tests for dynamic preview commands and IR validation
- perform one manual preview and one manual run for an exact-compatible and a fallback-only shape if behavior changes

## Rollout / Fallback / Compatibility Notes

- preserve the guided builder and raw JSON override together
- prefer more explicit preview metadata over silently broadening supported shapes
- keep unsupported-shape failures clear and actionable
- if schema support broadens later, update preview and run docs together

## Related Docs

- `REPO_Improvement/03_opt/01_opt_panel_core_2026-04-09.md`
- `REPO_Improvement/03_opt/02_opt_strategy_backend_2026-04-09.md`
- `REPO_Improvement/03_opt/05_opt_route_gpu_capability_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/10_research_source_policy_2026-04-08.md`
