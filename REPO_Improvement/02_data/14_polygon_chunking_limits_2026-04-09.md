# Polygon Chunking Limits Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to Polygon download chunking, retry, and cache-refresh rules so large-range requests remain predictable without changing current cost and safety posture accidentally.

## Scope Boundaries

In scope:

- current request limits and chunk-size constants
- retry and backoff behavior
- full-hit, append, and refresh decision branches
- free-plan lookback enforcement for top-off

Out of scope:

- new data providers
- live stream semantics
- options workflows

## Current User-Facing Behavior

The current Polygon downloader already has explicit scaling constants in `src-tauri/src/polygon.rs`: `BARS_PER_REQUEST_LIMIT = 50000`, `CHUNK_DAYS = 30`, and `FREE_PLAN_LOOKBACK_DAYS = 730`. The downloader retries network, `429`, and `5xx` failures with backoff and `Retry-After` support. It decides whether a request is a full cache hit, append-only extension, or full refresh/overwrite based on requested date coverage. Top-off reuses the manifest dataset and rejects requests where the last cached day is outside the free-plan lookback window.

Current product behavior therefore includes:

- bounded request fan-out for large date ranges
- conservative reuse of cached datasets when date coverage allows it
- explicit top-off rejection when the cached dataset is too old for free-plan access assumptions

## Current Code Surface

- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.ts`

## Current Verification Surface

- `src-tauri/src/polygon.rs`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`

## Strengths Already Present

- key scaling constants already live in one backend file instead of scattered magic numbers
- retry and cache-reuse behavior are already part of the current implementation rather than future work
- top-off policy already encodes free-plan constraints explicitly
- the command boundary still keeps the frontend abstracted away from internal chunking details

## Weaknesses / Failure Modes

- chunk splitting and append-versus-refresh behavior are subtle and only lightly tested today
- there is no dedicated implementation doc that freezes the decision tree and constants together
- UI status text and product expectations can drift from backend branching if constants change quietly
- current tests cover the free-plan window better than the branch structure around chunking and retries

## Regressions To Avoid

- changing chunk-size or limit constants without updating behavior docs and tests together
- removing retry or `Retry-After` support accidentally
- fetching non-1m archives directly when the current product rule is “cache 1m, derive later”
- changing append-versus-refresh decisions without evaluating cache correctness implications
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Polygon Aggregates / Custom Bars](https://polygon.io/docs/rest/stocks/aggregates/custom-bars) is relevant because the downloader maps onto that API’s aggregate retrieval semantics and limit posture.
- [MDN Retry-After](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Retry-After) is relevant because the current downloader already respects backoff hints for rate-limited or overloaded responses.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because top-off and download responses act like stable contracts even if backend chunking changes internally.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Document the current constants and the exact branch structure before proposing any optimization.
2. Freeze the difference between full-hit, append, and full-refresh behavior.
3. Document retry/backoff expectations separately from cache-coverage decisions.
4. Keep top-off lookback enforcement explicit and tied to the free-plan assumption currently encoded in the backend.
5. Add branch-level tests before touching chunk or retry behavior.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document `BARS_PER_REQUEST_LIMIT`, `CHUNK_DAYS`, and `FREE_PLAN_LOOKBACK_DAYS`.
- [ ] Document retry and `Retry-After` behavior.
- [ ] Document full-hit, append, and full-refresh decision rules.
- [ ] Document top-off lookback rejection semantics.
- [ ] Add or tighten tests for chunk branching and retry behavior if the implementation changes.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted Polygon backend tests, especially around top-off request rejection
- add targeted branch tests for append-versus-refresh and retry behavior before modifying logic
- run `npm --prefix web run test -- src/services/dataService.test.ts` after any response-shape changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- prefer documenting current limits before trying to optimize them
- do not change constants independently from tests and status/copy review
- keep the frontend abstracted from internal chunk math unless a user-facing need emerges
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/13_polygon_request_contract_2026-04-09.md`
- `REPO_Improvement/02_data/20_polygon_dataset_topoff_2026-04-09.md`
- `REPO_Improvement/02_data/16_polygon_dataset_listing_2026-04-09.md`
