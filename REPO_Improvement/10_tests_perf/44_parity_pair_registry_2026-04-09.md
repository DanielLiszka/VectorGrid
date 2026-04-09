# Parity Pair Registry Plan

Last updated: 2026-04-09

## Purpose

Define the checked-in registry of baseline/candidate artifact pairs that the parity checker is allowed to compare.

## Scope Boundaries

In scope:

- the pair registry `scripts/output_parity_pairs.json`
- baseline/candidate pairing rules for report JSON and XLSX outputs
- base-dir and path-shape expectations consumed by `check_output_parity.mjs`
- review rules for adding or removing parity pairs

Out of scope:

- options-related parity pairs
- the generation pack itself
- full semantic correctness of artifacts
- general file-manifest management unrelated to parity checking

## Current User-Facing Behavior

The current pair registry is intentionally small: it currently maps one report JSON pair and one report XLSX pair. That narrowness is good because every pair in the registry implies maintenance overhead and interpretive meaning. The registry therefore needs a doc that protects it from becoming a grab-bag of loosely related outputs.

## Current Code Surface

- `scripts/output_parity_pairs.json` is the current checked-in pair registry.
- `scripts/check_output_parity.mjs` consumes the registry and applies shape-specific comparisons.
- `scripts/headless_output_parity_pack.json` provides the candidate-generation side that produces the paired artifacts.
- `scripts/baselines/samples/` holds the checked-in baseline artifacts referenced by the registry.

## Current Verification Surface

- `node scripts/check_output_parity.mjs --pairs scripts/output_parity_pairs.json`
- `node scripts/run_output_parity_check.mjs --pack scripts/headless_output_parity_pack.json --pairs scripts/output_parity_pairs.json`
- manual review of the registry diff when pairs are added, removed, or repointed

## Strengths Already Present

- the registry is explicit and reviewable
- the current small pair set keeps parity meaning understandable
- the checker already reads the registry rather than hardcoding pairs in code
- baseline artifacts live in a clear checked-in location

## Weaknesses / Failure Modes

- registry coverage is intentionally incomplete and could be mistaken for exhaustive coverage
- base-dir and path assumptions can drift quietly if not documented
- a pair can remain present even if its semantic representativeness has weakened
- adding too many pairs would make parity maintenance noisy and shallow

## Regressions To Avoid

- growing the pair registry without a strong justification for each new pair
- repointing a pair to a different artifact without documenting why the pair still matters
- treating pair presence as proof of semantic correctness
- forgetting to update related docs when pair identities change
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [JSON Schema getting started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the pair registry is another versioned contract artifact with machine-readable meaning.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because some candidate/baseline artifacts involved in the broader sample set are CSV-shaped and require clear format assumptions.
- [rust_xlsxwriter docs](https://rustxlsxwriter.github.io/index.html) are relevant because one registry entry points to a workbook whose structure is part of parity review.
- [Node.js `path` API docs](https://nodejs.org/api/path.html) are relevant because the registry relies on relative-path interpretation through a base directory.
- [Clippy](https://github.com/rust-lang/rust-clippy)
  - relevant because public-repo quality work should include explicit static-analysis expectations alongside behavioral tests and perf gates

## Proposed Improvement Approach

1. Keep the pair registry intentionally small and representative.
2. Treat every pair addition or change as a contract review event, not a casual fixture update.
3. Document what each pair is meant to prove structurally.
4. Use the registry in conjunction with the pack and checker, not as a standalone proof surface.
5. Treat test/perf code quality as part of the implementation scope: prefer stable, reviewable gates, exact commands, environment provenance, and raw evidence capture over brittle harness sprawl.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current pairs and what each one proves structurally.
- [ ] Document base-dir and relative-path expectations.
- [ ] Keep the pair registry non-options only.
- [ ] Require rationale for every added or removed pair.
- [ ] Cross-link pair changes to the pack and parity-checker docs.
- [ ] Avoid implying exhaustive artifact coverage from the registry.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Record the raw evidence expected for completion: exact commands, hardware/toolchain provenance, output artifact location, and refresh trigger for this check.

## Verification Plan

- run the direct pair checker after registry changes
- run the full parity check with the pack when candidate-generation paths are involved
- inspect registry diffs carefully for path or artifact-identity changes
- confirm baseline files still exist and match the intended sample set
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- capture the exact commands, environment, raw outputs, and refresh reason needed to prove this verification or perf surface still deserves trust in a public repo

## Rollout / Fallback / Compatibility Notes

- preserve the current narrow registry until a new pair is clearly justified
- prefer stronger rationale for a few pairs over a sprawling registry
- if a pair becomes obsolete, remove it deliberately and explain the replacement coverage
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/10_tests_perf/43_output_parity_pack_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/45_artifact_report_shape_parity_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/05_baseline_samples_manifest_2026-04-09.md`
