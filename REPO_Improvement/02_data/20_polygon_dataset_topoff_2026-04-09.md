# Polygon Dataset Top-Off Plan

Last updated: 2026-04-09

## Purpose

Define the current top-off contract so manifest-derived requests, free-plan lookback checks, and load-plus-resample behavior remain explicit.

## Scope Boundaries

In scope:

- top-off request derivation from manifest datasets
- lookback-window enforcement
- top-off response behavior in command, service, and store layers
- base-load plus derived-load semantics after top-off

Out of scope:

- manual download-form requests not derived from a selected cached dataset
- live stream backfill
- options workflows

## Current User-Facing Behavior

Top-off is already a specialized path, not just a re-labeled download. `build_top_off_request_for_dataset()` derives a `PolygonDownloadRequest` from the manifest-backed selected dataset, using the stored market, ticker, adjusted posture, and timeframe, then checks whether the last cached day is still within the free-plan lookback window. `polygon_top_off_dataset` then downloads missing `1m` bars into the cache, reloads the base dataset, and returns either the base load or an in-memory resample to the requested timeframe.

Current user-facing behavior includes:

- top-off only works on a selected cached dataset already known to the manifest
- the response mirrors download-and-load behavior with both `baseLoad` and `load`
- top-off can reject stale datasets that fall outside the current free-plan lookback assumption

## Current Code Surface

- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`

## Current Verification Surface

- `src-tauri/src/polygon.rs`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`

## Strengths Already Present

- top-off is already manifest-driven instead of trusting loosely related UI form state
- lookback rejection is already explicit and backend-owned
- the command already returns both base and effective load results, which keeps frontend behavior consistent with download
- top-off reuses the same 1m-cache-then-resample product rule as direct Polygon download

## Weaknesses / Failure Modes

- top-off semantics are distinct enough from direct download that they need their own implementation doc
- lookback rejection can surprise users if the contract is not written down clearly
- top-off depends on manifest correctness and selected-path behavior, which ties it tightly to listing/selection contracts
- there is only light explicit test coverage for the manifest-derived request branch

## Regressions To Avoid

- allowing top-off on arbitrary paths not present in the manifest without a deliberate contract change
- dropping lookback checks or hiding their failure semantics
- breaking base-load plus derived-load response parity with direct download
- confusing top-off with re-downloading an arbitrary date range from the form

## External Research Summary

- [Polygon Aggregates / Custom Bars](https://polygon.io/docs/rest/stocks/aggregates/custom-bars) is relevant because top-off still rides on the same aggregate download path.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because top-off responses and manifest-derived requests behave like stable command contracts.
- [TimeSeriesSplit](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.TimeSeriesSplit.html) is relevant because chronological append behavior matters for later validation consumers.

## Proposed Improvement Approach

1. Document top-off as a manifest-derived append/refresh path, not as generic download.
2. Freeze lookback-window checks and their user-visible error posture.
3. Document the response contract and its parity with `polygon_download_1m_and_load`.
4. Tie top-off explicitly to listing/selection and resample rules.
5. Add targeted tests before changing top-off request derivation or returned shape.

## Checklist

- [ ] Document top-off preconditions and manifest dependence.
- [ ] Document lookback-window rejection behavior.
- [ ] Document base-load versus effective-load response semantics.
- [ ] Document requested-timeframe resample behavior after top-off.
- [ ] Add or tighten tests if top-off branching or response shape changes.

## Verification Plan

- run targeted Polygon backend tests for top-off request construction and rejection
- run `npm --prefix web test -- src/services/dataService.test.ts`
- run `npm --prefix web test -- src/stores/dataStore.actions.test.ts`
- manually top off one recent cached dataset after any top-off-flow change

## Rollout / Fallback / Compatibility Notes

- preserve manifest-derived request construction unless a safer replacement is already implemented
- keep top-off error posture explicit rather than silently degrading to a full download
- maintain direct response parity with download-and-load where possible

## Related Docs

- `REPO_Improvement/02_data/13_polygon_request_contract_2026-04-09.md`
- `REPO_Improvement/02_data/14_polygon_chunking_limits_2026-04-09.md`
- `REPO_Improvement/02_data/25_resample_no_redownload_2026-04-09.md`
