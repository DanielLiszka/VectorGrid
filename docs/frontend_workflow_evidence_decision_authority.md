# Frontend Workflow Evidence Decision Authority

Last updated: 2026-04-28

## Purpose

This note is a local engineering guardrail for frontend workflow state and evidence-to-decision UI. It is not a product spec and does not authorize new backend commands, artifact schemas, report schemas, or a generic workflow engine.

## Authority Order

1. Backend command contracts and request builders own whether an optimization, validation, ML, options, or live-execution request is legal.
2. `web/src/utils/core/workflowModel.ts` owns shell vocabulary, workflow-tab vocabulary, workflow-step vocabulary, and mode-specific allowed tab combinations.
3. `web/src/stores/workflowStore.ts` owns canonical mutable shell state after normalization and repair.
4. `web/src/services/workflowBootstrap.ts` owns startup restoration and must route persisted shell state through the workflow store repair path.
5. `web/src/App.vue` and `web/src/components/workbench/WorkbenchShell.vue` own visible projection and mount routing.
6. Panel-local stores and components own local action copy, disabled reasons, and evidence display.

## Vocabulary

- Workflow tabs are `data`, `optimize`, `validate`, `analyze`, `finalize`, `history`, and `settings`.
- Workflow steps are only `data`, `optimize`, `validate`, `analyze`, and `finalize`.
- `history` and `settings` must not be treated as workflow steps.
- Workspaces are `workflow`, `library`, `aggregate`, `pro`, `live`, and `settings`.
- `live` is a first-class workspace. Normalization, startup repair, quick switch, command palette, and workbench projection must preserve it.
- Conventional and ML Signal modes have separate allowed controls/results maps. Do not duplicate those maps outside `workflowModel.ts`.

## Invalid Transition Policy

- Repair pure shell state when stale localStorage, saved perspectives, or old runtime snapshots contain impossible workspace/workflow/tab combinations.
- Warn when an action would reinterpret evidence under materially different route, dataset, score scope, or provenance context.
- Block evidence-producing, exporting, or consequential actions when required prerequisites are absent or known stale.
- Do not block simple navigation just because evidence is incomplete.

## Evidence Display Policy

Evidence should stay action-local and existing-panel-first. The frontend may summarize existing route fallback, report state, score scope, dataset provenance, and live-readiness placeholders, but it must not invent artifact fields or persist evidence acknowledgements.

OS notifications are supplemental. Task Center and task/runtime stores remain the in-app authority for task review, cancellation, and diagnostics.

## Deferred Slices

Broader work remains gated by exact owner-file review and targeted tests:

- desktop/native menu internals and shared shortcut registry changes;
- footer/action-status evidence UI;
- history/detail evidence UI;
- Meta/ML confidence evidence UI;
- options quote/data provenance UI;
- live readiness/control gating beyond existing execution-store contracts.
