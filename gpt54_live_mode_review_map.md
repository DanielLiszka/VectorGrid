# GPT-5.4 live-mode implementation-plan review map

This file is a repo-grounded map for sending the live-mode implementation plans to another model for critique without requiring that model to ingest the whole repository.

It is intentionally narrower than a general feature-review map.

## Purpose

Use this document when you want a reviewer model to:

- critique the live-mode implementation plans before coding
- identify missing scope boundaries, missing safety details, and hidden assumptions
- identify rollout, migration, and verification gaps
- flag stale repo references or file-owner mistakes inside the plans
- suggest additive, low-regression improvements that fit the current repo posture

Do not use this document for broad live-trading ideation.

## Why this is worth doing

The current repo already has local context and a real, narrow live-execution surface. A separate GPT-5.4 Pro review pass can still be useful, but only if it is:

- tightly scoped
- repo-grounded
- explicit about current limitations
- prevented from inventing a broader trading platform than the repo actually implements

The biggest failure mode is context drift. This map is designed to limit that drift.

## Hard scope guard

The current live posture is narrow and should stay narrow during plan review.

The reviewer prompt should not drift into:

- options live trading
- multi-broker expansion
- multi-symbol parallel orchestration
- compliance-grade archival or enterprise audit systems
- portfolio dashboards or OMS redesign
- HFT or latency-marketing claims
- broad shell redesign outside the live-execution surface
- generic product ideation that is not required by the current plans

Out of scope unless a provided file proves it is required:

- new live strategy families beyond the current supported scope
- cloud sync for credentials, sessions, or broker state
- replacing the existing artifact system with a new platform
- replacing Tauri/Vue state ownership with a broad architecture rewrite

## Current repo live posture

The reviewer should assume these repo truths unless the provided files contradict them:

1. Live execution is real, but narrow.
2. The app currently supports live execution for the current non-options path only.
3. The current live stack is centered on:
   - `web/src/components/ExecutionWorkspacePanel.vue`
   - `web/src/stores/executionStore.ts`
   - `web/src/services/executionService.ts`
   - `src-tauri/src/commands_execution.rs`
   - `src-tauri/src/state.rs`
   - `src-tauri/src/lib.rs`
4. Broker integration is currently Alpaca-oriented.
5. Live market streaming is currently Polygon-oriented.
6. The current model is still single-stream and single-runner.
7. Live execution is still mounted inside the unified app shell rather than a fully separate top-level app.
8. The execution chart/account review surface is not yet fully complete.
9. Remembered live broker credentials are an identified hardening gap.

## Parallel review model

The goal is to let you run reviews in parallel with minimal dependency between prompts.

### Recommended execution shape

For each live bundle, use three prompts:

1. First-pass prompt
2. Second-pass challenge prompt
3. Bundle-local consolidation prompt

### Parallelism rule

- The first-pass prompt for every bundle can run in parallel.
- The second-pass prompt for every bundle can run in parallel.
- Each bundle-local consolidation prompt depends only on:
  - the original bundle packet
  - that bundle's first-pass output
  - that bundle's second-pass output
- No bundle depends on any other bundle.

That means you can run all first-pass prompts in parallel, all second-pass prompts in parallel, and later consolidate bundle by bundle in parallel as well.

### Fast path

If you want a cheaper sweep:

- run only the first-pass prompt for all bundles
- skip second-pass and consolidation unless a bundle produces questionable or high-risk feedback

### Highest-rigor path

If you want the strongest review:

- run first-pass and second-pass in parallel for every bundle
- run each bundle-local consolidation prompt
- then optionally run one cross-bundle synthesis prompt across all consolidated outputs

## Common prompt rules for GPT-5.4 Pro

Use these rules for every bundle.

- Base the review only on the files I provide and any official documentation explicitly cited by those files.
- Prefer repo-grounded critique over product brainstorming.
- Do not propose broad new product scope unless directly required to make the current plan internally coherent.
- Prefer smaller, additive, low-regression changes over large redesigns.
- Distinguish clearly between:
  - confirmed doc gap
  - likely issue
  - open question
  - optional improvement
- Cite the file and section or module for every material claim.
- If a plan references a stale or nonexistent repo file, flag it explicitly.
- Prioritize:
  - operator safety and truthfulness
  - migration clarity
  - backend/frontend ownership clarity
  - rollback and verification completeness
  - test gaps
  - repo-context drift
- Do not give generic praise.

If using Deep Research or browsing, restrict web sources to official docs only:

- Alpaca docs
- Polygon or Massive docs
- Tauri docs
- Vue docs
- Lightweight Charts docs
- Rust crate docs for directly implicated crates such as `keyring`

Do not use blog posts, generic trading-system advice, or compliance marketing pages.

## Common output format

Ask the model to return:

1. Surface understanding
2. Findings
3. Missing details or assumptions
4. Verification and rollout gaps
5. Suggested doc changes
6. Recommended implementation order adjustments

For each finding, require:

- Severity: critical / high / medium / low
- Type: scope / safety / correctness / operator-truth / migration / verification / testing / maintainability
- Location: file + section or file + module
- Why it matters
- Recommended change

## Common context packet

Use this common packet for every bundle unless context pressure forces you to trim it.

Always include:

- `AGENTS.md`
- `REPO_MAP.md`
- `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`
- the target live-mode bundle doc

Recommended always-include live owner files:

- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/lib.rs`

If `REPO_MAP.md` is too large for the prompt budget, prefer excerpting the live/shell sections instead of dropping it completely. The highest-signal areas are around:

- the frontend shell overview near the Vue workspace shell and `App.vue`
- the note that the settings workspace currently mounts the execution panel
- the live-execution scope and current constraints

## Prompt template: first pass

```text
You are reviewing one live-mode implementation-plan document from my Rust/Tauri trading desktop app.

Your job is to critique the plan for:
1. missing scope boundaries
2. hidden assumptions about the current repo behavior
3. operator-safety and truthfulness gaps
4. migration, rollout, and rollback gaps
5. verification and test gaps
6. stale repo references or missing owner files
7. additive, low-regression improvements to the plan

Important constraints:
- Base the review only on the files I provide.
- Treat the current live scope as narrow: current broker path, current feed path, single-stream, single-runner, non-options only.
- Do not propose broad new product families, portfolio dashboards, multi-broker expansion, options live trading, compliance platforms, or cloud sync unless directly required by the provided files.
- Prefer smaller, additive changes over redesigns.
- Distinguish clearly between:
  - confirmed doc gap
  - likely issue
  - open question
  - optional improvement
- Cite the file and section or module for every material claim.
- If a plan references a stale or missing file, call that out explicitly.
- Do not give generic praise.

For each finding, use:
- Severity
- Type
- Location
- Why it matters
- Recommended change

Then provide:
1. Surface understanding
2. Findings
3. Missing details or assumptions
4. Verification and rollout gaps
5. Suggested doc changes
6. Recommended implementation order adjustments

Bundle under review:
[paste bundle name here]

Files to review:
[paste file list here]

Bundle-specific focus:
[paste bundle-specific focus here]
```

## Prompt template: second-pass challenge

```text
You are doing an adversarial second-pass critique of one live-mode implementation-plan document from my Rust/Tauri trading desktop app.

Assume the plan may be overconfident, context-incomplete, or missing dangerous edge cases.

Your job is to stress-test the plan for:
1. hidden operator-risk or truthfulness problems
2. migration traps and ambiguous ownership
3. stale assumptions about the current repo
4. rollout paths that could silently weaken safety or trust
5. verification plans that are too weak for the proposed change
6. contradictions between the plan and the provided owner files

Important constraints:
- Base the review only on the files I provide.
- Stay inside the current narrow live scope unless the files prove otherwise.
- Do not turn this into a generic product brainstorming exercise.
- Prefer identifying dangerous omissions and contradictions over suggesting lots of new features.
- Distinguish clearly between:
  - confirmed doc gap
  - likely issue
  - open question
  - optional improvement
- Cite the file and section or module for every material claim.
- Flag any stale or nonexistent file references explicitly.
- Do not give generic praise.

For each finding, use:
- Severity
- Type
- Location
- Why it matters
- Recommended change

Then provide:
1. Surface understanding
2. Findings
3. Dangerous assumptions or contradictions
4. Verification and rollback weaknesses
5. Must-fix doc changes before implementation
6. Nice-to-have doc improvements

Bundle under review:
[paste bundle name here]

Files to review:
[paste file list here]

Bundle-specific challenge focus:
[paste bundle-specific challenge focus here]
```

## Prompt template: bundle-local consolidation

This prompt is the only serial step inside one bundle. It still does not depend on any other bundle.

```text
You are consolidating two prior critiques of one live-mode implementation-plan document from my Rust/Tauri trading desktop app.

Your inputs are:
1. the original bundle files
2. a first-pass critique
3. a second-pass adversarial critique

Your job is to produce one merged, repo-grounded conclusion.

Important constraints:
- Base the consolidation on the provided files first, and use the two prior critiques only as candidate claims to validate.
- Keep the current live scope narrow unless the files prove otherwise.
- Reject unsupported suggestions even if they sound attractive.
- Separate must-fix items from optional improvements.
- Cite the file and section or module for every material claim.
- Flag stale or nonexistent file references explicitly.

Return:
1. Surface understanding
2. Consolidated must-fix findings
3. Consolidated should-fix findings
4. Optional improvements worth considering
5. Suggestions to reject as out of scope or unsupported
6. Recommended doc edits
7. Recommended implementation order
```

## Optional prompt: cross-bundle synthesis

Use this only after you already have consolidated outputs for the bundles you care about.

```text
You are synthesizing multiple consolidated critiques for the live-mode implementation plans in my Rust/Tauri trading desktop app.

Your job is to identify:
1. contradictions across bundles
2. missing shared dependencies or shared prerequisites
3. implementation-order changes
4. overlap that should be merged or clarified
5. any missing plan that still appears necessary

Important constraints:
- Base the synthesis on the provided bundle docs, owner files, and consolidated critiques.
- Do not invent broad new product scope.
- Prefer repo-grounded sequencing and dependency cleanup over brainstorming.
- Cite the file and section or module for every material claim.

Return:
1. Program-level understanding
2. Cross-bundle contradictions
3. Shared prerequisites
4. Recommended execution order
5. Any missing plan still worth adding
6. Suggested edits to the umbrella live productization doc
```

## Bundle 05: live execution productization umbrella

### Required files

- `AGENTS.md`
- `REPO_MAP.md`
- `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/lib.rs`

### Useful optional files

- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-08.md`
- `REPO_Improvement/09_live/02_execution_workspace_ux_2026-04-09.md`
- `REPO_Improvement/09_live/06_broker_connectivity_2026-04-09.md`
- `REPO_Improvement/09_live/07_live_candle_stream_data_2026-04-08.md`
- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`

### Bundle-specific focus

- check whether the umbrella plan is missing any major live-mode hardening lane
- verify that the plan reflects the current narrow repo scope honestly
- check whether the planned child docs cover the real gaps without drifting into a trading-platform rewrite

### Bundle-specific challenge focus

- try to find hidden gaps in operator truth, current scope, or execution order
- check whether the umbrella plan still underspecifies credentials, chart/account review, or shell ownership

## Bundle 06: operator safety controls

### Required files

- `AGENTS.md`
- `REPO_MAP.md`
- `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`
- `REPO_Improvement_Epics/01_epics/06_live_execution_operator_safety_controls_2026-04-11.md`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `web/src/utils/executionRisk.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/lib.rs`

### Useful optional files

- `REPO_Improvement/09_live/05_execution_safety_modes_2026-04-09.md`
- `REPO_Improvement/09_live/03_execution_runner_lifecycle_2026-04-08.md`
- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`

### Bundle-specific focus

- confirm whether the doc cleanly separates stop runner, stop feed, cancel open orders, flatten, and emergency halt
- check backend-authoritative ownership and confirmation posture

### Bundle-specific challenge focus

- look for ambiguous control semantics that could mislead an operator about broker state
- look for missing disabled-state or blocked-reason rules

## Bundle 07: session audit trail

### Required files

- `AGENTS.md`
- `REPO_MAP.md`
- `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`
- `REPO_Improvement_Epics/01_epics/07_live_execution_session_audit_trail_2026-04-11.md`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/state.rs`
- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/stores/workflowArtifactsStore.ts`

### Useful optional files

- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`
- `REPO_Improvement/06_artifacts/04_artifact_hydration_contract_2026-04-09.md`
- `REPO_Improvement/06_artifacts/24_history_library_core_2026-04-09.md`

### Bundle-specific focus

- determine whether the plan defines a compact, truthful session artifact contract
- verify redaction, provenance, and reopen behavior

### Bundle-specific challenge focus

- look for hidden artifact-ownership drift, secret leakage risk, or a second artifact truth system

## Bundle 08: recovery and reconnect

### Required files

- `AGENTS.md`
- `REPO_MAP.md`
- `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`
- `REPO_Improvement_Epics/01_epics/08_live_execution_recovery_and_reconnect_2026-04-11.md`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/lib.rs`
- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`

### Useful optional files

- `REPO_Improvement/09_live/03_execution_runner_lifecycle_2026-04-08.md`
- `REPO_Improvement/09_live/07_live_candle_stream_data_2026-04-08.md`
- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`

### Bundle-specific focus

- confirm restart, reconnect, and stale-state behavior is conservative and explicit
- verify separation of feed state, runner state, and broker snapshot state

### Bundle-specific challenge focus

- look for any path that could silently re-arm live behavior after interruption
- look for missing stale-symbol or stale-broker-state detection

## Bundle 09: discrete surface promotion

### Required files

- `AGENTS.md`
- `REPO_MAP.md`
- `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`
- `REPO_Improvement_Epics/01_epics/09_live_mode_discrete_surface_promotion_2026-04-11.md`
- `web/src/App.vue`
- `web/src/components/VueWorkspaceShell.vue`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `web/src/stores/workflowStore.ts`
- `src-tauri/src/commands_execution.rs`

### Useful optional files

- `REPO_Improvement/09_live/02_execution_workspace_ux_2026-04-09.md`
- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`

### Bundle-specific focus

- determine whether the plan defines a credible discrete live-mode surface without broad app-shell redesign
- verify which current shell owners actually control placement and mode entry

### Bundle-specific challenge focus

- look for stale shell-owner assumptions
- note that this repo does not currently have `web/src/components/AppShell.vue` or `web/src/router`; challenge the doc if it assumes those owners

## Bundle 10: broker position and order reconciliation

### Required files

- `AGENTS.md`
- `REPO_MAP.md`
- `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`
- `REPO_Improvement_Epics/01_epics/10_live_execution_broker_position_order_reconciliation_2026-04-11.md`
- `src-tauri/src/commands_execution.rs`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `REPO_Improvement/09_live/04_execution_order_position_review_2026-04-09.md`

### Useful optional files

- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`
- `src-tauri/src/state.rs`

### Bundle-specific focus

- verify the plan preserves provenance between desired action, optimistic local state, broker-confirmed position, and open orders
- check whether stale or partial-fill cases are specified clearly enough

### Bundle-specific challenge focus

- look for any summary layer that could collapse broker uncertainty into fake certainty
- look for hidden changes to current delta-submission behavior

## Bundle 11: secure credential storage

### Required files

- `AGENTS.md`
- `REPO_MAP.md`
- `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`
- `REPO_Improvement_Epics/01_epics/11_live_execution_secure_credential_storage_2026-04-11.md`
- `web/src/stores/executionStore.ts`
- `web/src/services/executionService.ts`
- `web/src/utils/core/storageKeys.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_data.rs`

### Useful optional files

- `REPO_Improvement/09_live/11_settings_secret_storage_2026-04-09.md`
- `REPO_Improvement/09_live/06_broker_connectivity_2026-04-09.md`

### Bundle-specific focus

- verify the doc defines one clear secret owner, one migration path, and one precedence rule
- check whether frontend "remember keys" remains supported without frontend secret ownership

### Bundle-specific challenge focus

- look for silent migration failure, secret leakage, ambiguous precedence, or raw-secret overexposure back to the frontend

## Bundle 12: chart and account review completion

### Required files

- `AGENTS.md`
- `REPO_MAP.md`
- `REPO_Improvement_Epics/01_epics/05_live_execution_productization_2026-04-10.md`
- `REPO_Improvement_Epics/01_epics/12_live_execution_chart_and_account_review_completion_2026-04-11.md`
- `web/src/components/ExecutionWorkspacePanel.vue`
- `web/src/services/executionService.ts`
- `web/src/stores/executionStore.ts`
- `web/src/services/analysisService.ts`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/lib.rs`

### Useful optional files

- `REPO_Improvement/09_live/12_out_execution_monitoring_2026-04-09.md`
- `REPO_Improvement/09_live/04_execution_order_position_review_2026-04-09.md`

### Bundle-specific focus

- determine whether the doc cleanly separates truthful account review from the still-incomplete execution chart
- verify that chart overlays, markers, and account fields are grounded in explicit provenance

### Bundle-specific challenge focus

- look for any path where the chart becomes prettier but less truthful
- look for assumptions that the execution review surface already has a completed charting contract

## Recommended parallel run order

If you want the fastest broad sweep, start these in parallel first:

1. Bundle 11: secure credential storage
2. Bundle 12: chart and account review completion
3. Bundle 06: operator safety controls
4. Bundle 10: broker position and order reconciliation
5. Bundle 08: recovery and reconnect
6. Bundle 07: session audit trail
7. Bundle 09: discrete surface promotion
8. Bundle 05: live execution productization umbrella

Reason:

- 11 and 12 are the most likely to catch still-missing live-mode hardening details
- 06 and 10 test operator truth and broker-state semantics
- 08 and 07 test recovery and evidence posture
- 09 should be reviewed after the more safety-critical bundles exist in the reviewer context
- 05 umbrella review benefits from the reviewer having already thought through the narrower child plans

If you want zero cross-bundle serial dependence, ignore that suggested order and run all first-pass prompts together anyway. The packets are self-contained enough to support that.

## What to do with the outputs

Do not treat the external model's output as implementation instructions.

Use it as:

- a plan-quality audit
- a missing-detail detector
- a stale-assumption detector
- a test and rollout gap detector

Then validate every proposed change against the local repo before editing the implementation plans.
