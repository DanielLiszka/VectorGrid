# Repo CSV Discovery Plan

Last updated: 2026-04-09

## Purpose

Plan improvements to the repo-data discovery rules so workspace-root lookup, traversal limits, and recursive listing behavior stay explicit and safe on large trees.

## Scope Boundaries

In scope:

- workspace-root and repo-data directory lookup for CSV inventory
- recursive versus non-recursive listing
- max-file cap and truncation signaling
- sample CSV discovery as a distinct helper surface

Out of scope:

- user-picked arbitrary folder crawling outside the current command surface
- remote datasets
- options-specific data roots

## Current User-Facing Behavior

Repo CSV discovery currently happens through a bounded command-side search strategy. `src-tauri/src/commands_data.rs` looks for the workspace root or `data/` directory from current working directory, executable parent, and `CARGO_MANIFEST_DIR` fallbacks, then walks the repo data folder with `list_csvs_in_dir()`. `list_sample_csvs()` is a different path: it advertises a short curated set of known sample files under `src/data/` rather than crawling the repo.

Current behavior includes:

- path-root search from a small bounded ancestor walk instead of infinite probing
- sorted output after traversal
- `max_files` clamped to a bounded range to prevent runaway listing work
- explicit `truncated` signaling when the cap is hit
- a separate curated sample-CSV lane distinct from repo inventory

## Current Code Surface

- `src-tauri/src/commands_data.rs`
- `web/src/services/dataService.ts`
- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.ts`
- `web/src/components/DataWorkspacePanel.vue`

## Current Verification Surface

- `web/src/services/dataService.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `meta-service tests that exercise repo inventory consumers`

## Strengths Already Present

- repo discovery already has bounded root search and bounded file caps
- curated sample CSVs are intentionally separate from general repo listing
- the discovery path is command-owned instead of duplicated in frontend code
- sorted output keeps downstream consumers deterministic

## Weaknesses / Failure Modes

- root-discovery rules are encoded in helper functions but not yet frozen in documentation
- sample CSV discovery and repo inventory are easy to conflate without a dedicated plan
- there is no single doc that states the intended scale limits and traversal posture for large data trees
- some consumer code only sees normalized responses and not the discovery rules that produced them

## Regressions To Avoid

- turning bounded repo search into unrestricted recursive crawl by default
- breaking fallback root detection when running from built artifacts or unusual working directories
- merging curated sample CSV behavior with repo inventory behavior unintentionally
- changing truncation reporting without updating consumers

## External Research Summary

- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because discovery policy ultimately exists to locate CSV sources under a defined compatibility target.
- [Tauri Dialog Plugin](https://v2.tauri.app/plugin/dialog/) is relevant because repo discovery should stay distinct from user-picked path discovery surfaces already exposed elsewhere.
- [JSON Schema Getting Started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because discovery output behaves like a shared response contract with bounded optional fields.

## Proposed Improvement Approach

1. Document the exact repo-root lookup sequence and its bounded ancestor search.
2. Document curated sample-CSV discovery separately from repo inventory.
3. Freeze max-file, recursion, sorting, and truncation semantics.
4. Make it explicit which consumers need path-only listing and which need identity-enriched listing.
5. Preserve deterministic listing behavior across dev and built-runtime contexts.

## Checklist

- [ ] Document root-discovery fallbacks and ancestor-walk limits.
- [ ] Document the difference between `list_sample_csvs` and `list_repo_data_csvs`.
- [ ] Document recursive and cap behavior together with `truncated` semantics.
- [ ] Record current consumers of both discovery paths.
- [ ] Add tests if fallback root detection or traversal behavior changes.

## Verification Plan

- run targeted `dataService.test.ts` coverage for sample CSVs
- run meta-service tests for repo inventory consumers
- perform one manual command check from the current workspace root after any discovery refactor

## Rollout / Fallback / Compatibility Notes

- keep curated sample-CSV behavior intact unless a better user-facing replacement already exists
- prefer documenting current search rules before changing them
- avoid adding new discovery breadth without an explicit scale and perf review

## Related Docs

- `REPO_Improvement/02_data/01_repo_csv_inventory_2026-04-09.md`
- `REPO_Improvement/02_data/08_repo_data_browser_ux_2026-04-09.md`
- `REPO_Improvement/02_data/32_workflow_bootstrap_data_DATE.md`
