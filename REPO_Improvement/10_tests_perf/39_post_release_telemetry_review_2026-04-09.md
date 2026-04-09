# Post-Release Telemetry Review Plan

Last updated: 2026-04-09

## Purpose

Define the post-release supplemental review script and keep it clearly separate from the main perf-authority gates.

## Scope Boundaries

In scope:

- the script `scripts/post_release_telemetry_review.mjs`
- supplemental checks against workflow summaries, UI metrics, memory telemetry, and baseline sample manifests
- generated Markdown/JSON review artifacts under `scripts/baselines/post_release/`
- the distinction between post-release review and hard pre-release gates

Out of scope:

- options-related telemetry
- authoritative CPU or GPU perf gating
- production telemetry collection from deployed user environments
- broad automated release decision-making without human review

## Current User-Facing Behavior

The current post-release review script reads a workflow baseline and summary, UI capture metrics, optimization memory telemetry, and the baseline sample manifest, then emits a supplemental review result plus Markdown summary. It already labels itself as supplemental rather than authoritative, which is correct, but it still deserves a dedicated plan because the script can fail for missing artifacts or coarse thresholds even when the main perf gates are fine.

## Current Code Surface

- `scripts/post_release_telemetry_review.mjs` aggregates several existing evidence surfaces into one supplemental review.
- `scripts/baselines/workflow_regression_baseline.json` and `target/workflow_regression_summary.json` provide workflow context.
- `scripts/baselines/ui_capture/perf_metrics.json` and `scripts/baselines/ui_capture/optimization_memory_metrics.json` provide UI and host-memory inputs.
- `scripts/baselines/samples/manifest.json` provides baseline sample/artifact presence checks.

## Current Verification Surface

- `node scripts/post_release_telemetry_review.mjs`
- generated JSON and Markdown review files under `scripts/baselines/post_release/`
- manual comparison against the underlying source artifacts when the review flags something unexpected

## Strengths Already Present

- the script already composes multiple evidence surfaces without pretending they are one benchmark
- it writes human-readable Markdown in addition to JSON
- it explicitly frames itself as supplemental review
- it helps reviewers notice missing artifacts or obviously suspicious drift before release handoff

## Weaknesses / Failure Modes

- it depends on several upstream artifacts already being present
- its thresholds are coarse and therefore advisory rather than authoritative
- it can fail because an input artifact is missing even when the underlying app behavior is sound
- the name “telemetry review” can sound more authoritative than the script really is if the doc is weak

## Regressions To Avoid

- treating post-release review as equivalent to the main perf gates
- tightening thresholds until the script becomes a brittle pseudo-gate without the same rigor as the authority scripts
- using the script output without inspecting which upstream artifact actually triggered a failure
- dropping the explicit supplemental framing from the generated Markdown or docs

## External Research Summary

- [Playwright best practices](https://playwright.dev/docs/best-practices) are relevant because one of the upstream inputs is broad UI-capture evidence that should remain review-oriented, not benchmark authority.
- [JSON Schema getting started](https://json-schema.org/learn/getting-started-step-by-step) is relevant because this script effectively composes several versioned artifact contracts.
- [Cargo profiles reference](https://doc.rust-lang.org/cargo/reference/profiles.html) is relevant because the upstream workflow and telemetry inputs may come from different build profiles.
- [Criterion.rs book](https://bheisler.github.io/criterion.rs/book/) is relevant as a contrast because this script aggregates evidence and thresholds but does not provide the same statistical benchmark authority as Criterion-based gates.

## Proposed Improvement Approach

1. Keep post-release telemetry review explicitly supplemental and artifact-composition oriented.
2. Use it to catch missing evidence and obvious drift, not to replace the main gates.
3. Document each upstream source artifact and what a failing check actually means.
4. Treat unexpected failures as prompts to inspect source artifacts before adjusting thresholds.

## Checklist

- [ ] Document every upstream artifact the review expects.
- [ ] Document the meaning and limits of each check family.
- [ ] Keep the supplemental framing explicit in both code and docs.
- [ ] Keep post-release review non-options only.
- [ ] Review failures at the source-artifact level before changing thresholds.
- [ ] Cross-link this review to the main authority gates and telemetry inputs.

## Verification Plan

- run the script after the upstream workflow, UI, memory, and sample artifacts are available
- inspect the generated Markdown and JSON together
- trace any failure back to the specific upstream artifact before making a decision
- confirm the script still writes outputs even when some checks fail

## Rollout / Fallback / Compatibility Notes

- preserve the current supplemental posture and improve explanation around it
- prefer better provenance and source-artifact review over more thresholds
- if the script becomes noisy, narrow the checks deliberately rather than pretending they are authoritative

## Related Docs

- `REPO_Improvement/10_tests_perf/28_workflow_gate_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/37_optimization_memory_capture_2026-04-09.md`
- `REPO_Improvement/10_tests_perf/08_fe_ui_baseline_capture_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/07_local_vs_ci_authority_2026-04-09.md`
