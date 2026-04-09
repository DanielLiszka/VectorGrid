## Purpose

Define the improvement plan for campaign-summary persistence and checkpoint-style rewriting so long-running research campaigns remain resumable, inspectable, and performance-aware without scattering summary-save logic across the command.

## Scope Boundaries

In scope:

- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/bin/ta_campaign_perf.rs`
- `web/src/services/metaService.ts`
- campaign summary JSON saving, repeated rewrites, progress events, and checkpoint-save diagnostics

Out of scope:

- redesigning campaign optimization/validation strategy
- general checkpoint robustness statistics in validation
- options-related campaign flows

## Current User-Facing Behavior

Research campaigns already persist a live-updated summary artifact:

- an initial `vectorgrid_campaign_summary` JSON is saved early
- progress payloads carry `summary_json_path`
- the same summary file is repeatedly rewritten as campaign state changes
- the frontend can reload or reveal the most recent summary path
- final command responses return the summary path and bytes

This gives campaigns real operational persistence, but the save logic is currently very dispersed.

## Current Code Surface

- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/bin/ta_campaign_perf.rs`
- `web/src/services/metaService.ts`
- `web/src/services/metaService.test.ts`
- `web/src/services/metaServiceCampaignResume.test.ts`
- `web/src/services/metaServiceCampaignStrategy.test.ts`

Key current ownership:

- `run_artifacts.rs` provides `suggest_campaign_summary_path()`
- `commands_campaign.rs` builds the summary, saves it initially, then rewrites it at many checkpoints
- `commands_campaign.rs` emits `summary_json_path` in progress payloads
- `ta_campaign_perf.rs` consumes summary-path results for performance-oriented runs
- `metaService.ts` persists and reloads the last seen summary path client-side

## Current Verification Surface

- `metaService.test.ts`
- `metaServiceCampaignResume.test.ts`
- `metaServiceCampaignStrategy.test.ts`
- backend campaign command tests in `commands_campaign.rs`
- `ta_campaign_perf.rs` as an operational verification surface

## Strengths Already Present

- campaign summary persistence already exists and is not an afterthought
- summary path is emitted through progress as well as final response
- the same summary file is reused rather than generating a confusing pile of partial files
- diagnostics already reserve space for checkpoint-save timing
- the frontend already has a path-based resume/reveal model for summaries

## Weaknesses / Failure Modes

- checkpoint persistence is spread across many repeated save call sites
- there is no dedicated checkpoint/summary manager abstraction
- `checkpoint_save_ms` is modeled but appears easy to leave underpopulated or inconsistently populated
- heavy rewrite behavior can become noisy or costly without measurement
- frontend summary recovery is still last-path based rather than library/discovery based

## Regressions To Avoid

- losing campaign resumability while refactoring summary saves
- changing summary path semantics mid-run so progress payloads and final response disagree
- weakening compatibility with existing `vectorgrid_campaign_summary` files
- introducing more write amplification without measurement or explicit benefit

## External Research Summary

- [JSON Schema overview](https://json-schema.org/overview/what-is-jsonschema) is relevant because campaign summaries are long-lived operational artifacts and should be treated as structured contracts.
- [serde_json::Value docs](https://docs.rs/serde_json/latest/serde_json/value/) are relevant because request and per-item detail currently flow through flexible JSON payloads inside the summary.
- [Tauri event namespace](https://v2.tauri.app/es/reference/javascript/api/namespaceevent/) is relevant because campaign progress and summary-path updates are event-shaped runtime signals consumed by the frontend.
- [Tauri: calling Rust](https://v2.tauri.app/develop/calling-rust/) is relevant because campaign summary save/load/reveal logic relies on stable Rust/TS command boundaries.
- [Cargo targets](https://doc.rust-lang.org/cargo/reference/cargo-targets.html) are relevant because `ta_campaign_perf.rs` is part of the intended operational/perf surface for this persistence path.

## Proposed Improvement Approach

1. Treat campaign summary saving as a first-class subsystem rather than repeated inline plumbing.
2. Define one clear contract for:
   - initial summary creation
   - in-place summary rewrites
   - progress-payload summary path emission
   - final response summary path
3. Consolidate repeated save operations behind a smaller helper or manager once tests pin the current semantics.
4. Make checkpoint-save timing diagnostic collection explicit and measurable.
5. Keep frontend “last summary path” behavior stable until a fuller summary-library surface exists.

## Checklist

- [ ] Document the current campaign summary lifecycle from initial save through final response.
- [ ] Audit every summary rewrite call site in `commands_campaign.rs`.
- [ ] Add or tighten tests ensuring progress payloads and final responses agree on `summary_json_path`.
- [ ] Decide whether `checkpoint_save_ms` should become a required populated diagnostic field.
- [ ] Add or tighten performance-oriented checks for summary-save overhead where practical.
- [ ] Consider extracting a dedicated campaign-summary checkpoint helper once semantics are fixed.

## Verification Plan

- run targeted campaign command tests in `commands_campaign.rs`
- run `metaService.test.ts`
- run `metaServiceCampaignResume.test.ts`
- run `metaServiceCampaignStrategy.test.ts`
- run `ta_campaign_perf.rs` or equivalent bounded perf checks when save frequency or checkpoint handling changes

## Rollout / Fallback / Compatibility Notes

- preserve the current summary file naming and path semantics
- keep incremental rewrite behavior until a measured replacement exists
- if helper extraction obscures current semantics, prefer clearer inline save checkpoints plus tests
- treat campaign summary persistence as operational state, not disposable debug output

## Related Docs

- `REPO_Improvement/06_artifacts/31_batch_summary_artifacts_2026-04-09.md`
- `REPO_Improvement/06_artifacts/22_meta_stage3_report_flow_2026-04-09.md`
- `REPO_Improvement/08_ml/22_meta_selection_request_contract_2026-04-08.md`
- `REPO_Improvement/10_tests_perf/25_headless_report_builders_2026-04-08.md`
