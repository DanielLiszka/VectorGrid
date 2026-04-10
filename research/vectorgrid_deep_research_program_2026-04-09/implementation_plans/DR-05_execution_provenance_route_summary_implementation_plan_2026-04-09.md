# DR-05 Implementation Plan — Compact Execution Provenance Route Summary

Last updated: 2026-04-09

Status: proposed bounded additive implementation plan derived from the accepted `DR-05` research response.

## Source Inputs

- research response: `responses/deep-research-report (3).md`
- synthesis judgment: `Adopt`
- synthesis anchor: `FIRST_FIVE_RESPONSE_SYNTHESIS_2026-04-09.md`

## Purpose

Make route selection and fallback behavior easier to trust by adding a compact requested/resolved/realized execution-provenance summary to saved run artifacts and the main result/history surfaces.

## Repo-Fit Summary

The repo already persists and surfaces several useful route signals:

- `backend_route`
- `backend_fallback_reason`
- route summaries inside optimize and validation flows

What is still missing is a compact, normalized way to answer:

- what the user asked for
- what the planner resolved
- what actually happened
- whether the route changed and why

This plan deliberately avoids a large provenance system. The goal is a small, compare-friendly summary built on fields the repo already has.

## Scope Boundaries

In scope:

- non-options optimization run artifacts
- compare-friendly route provenance summary in result and history surfaces
- optional extension to validation display only if it reuses the same compact model

Out of scope:

- per-stage event timelines
- deep stage-by-stage provenance maps
- options-expression provenance
- a new telemetry or audit-log subsystem

## Current Code Surface

- `web/src/services/optimizeService.ts`
- `src-tauri/src/run_artifacts.rs`
- `web/src/components/OverviewWorkspacePanel.vue`
- `web/src/components/HistoryWorkspacePanel.vue`
- `web/src/services/validationService.ts`

## Current Strengths To Preserve

- current route and fallback text in optimization status handling
- validation route summaries already present today
- current artifact schema compatibility
- existing compare and history flows

## Problems This Plan Solves

- route/fallback truth is present but spread across fields and UI text
- requested backend policy is easy to lose once only the final result is visible
- compare surfaces do not yet have a compact provenance block to reuse

## Proposed Outcome

The MVP should produce one compact provenance block with:

- `requested_route`
- `resolved_route`
- `realized_route`
- `route_changed`
- `fallback_reason`

This block should be:

- persisted additively inside run artifacts
- exposed in hydration/summary payloads
- shown as a concise header or badge summary in Overview and History

## Implementation Slices

### Slice 1 — Normalize provenance fields on save

When saving non-options run artifacts, create a compact provenance object from already-known request/result data.

Primary inputs:

- request backend policy
- backend route returned by the result
- backend fallback reason

Rules:

- prefer normalization over inventing a second route pipeline
- mark `route_changed=true` only when the route changed materially
- do not duplicate long-form human text when a short normalized field is enough

### Slice 2 — Hydration and summary extraction

Expose the compact provenance block through the same payloads used by:

- current run hydration
- History selected-run detail
- future compare surfaces

This should let frontend surfaces consume one normalized shape instead of recomputing route truth from scattered fields every time.

### Slice 3 — Header/badge presentation in current result surfaces

Add a compact presentation in:

- Overview winner summary
- History selected-run detail

The first-wave display should answer:

- requested
- actual
- fallback

This should stay compact enough that it supports compare and review without becoming a diagnostics wall.

### Slice 4 — Reuse in compare work

If `DR-02` lands alongside this plan, the compare surface should be able to show route-provenance changes directly from the normalized block instead of diffing raw route fields separately.

## Data / Contract Notes

- keep all new provenance fields optional and additive
- reuse existing `backend_route` and `backend_fallback_reason` as source inputs
- do not remove or rename legacy fields in the same change

## UI / UX Notes

Preferred copy shape:

- `requested=...`
- `actual=...`
- `fallback=...` only when applicable

Avoid:

- stage dumps
- implementation jargon
- multiple competing route summaries in the same panel

## Checklist

- derive the provenance block from existing request and result route data
- keep all new provenance fields optional and additive
- preserve existing `backend_route` and `backend_fallback_reason` compatibility
- expose one normalized requested, resolved, and realized route shape for hydration and UI use
- mark `route_changed` only for material route changes
- surface compact provenance in Overview and History without creating a diagnostics wall
- avoid building a deep provenance or telemetry subsystem in this slice
- add targeted artifact, optimize-service, and component tests for provenance normalization and display
- verify frontend lint, frontend typecheck, targeted frontend tests, and `cargo check --workspace`

## Verification Plan

- targeted artifact-schema tests in `src-tauri/src/run_artifacts.rs`
- targeted optimize-service tests for provenance normalization and surface text
- targeted Overview and History component tests for the new provenance summary
- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- targeted `npm --prefix web run test -- ...`
- `cargo check --workspace`

## Regression Guardrails

- do not replace existing route/fallback fields until the compact provenance block is proven stable
- do not ship both a verbose and a compact route summary in the same visual slot
- keep the first slice non-options only

## Exit Criteria

- saved run artifacts carry a compact route-provenance block
- current run hydration exposes it cleanly
- Overview and History show a stable requested/resolved/realized route summary
- existing optimization and validation route behavior remains unchanged

## Related Docs

- `research/vectorgrid_deep_research_program_2026-04-09/FIRST_FIVE_RESPONSE_SYNTHESIS_2026-04-09.md`
- `REPO_Improvement/06_artifacts/11_art_fingerprint_provenance_2026-04-09.md`
- `REPO_Improvement/06_artifacts/16_report_route_fallback_2026-04-09.md`
- `REPO_Improvement/04_results/15_dataset_stale_provenance_cues_2026-04-09.md`

## Deferred Follow-Ups

- per-stage execution maps
- route provenance for additional non-run artifact families
- unified provenance language shared across options and non-options flows
