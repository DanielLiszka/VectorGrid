# Output Parity Pack Plan

Last updated: 2026-04-09

## Purpose

Define the checked-in headless output parity pack that regenerates comparable report artifacts for structural parity review.

## Scope Boundaries

In scope:

- the pack `scripts/headless_output_parity_pack.json`
- the headless producer bin `src-tauri/src/bin/ta_headless_validate.rs`
- the relationship between pack outputs and the parity-pair registry
- build-profile, timing-boundary, and output-path meaning inside the pack

Out of scope:

- options-related parity packs
- benchmark authority claims
- general report semantics outside structural parity
- the pair-registry file itself, which has its own doc

## Current User-Facing Behavior

The repo already keeps a checked-in pack that describes a deterministic headless validation/report generation run: package, bin, build profile, expected outputs, and the exact argument list. It is already annotated as supplemental structural parity rather than a perf gate. That is the right posture, but the pack still needs a stewardship doc so it is not treated as just another random JSON config.

## Current Code Surface

- `scripts/headless_output_parity_pack.json` is the current authoritative parity-pack descriptor.
- `src-tauri/src/bin/ta_headless_validate.rs` is the headless producer that the pack targets.
- `scripts/run_output_parity_check.mjs` consumes the pack alongside the pair registry to run structural checks.
- `_write_test/headless_reports/` is the current candidate output area referenced by the pack.

## Current Verification Surface

- `node scripts/run_output_parity_check.mjs --pack scripts/headless_output_parity_pack.json --pairs scripts/output_parity_pairs.json`
- manual inspection of generated headless report artifacts under `_write_test/headless_reports/`
- targeted report/XLSX tests when structural parity points to a deeper regression

## Strengths Already Present

- the pack makes the headless parity generation path explicit and checked in
- it already declares itself supplemental rather than a perf gate
- the argument list is stable and reviewable
- the pack cleanly separates generation inputs from parity comparison logic

## Weaknesses / Failure Modes

- the pack is only as representative as the chosen headless scenario
- path assumptions under `_write_test` can drift if not maintained carefully
- the pack does not by itself guarantee semantic report correctness
- it is easy to forget that build profile and output paths are part of the contract

## Regressions To Avoid

- changing pack arguments or output paths without updating the pair registry and docs
- treating the pack as proof of report correctness beyond structural parity
- removing the explicit supplemental timing-boundary note from the pack
- using the pack for product-performance claims

## External Research Summary

- [JSON Schema getting started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because the parity pack behaves like a versioned contract artifact.
- [Cargo profiles reference](https://doc.rust-lang.org/cargo/reference/profiles.html) are relevant because the pack explicitly names the build profile used by the headless producer.
- [rust_xlsxwriter docs](https://rustxlsxwriter.github.io/index.html) are relevant because one of the parity outputs is an XLSX workbook whose structure matters.
- [RFC 4180](https://www.rfc-editor.org/rfc/rfc4180) is relevant because parity output stewardship includes CSV-shaped artifacts adjacent to the report surfaces.

## Proposed Improvement Approach

1. Treat the parity pack as a versioned headless-generation contract rather than an incidental helper file.
2. Keep the pack narrow, deterministic, and explicit about build profile and output paths.
3. Tie pack changes to pair-registry review and parity-doc updates.
4. Use the pack for structural regeneration only, with semantics validated elsewhere.

## Checklist

- [ ] Document each field in the current parity pack and why it matters.
- [ ] Document how the pack relates to the pair registry and checker.
- [ ] Keep the parity pack non-options only.
- [ ] Preserve the supplemental timing-boundary note and output-path clarity.
- [ ] Review pack diffs carefully before updating related baselines.
- [ ] Avoid expanding the pack into a generic scenario catalog without governance.

## Verification Plan

- run the parity check command after pack changes
- inspect regenerated artifacts under `_write_test/headless_reports/`
- confirm the pack still points at the intended non-options headless validation path
- cross-check the pair registry after any output-path change

## Rollout / Fallback / Compatibility Notes

- preserve the current single-pack discipline while improving documentation
- add new packs only if a genuinely separate parity lane is needed
- prefer explicit versioning over silent reinterpretation of pack fields

## Related Docs

- `REPO_Improvement/10_tests_perf/44_parity_pair_registry_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/45_artifact_report_shape_parity_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/25_headless_report_builders_2026-04-08.md`
