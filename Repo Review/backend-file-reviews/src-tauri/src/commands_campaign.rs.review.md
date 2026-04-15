# Review Packet: `src-tauri/src/commands_campaign.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_campaign.rs`
- Packet status: `Reviewed`
- Recommended action bucket: `Larger refactor but defer`
- Sensitivity tags: `Public-surface-sensitive`, `Runtime-critical`, `Semantics-sensitive`, `Benchmark-sensitive`

## What This File Owns

`commands_campaign.rs` is the backend orchestration hub for the research-campaign workflow.

This file owns:

- the public Tauri command `run_research_campaign`
- headless campaign entrypoints used outside the desktop invoke path
- the campaign request, summary, progress, and response DTOs
- strategy-family routing and support policy
- dataset preparation and preload orchestration
- skip/reuse policy over saved optimization artifacts
- validation candidate-selection and admission policy wiring
- progress emission over the `research_campaign_progress` event
- rolling campaign-summary checkpointing
- optional end-of-campaign `auto_meta` and `auto_stage5` follow-up chaining

This file does not own:

- optimization kernels or optimization math
- validation engine implementations
- the artifact persistence implementation itself
- the frontend request builder or progress UI

Those concerns are delegated to:

- `ta_app_core`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_optimize.rs`
- frontend orchestration in `web/src/services/metaService.ts`

## Classification

- Public surface: yes
  - `run_research_campaign` is a `#[tauri::command]` at `src-tauri/src/commands_campaign.rs:3746-3757`
  - headless entrypoints are also exported for non-UI use
- Hot path: not a numerical kernel hot path
- Runtime-critical: yes
  - this file owns cancellation, run-guard usage, progress emission, orchestration sequencing, and artifact checkpointing
- Benchmark-sensitive: yes
  - campaign throughput depends materially on preload, VRAM prefetch orchestration, artifact reuse decisions, and control-flow shape
- Strategy-local: no
  - it routes among strategy families, but does not own strategy-specific compute
- Support/orchestration code: yes, but large and high-risk

## Direct Interactions

### Public entrypoints

- `run_research_campaign`: `src-tauri/src/commands_campaign.rs:3746-3757`
- `run_research_campaign_blocking_headless`: `src-tauri/src/commands_campaign.rs:3684-3690`
- `run_research_campaign_optimize_only_blocking_headless`: `src-tauri/src/commands_campaign.rs:3692-3704`
- `preload_research_campaign_dataset_pack_headless`: `src-tauri/src/commands_campaign.rs:3706-3729`
- `run_research_campaign_optimize_only_preloaded_pack_blocking_headless`: `src-tauri/src/commands_campaign.rs:3731-3744`

These are re-exported in `src-tauri/src/commands.rs:53-59` and the Tauri command is registered in `src-tauri/src/lib.rs:152`.

### Core orchestration owners inside the file

High-signal internal helpers:

- strategy support and family routing:
  - `resolve_campaign_strategy_kind`: `src-tauri/src/commands_campaign.rs:584`
  - `optimization_family_for_kind`: nearby routing helper cluster
- progress/event bridge:
  - `emit_campaign_progress`: `src-tauri/src/commands_campaign.rs:932`
  - `CampaignProgressSink`: progress bridge around `src-tauri/src/commands_campaign.rs:513`
- dataset preparation:
  - `prepare_campaign_dataset`: `src-tauri/src/commands_campaign.rs:959`
  - preload worker/state cluster around `src-tauri/src/commands_campaign.rs:351-456`
- summary shaping:
  - `build_campaign_summary`: `src-tauri/src/commands_campaign.rs:1877`
  - `save_campaign_summary_blocking`: `src-tauri/src/commands_campaign.rs:1913`
- main workflow loop:
  - `run_research_campaign_blocking_with_mode`: `src-tauri/src/commands_campaign.rs:2083`

### Delegated backend modules

- optimization and validation execution:
  - `ta_app_core` imports and progress types at `src-tauri/src/commands_campaign.rs:23-29`
- meta and stage5 follow-up:
  - `src-tauri/src/commands_meta.rs`
- product-mode normalization and progress sink utilities:
  - `src-tauri/src/commands_optimize.rs`
- artifact save/load/reuse and campaign summary pathing:
  - `src-tauri/src/run_artifacts.rs`
- data-cache and app cache dir support:
  - `crate::data_cache`
- loaded-run/cancel token guard:
  - `crate::state::AppState`

### Frontend contract anchors

Local product contract anchors in `REPO_MAP.md:350-363` explicitly assign:

- frontend orchestration:
  - `web/src/services/metaService.ts`
  - `web/src/services/metaServiceCampaignStrategy.test.ts`
  - `web/src/services/metaServiceCampaignResume.test.ts`
  - `web/src/stores/taskStoreCampaignProgress.test.ts`
- backend orchestration:
  - `src-tauri/src/commands_campaign.rs`

Representative frontend callers and listeners:

- campaign request assembly and invoke:
  - `web/src/services/metaService.ts:2057-2133`
- resume/retry from summary artifacts:
  - `web/src/services/metaService.ts:2288-2304`
- progress event consumption:
  - `web/src/services/metaService.ts:2060-2064`
  - `web/src/tauri/instrumentation.ts`
  - `web/src/stores/taskStore.ts`
- tracked command and cancellation mapping:
  - `web/src/stores/taskStore.ts:271`

### Headless/perf harness anchor

- `src-tauri/src/bin/ta_campaign_perf.rs` directly uses the headless campaign entrypoints and listens to `research_campaign_progress`

## Standards and Research Log

This packet was written only after a targeted research pass against official sources relevant to a large Tauri orchestration surface.

### Sources consulted

1. Tauri command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why: this file exposes a long-running public Tauri command and must preserve the async/blocking boundary correctly
   - Relevant guidance:
     - async commands are preferred for heavy work
     - command work should avoid UI/main-thread blocking
     - `Result` is the expected command error shape

2. Rust Book error handling
   - Source: <https://doc.rust-lang.org/book/ch09-00-error-handling.html>
   - Why: this file coordinates many fallible steps and returns string errors over a large public workflow surface

3. Rust Book testing
   - Source: <https://doc.rust-lang.org/book/ch11-00-testing.html>
   - Why: the file has strong helper coverage but incomplete end-to-end orchestration coverage

4. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why: this file defines public DTOs and a high-impact workflow contract where predictability, dependability, and debuggability matter

5. Clippy documentation
   - Source: <https://doc.rust-lang.org/clippy/>
   - Why: this file is large enough that complexity and correctness lint pressure remains relevant

6. NVIDIA CUDA Best Practices Guide
   - Source: <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>
   - Why: this file explicitly orchestrates preload and Double-MA VRAM prefetch decisions, so throughput recommendations must stay evidence-backed and iterative rather than speculative

## Applicable Current Standards

- Tauri orchestration standard:
  - keep long-running workflow execution off the immediate command boundary
  - preserve stable request/response and progress-event contracts
- Repo campaign standard from `REPO_MAP.md`:
  - request contract owns dataset list, execution mode, preload policy, validation payload, and optional auto-meta / auto-stage5 follow-ups
  - progress payload owns stage labels, counts, and summary-path disclosure
  - saved campaign summary artifacts are the source of truth for retry, resume, and reveal behavior
  - supported strategy surface is narrower than the full strategy catalog
- Review standard for large orchestration files:
  - prioritize control-flow correctness, state/cancellation discipline, artifact continuity, and regression resistance over local style churn

## Findings

### Correctness / Safety

No single local correctness bug was identified from static review alone, but this file is correctness-sensitive because many workflow semantics are centralized here.

Positive findings:

- explicit strategy-support gating exists
- run-guard and cancel-token use are present on the public Tauri entrypoint
- progress emission and summary checkpointing are explicit rather than implicit
- helper tests cover several policy-heavy branches

Risk concentration:

- correctness here is mostly orchestration correctness, not arithmetic correctness
- small edits can silently alter:
  - skip-existing behavior
  - holdout handling
  - admission-policy selection
  - artifact reuse semantics
  - summary state and warning accounting
  - optional `auto_meta` / `auto_stage5` follow-ups

### API and Naming

No naming defect was identified that justifies churn.

The public request/response DTO names are explicit and already tied into frontend and headless harness usage. Renaming them would create disproportionate migration risk.

### Error Handling

The file returns `Result<_, String>` and coordinates a very large number of fallible branches.

Assessment:

- this is workable in the current repo pattern
- it is not ideal for a public orchestration surface of this size because typed failures and structured categories would improve diagnosability
- however, changing it locally would be a high-risk, cross-surface contract migration

Review conclusion:

- treat structured orchestration error transport as a broader deferred refactor, not a local isolated change

### Module Boundaries / Technical Debt

This is the strongest objective issue in the file.

Objective issue:

- `commands_campaign.rs` is a very large public orchestration module at `4388` lines
- it mixes:
  - public DTO definitions
  - progress payloads
  - dataset preload worker infrastructure
  - strategy-routing policy
  - validation-admission policy
  - artifact-reuse policy
  - summary shaping and checkpointing
  - headless entrypoints
  - Tauri entrypoint
  - local tests

Assessment:

- this breadth is real technical debt
- it raises review cost and regression risk
- it makes change isolation difficult
- it increases the chance that future maintenance edits will accidentally touch unrelated campaign concerns

Refactor posture:

- a boundary refactor is justified conceptually
- but it should be deferred because this file is high-risk, public, and throughput-sensitive
- the correct future split would likely separate:
  - DTOs and public contract types
  - preload/preparation infrastructure
  - candidate/admission policy helpers
  - summary/artifact checkpointing
  - execution loop / public entrypoints

### Docs / Examples

There is local contract structure, but much of it is implicit in code and tests rather than explicit documentation.

Positive point:

- `REPO_MAP.md` documents the campaign contract at a high level

Gap:

- the file itself does not provide a high-level internal maintainer map for the execution loop and helper clusters

Low-risk improvement if touched:

- add a short internal maintainer comment near `run_research_campaign_blocking_with_mode` describing the orchestration stages and which helper clusters own each one

### Tests / Missing Verification

This file has meaningful local Rust tests, which is a strength.

Strong coverage exists for:

- supported strategy matrix and rejection behavior
- optimization-family mapping
- backend normalization policy
- GPU-route and prefetch policy helpers
- candidate-admission behavior
- some preload lifecycle behavior

Most important missing coverage:

- no obvious direct end-to-end Rust test for the public campaign workflow path
- no obvious local Rust test that exercises the full sequence:
  - prepare
  - optimize
  - optional validate
  - artifact write/checkpoint
  - optional meta/stage5
  - final summary return

Assessment:

- helper coverage is good
- full-loop orchestration coverage is weaker than it should be for a file with this many responsibilities
- this is the second major review finding after module breadth

### Perf / Allocation / CUDA Concerns

This file is not a kernel or numeric hot path, but it is campaign-throughput-sensitive.

Review-relevant points:

- preload policy, data staging, and VRAM prefetch coordination live here
- artifact reuse and checkpointing frequency live here
- orchestration structure can change wall-clock campaign throughput even if compute kernels do not change

Implication:

- avoid performance “cleanup” here without evidence
- treat changes in preload behavior, reuse behavior, VRAM prefetch policy, or checkpoint cadence as benchmark-sensitive

## Recommended Action

`Larger refactor but defer`

Reasoning:

- a large-file/module-boundary problem is objectively present
- there is also a meaningful missing end-to-end orchestration regression net
- but this file is too central and too sensitive for opportunistic restructuring

Preferred current posture:

- preserve current behavior
- document the orchestration stages more clearly
- only make small, bounded fixes with strong verification
- defer a real modularization effort until it can be planned and verified as a dedicated campaign

## Safe-Now vs Sensitive Work

### Safe now

- add maintainer-oriented internal comments or packet references documenting helper ownership
- add a focused end-to-end headless regression test for the full orchestration loop if a safe deterministic harness can be built
- tighten narrow helper-level invariants if done without moving responsibilities around

### Semantics-sensitive

- strategy support matrix changes
- skip-existing or artifact reuse behavior
- validation admission-policy selection
- holdout routing and item status accounting
- summary-state construction and warning propagation
- `auto_meta` / `auto_stage5` chaining
- progress payload or event semantics

### Runtime-sensitive

- run-guard and cancel-token behavior
- preload worker lifecycle and synchronization
- checkpoint save timing/frequency
- any control-flow change inside `run_research_campaign_blocking_with_mode`

### Benchmark-sensitive

- preload-policy changes
- host-only vs host-plus-VRAM prefetch behavior
- campaign loop checkpoint cadence
- artifact reuse decisions that affect campaign throughput

## Cross-File Coordination

Any meaningful change to this file crosses multiple owned surfaces:

- campaign request assembly and progress consumption in `web/src/services/metaService.ts`
- progress aggregation in `web/src/stores/taskStore.ts`
- follow-up chaining in `src-tauri/src/commands_meta.rs`
- optimize-side normalization helpers in `src-tauri/src/commands_optimize.rs`
- artifact persistence and summary pathing in `src-tauri/src/run_artifacts.rs`
- headless throughput harness coverage in `src-tauri/src/bin/ta_campaign_perf.rs`
- app-core optimization and validation entrypoints

Practical implication:

- campaign changes should be planned as coordination work, not “just edit this file”
- request DTOs, progress payloads, saved summary shape, and reuse rules all move together

## Code-Quality and Debt Register

The highest-value quality improvements are structural, but they need staging.

Concrete split candidates:

- DTOs and public contract types
- preload/preparation infrastructure
- admission/reuse policy helpers
- summary checkpointing and artifact shaping
- execution-loop control flow

Safe code-quality work:

- add a maintainer-oriented map of helper clusters
- add stronger characterization coverage for the current orchestration loop

Debt that should be deferred until behavior is more tightly frozen:

- large-scale file splitting
- moving policy ownership out of the orchestration file without a dedicated migration pass

## Technical-Debt Reduction Staging

`Phase 1`

- add orchestration characterization tests
- add an internal maintainer map near the main workflow loop
- tighten pure helper invariants without moving ownership

`Phase 2`

- extract summary/checkpoint helpers
- extract preload/preparation helpers
- keep behavior identical and verification broad

`Phase 3`

- extract admission/reuse policy modules only after current behavior is frozen by tests and throughput evidence
- revisit DTO/public-contract extraction only as a dedicated contract-preservation change

## Performance Opportunity Register

This file is one of the few orchestration modules where performance planning is meaningful.

The useful opportunities are measurement-first:

- preload hit rate and real user value
- VRAM prefetch benefit vs complexity
- checkpoint cadence cost
- artifact reuse effectiveness
- `auto_meta` / `auto_stage5` chaining overhead

Preferred evidence surface:

- `src-tauri/src/bin/ta_campaign_perf.rs`

Guideline:

- do not ship throughput-oriented refactors here without measurement
- performance work should target stage boundaries and policy choices, not micro-optimizing line-level Rust

## Implementation Staging

`Safe now`

- maintainers comments
- characterization tests
- narrow helper hardening

`Coordination required`

- request DTO changes
- progress payload changes
- artifact summary schema changes
- supported-strategy policy changes
- reuse/admission policy changes

`Defer`

- major module split
- ownership migration of policy modules
- checkpoint/preload redesign without perf evidence

## Change Blast Radius

- `Local only`
  - comments
  - narrow helper-level hardening
- `Same subsystem`
  - internal helper extraction that preserves public behavior
- `Cross backend/frontend contract`
  - `run_research_campaign` request/response
  - `research_campaign_progress`
  - saved summary artifact shape
  - resume/retry behavior
  - headless harness entrypoints
  - meta/stage5 follow-up behavior

## Observability Gaps

The current file is control-plane heavy, but observability is still thinner than ideal.

Notable gaps:

- no first-class per-stage timing summary
- no preload cache hit/miss reporting
- no explicit artifact-reuse reason codes for later diagnosis
- no checkpoint latency visibility
- no clear success-vs-warning telemetry for `auto_meta` and `auto_stage5`

These gaps matter because they slow down safe refactors and make throughput changes harder to justify.

## Open Questions and Assumptions

- Assumption:
  - current supported-strategy and rejected-strategy policy is intentional product policy, not incidental implementation
- Open question:
  - should `auto_meta` / `auto_stage5` follow-up failures remain warning-like, or should some become campaign-fatal?
- Open question:
  - is the saved summary schema/versioning stable enough for long-lived resume across future refactors?
- Open question:
  - should reuse/admission policy remain centralized here, or is there enough stability to move it into dedicated modules later?

## Exact Local Verification Required If Changed

If this file changes, run at minimum:

1. Backend confidence:
   - `cargo test --workspace`

2. Local campaign helper coverage:
   - `cargo test -p ta_desktop_demo_app commands_campaign`

3. Frontend campaign request/progress contract coverage:
   - `npm --prefix web run test -- src/services/metaServiceCampaignStrategy.test.ts src/services/metaServiceCampaignResume.test.ts src/stores/taskStoreCampaignProgress.test.ts src/tauri/instrumentation.test.ts`

4. Broad workflow regression evidence:
   - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`

Recommended additional evidence if preload or throughput behavior changes:

5. Campaign harness evidence:
   - use `src-tauri/src/bin/ta_campaign_perf.rs` as supplemental local evidence for preload/prefetch behavior

## Regression and Feature-Preservation Constraints

Any future change to this file must preserve:

- the `run_research_campaign` request/response contract
- the `research_campaign_progress` payload semantics
- current supported-strategy and rejected-strategy behavior unless product policy changes intentionally
- saved campaign summary resumability
- artifact reuse/fallback rules unless deliberately revised and revalidated
- headless harness compatibility used by `ta_campaign_perf.rs`

This file should not be casually simplified. It is a central control-plane module where feature preservation and regression control matter more than local elegance.
