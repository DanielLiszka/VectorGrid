# Broker Connectivity Plan

Last updated: 2026-04-09

## Purpose

Define the current broker-connectivity contract so Alpaca-only connectivity checks, snapshot behavior, credential posture, and broker-confirmed state remain explicit and distinct from optimistic UI state.

## Scope Boundaries

In scope:

- Alpaca connectivity checks
- broker snapshot/account/orders/position refresh
- live execution and runner connectivity prerequisites
- credential posture as it affects broker connectivity

Out of scope:

- adding new brokers
- options workflows
- full account-management UX

## Current User-Facing Behavior

The shipped broker path is Alpaca-only. Current behavior means:

- `Test account` checks broker connectivity
- refresh/snapshot review reads account, orders, and position state
- non-dry-run one-shot execution and runner execution depend on Alpaca connectivity
- broker-confirmed state is distinct from any optimistic local order row inserted before refresh
- only `DoubleMa` and `CompositeCpu` are currently supported for broker-backed execution

## Current Code Surface

- `src-tauri/src/commands_execution.rs`
  - Alpaca client construction, connectivity checks, snapshots, one-shot execution, and runner order submission
- `src-tauri/src/lib.rs`
  - Tauri registration of the execution command surface
- `web/src/services/executionService.ts`
  - frontend orchestration of connectivity and snapshot flows
- `web/src/stores/executionStore.ts`
  - execution mode, blocked reasons, and broker-facing posture state

## Current Verification Surface

- `web/src/services/executionService.test.ts`
- `web/src/stores/executionStore.actions.test.ts`
- targeted execution tests in `src-tauri/src/commands_execution.rs`

## Strengths Already Present

- broker connectivity is already centralized in the backend instead of being guessed in the frontend
- `dry_run=true` clearly avoids broker credential requirements
- paper and live broker modes already map to different Alpaca environments
- snapshot review and connectivity checks already exist as explicit actions instead of hidden side effects

## Weaknesses / Failure Modes

- Alpaca error bodies can surface raw backend/provider responses, which is truthful but uneven in UX quality
- symbol-scoped snapshots are not the same as a full portfolio/account review
- credentials are browser-storage-backed today rather than OS secret-storage-backed
- optimistic rows can temporarily sit beside broker-confirmed state if the user does not refresh immediately

## Regressions To Avoid

- weakening the current dry-run versus broker-armed separation
- hiding the Alpaca-only nature of the current connectivity surface
- collapsing broker-confirmed and optimistic local state into one misleading review model
- broadening connectivity promises beyond the currently supported strategies and broker path
- allowing this surface to stay behaviorally “working” while contributor readability, diagnostics, accessibility where relevant, static-analysis cleanliness, or regression coverage degrades in a public-facing repo

## External Research Summary

- [Working with /orders](https://docs.alpaca.markets/docs/working-with-orders)
  - relevant because broker connectivity is not just “can I reach the API,” it also governs order acceptance and state transitions
- [Working with /positions](https://docs.alpaca.markets/v1.3/docs/working-with-positions)
  - relevant because broker connectivity feeds position review and confirmed position state
- [Tauri State Management](https://v2.tauri.app/es/develop/state-management/)
  - relevant because broker connectivity and runner state remain authoritative on the Rust side
- [Tauri: Calling Rust from the Frontend](https://v2.tauri.app/develop/calling-rust/)
  - relevant because all broker-facing checks cross a structured command boundary
- [Alpaca Orders](https://docs.alpaca.markets/docs/orders-at-alpaca)
  - relevant because live execution, monitoring, and order-state handling sit behind provider contracts that must remain explicit in a public-facing trading app

## Proposed Improvement Approach

1. Document broker connectivity as an Alpaca-specific current contract, not a generic broker abstraction.
2. Separate connectivity checks, review snapshots, and order submission into distinct responsibilities.
3. Keep the difference between broker-confirmed and optimistic UI state explicit.
4. Document the current credential posture honestly, including browser-storage caveats.
5. Preserve the current supported-strategy limits in the connectivity doc.
6. Tie connectivity docs to snapshot and execution tests, not only one-shot execution tests.
7. Treat live/runtime code quality as part of the implementation scope: keep provider contracts, cancellation, observability, redaction, and operator safety semantics explicit, conservative, and testable.

Public-repo quality focus:
- keep touched-file ownership, adjacent-doc sync, and acceptance evidence explicit
- preserve contributor readability, diagnostics, and static-analysis health alongside user-visible behavior
- prefer additive hardening over broad cleanup-only refactors that widen blast radius
- update tests, fixtures, screenshots, workbooks, or artifacts in the same change when behavior shifts

## Checklist

- [ ] Document the current Alpaca-only broker path.
- [ ] Document paper/live endpoint behavior and `dry_run` credential rules.
- [ ] Document connectivity-check and snapshot-review commands separately.
- [ ] Document symbol-scoped snapshot limitations.
- [ ] Document optimistic-versus-broker-confirmed state distinctions.
- [ ] Document current credential-storage posture and its limits.
- [ ] Preserve service/store tests for connectivity and snapshot behavior.
- [ ] Preserve targeted backend tests for broker path normalization and execution gating.
- [ ] Identify the exact files, modules, fixtures, artifacts, and adjacent docs that must change together for this plan.
- [ ] Break the work into the smallest additive implementation slices that can be verified independently.
- [ ] Define the surface-specific code-quality expectations for naming, structure, diagnostics, comments or docs, accessibility where relevant, and contributor readability.
- [ ] Define the exact static-analysis, automated, artifact, and manual acceptance evidence required before this item can be closed.
- [ ] Record any required cross-doc sync updates in `README.md`, `REPO_MAP.md`, release notes, or adjacent implementation docs when behavior changes.
- [ ] Document the failure source, user-visible signal, persisted evidence, and redaction or secret-handling expectations for this live/runtime surface.

## Verification Plan

- run `npm --prefix web run test -- src/services/executionService.test.ts`
- run `npm --prefix web run test -- src/stores/executionStore.actions.test.ts`
- run targeted broker/execution tests in `src-tauri/src/commands_execution.rs`
- manually verify:
  - paper connectivity check
  - live connectivity check with current credentials model
  - broker snapshot refresh after a one-shot execution
- run the relevant repo quality gates for any touched surfaces, including `cargo check --workspace` or a justified narrower Rust target plus `npm --prefix web run lint` and `npm --prefix web run typecheck` where frontend or desktop code is touched
- run the relevant frontend, Rust, and desktop quality gates for touched live surfaces: `npm --prefix web run lint`, `npm --prefix web run typecheck`, `cargo check --workspace`, and any required desktop smoke or paper-trading/operator validation path
- capture the concrete acceptance evidence for closure, such as updated test output, screenshots, saved artifacts, report workbooks, or benchmark/profile artifacts as appropriate

## Rollout / Fallback / Compatibility Notes

- preserve the current Alpaca-only truth in docs and UX until another broker actually exists
- prefer additive credential hardening over pretending current storage already has OS-secret guarantees
- keep broker snapshot semantics narrow and explicit rather than overextending into portfolio-management claims
- keep changes additive, preserve existing functionality, and avoid broad cleanup-only refactors unless they provide objective user or maintainer value and include the necessary migration notes

## Related Docs

- `REPO_Improvement/09_live/01_execution_command_surface_2026-04-08.md`
- `REPO_Improvement/09_live/04_execution_order_position_review_2026-04-09.md`
- `REPO_Improvement/09_live/05_execution_safety_modes_2026-04-09.md`
- `REPO_Improvement/09_live/13_execution_service_store_2026-04-08.md`
- `REPO_Improvement/01_shell/13_settings_desktop_notifications_2026-04-08.md`
