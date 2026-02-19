# Stage 4 + Live Runner Implementation Tracker

Last updated: 2026-02-17
Scope: `ta_desktop_demo` point 2 and point 4 implementation.

## Objective

Implement two objectively useful upgrades without removing existing features:

1. Stage 4 policy upgrades:
- ML uncertainty per candidate
- blended final rank score
- decision buckets (`accept`, `review`, `reject`)

2. Continuous live execution orchestration:
- start/stop/status runner commands
- idempotent bar-triggered execution on Polygon live bars
- runtime status and event feedback in UI

## Checklist

- [x] Add Stage 4 request fields for blend weights and bucket thresholds.
- [x] Add Stage 4 result fields for bucket counts and policy settings used.
- [x] Compute ML uncertainty and blended final score in Stage 4 backend.
- [x] Attach `ml_uncertainty`, `final_rank_score`, and `decision_bucket` to selected groups.
- [x] Update Stage 4 frontend controls/output for new policy fields.
- [x] Persist new Stage 4 UI settings in local storage.
- [x] Add backend live runner state and status model.
- [x] Add commands: start live runner, stop live runner, runner status.
- [x] Reuse existing execution logic for runner-triggered actions.
- [x] Add idempotent bar processing guard (`last_bar_open_unix_ms`, inflight guard).
- [x] Feed Polygon live bars into live runner trigger path.
- [x] Add frontend controls and status for auto-runner.
- [x] Add frontend listener for runner event stream and output updates.
- [x] Compile checks pass for `ta_app_core` and `ta_desktop_demo_app`.
- [x] Targeted tests pass for modified backend components.

## Verification run

- `cargo check -p ta_desktop_demo_app`
- `cargo check -p ta_desktop_demo_app --features cuda,cuda-backtest-kernel`
- `cargo test -p ta_desktop_demo_app commands_execution::tests:: -- --nocapture`
- `node --check dist/app.js`
