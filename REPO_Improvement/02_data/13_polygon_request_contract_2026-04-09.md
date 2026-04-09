# Polygon Request Contract Plan

Last updated: 2026-04-09

## Purpose

Define the current Polygon download and top-off request contract so symbol normalization, timeframe handling, and resample behavior remain explicit and testable.

## Scope Boundaries

In scope:

- frontend request normalization for Polygon download and top-off
- backend accepted request fields and validation
- symbol normalization for stocks, FX, crypto, and native Polygon tickers
- requested-timeframe versus cached-timeframe semantics

Out of scope:

- live candle stream requests
- options workflows
- new Polygon markets not already supported

## Current User-Facing Behavior

`PolygonDownloadRequest` in `src-tauri/src/polygon.rs` already has a real contract: `market`, `symbol`, `from`, `to`, optional `adjusted`, and optional `timeframe`. Backend normalization supports `stocks`, `fx`, and `crypto`; accepts native Polygon tickers with `:`, stock tickers as direct symbols, and FX/crypto pair-style inputs. Dates must be `YYYY-MM-DD` and `to >= from`. Stocks default to `adjusted=true`. `src-tauri/src/commands_data.rs` adds one important product rule: `polygon_download_1m_and_load` always fetches and caches `1m` first, then returns either that base load or an in-memory resample to the requested display timeframe.

Current user-facing behavior therefore includes:

- request normalization in the Data panel and service layer before invoke

## Current Code Surface

- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- `src-tauri/src/polygon.rs`
- `web/src/stores/dataStore.actions.test.ts`

## Strengths Already Present

- market, symbol, date, adjusted, and timeframe semantics already live in backend request types
- frontend request normalization already keeps the invoke surface cleaner than raw form values
- the product rule “always cache 1m first” is already enforced at the command boundary
- top-off requests already derive from manifest dataset identity rather than ad hoc form state

## Weaknesses / Failure Modes

- requested timeframe versus on-disk cached timeframe is subtle and easy to misdocument
- symbol normalization rules for FX/crypto/native Polygon tickers are not obvious from the UI alone
- the same conceptual request family spans direct download and manifest-derived top-off
- there is no current dedicated doc that freezes default `adjusted=true` for stocks and other market-specific rules

## Regressions To Avoid

- documenting requested timeframe as the cache file timeframe
- dropping pair-style or native-colon symbol support by accident
- changing stock adjusted defaults without coordinating frontend and backend together
- breaking top-off request derivation from manifest-backed dataset identity
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Polygon Aggregates / Custom Bars](https://polygon.io/docs/rest/stocks/aggregates/custom-bars) is relevant because current request fields map directly onto Polygon aggregate requests.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because download requests behave like a versioned command payload crossing the frontend/backend boundary.
- [RFC 8259](https://www.rfc-editor.org/rfc/rfc8259) is relevant because the invoke payload is effectively a JSON request contract where null-versus-omitted field meaning matters.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180)
  - relevant because CSV-facing import, export, and repair behavior should remain interoperable and explicit instead of relying on provider-specific happy paths

## Proposed Improvement Approach

1. Document the request in three layers: frontend normalization, backend accepted contract, and the special top-off path.
2. Freeze the symbol normalization table for stocks, FX, crypto, and native ticker formats.
3. Make “always cache 1m, then resample” explicit in the contract rather than incidental code behavior.
4. Document stock-adjusted defaults and market-specific differences clearly.
5. Tie request changes back to both frontend tests and backend parsing tests.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [x] Document every request field and default.
- [x] Document supported market values and symbol forms.
- [x] Document requested timeframe versus cached base timeframe semantics.
- [x] Document top-off request derivation from manifest datasets.
- [x] Add or tighten tests if normalization rules or defaults change.
- [x] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [x] Break the work into the smallest additive implementation slices that can be verified independently.
- [x] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [x] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [x] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run `npm --prefix web run test -- src/services/dataService.test.ts`
- run targeted backend Polygon tests for request parsing and top-off request construction
- manually verify one stock, one FX/crypto, and one native-ticker request after any contract change
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current accepted symbol forms unless a migration is explicit
- prefer clearer documentation over silent narrowing of accepted requests
- keep base-1m caching as an explicit product rule until the architecture changes materially
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Execution Notes

- Added backend request-contract tests in `src-tauri/src/polygon.rs` for market-specific and native symbol normalization, covering stocks, FX pair notation, crypto pair notation, and native `X:` tickers without narrowing the accepted request forms.
- Added frontend service coverage in `web/src/services/dataService.test.ts` so request normalization now explicitly preserves native ticker and slash-pair shapes while still uppercasing symbols and defaulting blank timeframes to `1m`.
- Kept the command-layer product rule intact: the UI still requests display timeframes, while `polygon_download_1m_and_load` remains the authority that caches `1m` first and derives larger display timeframes later.
- Acceptance evidence captured during execution: `cargo test -p ta_desktop_demo_app polygon --lib`, `npm.cmd --prefix web run test -- src/services/dataService.test.ts src/stores/dataStore.actions.test.ts`, `npm.cmd --prefix web run lint`, `npm.cmd --prefix web run typecheck`, `cargo check -p ta_desktop_demo_app`, and `cargo fmt --all -- --check`.

## Related Docs

- `REPO_Improvement/02_data/14_polygon_chunking_limits_2026-04-09.md`
- `REPO_Improvement/02_data/20_polygon_dataset_topoff_2026-04-09.md`
- `REPO_Improvement/02_data/24_resample_rules_2026-04-09.md`
