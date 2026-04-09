# CSV Header Compatibility Plan

Last updated: 2026-04-09

## Purpose

Define and improve the non-options CSV header-compatibility contract so supported datasets load predictably, unsupported layouts fail clearly, and existing VectorTA-style and Polygon-style imports remain backward-safe.

## Scope Boundaries

In scope:

- CSV header alias recognition
- named-column vs positional-column precedence
- quoted-field and malformed-row behavior
- user-facing import compatibility expectations

Out of scope:

- Polygon download APIs themselves
- options data import paths
- broad dataset-provenance or cache-policy work beyond what header compatibility directly affects

## Current User-Facing Behavior

The app accepts VectorTA-style positional CSV layouts and some named-header layouts, including Polygon-style raw minute aggregates. Named-header mode only activates when both timestamp and close aliases are recognized. If named-header recognition fails, the loader can fall back to positional interpretation.

## Current Code Surface

- `crates/app_core/src/csv_compat.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- `src-tauri/src/state.rs` tests that load CSV-backed data
- backend data/command tests that exercise compatibility loading

## Strengths Already Present

- there is already a dedicated compatibility layer instead of many ad hoc CSV readers
- Polygon raw CSV support already exists
- optional O/H/L/V fields are recognized when present
- positional fallback preserves compatibility with older or simpler CSV layouts

## Weaknesses / Failure Modes

- named-header support is narrow and only lightly documented
- unsupported named timestamp or close headers can silently fall back to positional mapping
- parsing uses raw `split(',')`, so quoted commas and embedded newlines are outside the real contract
- blank optional fields become `NaN`, but that semantic contract is not visible to users
- tests appear biased toward the Polygon happy path instead of the full compatibility matrix

## Regressions To Avoid

- breaking existing Polygon raw imports
- breaking existing VectorTA positional imports
- adding permissive alias handling that creates ambiguous or incorrect mapping
- silently changing blank-cell semantics for optional OHLCV fields
- hiding malformed-row failures until much later in the data pipeline
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) defines the baseline CSV expectations for quoting, commas, and record structure.
- [Polygon flat-file minute aggregates](https://massive.com/docs/flat-files/stocks/minute-aggregates) is the authoritative external layout the current compatibility layer already targets.
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/) is relevant because loader failures need a crisp frontend-facing contract instead of opaque backend behavior.
- [Rust `csv` crate documentation](https://docs.rs/csv/latest/csv/) is relevant if the repo chooses to move from manual splitting to a standards-aware parser without changing the visible contract.

## Proposed Improvement Approach

1. Publish the exact accepted alias table for timestamp, close, and optional OHLCV fields.
2. Document named-header precedence and define whether unsupported named layouts should hard fail instead of silently falling back to positional parsing.
3. Decide whether the repo intentionally remains outside full RFC 4180 support or whether the compatibility layer should move to a standards-aware parser.
4. Document blank-cell and missing-column semantics so downstream OHLCV capability decisions are explainable.
5. Expand tests to cover alias variants, mixed supported/unsupported headers, quoted rows, and malformed data.
6. Treat data-surface code quality as part of the implementation scope: preserve explicit schemas, path safety, provenance clarity, diagnostics, and testable normalization rules rather than relying on permissive parsing.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the exact supported alias table.
- [ ] Document the supported positional layout(s).
- [ ] Define named-header vs positional fallback precedence.
- [ ] Decide and document quoted-field support or non-support explicitly.
- [ ] Document blank-cell semantics for optional columns.
- [ ] Add tests for supported alias variants and unsupported named-header cases.
- [ ] Add tests for malformed rows and quoted-field edge cases.
- [ ] Verify frontend error messaging reflects the real compatibility contract.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.

## Verification Plan

- run targeted `ta_app_core` CSV/data-loading tests
- run targeted `src-tauri` data command/state tests
- run `npm test -- src/services/dataService.test.ts`
- manually load representative VectorTA-style and Polygon-style CSV samples if the parser contract changes
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend and Rust quality gates for touched data surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, and `cargo check --workspace` or a justified narrower target
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve current Polygon and VectorTA-compatible layouts as hard compatibility targets
- if unsupported named-header cases move from silent fallback to explicit failure, document that change clearly and keep error copy actionable
- if standards-aware parsing is introduced, preserve visible behavior for currently supported files before widening support
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/02_data/03_csv_load_contract_2026-04-08.md`
- `REPO_Improvement/02_data/05_csv_field_rules_2026-04-09.md`
- `REPO_Improvement/02_data/06_csv_timestamp_rules_2026-04-09.md`
- `REPO_Improvement/02_data/07_data_cache_sources_2026-04-08.md`
- `REPO_Improvement/02_data/27_dataset_meta_schema_2026-04-08.md`
